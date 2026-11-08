mod common;

use common::parse_fully;
use lale::Rule;

/// Tests for the append operator (~) in the Lale grammar.
///
/// The append operator is used to concatenate values (e.g., strings, arrays).
/// Grammar rule: append = { unary ~ ("~" ~ unary)* }
///
/// These tests use `common::parse_fully`, which requires the rule to consume the
/// whole input. `LaleParser::parse` alone succeeds on a *prefix* match, so a bare
/// `assert!(parse(...).is_ok())` can pass for the wrong reason.

// ==================== BASIC APPEND TESTS ====================

#[test]
fn test_append_two_identifiers() {
  assert!(parse_fully(Rule::expression, "a ~ b").is_ok());
}

#[test]
fn test_append_three_identifiers() {
  assert!(parse_fully(Rule::expression, "a ~ b ~ c").is_ok());
}

#[test]
fn test_append_four_identifiers() {
  assert!(parse_fully(Rule::expression, "a ~ b ~ c ~ d").is_ok());
}

// ==================== APPEND WITH STRINGS ====================

#[test]
fn test_append_two_strings() {
  assert!(parse_fully(Rule::expression, "\"hello\" ~ \"world\"").is_ok());
}

#[test]
fn test_append_string_and_variable() {
  assert!(parse_fully(Rule::expression, "\"Hello, \" ~ name").is_ok());
}

#[test]
fn test_append_variable_and_string() {
  assert!(parse_fully(Rule::expression, "prefix ~ \"_suffix\"").is_ok());
}

#[test]
fn test_append_multiple_strings() {
  assert!(parse_fully(Rule::expression, "\"a\" ~ \"b\" ~ \"c\" ~ \"d\"").is_ok());
}

// ==================== APPEND WITH ARRAYS ====================

#[test]
fn test_append_two_arrays() {
  assert!(parse_fully(Rule::expression, "[1, 2] ~ [3, 4]").is_ok());
}

#[test]
fn test_append_array_and_variable() {
  assert!(parse_fully(Rule::expression, "arr1 ~ arr2").is_ok());
}

#[test]
fn test_append_multiple_arrays() {
  assert!(parse_fully(Rule::expression, "[1] ~ [2] ~ [3]").is_ok());
}

// ==================== APPEND WITH UNARY OPERATORS ====================

#[test]
fn test_append_with_not() {
  assert!(parse_fully(Rule::expression, "not a ~ not b").is_ok());
}

#[test]
fn test_append_with_invert() {
  assert!(parse_fully(Rule::expression, "invert x ~ invert y").is_ok());
}

#[test]
fn test_append_with_size_of() {
  assert!(parse_fully(Rule::expression, "#size of a ~ #size of b").is_ok());
}

#[test]
fn test_append_with_type_of() {
  assert!(parse_fully(Rule::expression, "#type of x ~ #type of y").is_ok());
}

#[test]
fn test_append_with_pointer_to() {
  assert!(parse_fully(Rule::expression, "pointer to a ~ pointer to b").is_ok());
}

#[test]
fn test_append_with_value_at() {
  assert!(
    parse_fully(
      Rule::expression,
      "unsafe value at ptr1 ~ unsafe value at ptr2"
    )
    .is_ok()
  );
}

// ==================== APPEND IN EXPRESSIONS ====================

#[test]
fn test_append_in_assignment() {
  assert!(parse_fully(Rule::r#var, "var result = a ~ b").is_ok());
}

#[test]
fn test_append_in_function_call() {
  assert!(parse_fully(Rule::fn_call, "process(a ~ b)").is_ok());
}

#[test]
fn test_append_in_function_multiple_args() {
  assert!(parse_fully(Rule::fn_call, "combine(x ~ y, z ~ w)").is_ok());
}

#[test]
fn test_append_with_parentheses() {
  assert!(parse_fully(Rule::expression, "(a ~ b) ~ (c ~ d)").is_ok());
}

#[test]
fn test_append_nested_in_parentheses() {
  assert!(parse_fully(Rule::expression, "((a ~ b) ~ c) ~ d").is_ok());
}

// ==================== APPEND WITH FUNCTION CALLS ====================

#[test]
fn test_append_function_results() {
  assert!(parse_fully(Rule::expression, "getPrefix() ~ getSuffix()").is_ok());
}

#[test]
fn test_append_method_results() {
  assert!(parse_fully(Rule::expression, "obj.getA() ~ obj.getB()").is_ok());
}

// ==================== APPEND WITH MEMBER ACCESS ====================

#[test]
fn test_append_member_access() {
  assert!(parse_fully(Rule::expression, "obj.field1 ~ obj.field2").is_ok());
}

#[test]
fn test_append_chained_member() {
  assert!(parse_fully(Rule::expression, "a.b.c ~ x.y.z").is_ok());
}

// ==================== APPEND WITH ARRAY ACCESS ====================

#[test]
fn test_append_array_elements() {
  assert!(parse_fully(Rule::expression, "arr[0] ~ arr[1]").is_ok());
}

#[test]
fn test_append_2d_array_elements() {
  assert!(parse_fully(Rule::expression, "matrix[0][0] ~ matrix[1][1]").is_ok());
}

// ==================== APPEND PRECEDENCE ====================

#[test]
fn test_append_lower_than_multiplication() {
  assert!(parse_fully(Rule::expression, "a * b ~ c * d").is_ok());
}

#[test]
fn test_append_lower_than_addition() {
  assert!(parse_fully(Rule::expression, "a + b ~ c + d").is_ok());
}

#[test]
fn test_append_chained_long() {
  assert!(parse_fully(Rule::expression, "a ~ b ~ c ~ d ~ e").is_ok());
}

// ==================== APPEND WITH LITERALS ====================

#[test]
fn test_append_integers() {
  assert!(parse_fully(Rule::expression, "1 ~ 2 ~ 3").is_ok());
}

#[test]
fn test_append_chars() {
  assert!(parse_fully(Rule::expression, "'a' ~ 'b' ~ 'c'").is_ok());
}

#[test]
fn test_append_booleans() {
  assert!(parse_fully(Rule::expression, "true ~ false").is_ok());
}

#[test]
fn test_append_hex_literals() {
  assert!(parse_fully(Rule::expression, "0xFF ~ 0x00").is_ok());
}

// ==================== REALISTIC APPEND EXAMPLES ====================

#[test]
fn test_append_path_building() {
  assert!(parse_fully(Rule::expression, "directory ~ \"/\" ~ filename").is_ok());
}

#[test]
fn test_append_message_formatting() {
  assert!(
    parse_fully(
      Rule::expression,
      "\"Error: \" ~ errorMsg ~ \" at line \" ~ lineNum",
    )
    .is_ok()
  );
}

#[test]
fn test_append_array_merge() {
  assert!(parse_fully(Rule::expression, "header ~ body ~ footer").is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_append_no_spaces() {
  assert!(parse_fully(Rule::expression, "a~b").is_ok());
}

#[test]
fn test_append_extra_spaces() {
  assert!(parse_fully(Rule::expression, "a   ~   b").is_ok());
}

#[test]
fn test_append_rejects_newline_separators() {
  // `expression` separates operands with `os` (spaces/tabs), not `os_ln`
  // (spaces or newlines), so a newline between `~` operands is not a single
  // expression — the rule stops after `a ~`. Whole-input parsing catches that;
  // a bare `is_ok()` would previously pass on the prefix match.
  let result = parse_fully(Rule::expression, "a ~\n    b ~\n    c");
  assert!(result.is_err());
}
