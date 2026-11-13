//! Content-addressed cache for the lowered prelude (builtins + standard library).
//!
//! The prelude — `builtins.lale` plus the selected standard-library modules — is
//! identical for every compilation that uses the same compiler binary, the same
//! standard-library sources, and the same options. Lowering it costs more than
//! executing a small user program, so the resulting IR is stored on disk and
//! re-parsed by later invocations. See LDP-0004.
//!
//! The cache is strictly an optimisation:
//!
//! - Every read, parse, key, or validation failure falls back to rebuilding the
//!   prelude; a cache problem never fails a compilation.
//! - Entries are written atomically (temporary file, then rename), so concurrent
//!   compilers never observe a partial entry.
//! - The key covers all inputs that can change the lowered IR: the compiler
//!   binary, the builtins and standard-library sources, the standard-library
//!   level, the target operating system, and overflow-checking mode.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::builtins::BUILTINS_SOURCE;
use crate::config::StdlibLevel;
use crate::error::CompileResult;
use crate::ir::{IR_FORMAT_MAJOR, Module, parse_module, print_module};
use crate::ir_gen::IrGenerator;
use crate::semantic_analysis::module_resolver::resolve_stdlib_path;

/// Identifies the cache layout. Bumping it invalidates every existing entry.
const CACHE_FORMAT: &str = "lale-prelude-v1";

/// Build the prelude module for `module_name`.
///
/// The prelude contains the compiler builtins and, when `include_stdlib` is
/// true, the selected standard-library modules. On a cache hit the module is
/// re-parsed from the cached IR instead of being lowered again.
pub fn load_prelude(
  module_name: &str,
  stdlib: StdlibLevel,
  checked_overflow: bool,
  include_stdlib: bool,
  use_cache: bool,
) -> CompileResult<Module> {
  let key = if use_cache {
    cache_key(stdlib, checked_overflow, include_stdlib)
  } else {
    None
  };

  if let Some(ref key) = key
    && let Some(module) = read_cache(key)
  {
    return Ok(module);
  }

  let mut module = Module::new(module_name);
  IrGenerator::load_builtins_into_module(&mut module, checked_overflow)?;
  if include_stdlib {
    IrGenerator::load_stdlib_into_module(&mut module, stdlib, checked_overflow)?;
  }

  if let Some(ref key) = key {
    write_cache(key, &module);
  }

  Ok(module)
}

/// Compute a content hash over every input that can change the prelude IR.
///
/// Returns `None` when the cache is disabled or when an input cannot be read;
/// callers then build the prelude without consulting the cache.
fn cache_key(stdlib: StdlibLevel, checked_overflow: bool, include_stdlib: bool) -> Option<String> {
  let mut hasher = Sha256::new();

  update_str(&mut hasher, CACHE_FORMAT);
  update_str(&mut hasher, env!("CARGO_PKG_VERSION"));
  update_str(&mut hasher, stdlib_tag(stdlib));
  update_str(&mut hasher, std::env::consts::OS);
  update_bytes(
    &mut hasher,
    &[u8::from(checked_overflow), u8::from(include_stdlib)],
  );
  update_bytes(&mut hasher, &IR_FORMAT_MAJOR.to_le_bytes());
  update_str(&mut hasher, BUILTINS_SOURCE);

  if include_stdlib {
    let stdlib_dir = resolve_stdlib_path();
    let mut files = Vec::new();
    let Ok(()) = collect_lale_files(&stdlib_dir, &mut files) else {
      return None;
    };
    files.sort();

    for path in &files {
      let relative = match path.strip_prefix(&stdlib_dir) {
        Ok(relative) => relative,
        Err(_) => path.as_path(),
      };
      update_str(&mut hasher, &relative.to_string_lossy());
      let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return None,
      };
      update_bytes(&mut hasher, &bytes);
    }
  }

  // A compiler rebuilt from different sources must never reuse an entry: the IR
  // generator lives inside the binary, and the crate version does not change on
  // every development build.
  if let Ok(executable) = std::env::current_exe()
    && let Ok(metadata) = std::fs::metadata(executable)
  {
    update_bytes(&mut hasher, &metadata.len().to_le_bytes());
    if let Ok(modified) = metadata.modified()
      && let Ok(age) = modified.duration_since(std::time::UNIX_EPOCH)
    {
      update_bytes(&mut hasher, &age.as_nanos().to_le_bytes());
    }
  }

  let digest = hasher.finalize();
  Some(
    digest
      .iter()
      .map(|byte| format!("{byte:02x}"))
      .collect::<String>(),
  )
}

fn stdlib_tag(level: StdlibLevel) -> &'static str {
  match level {
    StdlibLevel::None => "none",
    StdlibLevel::Core => "core",
    StdlibLevel::Full => "full",
  }
}

/// Hash a length-prefixed byte string so that concatenated inputs cannot alias.
fn update_bytes(hasher: &mut Sha256, bytes: &[u8]) {
  hasher.update((bytes.len() as u64).to_le_bytes());
  hasher.update(bytes);
}

fn update_str(hasher: &mut Sha256, value: &str) {
  update_bytes(hasher, value.as_bytes());
}

/// Collect every `.lale` file below `dir`, recursively.
fn collect_lale_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
  for entry in std::fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      collect_lale_files(&path, files)?;
    } else if path.extension().and_then(|extension| extension.to_str()) == Some("lale") {
      files.push(path);
    }
  }
  Ok(())
}

/// Directory holding cache entries, or `None` when caching is disabled.
fn cache_dir() -> Option<PathBuf> {
  if std::env::var_os("LALE_NO_STDLIB_CACHE").is_some() {
    return None;
  }

  let root = if let Some(dir) = std::env::var_os("LALE_CACHE_DIR") {
    PathBuf::from(dir)
  } else if cfg!(target_os = "macos") {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library").join("Caches"))?
  } else if cfg!(target_os = "windows") {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?
  } else {
    std::env::var_os("XDG_CACHE_HOME")
      .map(PathBuf::from)
      .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?
  };

  Some(root.join("lale").join("stdlib").join(CACHE_FORMAT))
}

fn entry_path(key: &str) -> Option<PathBuf> {
  Some(cache_dir()?.join(format!("{key}.ir")))
}

fn read_cache(key: &str) -> Option<Module> {
  let path = entry_path(key)?;
  let Ok(text) = std::fs::read_to_string(path) else {
    return None;
  };
  let Ok(module) = parse_module(&text) else {
    return None;
  };
  Some(module)
}

fn write_cache(key: &str, module: &Module) {
  let Some(directory) = cache_dir() else {
    return;
  };
  if std::fs::create_dir_all(&directory).is_err() {
    return;
  }

  let text = print_module(module);
  let final_path = directory.join(format!("{key}.ir"));
  let temporary_path = directory.join(format!("{key}.ir.tmp.{}", std::process::id()));

  if std::fs::write(&temporary_path, text).is_err() {
    return;
  }
  if std::fs::rename(&temporary_path, &final_path).is_err() {
    // Best effort: if cleanup also fails, a stale temporary file remains. It
    // shares this process's PID, so a later run never picks it up.
    let _cleanup = std::fs::remove_file(&temporary_path);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn key_is_stable_and_level_sensitive() {
    let full = cache_key(StdlibLevel::Full, true, true);
    let again = cache_key(StdlibLevel::Full, true, true);
    assert_eq!(full, again, "the key must be deterministic");

    let core = cache_key(StdlibLevel::Core, true, true);
    assert_ne!(full, core, "the standard-library level must change the key");

    let unchecked = cache_key(StdlibLevel::Full, false, true);
    assert_ne!(
      full, unchecked,
      "overflow-checking mode must change the key"
    );

    let no_stdlib = cache_key(StdlibLevel::Full, true, false);
    assert_ne!(
      full, no_stdlib,
      "including the standard library must change the key"
    );
  }
}
