//! Tests for semantic type checking errors.
//!
//! This test suite comprehensively covers all type-related semantic errors:
//! - Binary operation type mismatches
//! - Variable definition type mismatches
//! - Assignment type mismatches
//! - Condition type requirements (must be bool)
//! - Type name variations (case-insensitive handling)

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== HELPER FUNCTIONS ====================

fn analyze_code(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error_containing(
  analyzer: &lale::semantic_analysis::OwnedAnalyzer,
  substring: &str,
) -> bool {
  analyzer.get_errors().iter().any(|e| e.contains(substring))
}

// ==================== BINARY OPERATION TYPE MISMATCH TESTS ====================

#[test]
fn test_semantic_binary_string_int_comparison() {
  let code = r#"
var name as text = "Alice"
var age as i32 = 30
if name > age
    write "should not reach"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject string > int comparison"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in greater-than operation"
  ));
}

#[test]
fn test_semantic_binary_string_arithmetic() {
  let code = r#"
var text as text = "hello"
var num as i32 = 5
var result as i32 = text + num
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject string + int addition");
  assert!(has_error_containing(&analyzer, "Type mismatch in addition"));
}

#[test]
fn test_semantic_binary_bool_arithmetic() {
  let code = r#"
var flag as bool = true
var num as i32 = 5
var result as i32 = flag + num
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject bool + int addition");
  assert!(has_error_containing(&analyzer, "Type mismatch in addition"));
}

#[test]
fn test_semantic_binary_string_multiplication() {
  let code = r#"
var text as text = "hello"
var factor as f64 = 2.0
var result as text = text * factor
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject string * float multiplication"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in multiplication"
  ));
}

#[test]
fn test_semantic_binary_string_division() {
  let code = r#"
var text as text = "hello"
var divisor as i32 = 2
var result as text = text / divisor
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject string / int division");
  assert!(has_error_containing(&analyzer, "Type mismatch in division"));
}

#[test]
fn test_semantic_binary_char_int_comparison() {
  let code = r#"
var letter as char = 'A'
var num as i32 = 65
if letter > num
    write "should not reach"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject char > int comparison");
}

#[test]
fn test_semantic_binary_string_bitwise() {
  let code = r#"
var text as text = "hello"
var mask as u8 = 0xFF
var result as u8 = text bitwise and mask
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject string bitwise and u8");
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in bitwise AND"
  ));
}

// ==================== BINARY OPERATION NUMERIC COMPATIBILITY TESTS ====================
// Lale enforces strict type checking: no implicit conversions allowed!
// Different numeric types cannot be used in binary operations without explicit casting.

#[test]
fn test_semantic_binary_numeric_int_float_comparison_requires_conversion() {
  let code = r#"
var x as i32 = 10
var y as f64 = 3.14
if x > y
    write "numeric comparison works"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject int > float comparison without explicit conversion"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in greater-than operation"
  ));
}

#[test]
fn test_semantic_binary_numeric_int_float_comparison_with_conversion() {
  let code = r#"
var x as i32 = 10
var y as f64 = 3.14
if (x as f64) > y
    write "numeric comparison works"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow comparison with explicit conversion"
  );
}

#[test]
fn test_semantic_binary_numeric_float_int_arithmetic_requires_conversion() {
  let code = r#"
var x as f64 = 5.5
var y as i32 = 3
var result as f64 = x + y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject float + int addition without explicit conversion"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch in addition"));
}

#[test]
fn test_semantic_binary_numeric_float_int_arithmetic_with_conversion() {
  let code = r#"
var x as f64 = 5.5
var y as i32 = 3
var result as f64 = x + (y as f64)
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow addition with explicit conversion"
  );
}

#[test]
fn test_semantic_binary_numeric_different_int_sizes_requires_conversion() {
  let code = r#"
var x as u8 = 5
var y as i32 = 10
var result as i32 = x + y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject u8 + i32 addition without explicit conversion"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch in addition"));
}

#[test]
fn test_semantic_binary_numeric_different_int_sizes_with_conversion() {
  let code = r#"
var x as u8 = 5
var y as i32 = 10
var result as i32 = (x as i32) + y
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow addition with explicit conversion"
  );
}

#[test]
fn test_semantic_binary_numeric_u64_f32_comparison_requires_conversion() {
  let code = r#"
var x as u64 = 100
var y as f32 = 50.0
if x > y
    write "numeric comparison works"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject u64 > f32 comparison without explicit conversion"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in greater-than operation"
  ));
}

#[test]
fn test_semantic_binary_numeric_u64_f32_comparison_with_conversion() {
  let code = r#"
var x as u64 = 100
var y as f32 = 50.0
if (x as f32) > y
    write "numeric comparison works"
else
    move on
end if
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow comparison with explicit conversion"
  );
}

#[test]
fn test_semantic_binary_numeric_bitwise_different_sizes_requires_conversion() {
  let code = r#"
var x as u8 = 5
var y as u32 = 3
var result as u8 = x bitwise and y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject u8 bitwise and u32 without explicit conversion"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in bitwise AND"
  ));
}

#[test]
fn test_semantic_binary_numeric_bitwise_different_sizes_with_conversion() {
  // Design decision: variable narrowing conversions are rejected
  // because the compiler cannot verify the value fits at compile time.
  // Use literals instead: `x bitwise and 3 as u8`
  let code = r#"
var x as u8 = 5
var y as u32 = 3
var result as u8 = x bitwise and (y as u8)
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject variable narrowing conversion (u32 variable to u8)"
  );
  assert!(has_error_containing(&analyzer, "Narrowing"));
}

// ==================== VARIABLE DEFINITION TYPE MISMATCH TESTS ====================

#[test]
fn test_semantic_def_string_to_int() {
  let code = r#"
var x as i32 = "hello"
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject string initializer for int variable"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch for 'x'"));
}

#[test]
fn test_semantic_def_bool_to_int() {
  let code = r#"
var flag as i32 = true
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject bool initializer for int variable"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch for 'flag'"));
}

#[test]
fn test_semantic_def_int_to_string() {
  let code = r#"
var name as text = 42
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject int initializer for string variable"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch for 'name'"));
}

#[test]
fn test_semantic_def_numeric_literal_int() {
  let code = r#"
var x as f64 = 42
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow numeric literal to infer type (int -> f64)"
  );
}

#[test]
fn test_semantic_def_numeric_literal_float_to_int_rejected() {
  let code = r#"
var x as i32 = 3.14
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject float literal initializer for i32 (float → integer drops the fractional part)"
  );
}

#[test]
fn test_semantic_def_numeric_variable_type_mismatch() {
  let code = r#"
var x as f64 = 5.0
var y as i32 = x
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject f64 variable initializer for i32 (no implicit conversion)"
  );
  assert!(has_error_containing(&analyzer, "Type mismatch for 'y'"));
}

#[test]
fn test_semantic_def_float_to_int_conversion_rejected() {
  // Design decision: float to integer is always rejected (loses fractional part)
  // Use truncation functions or intermediate steps if this is intentional
  let code = r#"
var x as f64 = 5.0
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject float to integer conversion (f64 to i32)"
  );
  assert!(has_error_containing(&analyzer, "Narrowing"));
}

// ==================== ASSIGNMENT TYPE MISMATCH TESTS ====================

#[test]
fn test_semantic_assign_string_to_int() {
  let code = r#"
var x as i32 = 0
x = "hello"
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject string assignment to int variable"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in assignment to 'x'"
  ));
}

#[test]
fn test_semantic_assign_bool_to_float() {
  let code = r#"
var flag as f64 = 0.0
flag = true
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject bool assignment to float variable"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in assignment to 'flag'"
  ));
}

#[test]
fn test_semantic_assign_int_to_string() {
  let code = r#"
var name as text = "Alice"
name = 42
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject int assignment to string variable"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in assignment to 'name'"
  ));
}

#[test]
fn test_semantic_assign_numeric_variable_to_different_type() {
  let code = r#"
var x as f64 = 5.0
var y as i32 = 0
y = x
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject f64 variable assignment to i32 (no implicit conversion)"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in assignment to 'y'"
  ));
}

#[test]
fn test_semantic_assign_float_to_int_conversion_rejected() {
  // Design decision: float to integer is always rejected (loses fractional part)
  let code = r#"
var x as f64 = 5.0
var y as i32 = 0
y = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject float to integer conversion in assignment (f64 to i32)"
  );
  assert!(has_error_containing(&analyzer, "Narrowing"));
}

// ==================== CONDITION TYPE TESTS ====================

#[test]
fn test_semantic_condition_must_be_bool() {
  let code = r#"
	var x as i32 = 5
	if x
	    write "should not reach"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject non-boolean condition");
  assert!(has_error_containing(
    &analyzer,
    "Condition must be boolean; got 'i32'"
  ));
}

#[test]
fn test_semantic_condition_string_not_bool() {
  let code = r#"
	var text as text = "hello"
	if text
	    write "should not reach"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject string as condition");
  assert!(has_error_containing(&analyzer, "Condition must be boolean"));
}

#[test]
fn test_semantic_condition_float_not_bool() {
  let code = r#"
	var x as f64 = 3.14
	if x
	    write "should not reach"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "Should reject float as condition");
  assert!(has_error_containing(&analyzer, "Condition must be boolean"));
}

#[test]
fn test_semantic_condition_comparison_is_bool() {
  let code = r#"
	var x as i32 = 5
	if x > (0 as i32)
	    write "condition is bool"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should accept comparison (which yields bool) as condition"
  );
}

#[test]
fn test_semantic_condition_bool_literal_valid() {
  let code = r#"
	if true
	    write "bool literal is valid"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should accept bool literal as condition"
  );
}

#[test]
fn test_semantic_condition_bool_variable_valid() {
  let code = r#"
	var flag as bool = true
	if flag
	    write "bool variable is valid"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should accept bool variable as condition"
  );
}

// ==================== LOOP CONDITION TYPE TESTS ====================

#[test]
fn test_semantic_loop_while_condition_must_be_bool() {
  let code = r#"
var x as i32 = 0
loop when x
    write "should not reach"
end loop
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject non-boolean loop condition"
  );
  assert!(has_error_containing(&analyzer, "Condition must be boolean"));
}

#[test]
fn test_semantic_loop_do_while_condition_must_be_bool() {
  let code = r#"
var x as i32 = 0
loop
    write "loop"
end loop when x
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject non-boolean do-while condition"
  );
  assert!(has_error_containing(&analyzer, "Condition must be boolean"));
}

#[test]
fn test_semantic_loop_while_comparison_valid() {
  let code = r#"
var x as i32 = 0
loop when x < (5 as i32)
    write "loop"
end loop
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should accept comparison (which yields bool) in loop condition"
  );
}

// ==================== BIT SHIFT OPERATOR TYPE TESTS ====================
// Bit shifts require both operands to be the SAME type (LLVM semantics)

#[test]
fn test_semantic_shift_left_same_types() {
  let code = r#"
var x as u32 = 5
var y as u32 = 2
var result as u32 = x unsigned left shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow u32 unsigned left shift u32 (same types)"
  );
}

#[test]
fn test_semantic_shift_left_different_types() {
  let code = r#"
var x as u8 = 5
var y as u32 = 2
var result as u8 = x unsigned left shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject u8 unsigned left shift u32 (different types)"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in unsigned left shift operation"
  ));
}

#[test]
fn test_semantic_shift_right_same_types() {
  let code = r#"
var x as i32 = 20
var y as i32 = 2
var result as i32 = x signed right shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow i32 signed right shift i32 (same types)"
  );
}

#[test]
fn test_semantic_shift_right_different_types() {
  let code = r#"
var x as i32 = 20
var y as u32 = 2
var result as i32 = x signed right shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject i32 signed right shift u32 (different types)"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in signed right shift operation"
  ));
}

#[test]
fn test_semantic_shift_unsigned_right_same_types() {
  let code = r#"
var x as u32 = 20
var y as u32 = 2
var result as u32 = x unsigned right shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow u32 unsigned right shift u32 (same types)"
  );
}

#[test]
fn test_semantic_shift_unsigned_right_different_types() {
  let code = r#"
var x as u64 = 20
var y as u32 = 2
var result as u64 = x unsigned right shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject u64 unsigned right shift u32 (different types)"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in unsigned right shift operation"
  ));
}

#[test]
fn test_semantic_shift_mixed_signed_unsigned() {
  let code = r#"
var x as i16 = 10
var y as u16 = 1
var result as i16 = x unsigned left shift y
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject i16 unsigned left shift u16 (signed vs unsigned)"
  );
  assert!(has_error_containing(
    &analyzer,
    "Type mismatch in unsigned left shift operation"
  ));
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_semantic_binary_operation_with_explicit_conversion() {
  let code = r#"
var x as f64 = 5.0
var y as i32 = 3
var result as f64 = x + (y as f64)
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow explicit conversion: int to float via 'as'"
  );
}

#[test]
fn test_semantic_power_with_float_base_and_int_exponent() {
  let code = r#"
var x as f64 = 5.0
var y as i32 = 3
var result as f64 = x ^ y
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow float ^ int (matches LLVM llvm.powi semantics)"
  );
}

#[test]
fn test_semantic_power_with_float_base_and_u64_exponent() {
  let code = r#"
var base as f64 = 2.0
var exp as u64 = 10
var result as f64 = base ^ exp
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Should allow float ^ any integer type");
}

#[test]
fn test_semantic_power_with_int_base_and_int_exponent() {
  let code = r#"
var x as i32 = 5
var y as i32 = 3
var result as i32 = x ^ y
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Should allow int ^ int (same types)");
}

#[test]
fn test_semantic_comparison_cross_numeric_with_units() {
  let code = r#"
	var distance as f64 in <m> = 10.0
	var time as i32 in <s> = 5
	if distance > time
	    write "should not reach"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject comparison of quantities with different units"
  );
  // Unit check should fail, not type check
  assert!(has_error_containing(
    &analyzer,
    "Cannot compare quantities with different units"
  ));
}

#[test]
fn test_semantic_numeric_operations_require_matching_types() {
  let test_pairs = vec![
    ("i8", "i16"),
    ("i32", "i64"),
    ("u8", "u32"),
    ("u16", "u64"),
    ("f32", "f64"),
    ("i32", "f64"),
    ("u8", "f32"),
  ];

  for (type1, type2) in test_pairs {
    let code = format!(
      r#"
var x as {} = 0
var y as {} = 0
var result as {} = x + y
"#,
      type1, type2, type1
    );
    let analyzer = analyze_code(&code);
    assert!(
      !analyzer.is_valid() && has_error_containing(&analyzer, "Type mismatch in addition"),
      "Should reject {} + {} without explicit conversion",
      type1,
      type2
    );
  }
}

// ==================== BACKEND-SPECIFIC TYPE RESTRICTION TESTS ====================

use lale::compiler_options::CompilerOptions;
use lale::semantic_analysis::analyze_ast_with_options;

fn analyze_code_with_backend(
  code: &str,
  options: CompilerOptions,
) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast_with_options(&program, options)
}

#[test]
fn test_f16_allowed_with_interpreter_backend() {
  let code = r#"
var x as f16 = 3.14
"#;
  let analyzer = analyze_code_with_backend(code, CompilerOptions::default());
  assert!(
    analyzer.is_valid(),
    "F16 should be allowed with interpreter backend, but got errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_default_backend_is_interpreter() {
  let options = CompilerOptions::default();
  assert_eq!(
    options.backend,
    lale::compiler_options::Backend::Interpreter
  );
}

#[test]
fn test_f16_allowed_with_default_backend() {
  let code = r#"
var x as f16 = 3.14
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "F16 should be allowed with default (interpreter) backend"
  );
}

// ==================== UNDEFINED FUNCTION TESTS ====================

#[test]
fn test_undefined_function_call_error() {
  let code = r#"
var result as i32 = undefined_function(5)
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject call to undefined function"
  );
  assert!(has_error_containing(
    &analyzer,
    "Function 'undefined_function' is not defined"
  ));
}

#[test]
fn test_undefined_function_in_expression() {
  let code = r#"
var x as i32 = 10
var y as i32 = x + isNegative(x)
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject undefined function in expression"
  );
  assert!(has_error_containing(
    &analyzer,
    "Function 'isNegative' is not defined"
  ));
}

#[test]
fn test_undefined_function_in_condition() {
  let code = r#"
	var x as i32 = 5
	if checkValue(x)
	    write "ok"
	else
	    move on
	end if
	"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject undefined function in condition"
  );
  assert!(has_error_containing(
    &analyzer,
    "Function 'checkValue' is not defined"
  ));
}

#[test]
fn test_type_constructor_not_flagged_as_undefined() {
  let code = r#"
type Person
    name as text
    age as i32
end type

var p as Person = Person("Alice", 30)
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Type constructor should not be flagged as undefined function"
  );
}

#[test]
fn test_defined_function_not_flagged_as_undefined() {
  let code = r#"
fn myFunc(x as i32) returns i32
    return x * (2 as i32)
end fn

var result as i32 = myFunc(5)
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Defined function should not be flagged as undefined"
  );
}

// ==================== VECTOR TYPE TESTS ====================

#[test]
fn test_vector_dot_product_valid_vec2() {
  let code = r#"
unsafe decl a as vec2 of f64
unsafe decl b as vec2 of f64
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "vec2 · vec2 dot product should be valid"
  );
}

#[test]
fn test_vector_dot_product_valid_vec3() {
  let code = r#"
unsafe decl a as vec3 of f64
unsafe decl b as vec3 of f64
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "vec3 · vec3 dot product should be valid"
  );
}

#[test]
fn test_vector_dot_product_different_dimensions() {
  let code = r#"
unsafe decl a as vec2 of f64
unsafe decl b as vec3 of f64
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "vec2 · vec3 should be rejected");
  assert!(has_error_containing(&analyzer, "same dimension"));
}

#[test]
fn test_vector_dot_product_different_inner_types() {
  let code = r#"
unsafe decl a as vec3 of f64
unsafe decl b as vec3 of i32
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "vec3<f64> · vec3<i32> should be rejected"
  );
}

#[test]
fn test_vector_cross_product_valid_vec3() {
  let code = r#"
unsafe decl a as vec3 of f64
unsafe decl b as vec3 of f64
var result as vec3 of f64 = a cross b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "vec3 × vec3 cross product should be valid"
  );
}

#[test]
fn test_vector_cross_product_vec2_rejected() {
  let code = r#"
unsafe decl a as vec2 of f64
unsafe decl b as vec2 of f64
var result as vec2 of f64 = a cross b
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "vec2 × vec2 cross product should be rejected"
  );
  assert!(has_error_containing(
    &analyzer,
    "Cross product is not defined for 2D vectors"
  ));
}

#[test]
fn test_vector_cross_product_vec4_rejected() {
  let code = r#"
unsafe decl a as vec4 of f64
unsafe decl b as vec4 of f64
var result as vec4 of f64 = a cross b
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "vec4 × vec4 cross product should be rejected"
  );
  assert!(has_error_containing(
    &analyzer,
    "Cross product is only defined for 3D vectors"
  ));
}

#[test]
fn test_vector_cross_product_different_inner_types() {
  let code = r#"
unsafe decl a as vec3 of f64
unsafe decl b as vec3 of i32
var result as vec3 of f64 = a cross b
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "vec3<f64> × vec3<i32> should be rejected"
  );
  assert!(has_error_containing(&analyzer, "same inner type"));
}

#[test]
fn test_vector_mul_vec_rejected_with_helpful_message() {
  let code = r#"
unsafe decl a as vec3 of f64
unsafe decl b as vec3 of f64
var result as vec3 of f64 = a * b
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "vec * vec should be rejected");
  assert!(has_error_containing(
    &analyzer,
    "Cannot multiply two vectors with '*'"
  ));
}

#[test]
fn test_vector_dot_product_valid_vec4() {
  let code = r#"
unsafe decl a as vec4 of f64
unsafe decl b as vec4 of f64
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "vec4 · vec4 dot product should be valid"
  );
}

#[test]
fn test_vector_dot_scalar_valid_i32() {
  let code = r#"
var a as i32 = 10
var b as i32 = 20
var result as i32 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i32 · i32 scalar dot product should be valid"
  );
}

#[test]
fn test_vector_dot_scalar_mixed_rejected() {
  let code = r#"
var a as i32 = 10
var b as f64 = 20.0
var result as f64 = a dot b
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "i32 · f64 should be rejected");
  assert!(has_error_containing(&analyzer, "same type"));
}

#[test]
fn test_vector_cross_vec3_i32_valid() {
  let code = r#"
unsafe decl a as vec3 of i32
unsafe decl b as vec3 of i32
var result as vec3 of i32 = a cross b
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "vec3<i32> × vec3<i32> cross product should be valid"
  );
}

#[test]
fn test_vector_mul_vec2_rejected_with_message() {
  let code = r#"
unsafe decl a as vec2 of f64
unsafe decl b as vec2 of f64
var result as vec2 of f64 = a * b
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid(), "vec2 * vec2 should be rejected");
  assert!(has_error_containing(
    &analyzer,
    "Cannot multiply two vectors with '*'"
  ));
}
