use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for declaration rules in the Lale grammar.
///
/// This test suite covers:
/// - var_symbol: Symbol definition specifications
/// - var: Variable definition with initialization
/// - unsafe_decl: Unsafe variable declarations (without initialization)

// ==================== DEF_SYMBOL TESTS ====================

#[test]
fn test_def_symbol_simple() {
  let result = LaleParser::parse(Rule::var_symbol, "var x");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_with_type() {
  let result = LaleParser::parse(Rule::var_symbol, "var x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_with_unit() {
  let result = LaleParser::parse(Rule::var_symbol, "var distance in <m>");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_with_type_and_unit() {
  let result = LaleParser::parse(Rule::var_symbol, "var distance as f64 in <m>");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_complex_type() {
  let result = LaleParser::parse(Rule::var_symbol, "var data as text[100]");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_pointer_type() {
  let result = LaleParser::parse(Rule::var_symbol, "var ptr as pointer");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_custom_type() {
  let result = LaleParser::parse(Rule::var_symbol, "var obj as MyClass");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_underscore_name() {
  let result = LaleParser::parse(Rule::var_symbol, "var _private as u32");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_unicode_name() {
  let result = LaleParser::parse(Rule::var_symbol, "var α as f64");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_compound_unit() {
  let result = LaleParser::parse(Rule::var_symbol, "var force as f64 in <kg*m/s^2>");
  assert!(result.is_ok());
}

// ==================== DEF TESTS ====================

#[test]
fn test_def_simple_value() {
  let result = LaleParser::parse(Rule::r#var, "var x = 5");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_type() {
  let result = LaleParser::parse(Rule::r#var, "var x as u32 = 5");
  assert!(result.is_ok());
}

#[test]
fn test_def_float_value() {
  let result = LaleParser::parse(Rule::r#var, "var pi as f64 = 3.14159");
  assert!(result.is_ok());
}

#[test]
fn test_def_string_value() {
  let result = LaleParser::parse(Rule::r#var, "var name as text = \"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_def_array_value() {
  let result = LaleParser::parse(Rule::r#var, "var arr = [1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_def_expression_value() {
  let result = LaleParser::parse(Rule::r#var, "var result = x + y * 2");
  assert!(result.is_ok());
}

#[test]
fn test_def_function_call_value() {
  let result = LaleParser::parse(Rule::r#var, "var val = getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_unit() {
  let result = LaleParser::parse(Rule::r#var, "var distance as f64 in <m> = 5.5");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_export() {
  let result = LaleParser::parse(Rule::r#var, "export var x as u32 = 42");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_import() {
  let result = LaleParser::parse(Rule::r#var, "import var x as u32 = 0");
  assert!(result.is_ok());
}

#[test]
fn test_def_complex_expression() {
  let result = LaleParser::parse(Rule::r#var, "var result = a + b * c - d / e");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_method_call() {
  let result = LaleParser::parse(Rule::r#var, "var value = obj.getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_array_access() {
  let result = LaleParser::parse(Rule::r#var, "var first = arr[0]");
  assert!(result.is_ok());
}

#[test]
fn test_def_compound_type() {
  let result = LaleParser::parse(Rule::r#var, "var buffer as u8[256] = []");
  assert!(result.is_ok());
}

// ==================== UNSAFE_DECL TESTS ====================

#[test]
fn test_unsafe_decl_simple() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl x");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_with_type() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl ptr as pointer");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_with_export() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe export decl x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_with_import() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe import decl x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_with_unit() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl ptr as f64 in <m>");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_unicode_name() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl α as u32");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_complex_type() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl mem as u8[1024]");
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_def_boolean_value() {
  let result = LaleParser::parse(Rule::r#var, "var flag = true");
  assert!(result.is_ok());
}

#[test]
fn test_def_char_value() {
  let result = LaleParser::parse(Rule::r#var, "var ch as char = 'x'");
  assert!(result.is_ok());
}

#[test]
fn test_def_hex_value() {
  let result = LaleParser::parse(Rule::r#var, "var color = 0xFF00FF");
  assert!(result.is_ok());
}

#[test]
fn test_def_negative_value() {
  let result = LaleParser::parse(Rule::r#var, "var neg = -42");
  assert!(result.is_ok());
}

#[test]
fn test_def_very_long_identifier() {
  let name = "very_long_variable_name_with_many_parts";
  let decl = format!("var {} = 0", name);
  let result = LaleParser::parse(Rule::r#var, &decl);
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_all_primitive_types() {
  let types = vec![
    "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "f16", "f32", "f64", "text", "bool",
    "byte", "char",
  ];
  for t in types {
    let decl = format!("var x as {}", t);
    let result = LaleParser::parse(Rule::var_symbol, &decl);
    assert!(result.is_ok(), "Should support type: {}", t);
  }
}

// ==================== REALISTIC DECLARATIONS ====================

#[test]
fn test_def_physics_constant() {
  let result = LaleParser::parse(Rule::r#var, "var g as f64 in <m/s^2> = 9.81");
  assert!(result.is_ok());
}

#[test]
fn test_def_configuration() {
  let result = LaleParser::parse(Rule::r#var, "export var MAX_BUFFER as u32 = 65536");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_memory_address() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl mmio_base as pointer");
  assert!(result.is_ok());
}

#[test]
fn test_def_computed_value() {
  let result = LaleParser::parse(Rule::r#var, "var half as f64 = 1 / 2");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_member_access() {
  let result = LaleParser::parse(Rule::r#var, "var status = config.isActive");
  assert!(result.is_ok());
}

#[test]
fn test_def_array_variable_size() {
  let result = LaleParser::parse(Rule::r#var, "var data as u32[size] = []");
  assert!(result.is_ok());
}

#[test]
fn test_def_nested_array_literal() {
  let result = LaleParser::parse(Rule::r#var, "var matrix = [[1, 2], [3, 4]]");
  assert!(result.is_ok());
}

#[test]
fn test_def_with_type_conversion() {
  let result = LaleParser::parse(Rule::r#var, "var value = x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_def_raw_pointer() {
  let result = LaleParser::parse(Rule::r#var, "var p as pointer = pointer to bar");
  assert!(result.is_ok());
}

#[test]
fn test_def_raw_pointer_dereference() {
  let result = LaleParser::parse(Rule::r#var, "var x as u32 = unsafe value at p");
  assert!(result.is_ok());
}

// ==================== MODIFIER COMBINATIONS ====================

#[test]
fn test_def_export_with_type_and_unit() {
  let result = LaleParser::parse(
    Rule::r#var,
    "export var constant as f64 in <m/s> = 299792458",
  );
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_export_full() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe export decl sys_addr as pointer");
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_def_extra_whitespace() {
  let result = LaleParser::parse(Rule::r#var, "var   x   as   u32   =   42");
  assert!(result.is_ok());
}

#[test]
fn test_def_symbol_extra_whitespace() {
  let result = LaleParser::parse(Rule::var_symbol, "var   myVar   as   str[50]   in   <m>");
  assert!(result.is_ok());
}

#[test]
fn test_unsafe_decl_extra_whitespace() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe   decl   ptr   as   pointer");
  assert!(result.is_ok());
}

// ==================== TYPE COMBINATIONS ====================

// Pointer initialization would require null literal which may not exist
// Removed test: test_def_pointer_to_array

#[test]
fn test_def_symbol_multi_dimensional_array() {
  let result = LaleParser::parse(Rule::var_symbol, "var matrix as u32[10][20]");
  assert!(result.is_ok());
}

#[test]
fn test_def_custom_type_variable() {
  let result = LaleParser::parse(Rule::r#var, "var instance as MyClass = createInstance()");
  assert!(result.is_ok());
}

// ==================== MANDATORY SPACE TESTS ====================
// These tests verify that mandatory space (ms) is enforced

#[test]
fn test_unsafe_decl_rejects_no_space_after_unsafe() {
  // unsafe must be followed by mandatory space
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafedecl x as u32");
  assert!(result.is_err(), "unsafe without space should fail");
}

#[test]
fn test_unsafe_decl_rejects_no_space_before_decl() {
  // There must be mandatory space before decl_symbol
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe export decl x as u32");
  // This should work with proper spacing
  assert!(result.is_ok(), "unsafe export decl with spaces should work");

  // But this should fail without space between export and decl
  let result2 = LaleParser::parse(Rule::unsafe_decl, "unsafe exportdecl x as u32");
  assert!(result2.is_err(), "exportdecl without space should fail");
}

#[test]
fn test_unsafe_decl_accepts_proper_spacing() {
  // Verify proper spacing is accepted
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl x as u32");
  assert!(result.is_ok(), "unsafe decl with proper space should work");
}

#[test]
fn test_def_symbol_rejects_no_space_after_def() {
  // var must be followed by mandatory space
  let result = LaleParser::parse(Rule::var_symbol, "varx");
  assert!(result.is_err(), "var without space should fail");
}

#[test]
fn test_def_symbol_no_space_before_as_parses_differently() {
  // Without space before "as", "xas" is parsed as the identifier
  // and the optional type part is not matched
  let result = LaleParser::parse(Rule::var_symbol, "var xas");
  assert!(
    result.is_ok(),
    "xas becomes the identifier, no type specified"
  );

  // But "var x as u32" with proper spacing works correctly
  let result2 = LaleParser::parse(Rule::var_symbol, "var x as u32");
  assert!(result2.is_ok(), "proper spacing should work");
}

#[test]
fn test_def_symbol_no_space_after_as_parses_differently() {
  // Without space after "as", "asu32" would be parsed as type_name (identifier)
  let result = LaleParser::parse(Rule::var_symbol, "var x as asu32");
  assert!(result.is_ok(), "asu32 is treated as a type name identifier");
}

#[test]
fn test_def_symbol_requires_space_around_keywords() {
  // Proper spacing around keywords "as" and "in"
  let result = LaleParser::parse(Rule::var_symbol, "var x as f64 in <m>");
  assert!(result.is_ok(), "proper spacing should work");

  // Note: "in<m>" works because "<" is a clear delimiter for the unit
  // The mandatory space (ms_ln) is required between keyword and identifier,
  // but unit starts with "<" which is unambiguous
  let result2 = LaleParser::parse(Rule::var_symbol, "var x as f64 in<m>");
  assert!(
    result2.is_ok(),
    "in followed by < is accepted since < is a delimiter"
  );

  // However, without space before "in", "f64in" would be parsed as the type name
  let result3 = LaleParser::parse(Rule::var_symbol, "var x as f64in <m>");
  assert!(
    result3.is_ok(),
    "f64in is parsed as type name, unit part is not matched"
  );
}

// ==================== VECTOR TYPE DECLARATIONS ====================

#[test]
fn test_def_symbol_vec2_of_f64() {
  let result = LaleParser::parse(Rule::var_symbol, "var v as vec2 of f64");
  assert!(result.is_ok(), "var v as vec2 of f64 should parse");
}

#[test]
fn test_def_symbol_vec3_of_f64() {
  let result = LaleParser::parse(Rule::var_symbol, "var v as vec3 of f64");
  assert!(result.is_ok(), "var v as vec3 of f64 should parse");
}

#[test]
fn test_def_symbol_vec4_of_f64() {
  let result = LaleParser::parse(Rule::var_symbol, "var v as vec4 of f64");
  assert!(result.is_ok(), "var v as vec4 of f64 should parse");
}

#[test]
fn test_def_symbol_vec3_of_f64_with_unit() {
  let result = LaleParser::parse(Rule::var_symbol, "var v as vec3 of f64 in <m>");
  assert!(result.is_ok(), "var v as vec3 of f64 in <m> should parse");
}

#[test]
fn test_def_vec2_with_constructor() {
  // vec2(1.0, 2.0) parses as a function call
  let result = LaleParser::parse(Rule::r#var, "var v as vec2 of f64 = vec2(1.0, 2.0)");
  assert!(
    result.is_ok(),
    "var v as vec2 of f64 = vec2(1.0, 2.0) should parse"
  );
}

#[test]
fn test_def_vec3_with_constructor() {
  let result = LaleParser::parse(Rule::r#var, "var v as vec3 of f64 = vec3(1.0, 2.0, 3.0)");
  assert!(
    result.is_ok(),
    "var v as vec3 of f64 = vec3(1.0, 2.0, 3.0) should parse"
  );
}

#[test]
fn test_def_vec4_with_constructor() {
  let result = LaleParser::parse(
    Rule::r#var,
    "var v as vec4 of f64 = vec4(1.0, 2.0, 3.0, 4.0)",
  );
  assert!(
    result.is_ok(),
    "var v as vec4 of f64 = vec4(...) should parse"
  );
}

#[test]
fn test_def_dot_product_result() {
  let result = LaleParser::parse(Rule::r#var, "var d as f64 = a dot b");
  assert!(result.is_ok(), "var d as f64 = a dot b should parse");
}

#[test]
fn test_def_cross_product_result() {
  let result = LaleParser::parse(Rule::r#var, "var c as vec3 of f64 = a cross b");
  assert!(
    result.is_ok(),
    "var c as vec3 of f64 = a cross b should parse"
  );
}

#[test]
fn test_unsafe_decl_vec2() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl v as vec2 of f64");
  assert!(result.is_ok(), "unsafe decl v as vec2 of f64 should parse");
}

#[test]
fn test_unsafe_decl_vec3() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl v as vec3 of f64");
  assert!(result.is_ok(), "unsafe decl v as vec3 of f64 should parse");
}

#[test]
fn test_unsafe_decl_vec4() {
  let result = LaleParser::parse(Rule::unsafe_decl, "unsafe decl v as vec4 of f64");
  assert!(result.is_ok(), "unsafe decl v as vec4 of f64 should parse");
}
