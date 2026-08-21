use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for the append operator (~) in the Lale grammar.
///
/// The append operator is used to concatenate values (e.g., strings, arrays).
/// Grammar rule: append = { unary ~ ("~" ~ unary)* }

// ==================== BASIC APPEND TESTS ====================

#[test]
fn test_append_two_identifiers() {
  let result = LaleParser::parse(Rule::expression, "a ~ b");
  assert!(result.is_ok());
}

#[test]
fn test_append_three_identifiers() {
  let result = LaleParser::parse(Rule::expression, "a ~ b ~ c");
  assert!(result.is_ok());
}

#[test]
fn test_append_four_identifiers() {
  let result = LaleParser::parse(Rule::expression, "a ~ b ~ c ~ d");
  assert!(result.is_ok());
}

// ==================== APPEND WITH STRINGS ====================

#[test]
fn test_append_two_strings() {
  let result = LaleParser::parse(Rule::expression, "\"hello\" ~ \"world\"");
  assert!(result.is_ok());
}

#[test]
fn test_append_string_and_variable() {
  let result = LaleParser::parse(Rule::expression, "\"Hello, \" ~ name");
  assert!(result.is_ok());
}

#[test]
fn test_append_variable_and_string() {
  let result = LaleParser::parse(Rule::expression, "prefix ~ \"_suffix\"");
  assert!(result.is_ok());
}

#[test]
fn test_append_multiple_strings() {
  let result = LaleParser::parse(Rule::expression, "\"a\" ~ \"b\" ~ \"c\" ~ \"d\"");
  assert!(result.is_ok());
}

// ==================== APPEND WITH ARRAYS ====================

#[test]
fn test_append_two_arrays() {
  let result = LaleParser::parse(Rule::expression, "[1, 2] ~ [3, 4]");
  assert!(result.is_ok());
}

#[test]
fn test_append_array_and_variable() {
  let result = LaleParser::parse(Rule::expression, "arr1 ~ arr2");
  assert!(result.is_ok());
}

#[test]
fn test_append_multiple_arrays() {
  let result = LaleParser::parse(Rule::expression, "[1] ~ [2] ~ [3]");
  assert!(result.is_ok());
}

// ==================== APPEND WITH UNARY OPERATORS ====================

#[test]
fn test_append_with_not() {
  let result = LaleParser::parse(Rule::expression, "not a ~ not b");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_invert() {
  let result = LaleParser::parse(Rule::expression, "invert x ~ invert y");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_size_of() {
  let result = LaleParser::parse(Rule::expression, "#size of a ~ #size of b");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_type_of() {
  let result = LaleParser::parse(Rule::expression, "#type of x ~ #type of y");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_pointer_to() {
  let result = LaleParser::parse(Rule::expression, "pointer to a ~ pointer to b");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_value_at() {
  let result = LaleParser::parse(Rule::expression, "unsafe value at ptr1 ~ unsafe value at ptr2");
  assert!(result.is_ok());
}

// ==================== APPEND IN EXPRESSIONS ====================

#[test]
fn test_append_in_assignment() {
  let result = LaleParser::parse(Rule::r#var, "var result = a ~ b");
  assert!(result.is_ok());
}

#[test]
fn test_append_in_function_call() {
  let result = LaleParser::parse(Rule::fn_call, "process(a ~ b)");
  assert!(result.is_ok());
}

#[test]
fn test_append_in_function_multiple_args() {
  let result = LaleParser::parse(Rule::fn_call, "combine(x ~ y, z ~ w)");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_parentheses() {
  let result = LaleParser::parse(Rule::expression, "(a ~ b) ~ (c ~ d)");
  assert!(result.is_ok());
}

#[test]
fn test_append_nested_in_parentheses() {
  let result = LaleParser::parse(Rule::expression, "((a ~ b) ~ c) ~ d");
  assert!(result.is_ok());
}

// ==================== APPEND WITH FUNCTION CALLS ====================

#[test]
fn test_append_function_results() {
  let result = LaleParser::parse(Rule::expression, "getPrefix() ~ getSuffix()");
  assert!(result.is_ok());
}

#[test]
fn test_append_method_results() {
  let result = LaleParser::parse(Rule::expression, "obj.getA() ~ obj.getB()");
  assert!(result.is_ok());
}

// ==================== APPEND WITH MEMBER ACCESS ====================

#[test]
fn test_append_member_access() {
  let result = LaleParser::parse(Rule::expression, "obj.field1 ~ obj.field2");
  assert!(result.is_ok());
}

#[test]
fn test_append_chained_member() {
  let result = LaleParser::parse(Rule::expression, "a.b.c ~ x.y.z");
  assert!(result.is_ok());
}

// ==================== APPEND WITH ARRAY ACCESS ====================

#[test]
fn test_append_array_elements() {
  let result = LaleParser::parse(Rule::expression, "arr[0] ~ arr[1]");
  assert!(result.is_ok());
}

#[test]
fn test_append_2d_array_elements() {
  let result = LaleParser::parse(Rule::expression, "matrix[0][0] ~ matrix[1][1]");
  assert!(result.is_ok());
}

// ==================== APPEND PRECEDENCE ====================

#[test]
fn test_append_lower_than_multiplication() {
  let result = LaleParser::parse(Rule::expression, "a * b ~ c * d");
  assert!(result.is_ok());
}

#[test]
fn test_append_lower_than_addition() {
  let result = LaleParser::parse(Rule::expression, "a + b ~ c + d");
  assert!(result.is_ok());
}

#[test]
fn test_append_chained_long() {
  let result = LaleParser::parse(Rule::expression, "a ~ b ~ c ~ d ~ e");
  assert!(result.is_ok());
}

// ==================== APPEND WITH LITERALS ====================

#[test]
fn test_append_integers() {
  let result = LaleParser::parse(Rule::expression, "1 ~ 2 ~ 3");
  assert!(result.is_ok());
}

#[test]
fn test_append_chars() {
  let result = LaleParser::parse(Rule::expression, "'a' ~ 'b' ~ 'c'");
  assert!(result.is_ok());
}

#[test]
fn test_append_booleans() {
  let result = LaleParser::parse(Rule::expression, "true ~ false");
  assert!(result.is_ok());
}

#[test]
fn test_append_hex_literals() {
  let result = LaleParser::parse(Rule::expression, "0xFF ~ 0x00");
  assert!(result.is_ok());
}

// ==================== REALISTIC APPEND EXAMPLES ====================

#[test]
fn test_append_path_building() {
  let result = LaleParser::parse(Rule::expression, "directory ~ \"/\" ~ filename");
  assert!(result.is_ok());
}

#[test]
fn test_append_message_formatting() {
  let result = LaleParser::parse(
    Rule::expression,
    "\"Error: \" ~ errorMsg ~ \" at line \" ~ lineNum",
  );
  assert!(result.is_ok());
}

#[test]
fn test_append_array_merge() {
  let result = LaleParser::parse(Rule::expression, "header ~ body ~ footer");
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_append_no_spaces() {
  let result = LaleParser::parse(Rule::expression, "a~b");
  assert!(result.is_ok());
}

#[test]
fn test_append_extra_spaces() {
  let result = LaleParser::parse(Rule::expression, "a   ~   b");
  assert!(result.is_ok());
}

#[test]
fn test_append_with_newlines() {
  let result = LaleParser::parse(Rule::expression, "a ~\n    b ~\n    c");
  assert!(result.is_ok());
}
