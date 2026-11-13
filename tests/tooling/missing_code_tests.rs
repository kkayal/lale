//! Tests for the `missing code` statement in the Lale language.
//!
//! `missing code` is a deferred implementation placeholder. In debug mode
//! (the default), it prints a runtime warning to stderr and continues. In
//! release mode (`--release`), the compiler refuses to finish if any
//! `missing code` remains.
//!
//! This test suite covers:
//! - Parser: standalone
//! - Execution (debug mode): warning printed, program continues
//! - Execution (debug mode): inside if/else, when, match arm,
//!   match default, switch case, switch default
//! - Release mode: compilation failure with line number in error

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSER TESTS ====================

#[test]
fn test_missing_code_standalone() {
  let result = LaleParser::parse(Rule::missing_code_stmt, "missing code");
  assert!(result.is_ok());
}

// ==================== EXECUTION TESTS (DEBUG MODE) ====================

#[cfg(test)]
mod execution_tests {
  use std::fs;
  use std::process::Command;

  /// Helper: compiles and runs the given Lale code, returning (stdout, stderr, success).
  fn run_lale(code: &str) -> (String, String, bool) {
    let dir = tempfile::tempdir().unwrap();
    let temp_file = dir.path().join("test.lale");
    fs::write(&temp_file, code).expect("Failed to write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_lale"))
      .args(["run", temp_file.to_str().unwrap()])
      .output()
      .expect("Failed to run lale compiler");

    // tempdir auto-cleans up on drop

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
  }

  #[test]
  fn test_missing_code_prints_warning() {
    // In debug mode, `missing code` prints a warning to stderr
    let code = r#"
var x as i32 = 0
missing code
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      success,
      "Expected success in debug mode, stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("0"),
      "Expected '0' in stdout, got: {}",
      stdout
    );
    // In debug mode, stderr should contain a warning about missing code
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_program_continues() {
    // Program continues executing after a `missing code` statement
    let code = r#"
var x as i32 = 1
missing code
x = 42 as i32
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_if_else_branch() {
    // `else missing code` prints a warning but program continues
    let code = r#"
var x as i32 = 0
if false
    x = 99 as i32
else
    missing code
end if
x = 42 as i32
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after else missing code, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_when_body() {
    // `missing code` inside a when body prints a warning
    let code = r#"
var x as i32 = 0
when true
    missing code
end when
x = 42 as i32
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code in when, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_match_arm() {
    // `missing code` inside a match arm body prints a warning
    let code = r#"
var x as i32 = 0
match
    when true:
        missing code
        x = 42 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code in match arm, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_match_default() {
    // `missing code` inside a match default arm prints a warning
    let code = r#"
var x as i32 = 0
match
    when false:
        x = 99 as i32
    else:
        missing code
        x = 42 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code in match default, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_switch_case() {
    // `missing code` inside a switch case body prints a warning
    let code = r#"
enum Val One Two end enum
var v = Val.One
var x as i32 = 0
switch v
    case One:
        missing code
        x = 42 as i32
    case Two: move on
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code in switch case, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  #[test]
  fn test_missing_code_in_switch_default() {
    // `missing code` inside a switch default body prints a warning
    let code = r#"
enum Val One Two end enum
var v = Val.Two
var x as i32 = 0
switch v
    case One: x = 99 as i32
    default:
        missing code
        x = 42 as i32
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after missing code in switch default, got: {}",
      stdout
    );
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  // ==================== RELEASE MODE TESTS ====================

  #[test]
  fn test_missing_code_release_mode_fails() {
    // In release mode (--release), the compiler refuses to finish
    let code = r#"
var x as i32 = 42
missing code
write x
"#;
    let dir = tempfile::tempdir().unwrap();
    let temp_file = dir.path().join("test.lale");
    fs::write(&temp_file, code).expect("Failed to write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_lale"))
      .args(["run", temp_file.to_str().unwrap(), "--release"])
      .output()
      .expect("Failed to run lale compiler");

    assert!(
      !output.status.success(),
      "Release mode should reject missing code. stdout: {}\nstderr: {}",
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    );
  }

  #[test]
  fn test_missing_code_release_mode_error_mentions_line() {
    // The error message in release mode should mention the line number
    let code = r#"
var x as i32 = 42
missing code
write x
"#;
    let dir = tempfile::tempdir().unwrap();
    let temp_file = dir.path().join("test.lale");
    fs::write(&temp_file, code).expect("Failed to write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_lale"))
      .args(["run", temp_file.to_str().unwrap(), "--release"])
      .output()
      .expect("Failed to run lale compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
      stderr.contains("missing code"),
      "Error should mention 'missing code', got: {}",
      stderr
    );
    // The `missing code` is on line 3 of the source
    assert!(
      stderr.contains("3"),
      "Error should mention the line number, got: {}",
      stderr
    );
  }
}
