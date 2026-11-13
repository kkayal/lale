//! Tests for compile-time control flow (`#when`, `#match`, `#switch`).
//!
//! These are the `#`-prefixed counterparts to the runtime `when`, `match`,
//! and `switch` statements. They are evaluated during compilation and strip
//! the non-taken branches from the IR.

use std::fs;
use std::process::Command;

/// Compiles and runs the given Lale code, returning (stdout, stderr, success).
fn run_lale(code: &str) -> (String, String, bool) {
  let dir = tempfile::tempdir().unwrap();
  let temp_file = dir.path().join("test.lale");
  fs::write(&temp_file, code).expect("Failed to write temp file");

  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", temp_file.to_str().unwrap()])
    .output()
    .expect("Failed to run lale compiler");

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();

  (stdout, stderr, success)
}

#[test]
fn test_ct_when_true_branch_taken() {
  let code = "#when true\n  write \"yes\"\n#end when\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("yes"), "Expected 'yes', got: {}", stdout);
}

#[test]
fn test_ct_when_false_branch_removed() {
  let code = "#when false\n  write \"no\"\n#end when\nwrite \"after\"\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    !stdout.contains("no"),
    "Expected 'no' to be absent, got: {}",
    stdout
  );
  assert!(
    stdout.contains("after"),
    "Expected 'after', got: {}",
    stdout
  );
}

#[test]
fn test_ct_match_first_true_arm_wins() {
  let code = "#match\n  #when #posix:\n    write \"posix\"\n  #when true:\n    write \"true\"\n  #else:\n    write \"else\"\n#end match\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    stdout.contains("posix"),
    "Expected 'posix', got: {}",
    stdout
  );
  assert!(
    !stdout.contains("true"),
    "Second arm should be skipped, got: {}",
    stdout
  );
}

#[test]
fn test_ct_match_else_fallback() {
  let code =
    "#match\n  #when false:\n    write \"false\"\n  #else:\n    write \"else\"\n#end match\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("else"), "Expected 'else', got: {}", stdout);
}

#[test]
fn test_ct_switch_int_match() {
  let code = "#switch 5\n  #case 3:\n    write \"three\"\n  #case 5:\n    write \"five\"\n  #default:\n    write \"default\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("five"), "Expected 'five', got: {}", stdout);
  assert!(
    !stdout.contains("three"),
    "Expected 'three' absent, got: {}",
    stdout
  );
}

#[test]
fn test_ct_switch_default_fallback() {
  let code =
    "#switch 42\n  #case 1:\n    write \"one\"\n  #default:\n    write \"default\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    stdout.contains("default"),
    "Expected 'default', got: {}",
    stdout
  );
}

#[test]
fn test_ct_switch_string_match() {
  let code = "#switch \"hello\"\n  #case \"world\":\n    write \"world\"\n  #case \"hello\":\n    write \"hello\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    stdout.contains("hello"),
    "Expected 'hello', got: {}",
    stdout
  );
  assert!(
    !stdout.contains("world"),
    "Expected 'world' absent, got: {}",
    stdout
  );
}
