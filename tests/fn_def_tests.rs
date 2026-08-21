use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for function definition (fn_def) and function signature (fn_sig) rules.
///
/// This test suite covers:
/// - fn_def: Complete function definitions with body statements
/// - fn_sig: Function signatures (declarations without body)
///
/// Note: fn_def requires at least one statement in the body (like the example files),
/// while fn_sig is for pure declarations without a body.

// ==================== FN_DEF WITH BODY TESTS ====================

#[test]
fn test_fn_def_with_function_call() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn greet() returns nothing\n  print()\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_return_statement() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getValue() returns u32\n  return 42\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_multiple_statements() {
  let code = "fn compute(x as u32) returns u32\n  var y = x + 1\n  return y\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_declaration() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn init() returns nothing\n  var x = 0\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_io() {
  let code = "fn greet(name as str) returns nothing\n  write \"Hello\"\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

// ==================== FN_DEF RETURN TYPES ====================

#[test]
fn test_fn_def_returns_u8() {
  let result = LaleParser::parse(Rule::fn_def, "fn getByte() returns u8\n  return 0\nend fn");
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_i64() {
  let result = LaleParser::parse(Rule::fn_def, "fn getLong() returns i64\n  return 0\nend fn");
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_f32() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getFloat() returns f32\n  return 0.0\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_f64() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getDouble() returns f64\n  return 0.0\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_bool() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn isValid() returns bool\n  return true\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_str() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getName() returns str\n  return \"test\"\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_char() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getChar() returns char\n  return 'x'\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_array() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getArray() returns u32[10]\n  return []\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_pointer() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getPtr() returns pointer\n  return ptr\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_custom_type() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn createNode() returns Node\n  return node\nend fn",
  );
  assert!(result.is_ok());
}

// ==================== FN_DEF WITH UNITS ====================

#[test]
fn test_fn_def_returns_with_unit() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getDistance() returns f64 in <m>\n  return 0.0\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_returns_with_compound_unit() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn getVelocity() returns f64 in <m/s>\n  return 0.0\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_param_with_unit() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn calculate(distance as f64 in <m>) returns f64\n  return distance\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_both_param_and_return_with_units() {
  let code = "fn convertSpeed(v as f64 in <m/s>) returns f64 in <km/h>\n  return v * 3.6\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

// ==================== FN_DEF WITH EXPORT ====================

#[test]
fn test_fn_def_export() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "export fn publicFunc() returns nothing\n  log()\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_export_with_params() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "export fn api(x as u32) returns u32\n  return x\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_export_with_unit() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "export fn getG() returns f64 in <m/s^2>\n  return 9.81\nend fn",
  );
  assert!(result.is_ok());
}

// ==================== FN_DEF WITH COPY PARAMETERS ====================

#[test]
fn test_fn_def_copy_param() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn process(copy x as u32) returns u32\n  return x\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_mixed_copy_params() {
  let code =
    "fn calc(copy a as u32, b as u32, copy c as f64) returns f64\n  return a + b + c\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

// ==================== FN_DEF COMPLEX TYPES ====================

#[test]
fn test_fn_def_pointer_param() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn deref(ptr as pointer) returns u32\n  return unsafe value at ptr\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_array_param() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn sum(arr as u32[10]) returns u32\n  return arr[0]\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_2d_array_param() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn process(matrix as u32[10][10]) returns u32\n  return matrix[0][0]\nend fn",
  );
  assert!(result.is_ok());
}

// ==================== FN_DEF WITH UNICODE ====================

#[test]
fn test_fn_def_unicode_name() {
  let result = LaleParser::parse(Rule::fn_def, "fn αβγ() returns nothing\n  log()\nend fn");
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_unicode_param() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn compute(α as f64, β as f64) returns f64\n  return α + β\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_hiragana_name() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn かんすう() returns nothing\n  log()\nend fn",
  );
  assert!(result.is_ok());
}

// ==================== FN_DEF WHITESPACE HANDLING ====================

#[test]
fn test_fn_def_extra_whitespace() {
  let result = LaleParser::parse(
    Rule::fn_def,
    "fn   foo  (  )   returns   nothing\n  log()\nend fn",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_newlines_in_params() {
  let code = "fn add(\n  a as u32,\n  b as u32\n) returns u32\n  return a + b\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_multiline_body() {
  let code = "fn example() returns nothing\n  var x = 1\n  var y = 2\n  write x + y\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

// ==================== FN_SIG TESTS ====================

#[test]
fn test_fn_sig_minimal() {
  let result = LaleParser::parse(Rule::fn_signature, "fn signature foo() returns nothing");
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_space_only() {
  let result = LaleParser::parse(Rule::fn_signature, "fn signature foo() returns nothing");
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_space_before_end() {
  let result = LaleParser::parse(Rule::fn_def, "fn foo() returns nothing log() end fn");
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_with_params() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "fn signature add(a as u32, b as u32) returns u32",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_with_unit() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "fn signature getSpeed() returns f64 in <m/s>",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_import() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "import fn signature sqrt(x as f64) returns f64",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_copy_param() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "fn signature process(copy x as u32) returns u32",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_pointer_return() {
  let result = LaleParser::parse(Rule::fn_signature, "fn signature alloc() returns pointer");
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_nothing_param() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "fn signature empty(nothing) returns nothing",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_array_return() {
  let result = LaleParser::parse(Rule::fn_signature, "fn signature getData() returns u8[256]");
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_custom_type_return() {
  let result = LaleParser::parse(Rule::fn_signature, "fn signature create() returns MyStruct");
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_complex_params() {
  let code = "fn signature transform(input as u32[100], copy scale as f64 in <m>) returns u32[100]";
  let result = LaleParser::parse(Rule::fn_signature, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_multiple_params() {
  let result = LaleParser::parse(
    Rule::fn_signature,
    "fn signature add(a as u32, b as u32) returns u32",
  );
  assert!(result.is_ok());
}

#[test]
fn test_fn_sig_all_int_types() {
  let types = ["u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64"];
  for t in types {
    let code = format!("fn signature get() returns {}", t);
    let result = LaleParser::parse(Rule::fn_signature, &code);
    assert!(result.is_ok(), "Failed for type: {}", t);
  }
}

#[test]
fn test_fn_sig_all_float_types() {
  let types = ["f16", "f32", "f64"];
  for t in types {
    let code = format!("fn signature get() returns {}", t);
    let result = LaleParser::parse(Rule::fn_signature, &code);
    assert!(result.is_ok(), "Failed for type: {}", t);
  }
}

// ==================== REALISTIC FUNCTION EXAMPLES ====================

#[test]
fn test_fn_def_fibonacci() {
  let code = "fn fibonacci(n as u32) returns u32\n  if n < 2\n    return n\n  end if\n  return fibonacci(n - 1) + fibonacci(n - 2)\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_physics_calculation() {
  let code = "export fn calculateForce(mass as f64 in <kg>, acceleration as f64 in <m/s^2>) returns f64 in <kg*m/s^2>\n  return mass * acceleration\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_loop() {
  let code = "fn sumArray(arr as u32[10]) returns u32\n  var sum as u32 = 0\n  loop over i as u32 from 0 to 10\n    var sum = sum + arr[i]\n  end loop\n  return sum\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_with_conditionals() {
  let code = "fn max(a as u32, b as u32) returns u32\n  if a > b\n    return a\n  else\n    return b\n  end if\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}

#[test]
fn test_fn_def_example_from_file() {
  let code = "export fn fn1(p1 as i32 in <s>, copy p2 as i32 in <g>) returns nothing\n  write \"Hallo\"\n  var bar as i64 in <kg⁻³³> = 2+3\n  return -2+3*4^5\nend fn";
  let result = LaleParser::parse(Rule::fn_def, code);
  assert!(result.is_ok());
}
