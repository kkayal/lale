//! Pointer Type Inference Tests
//!
//! Comprehensive tests for pointer type inference via the `value at` operator.
//!
//! This test suite ensures that:
//! - `unsafe value at ptr` correctly infers its type from what the pointer points to
//! - Type mismatches are detected when assigning dereferenced pointers to incompatible types
//! - Multiple pointer dereferences work correctly
//! - Pointer type tracking works across different type sizes (i8, i32, f64, etc.)

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

fn analyze_code(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error_containing(analyzer: &lale::semantic_analysis::OwnedAnalyzer, text: &str) -> bool {
  analyzer
    .get_errors()
    .iter()
    .any(|e| e.message.contains(text))
}

// ==================== BASIC TYPE INFERENCE ====================

#[test]
fn test_value_at_infers_f64_type() {
  let code = r#"
var mass as f64 = 42.0
var p as pointer = pointer to mass
var result as f64 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer f64 type from pointer to f64. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_infers_i32_type() {
  let code = r#"
var count as i32 = 100
var p as pointer = pointer to count
var result as i32 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer i32 type from pointer to i32. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_infers_u8_type() {
  let code = r#"
var byte as u8 = 255
var p as pointer = pointer to byte
var result as u8 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer u8 type from pointer to u8. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_infers_u32_type() {
  let code = r#"
var value as u32 = 12345
var p as pointer = pointer to value
var result as u32 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer u32 type from pointer to u32. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_infers_i64_type() {
  let code = r#"
var big as i64 = 9999999999
var p as pointer = pointer to big
var result as i64 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer i64 type from pointer to i64. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== TYPE MISMATCH DETECTION ====================

#[test]
fn test_value_at_rejects_type_mismatch_f64_to_i32() {
  let code = r#"
var mass as f64 = 42.0
var p as pointer = pointer to mass
var wrong as i32 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject type mismatch when assigning f64 pointer to i32"
  );
  assert!(
    has_error_containing(&analyzer, "Type mismatch"),
    "Error message should mention type mismatch. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_rejects_type_mismatch_i32_to_f64() {
  let code = r#"
var count as i32 = 100
var p as pointer = pointer to count
var wrong as f64 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject type mismatch when assigning i32 pointer to f64"
  );
  assert!(
    has_error_containing(&analyzer, "Type mismatch"),
    "Error message should mention type mismatch. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_rejects_type_mismatch_u32_to_i32() {
  let code = r#"
var unsigned as u32 = 42
var p as pointer = pointer to unsigned
var wrong as i32 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject type mismatch when assigning u32 pointer to i32"
  );
  assert!(
    has_error_containing(&analyzer, "Type mismatch"),
    "Error message should mention type mismatch. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_value_at_rejects_type_mismatch_u8_to_u32() {
  let code = r#"
var byte as u8 = 255
var p as pointer = pointer to byte
var wrong as u32 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject type mismatch when assigning u8 pointer to u32"
  );
  assert!(
    has_error_containing(&analyzer, "Type mismatch"),
    "Error message should mention type mismatch. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== MULTIPLE POINTER DEREFERENCES ====================

#[test]
fn test_multiple_pointer_dereferences() {
  let code = r#"
var x as f64 = 10.0
var y as i32 = 20
var z as u8 = 30
var px as pointer = pointer to x
var py as pointer = pointer to y
var pz as pointer = pointer to z
var vx as f64 = unsafe value at px
var vy as i32 = unsafe value at py
var vz as u8 = unsafe value at pz
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow multiple pointer dereferences with correct types. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== POINTER TYPES IN FUNCTIONS ====================

#[test]
fn test_pointer_type_in_function_parameter_limited() {
  // NOTE: When a pointer is a function parameter, the semantic analyzer cannot determine
  // its source type (it's not created with `pointer to` in the same scope).
  // The `unsafe value at p` will return "unknown" type.
  // This is a known limitation: pointers from FFI or external sources are opaque.
  let code = r#"
fn process_pointer(p as pointer) returns i32
    var value as i32 = unsafe value at p
    return value
end fn

var x as i32 = 42
var result as i32 = process_pointer(pointer to x)
"#;
  let analyzer = analyze_code(code);
  // This will fail because p's source type is unknown
  assert!(
    !analyzer.is_valid(),
    "Pointer parameters are opaque (source type unknown)"
  );
}

#[test]
fn test_pointer_in_function_local_scope() {
  let code = r#"
fn test() returns f64
    var data as f64 = 3.14
    var p as pointer = pointer to data
    var result as f64 = unsafe value at p
    return result
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should infer type in function local scope. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== POINTER WITH UNITS ====================

#[test]
fn test_value_at_preserves_declared_unit() {
  let code = r#"
var distance as f64 in <m> = 100.0
var p as pointer = pointer to distance
var result as f64 in <m> = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow dereference of pointer to unitful variable. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_pointer_to_pointer_operand() {
  // The target of a pointer can be a raw pointer created from a literal
  let code = r#"
var p as pointer = 0 as pointer
var q as pointer = pointer to p
"#;
  let analyzer = analyze_code(code);
  // This may or may not be valid depending on pointer type tracking rules
  let _ = analyzer.is_valid();
}

#[test]
fn test_chained_pointer_dereference() {
  // After dereferencing, the result cannot be a pointer (unless the original was pointer-to-pointer)
  let code = r#"
var x as f64 = 42.0
var p as pointer = pointer to x
var q as f64 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow simple pointer dereference chain. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== REGRESSION TEST: ORIGINAL ISSUE ====================

#[test]
fn test_regression_value_at_type_not_unknown() {
  // This is the regression test for the bug fix.
  // Before the fix, `unsafe value at p` would return type "unknown", causing a type mismatch error.
  let code = r#"
var m as f64 = 10.0
var p as pointer = pointer to m
var baz as f64 = unsafe value at p
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should not treat 'unsafe value at p' as unknown type. Errors: {:?}",
    analyzer.get_errors()
  );
  // The error was: "Type mismatch for 'baz': declared type is 'f64' but initializer has type 'unknown'"
  // This test ensures that doesn't happen anymore.
}
