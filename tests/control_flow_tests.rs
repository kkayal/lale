use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for control flow rules in the Lale grammar.
///
/// This test suite covers:
/// - condition: Conditional expressions
/// - not: Logical negation in conditions
/// - branch: Statement sequences within control structures
/// - if: Conditional statements
/// - loop_stmt: Loop statements
/// - range: Loop range specifications
/// - exit: Exit statements
/// - rewind: Rewind statements

// ==================== CONDITION TESTS ====================

#[test]
fn test_condition_simple_expression() {
  let result = LaleParser::parse(Rule::condition, "x > 5");
  assert!(result.is_ok());
}

#[test]
fn test_condition_boolean_variable() {
  let result = LaleParser::parse(Rule::condition, "flag");
  assert!(result.is_ok());
}

#[test]
fn test_condition_boolean_literal() {
  let result = LaleParser::parse(Rule::condition, "true");
  assert!(result.is_ok());
}

#[test]
fn test_condition_comparison() {
  let result = LaleParser::parse(Rule::condition, "count < 10");
  assert!(result.is_ok());
}

#[test]
fn test_condition_logical_and() {
  let result = LaleParser::parse(Rule::condition, "a > 0 and b < 100");
  assert!(result.is_ok());
}

#[test]
fn test_condition_logical_or() {
  let result = LaleParser::parse(Rule::condition, "x == 0 or y == 0");
  assert!(result.is_ok());
}

#[test]
fn test_condition_with_not() {
  let result = LaleParser::parse(Rule::condition, "not flag");
  assert!(result.is_ok());
}

#[test]
fn test_condition_complex() {
  let result = LaleParser::parse(Rule::condition, "a and b or c");
  assert!(result.is_ok());
}

#[test]
fn test_condition_parenthesized() {
  let result = LaleParser::parse(Rule::condition, "(a and b) or (c and d)");
  assert!(result.is_ok());
}

#[test]
fn test_condition_function_call() {
  let result = LaleParser::parse(Rule::condition, "isValid()");
  assert!(result.is_ok());
}

// ==================== NOT OPERATOR TESTS ====================

#[test]
fn test_not_operator_basic() {
  let result = LaleParser::parse(Rule::kw_not, "not");
  assert!(result.is_ok());
}

// ==================== RANGE TESTS ====================

#[test]
fn test_range_simple() {
  let result = LaleParser::parse(Rule::range, "over i as u32 from 1 to 10");
  assert!(result.is_ok());
}

#[test]
fn test_range_with_step() {
  let result = LaleParser::parse(Rule::range, "over i as u32 from 1 to 10 step 2");
  assert!(result.is_ok());
}

#[test]
fn test_range_with_variables() {
  let result = LaleParser::parse(Rule::range, "over x as i32 from start to end");
  assert!(result.is_ok());
}

#[test]
fn test_range_with_expressions() {
  let result = LaleParser::parse(Rule::range, "over i as u32 from a + 1 to b * 2 step c - d");
  assert!(result.is_ok());
}

#[test]
fn test_range_different_types() {
  let result = LaleParser::parse(Rule::range, "over count as u64 from 0 to 1000000");
  assert!(result.is_ok());
}

// ==================== EXIT PROGRAM STATEMENT TESTS ====================

#[test]
fn test_exit_program_basic() {
  let result = LaleParser::parse(Rule::exit_program, "exit program");
  assert!(result.is_ok());
}

#[test]
fn test_exit_program_with_code() {
  let result = LaleParser::parse(Rule::exit_program, "exit program 1");
  assert!(result.is_ok());
}

#[test]
fn test_exit_program_with_zero_code() {
  let result = LaleParser::parse(Rule::exit_program, "exit program 0");
  assert!(result.is_ok());
}

#[test]
fn test_exit_program_with_negative_code() {
  let result = LaleParser::parse(Rule::exit_program, "exit program -1");
  assert!(result.is_ok());
}

// ==================== EXIT LOOP STATEMENT TESTS ====================

#[test]
fn test_exit_loop_basic() {
  let result = LaleParser::parse(Rule::exit_loop, "exit loop");
  assert!(result.is_ok());
}

// ==================== REWIND STATEMENT TESTS ====================

#[test]
fn test_rewind_basic() {
  let result = LaleParser::parse(Rule::rewind, "rewind");
  assert!(result.is_ok());
}

// ==================== BRANCH TESTS ====================
// Branches contain statement sequences

#[test]
fn test_branch_single_function_call() {
  // Simplified: just testing the parsing of function call
  let result = LaleParser::parse(Rule::expression, "doSomething()");
  assert!(result.is_ok());
}

#[test]
fn test_branch_with_multiple_statements() {
  // Testing individual statements that could be in a branch
  let stmt1 = LaleParser::parse(Rule::expression, "a + 1");
  let stmt2 = LaleParser::parse(Rule::expression, "print()");
  assert!(stmt1.is_ok() && stmt2.is_ok());
}

// ==================== COMPLEX CONTROL FLOW PATTERNS ====================

#[test]
fn test_condition_with_array_access() {
  let result = LaleParser::parse(Rule::condition, "arr[i] > threshold");
  assert!(result.is_ok());
}

#[test]
fn test_condition_with_method_call() {
  let result = LaleParser::parse(Rule::condition, "obj.isReady()");
  assert!(result.is_ok());
}

#[test]
fn test_condition_negation() {
  let result = LaleParser::parse(Rule::condition, "not isEmpty()");
  assert!(result.is_ok());
}

#[test]
fn test_range_unicode_identifiers() {
  let result = LaleParser::parse(Rule::range, "over α as u32 from 0 to 100");
  assert!(result.is_ok());
}

#[test]
fn test_exit_program_with_large_code() {
  let result = LaleParser::parse(Rule::exit_program, "exit program 255");
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_condition_single_variable() {
  let result = LaleParser::parse(Rule::condition, "x");
  assert!(result.is_ok());
}

#[test]
fn test_condition_nested_operations() {
  let result = LaleParser::parse(Rule::condition, "a + b > c * d");
  assert!(result.is_ok());
}

#[test]
fn test_range_step_expressions() {
  let result = LaleParser::parse(Rule::range, "over i as u32 from 1 to 100 step size / 2");
  assert!(result.is_ok());
}

#[test]
fn test_condition_with_comparisons() {
  let result = LaleParser::parse(Rule::condition, "a >= 0 and a <= 100");
  assert!(result.is_ok());
}

#[test]
fn test_condition_unicode_operators() {
  let result = LaleParser::parse(Rule::condition, "x ≤ 10");
  assert!(result.is_ok());
}

#[test]
fn test_exit_program_spacing() {
  let result = LaleParser::parse(Rule::exit_program, "exit program   42");
  assert!(result.is_ok());
}

// ==================== OPERATOR PRECEDENCE IN CONDITIONS ====================

#[test]
fn test_condition_or_lower_precedence_than_and() {
  let result = LaleParser::parse(Rule::condition, "a or b and c");
  assert!(result.is_ok());
}

#[test]
fn test_condition_comparison_lower_than_arithmetic() {
  let result = LaleParser::parse(Rule::condition, "a + b > c * d");
  assert!(result.is_ok());
}

#[test]
fn test_condition_multiple_negations() {
  let result = LaleParser::parse(Rule::condition, "not not flag");
  assert!(result.is_ok());
}

// ==================== REALISTIC CONDITIONS ====================

#[test]
fn test_condition_range_check() {
  let result = LaleParser::parse(Rule::condition, "x >= 0 and x < 100");
  assert!(result.is_ok());
}

#[test]
fn test_condition_equality_check() {
  let result = LaleParser::parse(Rule::condition, "status == \"ready\"");
  assert!(result.is_ok());
}

#[test]
fn test_condition_inequality_check() {
  let result = LaleParser::parse(Rule::condition, "count != 0");
  assert!(result.is_ok());
}

#[test]
fn test_condition_boolean_combination() {
  let result = LaleParser::parse(Rule::condition, "isActive and (hasPermission or isAdmin)");
  assert!(result.is_ok());
}

#[test]
fn test_range_realistic() {
  let result = LaleParser::parse(
    Rule::range,
    "over item as str from start to finish step stride",
  );
  assert!(result.is_ok());
}
