use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for return statement rules in the Lale grammar.
///
/// This test suite covers:
/// - Basic return statements
/// - Return with expressions
/// - Return with literals
/// - Return with function calls

// ==================== BASIC RETURN TESTS ====================

#[test]
fn test_return_empty() {
  let result = LaleParser::parse(Rule::return_stmt, "return ");
  assert!(result.is_ok());
}

#[test]
fn test_return_integer() {
  let result = LaleParser::parse(Rule::return_stmt, "return 42");
  assert!(result.is_ok());
}

#[test]
fn test_return_zero() {
  let result = LaleParser::parse(Rule::return_stmt, "return 0");
  assert!(result.is_ok());
}

#[test]
fn test_return_negative() {
  let result = LaleParser::parse(Rule::return_stmt, "return -1");
  assert!(result.is_ok());
}

#[test]
fn test_return_float() {
  let result = LaleParser::parse(Rule::return_stmt, "return 3.14");
  assert!(result.is_ok());
}

#[test]
fn test_return_negative_float() {
  let result = LaleParser::parse(Rule::return_stmt, "return -2.5");
  assert!(result.is_ok());
}

#[test]
fn test_return_variable() {
  let result = LaleParser::parse(Rule::return_stmt, "return result");
  assert!(result.is_ok());
}

// ==================== RETURN WITH LITERALS ====================

#[test]
fn test_return_true() {
  let result = LaleParser::parse(Rule::return_stmt, "return true");
  assert!(result.is_ok());
}

#[test]
fn test_return_false() {
  let result = LaleParser::parse(Rule::return_stmt, "return false");
  assert!(result.is_ok());
}

#[test]
fn test_return_string() {
  let result = LaleParser::parse(Rule::return_stmt, "return \"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_return_char() {
  let result = LaleParser::parse(Rule::return_stmt, "return 'x'");
  assert!(result.is_ok());
}

#[test]
fn test_return_hex() {
  let result = LaleParser::parse(Rule::return_stmt, "return 0xFF");
  assert!(result.is_ok());
}

#[test]
fn test_return_array() {
  let result = LaleParser::parse(Rule::return_stmt, "return [1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_return_empty_array() {
  let result = LaleParser::parse(Rule::return_stmt, "return []");
  assert!(result.is_ok());
}

// ==================== RETURN WITH EXPRESSIONS ====================

#[test]
fn test_return_addition() {
  let result = LaleParser::parse(Rule::return_stmt, "return a + b");
  assert!(result.is_ok());
}

#[test]
fn test_return_subtraction() {
  let result = LaleParser::parse(Rule::return_stmt, "return x - y");
  assert!(result.is_ok());
}

#[test]
fn test_return_multiplication() {
  let result = LaleParser::parse(Rule::return_stmt, "return a * b");
  assert!(result.is_ok());
}

#[test]
fn test_return_division() {
  let result = LaleParser::parse(Rule::return_stmt, "return x / y");
  assert!(result.is_ok());
}

#[test]
fn test_return_modulo() {
  let result = LaleParser::parse(Rule::return_stmt, "return n % 2");
  assert!(result.is_ok());
}

#[test]
fn test_return_power() {
  let result = LaleParser::parse(Rule::return_stmt, "return base ^ exp");
  assert!(result.is_ok());
}

#[test]
fn test_return_complex_expression() {
  let result = LaleParser::parse(Rule::return_stmt, "return a + b * c - d / e");
  assert!(result.is_ok());
}

#[test]
fn test_return_parenthesized() {
  let result = LaleParser::parse(Rule::return_stmt, "return (a + b) * c");
  assert!(result.is_ok());
}

// ==================== RETURN WITH COMPARISONS ====================

#[test]
fn test_return_comparison() {
  let result = LaleParser::parse(Rule::return_stmt, "return x > 0");
  assert!(result.is_ok());
}

#[test]
fn test_return_equality() {
  let result = LaleParser::parse(Rule::return_stmt, "return a == b");
  assert!(result.is_ok());
}

#[test]
fn test_return_inequality() {
  let result = LaleParser::parse(Rule::return_stmt, "return x != 0");
  assert!(result.is_ok());
}

#[test]
fn test_return_logical_and() {
  let result = LaleParser::parse(Rule::return_stmt, "return a and b");
  assert!(result.is_ok());
}

#[test]
fn test_return_logical_or() {
  let result = LaleParser::parse(Rule::return_stmt, "return a or b");
  assert!(result.is_ok());
}

// ==================== RETURN WITH FUNCTION CALLS ====================

#[test]
fn test_return_function_call() {
  let result = LaleParser::parse(Rule::return_stmt, "return getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_return_function_with_args() {
  let result = LaleParser::parse(Rule::return_stmt, "return calculate(x, y)");
  assert!(result.is_ok());
}

#[test]
fn test_return_method_call() {
  let result = LaleParser::parse(Rule::return_stmt, "return obj.getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_return_chained_method() {
  let result = LaleParser::parse(Rule::return_stmt, "return a.b.c()");
  assert!(result.is_ok());
}

#[test]
fn test_return_nested_function() {
  let result = LaleParser::parse(Rule::return_stmt, "return outer(inner(x))");
  assert!(result.is_ok());
}

// ==================== RETURN WITH MEMBER ACCESS ====================

#[test]
fn test_return_member() {
  let result = LaleParser::parse(Rule::return_stmt, "return obj.field");
  assert!(result.is_ok());
}

#[test]
fn test_return_chained_member() {
  let result = LaleParser::parse(Rule::return_stmt, "return a.b.c");
  assert!(result.is_ok());
}

// ==================== RETURN WITH ARRAY ACCESS ====================

#[test]
fn test_return_array_element() {
  let result = LaleParser::parse(Rule::return_stmt, "return arr[0]");
  assert!(result.is_ok());
}

#[test]
fn test_return_array_variable_index() {
  let result = LaleParser::parse(Rule::return_stmt, "return arr[i]");
  assert!(result.is_ok());
}

#[test]
fn test_return_2d_array() {
  let result = LaleParser::parse(Rule::return_stmt, "return matrix[row][col]");
  assert!(result.is_ok());
}

// ==================== RETURN WITH UNARY OPERATORS ====================

#[test]
fn test_return_not() {
  let result = LaleParser::parse(Rule::return_stmt, "return not flag");
  assert!(result.is_ok());
}

#[test]
fn test_return_invert() {
  let result = LaleParser::parse(Rule::return_stmt, "return invert bits");
  assert!(result.is_ok());
}

#[test]
fn test_return_size_of() {
  let result = LaleParser::parse(Rule::return_stmt, "return #size of arr");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_of() {
  let result = LaleParser::parse(Rule::return_stmt, "return #type of x");
  assert!(result.is_ok());
}

#[test]
fn test_return_pointer_to() {
  let result = LaleParser::parse(Rule::return_stmt, "return pointer to x");
  assert!(result.is_ok());
}

#[test]
fn test_return_value_at() {
  let result = LaleParser::parse(Rule::return_stmt, "return unsafe value at ptr");
  assert!(result.is_ok());
}

#[test]
fn test_return_conversion() {
  let result = LaleParser::parse(Rule::return_stmt, "return x as u32");
  assert!(result.is_ok());
}

// ==================== RETURN WITH UNICODE ====================

#[test]
fn test_return_unicode_variable() {
  let result = LaleParser::parse(Rule::return_stmt, "return α");
  assert!(result.is_ok());
}

#[test]
fn test_return_unicode_comparison() {
  let result = LaleParser::parse(Rule::return_stmt, "return x ≤ 10");
  assert!(result.is_ok());
}

#[test]
fn test_return_unicode_multiplication() {
  let result = LaleParser::parse(Rule::return_stmt, "return a ⋅ b");
  assert!(result.is_ok());
}

// ==================== REALISTIC RETURN EXAMPLES ====================

#[test]
fn test_return_conditional_result() {
  let result = LaleParser::parse(Rule::return_stmt, "return x > 0 and x < 100");
  assert!(result.is_ok());
}

#[test]
fn test_return_computed_value() {
  let result = LaleParser::parse(Rule::return_stmt, "return base * rate / 100");
  assert!(result.is_ok());
}

#[test]
fn test_return_string_concatenation() {
  let result = LaleParser::parse(Rule::return_stmt, "return prefix ~ suffix");
  assert!(result.is_ok());
}

#[test]
fn test_return_with_unit_value() {
  let result = LaleParser::parse(Rule::return_stmt, "return 9.81<m/s^2>");
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_return_extra_whitespace() {
  let result = LaleParser::parse(Rule::return_stmt, "return   42");
  assert!(result.is_ok());
}

#[test]
fn test_return_tabs() {
  let result = LaleParser::parse(Rule::return_stmt, "return\t\tvalue");
  assert!(result.is_ok());
}
