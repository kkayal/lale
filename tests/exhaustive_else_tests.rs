//! Tests for the exhaustive `else` requirement on `if` statements.
//!
//! The Lale compiler requires every `if` statement to have an `else` branch.
//! The error message suggests using `else move on`, `else missing code`, or
//! `when` as alternatives.
//!
//! This test suite covers:
//! - Bare `if` without else → compile error with hint
//! - `if` with `else move on` → compiles and runs
//! - `if` with `else missing code` → compiles in debug mode
//! - `if` with `else if` chain → compile error (`else if` is not supported)
//! - Nested `if` scenarios
//! - `when` and `match` do not require else

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSER TESTS ====================

// The parser itself accepts `if` without `else` — the requirement is enforced
// during semantic analysis. The parser tests verify that various `if` forms
// parse correctly at the grammar level.

#[test]
fn test_if_without_else_parses() {
  // Bare `if` parses fine — the error is semantic, not syntactic
  let result = LaleParser::parse(Rule::if_stmt, "if true write \"ok\" end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_with_else_move_on_parses() {
  let result = LaleParser::parse(Rule::if_stmt, "if true write \"ok\" else move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_with_else_missing_code_parses() {
  let result = LaleParser::parse(
    Rule::if_stmt,
    "if true write \"ok\" else missing code end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_if_else_if_chain_no_final_else_parses() {
  let code = "if a doA() else if b doB() end if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_else_if_chain_with_final_else_parses() {
  let code = "if a doA() else if b doB() else doC() end if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== EXECUTION / SEMANTIC TESTS ====================

#[cfg(test)]
mod semantic_tests {
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
  fn test_bare_if_without_else_compile_error() {
    // Bare `if` without else should fail with a hint about move on, missing code, and when
    let code = r#"
var x as i32 = 0
if true
    x = 42 as i32
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Bare if without else should fail to compile. stdout: {}",
      stdout
    );
    let combined = format!("{}{}", stdout, stderr);
    assert!(
      combined.contains("else branch"),
      "Error should mention 'else branch', got combined output: {}",
      combined
    );
    assert!(
      combined.contains("move on"),
      "Error should suggest 'move on', got combined output: {}",
      combined
    );
    assert!(
      combined.contains("missing code"),
      "Error should suggest 'missing code', got combined output: {}",
      combined
    );
    assert!(
      combined.contains("when"),
      "Error should suggest 'when', got combined output: {}",
      combined
    );
  }

  #[test]
  fn test_if_with_else_move_on_compiles_and_runs() {
    // `if` with `else move on` should compile and execute normally
    let code = r#"
var x as i32 = 0
if true
    x = 42 as i32
else
    move on
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_if_with_else_missing_code_compiles_in_debug() {
    // `if` with `else missing code` should compile in debug mode (default)
    let code = r#"
var x as i32 = 0
if true
    x = 42 as i32
else
    missing code
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      success,
      "Expected success in debug mode, stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout, got: {}",
      stdout
    );
    // The else branch isn't taken (condition is true), so no warning printed
  }

  #[test]
  fn test_else_if_chain_no_final_else_compile_error() {
    // `if` with `else if` chain → compile error because `else if` is no longer supported
    let code = r#"
var x as i32 = 0
if false
    x = 1 as i32
else if false
    x = 2 as i32
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "else if should be a compile error. stdout: {}",
      stdout
    );
    let combined = format!("{}{}", stdout, stderr);
    assert!(
      combined.contains("not supported"),
      "Error should mention \"not supported\", got combined output: {}",
      combined
    );
  }

  #[test]
  fn test_else_if_chain_with_final_else_errors() {
    // `if` with `else if` chain + final `else` → compile error because `else if` is not supported
    let code = r#"
var x as i32 = 0
if false
    x = 1 as i32
else if false
    x = 2 as i32
else
    x = 42 as i32
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "else if should be a compile error. stdout: {}",
      stdout
    );
    let combined = format!("{}{}", stdout, stderr);
    assert!(
      combined.contains("not supported"),
      "Error should mention \"not supported\", got combined output: {}",
      combined
    );
  }

  #[test]
  fn test_nested_inner_if_without_else_error() {
    // Nested `if` where only the inner `if` has no else → error on inner if
    let code = r#"
var x as i32 = 0
if true
    if true
        x = 42 as i32
    end if
else
    x = 99 as i32
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Nested inner if without else should fail. stdout: {}",
      stdout
    );
    let combined = format!("{}{}", stdout, stderr);
    assert!(
      combined.contains("else branch"),
      "Error should mention 'else branch' for inner if, got combined output: {}",
      combined
    );
  }

  #[test]
  fn test_nested_both_have_else_compiles() {
    // Nested `if` where both inner and outer have `else` → compiles
    let code = r#"
var x as i32 = 0
if true
    if true
        x = 42 as i32
    else
        x = 99 as i32
    end if
else
    x = 99 as i32
end if
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout from nested if, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_does_not_require_else() {
    // `when` does not require else — compiles without error
    let code = r#"
var x as i32 = 0
when true
    x = 42 as i32
end when
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      success,
      "when should compile without else, stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout from when, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_without_default_compiles() {
    // `match` does not require a default arm — compiles without error
    let code = r#"
var x as i32 = 0
match
    when true:
        x = 42 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      success,
      "match without default should compile, stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout from match without default, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_with_default_compiles() {
    // `match` with a default arm also compiles
    let code = r#"
var x as i32 = 0
match
    when false:
        x = 99 as i32
    else:
        x = 42 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      success,
      "match with default should compile, stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout from match default, got: {}",
      stdout
    );
  }
}
