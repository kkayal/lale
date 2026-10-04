//! Compiler Options
//!
//! Internal configuration for the compiler.
//! This module defines the data structures passed to semantic analysis,
//! not CLI argument parsing (which is handled in main.rs).
//!
//! # Backends
//!
//! - **Interpreter**: Runtime execution using the built-in structured IR interpreter.
//!   Portable and flexible. The only active backend.
//!
//! The `Backend` enum is designed to accommodate future pluggable backends
//! (e.g., LLVM, Cranelift). To add a backend:
//! 1. Add a variant to `Backend`
//! 2. Implement the codegen module
//! 3. Add CLI commands in `main.rs`

use std::io::IsTerminal;

/// Code generation backend selection.
/// Currently only the interpreter is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
  /// Interpreter backend — structured IR execution
  #[default]
  Interpreter,
}

impl Backend {
  /// Returns a human-readable name for this backend.
  pub fn name(&self) -> &'static str {
    "Interpreter"
  }
}

/// Standard library inclusion level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdlibLevel {
  /// No standard library modules (only compiler built-ins)
  None,
  /// Core standard library modules only (core.lale but not full.lale)
  Core,
  /// Full standard library (core.lale and full.lale, default)
  #[default]
  Full,
}

impl StdlibLevel {
  /// Returns true if stdlib is disabled.
  pub fn is_none(&self) -> bool {
    *self == StdlibLevel::None
  }

  /// Returns true if stdlib is enabled (Core or Full).
  pub fn is_enabled(&self) -> bool {
    *self != StdlibLevel::None
  }

  /// Parse a string to a StdlibLevel.
  pub fn parse(s: &str) -> Result<Self, String> {
    match s.to_lowercase().as_str() {
      "none" => Ok(StdlibLevel::None),
      "core" => Ok(StdlibLevel::Core),
      "full" => Ok(StdlibLevel::Full),
      _ => Err(format!(
        "Invalid stdlib level '{}'. Valid options: none, core, full",
        s
      )),
    }
  }
}

/// Whether terminal output should be colored.
///
/// Resolved from the `--color` CLI flag. `Auto` performs TTY detection so that
/// piped output (logs, CI, redirection) is not polluted with ANSI escapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorChoice {
  /// Color only when the target stream is a terminal.
  #[default]
  Auto,
  /// Always emit ANSI color, regardless of the target stream.
  Always,
  /// Never emit ANSI color.
  Never,
}

impl ColorChoice {
  /// Parse a CLI value. Accepts the lowercase names `auto`, `always`, `never`.
  pub fn parse(s: &str) -> Result<Self, String> {
    match s.to_lowercase().as_str() {
      "auto" => Ok(ColorChoice::Auto),
      "always" => Ok(ColorChoice::Always),
      "never" => Ok(ColorChoice::Never),
      _ => Err(format!(
        "Invalid color mode '{}'. Valid options: auto, always, never",
        s
      )),
    }
  }

  /// The canonical CLI spelling for this choice (used when forwarding to a
  /// backend subprocess).
  pub fn as_str(&self) -> &'static str {
    match self {
      ColorChoice::Auto => "auto",
      ColorChoice::Always => "always",
      ColorChoice::Never => "never",
    }
  }

  /// Resolve the choice to a concrete boolean. `Auto` checks whether stderr is
  /// a terminal (runtime errors and compiler diagnostics go to stderr).
  pub fn use_color(&self) -> bool {
    match self {
      ColorChoice::Auto => std::io::stderr().is_terminal(),
      ColorChoice::Always => true,
      ColorChoice::Never => false,
    }
  }
}

/// Compiler options that affect semantic analysis.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
  /// Selected code generation backend (interpreter only)
  pub backend: Backend,
  /// Source file name (for error messages)
  pub source_file: Option<String>,
  /// Standard library inclusion level
  pub stdlib: StdlibLevel,
  /// Debug mode: true by default, false when --release is set.
  /// Controls the `#debug` compile-time constant and debug-level diagnostics.
  pub is_debug: bool,
  /// Whether integer arithmetic traps on overflow. True by default; disabled
  /// only with `--unchecked_overflow`.
  pub checked_overflow: bool,
  /// Execution mode: false = run (default), true = test.
  /// Controls the `#mode` compile-time constant and test-suite gating.
  pub test_mode: bool,
}

impl Default for CompilerOptions {
  fn default() -> Self {
    Self {
      backend: Backend::default(),
      source_file: None,
      stdlib: StdlibLevel::default(),
      is_debug: true,
      checked_overflow: true,
      test_mode: false,
    }
  }
}

impl CompilerOptions {
  /// Create default compiler options.
  pub fn new() -> Self {
    Self::default()
  }

  /// Set the source file name.
  pub fn source_file(mut self, file: impl Into<String>) -> Self {
    self.source_file = Some(file.into());
    self
  }

  /// Set the standard library inclusion level.
  pub fn stdlib(mut self, level: StdlibLevel) -> Self {
    self.stdlib = level;
    self
  }

  /// Set debug mode (true by default, false with --release).
  pub fn debug(mut self, is_debug: bool) -> Self {
    self.is_debug = is_debug;
    self
  }

  /// Set whether integer overflow traps (true by default).
  pub fn checked_overflow(mut self, checked_overflow: bool) -> Self {
    self.checked_overflow = checked_overflow;
    self
  }

  /// Set execution mode (false = run, true = test).
  pub fn test_mode(mut self, test_mode: bool) -> Self {
    self.test_mode = test_mode;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_backend() {
    let options = CompilerOptions::default();
    assert_eq!(options.backend, Backend::Interpreter);
    assert!(options.is_debug, "#debug should default to true");
    assert!(
      options.checked_overflow,
      "overflow trapping should default to true"
    );
  }

  #[test]
  fn test_default_stdlib() {
    let options = CompilerOptions::default();
    assert_eq!(options.stdlib, StdlibLevel::Full);
  }

  #[test]
  fn test_stdlib_is_enabled() {
    assert!(StdlibLevel::Core.is_enabled());
    assert!(StdlibLevel::Full.is_enabled());
    assert!(!StdlibLevel::None.is_enabled());
  }

  #[test]
  fn test_stdlib_is_none() {
    assert!(StdlibLevel::None.is_none());
    assert!(!StdlibLevel::Core.is_none());
    assert!(!StdlibLevel::Full.is_none());
  }
}
