//! Integration tests for the standard-library analysis cache (LDP-0004).
//!
//! Every test runs the compiler binary in a temporary `LALE_CACHE_DIR`, so the
//! developer's real cache is never touched and cold/warm states are controlled.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Run `lale run --no-color <args> -` with `code` on stdin and `cache` as the
/// cache directory. Returns `(stdout, stderr, success)`.
fn run_with_cache(cache: &Path, args: &[&str], code: &str) -> (String, String, bool) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "--no-color"])
    .args(args)
    .arg("-")
    .env("LALE_CACHE_DIR", cache)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("failed to spawn lale");

  child
    .stdin
    .take()
    .expect("failed to open stdin")
    .write_all(code.as_bytes())
    .expect("failed to write stdin");

  let output = child.wait_with_output().expect("failed to wait for lale");
  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status.success(),
  )
}

/// Every `.ir` file below `dir`, sorted.
fn cache_entries(dir: &Path) -> Vec<PathBuf> {
  fn walk(dir: &Path, entries: &mut Vec<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
      return;
    };
    for entry in read_dir.flatten() {
      let path = entry.path();
      if path.is_dir() {
        walk(&path, entries);
      } else if path.extension().and_then(|extension| extension.to_str()) == Some("ir") {
        entries.push(path);
      }
    }
  }

  let mut entries = Vec::new();
  walk(dir, &mut entries);
  entries.sort();
  entries
}

#[test]
fn cold_and_warm_runs_produce_identical_output() {
  let cache = tempfile::tempdir().expect("tempdir");
  let args = ["--print-ast", "--print-ir", "--print-symbols"];
  let code = "var arr as i64[3] = [1, 2, 3]\narr[1] = 100\nwrite arr[1]\n";

  let cold = run_with_cache(cache.path(), &args, code);
  assert!(cold.2, "cold run failed: {}", cold.1);
  assert_eq!(
    cache_entries(cache.path()).len(),
    1,
    "expected exactly one cache entry after a cold run"
  );

  let warm = run_with_cache(cache.path(), &args, code);
  assert_eq!(
    cold, warm,
    "a cached run must produce byte-identical output"
  );
}

#[test]
fn corrupt_entry_is_ignored_and_rebuilt() {
  let cache = tempfile::tempdir().expect("tempdir");
  let code = "write \"ok\"\n";

  let cold = run_with_cache(cache.path(), &[], code);
  assert!(cold.2, "cold run failed: {}", cold.1);

  let entries = cache_entries(cache.path());
  assert_eq!(entries.len(), 1, "expected one cache entry");
  std::fs::write(&entries[0], b"this is not an IR module").expect("corrupt the cache entry");

  let recovered = run_with_cache(cache.path(), &[], code);
  assert_eq!(
    cold, recovered,
    "a corrupt cache must not change a successful compilation"
  );
  assert_eq!(
    cache_entries(cache.path()).len(),
    1,
    "the entry must be rebuilt, not duplicated"
  );
}

#[test]
fn concurrent_cold_runs_produce_one_entry() {
  let cache = tempfile::tempdir().expect("tempdir");
  let code = "write \"ok\"\n";

  let cache_path = cache.path().to_path_buf();
  let handles: Vec<_> = (0..8)
    .map(|_| {
      let path = cache_path.clone();
      std::thread::spawn(move || run_with_cache(&path, &[], code))
    })
    .collect();

  for handle in handles {
    let (stdout, stderr, success) = handle.join().expect("worker thread");
    assert!(success, "concurrent run failed: {stderr}");
    assert_eq!(stdout.trim(), "ok");
  }

  assert_eq!(
    cache_entries(cache.path()).len(),
    1,
    "concurrent writers must leave exactly one well-formed entry"
  );
}

#[test]
fn no_cache_flag_writes_no_entry() {
  let cache = tempfile::tempdir().expect("tempdir");
  let (stdout, stderr, success) =
    run_with_cache(cache.path(), &["--no-stdlib-cache"], "write \"ok\"\n");

  assert!(success, "run failed: {stderr}");
  assert_eq!(stdout.trim(), "ok");
  assert!(
    cache_entries(cache.path()).is_empty(),
    "the opt-out must not write a cache entry"
  );
}

#[test]
fn changing_stdlib_source_invalidates_the_entry() {
  let cache = tempfile::tempdir().expect("tempdir");
  let home = tempfile::tempdir().expect("tempdir");

  // Copy the real standard library into a disposable LALE_HOME so the test can
  // modify one source file without touching the repository.
  let stdlib_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib/src");
  let copied = home.path().join("lib/std/src");
  std::fs::create_dir_all(&copied).expect("create copied stdlib directory");
  for entry in std::fs::read_dir(&stdlib_src)
    .expect("read stdlib directory")
    .flatten()
  {
    let path = entry.path();
    if path.extension().and_then(|extension| extension.to_str()) == Some("lale") {
      std::fs::copy(&path, copied.join(entry.file_name())).expect("copy stdlib file");
    }
  }

  let run = || {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
      .current_dir(env!("CARGO_MANIFEST_DIR"))
      .args(["run", "--no-color", "-"])
      .env("LALE_CACHE_DIR", cache.path())
      .env("LALE_HOME", home.path())
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .expect("failed to spawn lale");
    child
      .stdin
      .take()
      .expect("failed to open stdin")
      .write_all(b"write \"ok\"\n")
      .expect("failed to write stdin");
    child.wait_with_output().expect("failed to wait for lale")
  };

  assert!(run().status.success(), "first run failed");
  assert_eq!(cache_entries(cache.path()).len(), 1);

  let core = copied.join("core.lale");
  let mut text = std::fs::read_to_string(&core).expect("read core.lale");
  text.push('\n');
  std::fs::write(&core, text).expect("write core.lale");

  assert!(run().status.success(), "run after editing stdlib failed");
  assert_eq!(
    cache_entries(cache.path()).len(),
    2,
    "editing a standard-library source must invalidate the entry"
  );
}

#[test]
fn stdlib_levels_use_separate_entries() {
  let cache = tempfile::tempdir().expect("tempdir");
  let code = "write \"ok\"\n";

  let (_, stderr, success) = run_with_cache(cache.path(), &["--stdlib-level", "none"], code);
  assert!(success, "none-level run failed: {stderr}");

  let (_, stderr, success) = run_with_cache(cache.path(), &["--stdlib-level", "full"], code);
  assert!(success, "full-level run failed: {stderr}");

  assert_eq!(
    cache_entries(cache.path()).len(),
    2,
    "different standard-library levels must not share an entry"
  );
}

#[test]
fn disabled_cache_environment_variable_is_honoured() {
  let cache = tempfile::tempdir().expect("tempdir");
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "--no-color", "-"])
    .env("LALE_CACHE_DIR", cache.path())
    .env("LALE_NO_STDLIB_CACHE", "1")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("failed to spawn lale");

  child
    .stdin
    .take()
    .expect("failed to open stdin")
    .write_all(b"write \"ok\"\n")
    .expect("failed to write stdin");

  let output = child.wait_with_output().expect("failed to wait for lale");
  assert!(
    output.status.success(),
    "stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  assert!(
    cache_entries(cache.path()).is_empty(),
    "LALE_NO_STDLIB_CACHE must disable the cache"
  );
}
