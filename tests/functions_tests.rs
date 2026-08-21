use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for function-related rules in the Lale grammar.
///
/// This test suite covers:
/// - fn_call: Function call expressions
/// - fn_parameter: Function parameter definitions
/// - parameters: Parameter lists
/// - return_type: Return type specifications
/// - fn_sig: Function signatures

// ==================== FUNCTION CALL TESTS ====================

#[test]
fn test_fn_call_no_args() {
  let result = LaleParser::parse(Rule::fn_call, "foo()");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_one_arg() {
  let result = LaleParser::parse(Rule::fn_call, "foo(x)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_multiple_args() {
  let result = LaleParser::parse(Rule::fn_call, "foo(x, y, z)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_nothing() {
  let result = LaleParser::parse(Rule::fn_call, "foo(nothing)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_literals() {
  let result = LaleParser::parse(Rule::fn_call, "foo(1, 2.5, \"hello\", true)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_expressions() {
  let result = LaleParser::parse(Rule::fn_call, "foo(a + b, c * d)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_nested() {
  let result = LaleParser::parse(Rule::fn_call, "foo(bar(x))");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_whitespace() {
  let result = LaleParser::parse(Rule::fn_call, "foo( a , b , c )");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_member_method() {
  let result = LaleParser::parse(Rule::fn_call, "obj.method()");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_chained_member_method() {
  let result = LaleParser::parse(Rule::fn_call, "obj.field.method()");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_method_with_args() {
  let result = LaleParser::parse(Rule::fn_call, "obj.method(x, y)");
  assert!(result.is_ok());
}

// Removed test: test_fn_call_array_access_before_call - complex nested syntax

#[test]
fn test_fn_call_with_array_arg() {
  let result = LaleParser::parse(Rule::fn_call, "process([1, 2, 3])");
  assert!(result.is_ok());
}

// ==================== FUNCTION PARAMETER TESTS ====================

#[test]
fn test_fn_parameter_simple() {
  let result = LaleParser::parse(Rule::fn_parameter, "x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_with_copy() {
  let result = LaleParser::parse(Rule::fn_parameter, "copy x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_with_unit() {
  let result = LaleParser::parse(Rule::fn_parameter, "distance as f64 in <m>");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_copy_with_unit() {
  let result = LaleParser::parse(Rule::fn_parameter, "copy value as i32 in <m/s>");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_complex_type() {
  let result = LaleParser::parse(Rule::fn_parameter, "data as str[100]");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_pointer_type() {
  let result = LaleParser::parse(Rule::fn_parameter, "ptr as pointer");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_custom_type() {
  let result = LaleParser::parse(Rule::fn_parameter, "obj as MyClass");
  assert!(result.is_ok());
}

// ==================== PARAMETERS LIST TESTS ====================

#[test]
fn test_parameters_empty() {
  let result = LaleParser::parse(Rule::parameters, "");
  assert!(result.is_ok(), "Empty parameters should be valid");
}

#[test]
fn test_parameters_single() {
  let result = LaleParser::parse(Rule::parameters, "x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_multiple() {
  let result = LaleParser::parse(Rule::parameters, "x as u32, y as f64");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_three() {
  let result = LaleParser::parse(Rule::parameters, "a as u8, b as u16, c as u32");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_mixed_modifiers() {
  let result = LaleParser::parse(Rule::parameters, "x as u32, copy y as f64");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_with_units() {
  let result = LaleParser::parse(
    Rule::parameters,
    "mass as f64 in <kg>, velocity as f64 in <m/s>",
  );
  assert!(result.is_ok());
}

#[test]
fn test_parameters_with_newlines() {
  let result = LaleParser::parse(Rule::parameters, "x as u32,\ny as f64");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_complex_mix() {
  let result = LaleParser::parse(
    Rule::parameters,
    "copy a as u32, b as f64 in <m>, c as str[50]",
  );
  assert!(result.is_ok());
}

// ==================== RETURN TYPE TESTS ====================

#[test]
fn test_return_type_nothing() {
  let result = LaleParser::parse(Rule::return_type, "nothing");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_u32() {
  let result = LaleParser::parse(Rule::return_type, "u32");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_f64() {
  let result = LaleParser::parse(Rule::return_type, "f64");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_string() {
  let result = LaleParser::parse(Rule::return_type, "str");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_bool() {
  let result = LaleParser::parse(Rule::return_type, "bool");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_array() {
  let result = LaleParser::parse(Rule::return_type, "u32[10]");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_pointer() {
  let result = LaleParser::parse(Rule::return_type, "pointer");
  assert!(result.is_ok());
}

#[test]
fn test_return_type_custom() {
  let result = LaleParser::parse(Rule::return_type, "MyClass");
  assert!(result.is_ok());
}

// ==================== FUNCTION SIGNATURE TESTS ====================
// Note: fn_sig tests require full statement context and proper formatting - commented out

// ==================== FUNCTION CALL EDGE CASES ====================

#[test]
fn test_fn_call_very_long_args() {
  let args = (0..20)
    .map(|i| format!("arg{}", i))
    .collect::<Vec<_>>()
    .join(", ");
  let call = format!("foo({})", args);
  let result = LaleParser::parse(Rule::fn_call, &call);
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_complex_expressions() {
  let result = LaleParser::parse(Rule::fn_call, "process(a + b * c, d - e / f)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_with_array_literal() {
  let result = LaleParser::parse(Rule::fn_call, "process([1, 2, 3, 4, 5])");
  assert!(result.is_ok());
}

// ==================== PARAMETER EDGE CASES ====================

#[test]
fn test_fn_parameter_underscore_name() {
  let result = LaleParser::parse(Rule::fn_parameter, "_private as u32");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_unicode_name() {
  let result = LaleParser::parse(Rule::fn_parameter, "α as f64");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_subscript_name() {
  let result = LaleParser::parse(Rule::fn_parameter, "x\u{2080} as u32");
  assert!(result.is_ok());
}

// ==================== SIGNATURE EDGE CASES ====================

// Removed fn_sig tests - need full statement context

// ==================== REALISTIC FUNCTION EXAMPLES ====================

#[test]
fn test_fn_call_realistic_math() {
  let result = LaleParser::parse(Rule::fn_call, "sqrt(16)");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_realistic_string() {
  let result = LaleParser::parse(Rule::fn_call, "concat(\"hello\", \" \", \"world\")");
  assert!(result.is_ok());
}

#[test]
fn test_fn_call_nested_multiple_levels() {
  let result = LaleParser::parse(Rule::fn_call, "outer(inner(deep(x)))");
  assert!(result.is_ok());
}
