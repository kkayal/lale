use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for if statement rules in the Lale grammar.
///
/// This test suite covers:
/// - Basic if statements
/// - If with else
/// - If with else if chains
/// - If with body statements
/// - Nested if statements

// ==================== BASIC IF TESTS ====================

#[test]
fn test_if_minimal() {
  let result = LaleParser::parse(Rule::if_stmt, "if true move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_false_literal() {
  let result = LaleParser::parse(Rule::if_stmt, "if false move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_variable_condition() {
  let result = LaleParser::parse(Rule::if_stmt, "if flag move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_comparison_greater() {
  let result = LaleParser::parse(Rule::if_stmt, "if x > 0 move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_comparison_less() {
  let result = LaleParser::parse(Rule::if_stmt, "if count < max move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_comparison_greater_equal() {
  let result = LaleParser::parse(Rule::if_stmt, "if value >= threshold move on\nend if");
  assert!(result.is_ok());
}

#[test]
fn test_if_comparison_less_equal() {
  let result = LaleParser::parse(Rule::if_stmt, "if index <= limit move on\nend if");
  assert!(result.is_ok());
}

#[test]
fn test_if_equality() {
  let result = LaleParser::parse(Rule::if_stmt, "if status == 0 move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_inequality() {
  let result = LaleParser::parse(Rule::if_stmt, "if error != 0 move on end if");
  assert!(result.is_ok());
}

// ==================== IF WITH NOT ====================

#[test]
fn test_if_not_condition() {
  let result = LaleParser::parse(Rule::if_stmt, "if not done move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_not_function_call() {
  let result = LaleParser::parse(Rule::if_stmt, "if not isEmpty() move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_not_comparison() {
  let result = LaleParser::parse(Rule::if_stmt, "if not x > 10 move on end if");
  assert!(result.is_ok());
}

// ==================== IF WITH LOGICAL OPERATORS ====================

#[test]
fn test_if_logical_and() {
  let result = LaleParser::parse(Rule::if_stmt, "if a and b move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_logical_or() {
  let result = LaleParser::parse(Rule::if_stmt, "if a or b move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_logical_xor() {
  let result = LaleParser::parse(Rule::if_stmt, "if a xor b move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_complex_logical() {
  let result = LaleParser::parse(Rule::if_stmt, "if a and b or c move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_parenthesized_logical() {
  let result = LaleParser::parse(Rule::if_stmt, "if (a or b) and c move on end if");
  assert!(result.is_ok());
}

// ==================== IF WITH BODY ====================

#[test]
fn test_if_with_function_call() {
  let result = LaleParser::parse(Rule::if_stmt, "if condition\n    doSomething()\nend if");
  assert!(result.is_ok());
}

#[test]
fn test_if_with_multiple_statements() {
  let code = "if valid\n    var x = 1\n    write x\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_with_return() {
  let code = "if error\n    return -1\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_with_exit_program() {
  let code = "if done\n    exit program 0\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== IF ELSE ====================

#[test]
fn test_if_empty_if_and_else_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if true\nelse\nend if");
  assert!(
    result.is_err(),
    "if with empty body and empty else must fail to parse"
  );
}

#[test]
fn test_if_else_with_bodies() {
  let code = "if condition\n    doA()\nelse\n    doB()\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_else_multiple_statements() {
  let code = "if x > 0\n    write \"positive\"\n    var result = x\nelse\n    write \"non-positive\"\n    var result = 0\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== IF ELSE IF ====================

#[test]
fn test_if_empty_if_and_else_if_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if a\nelse if b\nend if");
  assert!(
    result.is_err(),
    "if with empty body and empty else if must fail to parse"
  );
}

#[test]
fn test_if_else_if_with_bodies() {
  let code = "if x > 0\n    write \"positive\"\nelse if x < 0\n    write \"negative\"\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_else_if_chain() {
  let code = "if grade >= 90\n    write \"A\"\nelse if grade >= 80\n    write \"B\"\nelse if grade >= 70\n    write \"C\"\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_else_if_else() {
  let code = "if x > 0\n    write \"positive\"\nelse if x < 0\n    write \"negative\"\nelse\n    write \"zero\"\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_many_else_ifs() {
  let code = "if a == 1\n    handle1()\nelse if a == 2\n    handle2()\nelse if a == 3\n    handle3()\nelse if a == 4\n    handle4()\nelse\n    handleDefault()\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== NESTED IF ====================

#[test]
fn test_if_nested() {
  let code = "if outer\n    if inner\n        doIt()\n    end if\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_deeply_nested() {
  let code = "if a\n    if b\n        if c\n            deep()\n        end if\n    end if\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_nested_in_else() {
  let code =
    "if condition1\n    branch1()\nelse\n    if condition2\n        branch2()\n    end if\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== IF WITH UNICODE ====================

#[test]
fn test_if_unicode_comparison() {
  let result = LaleParser::parse(Rule::if_stmt, "if x ≤ 10 move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_unicode_not_equal() {
  let result = LaleParser::parse(Rule::if_stmt, "if status ≠ 0 move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_unicode_greater_equal() {
  let result = LaleParser::parse(Rule::if_stmt, "if value ≥ min move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_unicode_xor() {
  let result = LaleParser::parse(Rule::if_stmt, "if a ⊻ b move on end if");
  assert!(result.is_ok());
}

// ==================== IF WITH FUNCTION CALLS ====================

#[test]
fn test_if_function_call_condition() {
  let result = LaleParser::parse(Rule::if_stmt, "if isReady() move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_method_call_condition() {
  let result = LaleParser::parse(Rule::if_stmt, "if obj.isValid() move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_function_comparison() {
  let result = LaleParser::parse(Rule::if_stmt, "if getCount() > 0 move on end if");
  assert!(result.is_ok());
}

// ==================== IF WITH ARRAY ACCESS ====================

#[test]
fn test_if_array_access() {
  let result = LaleParser::parse(Rule::if_stmt, "if arr[i] > 0 move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_2d_array_access() {
  let result = LaleParser::parse(
    Rule::if_stmt,
    "if matrix[row][col] == target move on end if",
  );
  assert!(result.is_ok());
}

// ==================== REALISTIC IF EXAMPLES ====================

#[test]
fn test_if_bounds_check() {
  let code = "if index >= 0 and index < length\n    var value = arr[index]\nelse\n    exit program 1\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_error_handling() {
  let code = "if result < 0\n    warn \"Error occurred\"\n    return result\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_type_dispatch() {
  let code = "if kind == 1\n    handleInt()\nelse if kind == 2\n    handleFloat()\nelse if kind == 3\n    handleString()\nelse\n    handleUnknown()\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_null_check() {
  let code = "if not isNull(ptr)\n    var value = unsafe value at ptr\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_validation() {
  let code = "if name == \"\" or age < 0\n    warn \"Invalid input\"\n    return false\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_if_extra_whitespace() {
  let result = LaleParser::parse(Rule::if_stmt, "if   x   >   0   move on end if");
  assert!(result.is_ok());
}

#[test]
fn test_if_tab_indentation() {
  let code = "if condition\n\tdoSomething()\nelse\n\tdoOther()\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_if_multiple_newlines() {
  let code = "if condition\n\n    doSomething()\n\nelse\n\n    doOther()\n\nend if";
  let result = LaleParser::parse(Rule::if_stmt, code);
  assert!(result.is_ok());
}

// ==================== IF EDGE CASES ====================

#[test]
fn test_if_empty_body_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if true end if");
  assert!(
    result.is_err(),
    "if without any statement must fail to parse"
  );
}

#[test]
fn test_if_empty_else_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if condition\n    doIt()\nelse\nend if");
  assert!(
    result.is_err(),
    "else without any statement must fail to parse"
  );
}

#[test]
fn test_if_empty_else_if_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if a\nelse if b\nelse\nend if");
  assert!(
    result.is_err(),
    "else if without any statement must fail to parse"
  );
}

#[test]
fn test_if_empty_if_body_with_else_rejected() {
  let result = LaleParser::parse(Rule::if_stmt, "if condition\nelse\n    fallback()\nend if");
  assert!(
    result.is_err(),
    "if with empty body but non-empty else must fail to parse"
  );
}
