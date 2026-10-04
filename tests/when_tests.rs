use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for `when` statement rules in the Lale grammar.
///
/// The `when` statement is a one-sided action — no `else`, no `else if`.
/// If the condition is true, the body executes. If not, the program moves on.
///
/// Syntax:
/// ```lale
/// when <condition>
///     <body>
/// end when
/// ```
///
/// This test suite covers:
/// - Basic when statements (parser-only)
/// - When with conditions (comparisons, logical operators, function calls)
/// - When with body statements (single, multiple, return, exit program)
/// - Unicode operators in conditions
/// - Whitespace handling
/// - Negative tests (when without end when, when with else)
/// - Execution tests (verifying runtime behavior via the compiler binary)

// ==================== MINIMAL WHEN ====================

#[test]
fn test_when_minimal_true_literal() {
  let result = LaleParser::parse(Rule::when_stmt, "when true move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_minimal_false_literal() {
  let result = LaleParser::parse(Rule::when_stmt, "when false move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_variable_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when flag move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH COMPARISONS ====================

#[test]
fn test_when_comparison_greater() {
  let result = LaleParser::parse(Rule::when_stmt, "when x > 0 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_comparison_less() {
  let result = LaleParser::parse(Rule::when_stmt, "when count < max move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_comparison_greater_equal() {
  let result = LaleParser::parse(Rule::when_stmt, "when value >= threshold move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_comparison_less_equal() {
  let result = LaleParser::parse(Rule::when_stmt, "when index <= limit move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_equality() {
  let result = LaleParser::parse(Rule::when_stmt, "when status == 0 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_inequality() {
  let result = LaleParser::parse(Rule::when_stmt, "when error != 0 move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH NOT ====================

#[test]
fn test_when_not_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when not done move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_not_function_call() {
  let result = LaleParser::parse(Rule::when_stmt, "when not isEmpty() move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_not_comparison() {
  let result = LaleParser::parse(Rule::when_stmt, "when not x > 10 move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH LOGICAL OPERATORS ====================

#[test]
fn test_when_logical_and() {
  let result = LaleParser::parse(Rule::when_stmt, "when a and b move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_logical_or() {
  let result = LaleParser::parse(Rule::when_stmt, "when a or b move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_logical_xor() {
  let result = LaleParser::parse(Rule::when_stmt, "when a xor b move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_complex_logical() {
  let result = LaleParser::parse(Rule::when_stmt, "when a and b or c move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_parenthesized_logical() {
  let result = LaleParser::parse(Rule::when_stmt, "when (a or b) and c move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH FUNCTION CALLS ====================

#[test]
fn test_when_function_call_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when isReady() move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_method_call_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when obj.isValid() move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_function_comparison() {
  let result = LaleParser::parse(Rule::when_stmt, "when getCount() > 0 move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH BODY ====================

#[test]
fn test_when_with_single_statement() {
  let code = "when condition\n    doSomething()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_multiple_statements() {
  let code = "when valid\n    var x = 1\n    write x\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_return() {
  let code = "when error\n    return -1\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_exit_program() {
  let code = "when done\n    exit program 0\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_exit_loop() {
  let code = "when found\n    exit loop\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_variable_declaration() {
  let code = "when needsInit\n    var x as i32 = 42\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_assign() {
  let code = "when changed\n    count = count + 1 as u32\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_warn() {
  let code = "when bad\n    warn \"Invalid state\"\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_write_inline() {
  let code = "when verbose\n    write inline \"Debug info\"\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

// ==================== WHEN WITH ARRAY ACCESS ====================

#[test]
fn test_when_array_access() {
  let result = LaleParser::parse(Rule::when_stmt, "when arr[i] > 0 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_2d_array_access() {
  let result = LaleParser::parse(
    Rule::when_stmt,
    "when matrix[row][col] == target move on end when",
  );
  assert!(result.is_ok());
}

// ==================== WHEN WITH UNICODE ====================

#[test]
fn test_when_unicode_less_equal() {
  let result = LaleParser::parse(Rule::when_stmt, "when x ≤ 10 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_unicode_greater_equal() {
  let result = LaleParser::parse(Rule::when_stmt, "when value ≥ min move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_unicode_not_equal() {
  let result = LaleParser::parse(Rule::when_stmt, "when status ≠ 0 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_unicode_xor() {
  let result = LaleParser::parse(Rule::when_stmt, "when a ⊻ b move on end when");
  assert!(result.is_ok());
}

// ==================== WHEN WITH COMMENTS ====================

#[test]
fn test_when_with_comment_before() {
  let code = "// Check if ready\nwhen isReady()\n    go()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_inline_comment() {
  let code = "when x > 0 // ensure positive\n    process()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_doc_comment() {
  let code = "/// Guard clause\nwhen error\n    return -1\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

// ==================== REALISTIC WHEN EXAMPLES ====================

#[test]
fn test_when_bounds_check() {
  let code = "when index >= length\n    exit program 1\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_error_guard() {
  let code = "when result < 0\n    warn \"Error occurred\"\n    return result\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_null_check() {
  let code = "when not isNull(ptr)\n    var value = unsafe value at ptr\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_validation() {
  let code = "when name == \"\" or age < 0\n    warn \"Invalid input\"\n    return false\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_multiple_guards() {
  // Multiple when-statements in sequence (each is a standalone parse)
  let code1 = "when x <= 0\n    warn \"x must be positive\"\n    return -1\nend when";
  let result1 = LaleParser::parse(Rule::when_stmt, code1);
  assert!(result1.is_ok());

  let code2 = "when y <= 0\n    warn \"y must be positive\"\n    return -1\nend when";
  let result2 = LaleParser::parse(Rule::when_stmt, code2);
  assert!(result2.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_when_extra_whitespace() {
  let result = LaleParser::parse(Rule::when_stmt, "when   x   >   0   move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_tab_indentation() {
  let code = "when condition\n\tdoSomething()\n\tdoOther()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_multiple_newlines() {
  let code = "when condition\n\n    doSomething()\n\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_compact_single_line_body() {
  // Body can be on the same line if only whitespace separates
  let code = "when flag doIt() end when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_mixed_whitespace() {
  // Note: "end when" must have exactly one space between end and when
  let code = "when   flag  \n   \t  doIt()  \n   end when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

// ==================== WHEN EDGE CASES ====================

#[test]
fn test_when_empty_body_rejected() {
  let result = LaleParser::parse(Rule::when_stmt, "when true end when");
  assert!(
    result.is_err(),
    "when without any statement must fail to parse"
  );
}

#[test]
fn test_when_complex_condition() {
  let code = "when x > 0 and x < 100 and not isLocked\n    access()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_when_with_conversion_in_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when x as i64 > 0 move on end when");
  assert!(result.is_ok());
}

#[test]
fn test_when_with_pointer_condition() {
  let result = LaleParser::parse(Rule::when_stmt, "when pointer to x != nil move on end when");
  assert!(result.is_ok());
}

// ==================== NEGATIVE PARSER TESTS ====================

#[test]
fn test_when_without_end_when_fails() {
  let result = LaleParser::parse(Rule::when_stmt, "when true\n    doIt()\n");
  assert!(
    result.is_err(),
    "when without end when should be a parser error"
  );
}

#[test]
fn test_when_parses_with_else() {
  // The parser accepts `else` in `when` so the semantic analyzer
  // can produce a helpful error message instead of a cryptic parser error.
  let code = "when condition\n    doA()\nelse\n    doB()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(
    result.is_ok(),
    "when with else should parse (semantic analyzer handles the error): {:?}",
    result
  );
}

#[test]
fn test_when_with_else_if_fails() {
  let code = "when a\n    doA()\nelse if b\n    doB()\nend when";
  let result = LaleParser::parse(Rule::when_stmt, code);
  assert!(
    result.is_err(),
    "when with else if should be a parser error"
  );
}

#[test]
fn test_when_missing_end_keyword_fails() {
  let result = LaleParser::parse(Rule::when_stmt, "when true end");
  assert!(
    result.is_err(),
    "when with just 'end' (missing 'when') should fail"
  );
}

#[test]
fn test_when_wrong_close_keyword_fails() {
  let result = LaleParser::parse(Rule::when_stmt, "when true end if");
  assert!(
    result.is_err(),
    "when closed with end if should be a parser error"
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

  // ==================== BASIC EXECUTION ====================

  #[test]
  fn test_when_true_condition_executes_body() {
    let code = r#"
var flag as bool = false
when true
    flag = true
end when
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
  fn test_when_false_condition_skips_body() {
    let code = r#"
var flag as bool = false
when false
    flag = true
end when
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
  fn test_when_with_not_inverts_condition() {
    let code = r#"
var flag as bool = false
when not true
    flag = true
end when
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    // not true = false, so body skipped, flag stays false
    assert!(
      stdout.contains("false"),
      "Expected 'false' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_not_false_executes_body() {
    let code = r#"
var flag as bool = false
when not false
    flag = true
end when
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    // not false = true, so body executes, flag becomes true
    assert!(
      stdout.contains("true"),
      "Expected 'true' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_with_comparison_condition() {
    let code = r#"
var x as i32 = 42
var flag as bool = false
when x > 0
    flag = true
end when
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
  fn test_when_comparison_false_skips_body() {
    let code = r#"
var x as i32 = -5
var flag as bool = false
when x > 0
    flag = true
end when
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
  fn test_when_with_logical_and() {
    let code = r#"
var a as bool = true
var b as bool = true
var flag as bool = false
when a and b
    flag = true
end when
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
  fn test_when_with_logical_or() {
    let code = r#"
var a as bool = true
var b as bool = false
var flag as bool = false
when a or b
    flag = true
end when
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
  fn test_when_logical_and_short_circuits_false() {
    let code = r#"
var a as bool = false
var b as bool = true
var flag as bool = false
when a and b
    flag = true
end when
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    // a is false, so a and b is false, body skipped
    assert!(
      stdout.contains("false"),
      "Expected 'false' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_with_equality_check() {
    let code = r#"
var status as i32 = 200
var flag as bool = false
when status == 200
    flag = true
end when
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
  fn test_when_equality_false_skips() {
    let code = r#"
var status as i32 = 404
var flag as bool = false
when status == 200
    flag = true
end when
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
  fn test_when_inequality_check() {
    let code = r#"
var code as i32 = 500
var flag as bool = false
when code != 200
    flag = true
end when
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
  fn test_when_with_boolean_variable_condition() {
    let code = r#"
var ready as bool = true
var flag as bool = false
when ready
    flag = true
end when
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

  // ==================== WHEN WITH WRITE OUTPUT ====================

  #[test]
  fn test_when_with_write_in_body() {
    let code = r#"
when true
    write "executed"
end when
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("executed"),
      "Expected 'executed' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_false_skips_write() {
    let code = r#"
when false
    write "should not appear"
end when
write "after when"
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      !stdout.contains("should not appear"),
      "Should not contain 'should not appear', stdout: {}",
      stdout
    );
    assert!(
      stdout.contains("after when"),
      "Expected 'after when' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_multiple_statements_in_body() {
    let code = r#"
var a as i32 = 0
var b as i32 = 0
when true
    a = 10 as i32
    b = 20 as i32
end when
write a
write " "
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

  #[test]
  fn test_when_nested_inside_if() {
    let code = r#"
var x as i32 = 10
var flag as bool = false
if x > 0
    when x > 5
        flag = true
    end when
else
    move on
end if
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
  fn test_when_nested_inside_if_false_outer() {
    let code = r#"
var x as i32 = 0
var flag as bool = false
if x > 0
    when true
        flag = true
    end when
else
    move on
end if
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    // outer if is false, so when body never reached
    assert!(
      stdout.contains("false"),
      "Expected 'false' in stdout, got: {}",
      stdout
    );
  }

  // ==================== WHEN WITH EXIT PROGRAM ====================

  #[test]
  fn test_when_with_exit_program() {
    let code = r#"
when true
    exit program 7
end when
write "unreachable"
"#;
    let (_stdout, _stderr, success) = run_lale(code);
    // exit program 7 means the process exits with code 7
    assert!(!success, "Expected failure due to exit program 7");
  }

  #[test]
  fn test_when_false_does_not_exit() {
    let code = r#"
when false
    exit program 7
end when
write "reached"
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("reached"),
      "Expected 'reached' in stdout, got: {}",
      stdout
    );
  }

  // ==================== WHEN WITH FUNCTION ====================

  #[test]
  fn test_when_inside_non_main_function() {
    let code = r#"
fn test_func() returns i32
    when true
        return 42
    end when
    return 0
end fn

var result as i32 = test_func()
write result
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
  fn test_when_return_not_taken_in_function() {
    let code = r#"
fn test_func() returns i32
    when false
        return 42
    end when
    return 99
end fn

var result as i32 = test_func()
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("99"),
      "Expected '99' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_guard_clause_pattern() {
    // Guard clause: when x >= 0, return x (early return)
    let code = r#"
fn abs_val(x as i32) returns i32
    when x >= 0
        return x
    end when
    return 0 - (x as i32)
end fn

var result as i32 = abs_val(-5 as i32)
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("5"),
      "Expected '5' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_guard_clause_positive_path() {
    let code = r#"
fn abs_val(x as i32) returns i32
    when x >= 0
        return x
    end when
    return 0 - (x as i32)
end fn

var result as i32 = abs_val(7 as i32)
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("7"),
      "Expected '7' in stdout, got: {}",
      stdout
    );
  }

  #[test]
  fn test_when_function_call_condition() {
    let code = r#"
fn is_positive(x as i32) returns bool
    return x > 0
end fn

var flag as bool = false
when is_positive(42 as i32)
    flag = true
end when
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
  fn test_when_empty_body_rejected() {
    // Empty body is now a parser error — a `when` must contain a statement.
    let code = r#"
when true
end when
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Expected failure for empty when body, stdout={}, stderr={}",
      stdout, stderr
    );
  }

  // ==================== EDGE CASES ====================

  #[test]
  fn test_when_with_less_equal_comparison() {
    let code = r#"
var x as i32 = 5
var flag as bool = false
when x <= 10
    flag = true
end when
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
  fn test_when_with_greater_equal_comparison() {
    let code = r#"
var x as i32 = 100
var flag as bool = false
when x >= 50
    flag = true
end when
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

  // ==================== NEGATIVE: WHEN WITH ELSE ====================

  #[test]
  fn test_when_with_else_produces_semantic_error() {
    // The parser accepts `else` in `when` so the semantic analyzer
    // can produce a helpful error instead of a cryptic parser error.
    let code = r#"
when true
    write "ok"
else
    write "bad"
end when
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "when with else should fail: stdout={}, stderr={}",
      stdout, stderr
    );
    assert!(
      stderr.contains("does not support") && stderr.contains("else"),
      "Error should mention 'does not support' and 'else', got: {}",
      stderr
    );
    assert!(
      stderr.contains("use `if` instead of `when`"),
      "Error should suggest using if instead of when, got: {}",
      stderr
    );
  }
}
