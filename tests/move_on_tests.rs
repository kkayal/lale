//! Tests for the `move on` statement in the Lale language.
//!
//! `move on` is an intentional no-op placeholder. It generates zero
//! instructions and can appear anywhere a statement is valid.
//!
//! This test suite covers:
//! - Parser: standalone and with comments
//! - Execution: inside if/else, when, match, switch (case + default),
//!   at top level, in functions, and in loops

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSER TESTS ====================

#[test]
fn test_move_on_standalone() {
  let result = LaleParser::parse(Rule::move_on_stmt, "move on");
  assert!(result.is_ok());
}

#[test]
fn test_move_on_with_leading_comment() {
  let result = LaleParser::parse(Rule::move_on_stmt, "// Nothing to do here\nmove on");
  assert!(result.is_ok());
}

#[test]
fn test_move_on_with_inline_comment() {
  let result = LaleParser::parse(Rule::move_on_stmt, "move on // skip this branch");
  assert!(result.is_ok());
}

// ==================== EXECUTION TESTS ====================

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
  fn test_move_on_in_if_else_branch() {
    // `else move on` should be a no-op; body after if still executes
    let code = r#"
var x as i32 = 0
if false
    x = 99 as i32
else
    move on
end if
x = 42 as i32
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after else move on, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_when_body() {
    // `move on` inside when body is a no-op; code after when still executes
    let code = r#"
var x as i32 = 0
when true
    move on
end when
x = 42 as i32
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in when, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_match_arm() {
    // `move on` inside a match arm body is a no-op; body after arm still runs
    let code = r#"
var x as i32 = 0
match
    when true:
        move on
        x = 42 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in match arm, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_switch_case() {
    // `move on` inside a switch case body is a no-op
    let code = r#"
enum Val One Two end enum
var v = Val->One
var x as i32 = 0
switch v
    case One:
        move on
        x = 42 as i32
    case Two: move on
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in switch case, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_switch_default() {
    // `move on` inside a switch default body is a no-op
    let code = r#"
enum Val One Two end enum
var v = Val->Two
var x as i32 = 0
switch v
    case One: x = 99 as i32
    default:
        move on
        x = 42 as i32
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in switch default, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_at_top_level() {
    // `move on` at top level is a no-op; program completes normally
    let code = r#"
move on
var x as i32 = 42
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after top-level move on, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_function_body() {
    // `move on` inside a function body is a no-op
    let code = r#"
fn foo() returns i32
    move on
    return 42
end fn
write foo()
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in function, got: {}",
      stdout
    );
  }

  #[test]
  fn test_move_on_in_loop_body() {
    // `move on` inside a loop body is a no-op
    let code = r#"
var count as i32 = 0
loop
    move on
    count = 42 as i32
    exit loop
end loop
write count
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on in loop, got: {}",
      stdout
    );
  }
}
