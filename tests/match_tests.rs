//! Tests for the `match` statement in the Lale language.
//!
//! The `match` statement evaluates guard conditions in order and runs
//! the body of the first matching arm, then jumps to `end match`.
//! An `else` arm catches anything not matched.
//!
//! Syntax:
//! ```lale
//! match
//!     when <condition>:
//!         <body>
//!     when <condition>:
//!         <body>
//!     else:
//!         <body>
//! end match
//! ```
//!
//! This test suite covers:
//! - Basic match statements (parser-only)
//! - Multiple arms and `else`
//! - Logical/negation operators in guards
//! - Empty body, move on, missing code
//! - Execution tests (runtime behavior via the compiler binary)
//! - Short-circuit evaluation, nesting, multiple statements per arm

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSER TESTS ====================

// --- Test 1: Minimal single-arm match ---

#[test]
fn test_match_minimal_single_arm() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: write \"ok\" end match");
  assert!(result.is_ok());
}

// --- Test 2: Two arms ---

#[test]
fn test_match_two_arms() {
  let result = LaleParser::parse(
    Rule::match_stmt,
    "match when x > 0: write \"pos\" when x < 0: write \"neg\" end match",
  );
  assert!(result.is_ok());
}

// --- Test 3: With else arm ---

#[test]
fn test_match_with_default() {
  let result = LaleParser::parse(
    Rule::match_stmt,
    "match when x > 0: write \"pos\" else: write \"zero\" end match",
  );
  assert!(result.is_ok());
}

// --- Test 4: Multiple conditions (three arms) ---

#[test]
fn test_match_multiple_conditions() {
  let code = "match when a: doA() when b: doB() when c: doC() end match";
  let result = LaleParser::parse(Rule::match_stmt, code);
  assert!(result.is_ok());
}

// --- Test 5: Compound logical in guard ---

#[test]
fn test_match_compound_logical_guard() {
  let result = LaleParser::parse(
    Rule::match_stmt,
    "match when x > 0 and x < 10: write \"in range\" end match",
  );
  assert!(result.is_ok());
}

// --- Test 6: Not in guard ---

#[test]
fn test_match_not_in_guard() {
  let result = LaleParser::parse(
    Rule::match_stmt,
    "match when not x > 0: write \"non-positive\" end match",
  );
  assert!(result.is_ok());
}

// --- Test 7: Empty body in arm ---

#[test]
fn test_match_empty_body_rejected() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: end match");
  assert!(
    result.is_err(),
    "match arm without any statement must fail to parse"
  );
}

#[test]
fn test_match_default_empty_body_rejected() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: move on else: end match");
  assert!(
    result.is_err(),
    "match default without any statement must fail to parse"
  );
}

#[test]
fn test_match_empty_rejected() {
  let result = LaleParser::parse(Rule::match_stmt, "match end match");
  assert!(result.is_err(), "match without any arm must fail to parse");
}

// --- Test 8: move on in body ---

#[test]
fn test_match_move_on_in_body() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: move on end match");
  assert!(result.is_ok());
}

// --- Additional parser tests for edge cases ---

#[test]
fn test_match_complex_guard() {
  let code = "match when x > 0 and x < 100 and not isLocked: access() end match";
  let result = LaleParser::parse(Rule::match_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_match_multi_line() {
  let code = "match\n    when true:\n        write \"ok\"\nend match";
  let result = LaleParser::parse(Rule::match_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_match_multi_line_with_default() {
  let code =
    "match\n    when x > 0:\n        write \"pos\"\n    else:\n        write \"other\"\nend match";
  let result = LaleParser::parse(Rule::match_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_match_with_not_and_and() {
  let result = LaleParser::parse(
    Rule::match_stmt,
    "match when not x > 0 and y < 10: write \"ok\" end match",
  );
  assert!(result.is_ok());
}

// --- Negative parser tests ---

#[test]
fn test_match_without_end_match_fails() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: write \"ok\"\n");
  assert!(
    result.is_err(),
    "match without end match should be a parser error"
  );
}

#[test]
fn test_match_missing_case_keyword_fails() {
  let result = LaleParser::parse(Rule::match_stmt, "match true write \"ok\" end match");
  assert!(
    result.is_err(),
    "match missing when keyword should be a parser error"
  );
}

#[test]
fn test_match_missing_end_keyword_fails() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: end");
  assert!(
    result.is_err(),
    "match with just 'end' (missing 'match') should fail"
  );
}

#[test]
fn test_match_wrong_close_keyword_fails() {
  let result = LaleParser::parse(Rule::match_stmt, "match when true: end when");
  assert!(
    result.is_err(),
    "match closed with end when should be a parser error"
  );
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

  // --- Test 9: First matching arm runs (true condition) ---

  #[test]
  fn test_match_first_arm_runs() {
    let code = r#"
var flag as bool = false
match
    when true:
        flag = true
    when not false:
        flag = false
end match
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 10: Second arm runs when first is false ---

  #[test]
  fn test_match_second_arm_runs() {
    let code = r#"
var flag as bool = false
match
    when false:
        flag = false
    when true:
        flag = true
end match
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 11: else arm runs when no when arm matches ---

  #[test]
  fn test_match_default_runs() {
    let code = r#"
var flag as bool = false
match
    when false:
        flag = false
    else:
        flag = true
end match
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 12: Short-circuit — later conditions not evaluated after match ---

  #[test]
  fn test_match_short_circuit() {
    let code = r#"
var x as i32 = 0
match
    when true:
        x = 1 as i32
    when not false:
        x = 2 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("1"),
      "Expected '1' in stdout, got: {}",
      stdout
    );
    // Ensure the second arm did NOT execute
    assert!(
      !stdout.contains("2"),
      "Second arm should not have executed, got: {}",
      stdout
    );
  }

  // --- Test 13: move on in match arm body ---

  #[test]
  fn test_match_move_on_in_arm_body() {
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
      "Expected '42' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 14: missing code in match arm body (debug mode warns) ---

  #[test]
  fn test_match_missing_code_in_arm() {
    let code = r#"
var x as i32 = 1
match
    when true:
        missing code
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    // Program should still run; missing code is a warning, not a fatal error
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("1"),
      "Expected '1' in stdout, got: {}",
      stdout
    );
    // In debug mode, stderr should contain a warning
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  // --- Test 15: Nested match inside function ---

  #[test]
  fn test_match_nested_inside_function() {
    let code = r#"
fn test() returns i32
    var result as i32 = 0
    match
        when true:
            match
                when true:
                    result = 42 as i32
            end match
    end match
    return result
end fn
write test()
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 16: Multiple statements per arm ---

  #[test]
  fn test_match_multiple_statements_per_arm() {
    let code = r#"
var a as i32 = 0
var b as i32 = 0
match
    when true:
        a = 10 as i32
        b = 20 as i32
end match
write a
write b
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("10"),
      "Expected '10' in stdout, got: {}",
      stdout
    );
    assert!(
      stdout.contains("20"),
      "Expected '20' in stdout, got: {}",
      stdout
    );
  }

  // --- Additional execution tests ---

  #[test]
  fn test_match_with_comparison_guard() {
    let code = r#"
var x as i32 = 5
var result as bool = false
match
    when x > 0:
        result = true
    else:
        result = false
end match
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_comparison_false_falls_through() {
    let code = r#"
var x as i32 = -5
var result as bool = false
match
    when x > 0:
        result = false
    else:
        result = true
end match
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_with_not_guard() {
    let code = r#"
var flag as bool = false
match
    when not true:
        flag = true
    else:
        flag = false
end match
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("false"),
      "Expected 'false' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_empty_body_rejected() {
    // Empty body is now a parser error — a match arm must contain a statement.
    let code = r#"
match
    when true:
end match
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Expected failure for empty match arm body, stdout={}, stderr={}",
      stdout, stderr
    );
  }

  #[test]
  fn test_match_default_short_circuits_past_arms() {
    // When no arm matches, else runs; later checks are not evaluated
    let code = r#"
var flag as bool = false
match
    when false:
        flag = false
    else:
        flag = true
end match
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_three_arms_middle_matches() {
    let code = r#"
var result as i32 = 0
match
    when false:
        result = 1 as i32
    when true:
        result = 2 as i32
    when not false:
        result = 3 as i32
end match
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("2"),
      "Expected '2' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_all_arms_false_no_default() {
    // When no arm matches and there's no else, body is skipped
    let code = r#"
var x as i32 = 0
match
    when false:
        x = 1 as i32
    when not true:
        x = 2 as i32
end match
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("0"),
      "Expected '0' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_match_with_write_in_body() {
    let code = r#"
match
    when true:
        write "matched"
    else:
        write "default"
end match
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("matched"),
      "Expected 'matched' in stdout, got: {}",
      stdout
    );
  }
}
