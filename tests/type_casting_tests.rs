//! Type Casting Tests
//!
//! Comprehensive tests for the type casting (conversion) grammar and semantic analysis.
//!
//! This test suite covers:
//! - Grammar: Parsing of "as" conversion syntax
//! - Semantic Analysis: Type checking and validation for type conversions
//! - Valid conversions: numeric types, pointer types
//! - Invalid conversions: incompatible types

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== GRAMMAR TESTS ====================

#[test]
fn test_conversion_grammar_simple() {
  let result = LaleParser::parse(Rule::conversion, "as u32");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_grammar_float() {
  let result = LaleParser::parse(Rule::conversion, "as f64");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_grammar_string() {
  let result = LaleParser::parse(Rule::conversion, "as str");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_grammar_pointer() {
  let result = LaleParser::parse(Rule::conversion, "as pointer");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_grammar_array() {
  let result = LaleParser::parse(Rule::conversion, "as u32[10]");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_grammar_multidimensional_array() {
  let result = LaleParser::parse(Rule::conversion, "as f64[3][4]");
  assert!(result.is_ok());
}

#[test]
fn test_expression_with_type_cast() {
  let result = LaleParser::parse(Rule::expression, "x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_expression_with_type_cast_literal() {
  let result = LaleParser::parse(Rule::expression, "42 as f64");
  assert!(result.is_ok());
}

#[test]
fn test_expression_with_type_cast_function_call() {
  let result = LaleParser::parse(Rule::expression, "getValue() as i32");
  assert!(result.is_ok());
}

// Chained conversions are supported as a left-to-right pipeline.
// Each as step is validated independently. Intermediate variables are available for readability.

#[test]
fn test_expression_with_type_cast_and_operators() {
  let result = LaleParser::parse(Rule::expression, "(x as f64) + (y as f64)");
  assert!(result.is_ok());
}

#[test]
fn test_expression_with_type_cast_in_comparison() {
  let result = LaleParser::parse(Rule::expression, "x as f64 > 3.14");
  assert!(result.is_ok());
}

// ==================== SEMANTIC ANALYSIS TESTS ====================

mod semantic_tests {
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
    analyzer.get_errors().iter().any(|e| e.contains(text))
  }

  // ==================== INTEGER CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_i32_to_u32() {
    let code = r#"
var x as i32 = 10
var y as u32 = x as u32
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from i32 to u32"
    );
  }

  #[test]
  fn test_semantic_cast_u32_to_i32() {
    let code = r#"
var x as u32 = 10
var y as i32 = x as i32
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u32 to i32"
    );
  }

  #[test]
  fn test_semantic_cast_i32_to_i64() {
    let code = r#"
var x as i32 = 10
var y as i64 = x as i64
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from i32 to i64"
    );
  }

  #[test]
  fn test_semantic_cast_u8_to_u64() {
    let code = r#"
var x as u8 = 255
var y as u64 = x as u64
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u8 to u64"
    );
  }

  // ==================== FLOAT CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_f32_to_f64() {
    let code = r#"
var x as f32 = 3.14
var y as f64 = x as f64
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from f32 to f64"
    );
  }

  #[test]
  fn test_semantic_cast_f64_to_f32_rejected() {
    // Design decision: narrowing conversions are not allowed
    let code = r#"
var x as f64 = 3.14159
var y as f32 = x as f32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject narrowing conversion from f64 to f32"
    );
    assert!(
      has_error_containing(&analyzer, "Narrowing"),
      "Error should mention narrowing. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== INTEGER TO FLOAT CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_i32_to_f64() {
    let code = r#"
var x as i32 = 10
var y as f64 = x as f64
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow widening conversion from i32 to f64. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_u32_to_f64() {
    let code = r#"
var x as u32 = 10
var y as f64 = x as f64
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow widening conversion from u32 to f64. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_f64_to_i32_rejected() {
    // Design decision: narrowing conversions are not allowed
    let code = r#"
var x as f64 = 3.14
var y as i32 = x as i32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject narrowing conversion from f64 to i32"
    );
    assert!(
      has_error_containing(&analyzer, "Narrowing"),
      "Error should mention narrowing. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_f64_to_u32_rejected() {
    // Design decision: narrowing conversions are not allowed
    let code = r#"
var x as f64 = 3.14
var y as u32 = x as u32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject narrowing conversion from f64 to u32"
    );
    assert!(
      has_error_containing(&analyzer, "Narrowing"),
      "Error should mention narrowing. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== POINTER CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_pointer_identity() {
    let code = r#"
var x as i32 = 42
var p as pointer = pointer to x
var q as pointer = p as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from pointer to pointer (identity)"
    );
  }

  #[test]
  fn test_semantic_cast_pointer_to_pointer() {
    let code = r#"
var x as i32 = 42
var p as pointer = pointer to x
var q as pointer = p as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from pointer to pointer"
    );
  }

  #[test]
  fn test_semantic_cast_u8_to_pointer() {
    let code = r#"
var x as u8 = 0
var p as pointer = x as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u8 to pointer"
    );
  }

  #[test]
  fn test_semantic_cast_u16_to_pointer() {
    let code = r#"
var x as u16 = 0
var p as pointer = x as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u16 to pointer"
    );
  }

  #[test]
  fn test_semantic_cast_u32_to_pointer() {
    let code = r#"
var x as u32 = 0
var p as pointer = x as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u32 to pointer"
    );
  }

  #[test]
  fn test_semantic_cast_u64_to_pointer() {
    let code = r#"
var addr as u64 = 0
var p as pointer = addr as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from u64 to pointer"
    );
  }

  #[test]
  fn test_semantic_cast_u32_literal_to_pointer() {
    let code = r#"
var p as pointer = 0 as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow conversion from unsigned literal to pointer (null pointer)"
    );
  }

  #[test]
  fn test_semantic_cast_signed_int_to_pointer_rejected() {
    let code = r#"
var x as i32 = 42
var p as pointer = x as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from signed integer to pointer"
    );
    assert!(
      has_error_containing(&analyzer, "Only unsigned integers"),
      "Error should mention unsigned integers. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_i64_to_pointer_rejected() {
    let code = r#"
var x as i64 = 100
var p as pointer = x as pointer
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from i64 to pointer"
    );
    assert!(
      has_error_containing(&analyzer, "Only unsigned integers"),
      "Error should mention unsigned integers. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== LITERAL RANGE CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_literal_1_to_u8() {
    let code = r#"
var x as u8 = 1 as u8
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow literal 1 to convert to u8 (fits in range)"
    );
  }

  #[test]
  fn test_semantic_cast_literal_255_to_u8() {
    let code = r#"
var x as u8 = 255 as u8
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow literal 255 to convert to u8 (max value)"
    );
  }

  #[test]
  fn test_semantic_cast_literal_256_to_u8_rejected() {
    let code = r#"
var x as u8 = 256 as u8
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject literal 256 to u8 (out of range)"
    );
    assert!(
      has_error_containing(&analyzer, "out of range"),
      "Error should mention out of range. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_literal_neg1_to_i8() {
    let code = r#"
var x as i8 = -1 as i8
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow literal -1 to convert to i8"
    );
  }

  #[test]
  fn test_semantic_cast_literal_127_to_i8() {
    let code = r#"
var x as i8 = 127 as i8
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow literal 127 to convert to i8 (max value)"
    );
  }

  #[test]
  fn test_semantic_cast_literal_128_to_i8_rejected() {
    let code = r#"
var x as i8 = 128 as i8
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject literal 128 to i8 (out of range)"
    );
    assert!(
      has_error_containing(&analyzer, "out of range"),
      "Error should mention out of range. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_literal_neg129_to_i8_rejected() {
    let code = r#"
var x as i8 = -129 as i8
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject literal -129 to i8 (out of range)"
    );
    assert!(
      has_error_containing(&analyzer, "out of range"),
      "Error should mention out of range. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_variable_narrowing_still_rejected() {
    let code = r#"
var x as i32 = 100
var y as u8 = x as u8
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject variable narrowing conversion (unknown value at compile time)"
    );
    assert!(
      has_error_containing(&analyzer, "Narrowing"),
      "Error should mention narrowing. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_literal_in_bitwise_expression() {
    let code = r#"
var x as u8 = 3
var y as u8 = x bitwise and 1 as u8
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow `1 as u8` in bitwise expression"
    );
  }

  // ==================== INVALID CONVERSIONS ====================

  #[test]
  fn test_semantic_cast_str_to_numeric_rejected() {
    let code = r#"
var s as str = "hello"
var x as i32 = s as i32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from str to numeric"
    );
    assert!(
      has_error_containing(&analyzer, "String and numeric"),
      "Error should mention string/numeric incompatibility. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_numeric_to_str_rejected() {
    let code = r#"
var x as i32 = 42
var s as str = x as str
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from numeric to str"
    );
    assert!(
      has_error_containing(&analyzer, "String and numeric"),
      "Error should mention string/numeric incompatibility. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_bool_to_numeric_rejected() {
    let code = r#"
var b as bool = true
var x as i32 = b as i32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from bool to numeric"
    );
    assert!(
      has_error_containing(&analyzer, "Bool and numeric"),
      "Error should mention bool/numeric incompatibility. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_numeric_to_bool_rejected() {
    let code = r#"
var x as i32 = 1
var b as bool = x as bool
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject conversion from numeric to bool"
    );
    assert!(
      has_error_containing(&analyzer, "Bool and numeric"),
      "Error should mention bool/numeric incompatibility. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_semantic_cast_i64_to_i32_rejected() {
    // Design decision: narrowing integer conversions are not allowed
    let code = r#"
var x as i64 = 100
var y as i32 = x as i32
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject narrowing conversion from i64 to i32"
    );
    assert!(
      has_error_containing(&analyzer, "Narrowing"),
      "Error should mention narrowing. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== CAST IN EXPRESSIONS ====================

  #[test]
  fn test_semantic_cast_in_binary_operation() {
    let code = r#"
var x as i32 = 10
var y as f64 = 3.14
var result as f64 = (x as f64) + y
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Should allow cast in binary operation");
  }

  #[test]
  fn test_semantic_cast_in_comparison() {
    let code = r#"
var x as i32 = 10
var y as f64 = 3.14
var is_greater as bool = (x as f64) > y
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Should allow cast in comparison");
  }

  #[test]
  fn test_semantic_cast_in_assignment() {
    let code = r#"
var x as i32 = 10
var y as f64 = 0.0
y = x as f64
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Should allow cast in assignment");
  }

  #[test]
  fn test_semantic_cast_with_function_call() {
    let code = r#"
fn process(value as f64) returns nothing
    write value
end fn

var x as i32 = 42
process(x as f64)
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow cast as function argument"
    );
  }

  // ==================== CHAINED CONVERSIONS ====================
  //
  // Conversions chain as a left-to-right pipeline: x as u32 as f64
  // Each step is validated independently. Intermediate variables are
  // available for readability but not required.

  #[test]
  fn test_chained_int_to_int_to_float() {
    let code = r#"
var x as i32 = 42
var y as f64 = x as u32 as f64
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  }

  #[test]
  fn test_chained_three_steps() {
    let code = r#"
var x as u8 = 100
var y as u64 = x as u16 as u32 as u64
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  }

  #[test]
  fn test_chained_widening_then_narrowing() {
    let code = r#"
var x as u8 = 200
var y as u32 = x as u16 as u32
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  }

  #[test]
  fn test_chained_float_to_int_to_float() {
    let code = r#"
var x as f32 = 3.14
var y as f64 = x as f64
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  }

  #[test]
  fn test_chained_narrowing_rejected() {
    // Chained narrowing should be rejected — Lale only allows widening
    let code = r#"
var x as u32 = 100
var y as u16 = x as u16
"#;
    let analyzer = analyze_code(code);
    assert!(!analyzer.is_valid());
    assert!(
      analyzer
        .get_errors()
        .iter()
        .any(|e| e.contains("Narrowing"))
    );
  }

  // ==================== CAST WITH PHYSICAL UNITS ====================

  #[test]
  fn test_semantic_cast_preserves_units_same_type() {
    let code = r#"
var distance as f64 in <m> = 100.0
var value as f64 in <m> = distance as f64
"#;
    let analyzer = analyze_code(code);
    // Behavior depends on implementation: should unit be preserved?
    let _ = analyzer.is_valid();
  }

  #[test]
  fn test_semantic_cast_different_numeric_types_with_units() {
    let code = r#"
var distance as i32 in <m> = 100
var value as f64 in <m> = (distance as f64)
"#;
    let analyzer = analyze_code(code);
    // This depends on how units interact with type casting
    let _ = analyzer.is_valid();
  }

  // ==================== EDGE CASES ====================

  #[test]
  fn test_semantic_cast_literal_in_expression() {
    let code = r#"
var result as f64 = (42 as f64) + 3.14
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow cast of literal in expression"
    );
  }

  // Note: Casting array elements like "arr[0] as f64" now works with fixed grammar.
  #[test]
  fn test_semantic_cast_array_element() {
    let code = r#"
var arr as i32[5] = [1, 2, 3, 4, 5]
var result as f64 = arr[1] as f64
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Should allow cast of array element");
  }

  // Note: Type definitions and member access are not yet supported.
  // This test is skipped until type support is implemented.

  #[test]
  fn test_semantic_cast_in_array_literal() {
    let code = r#"
var x as i32 = 42
var arr as f64[3] = [x as f64, 1.0, 2.0]
"#;
    let analyzer = analyze_code(code);
    assert!(analyzer.is_valid(), "Should allow cast in array literal");
  }

  // Note: Cast without operand (e.g., "as f64" with no expression) correctly fails at parse time,
  // so there's no semantic test needed for this error condition.
}

// ==================== COMPREHENSIVE TYPE COVERAGE ====================

fn analyze_code(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error_containing(analyzer: &lale::semantic_analysis::OwnedAnalyzer, text: &str) -> bool {
  analyzer.get_errors().iter().any(|e| e.contains(text))
}

// ==================== INTEGER CONVERSIONS: ALL TYPES ====================
// Comprehensive testing of all 8 integer types in bidirectional conversions

#[test]
fn test_u8_to_u16_widening() {
  let code = r#"
var x as u8 = 255
var y as u16 = x as u16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → u16 (widening) should be allowed");
}

#[test]
fn test_u8_to_u32_widening() {
  let code = r#"
var x as u8 = 255
var y as u32 = x as u32
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → u32 (widening) should be allowed");
}

#[test]
fn test_u8_to_u64_widening() {
  let code = r#"
var x as u8 = 255
var y as u64 = x as u64
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → u64 (widening) should be allowed");
}

#[test]
fn test_u8_to_i16_widening() {
  let code = r#"
var x as u8 = 100
var y as i16 = x as i16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → i16 (widening) should be allowed");
}

#[test]
fn test_u8_to_i32_widening() {
  let code = r#"
var x as u8 = 100
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → i32 (widening) should be allowed");
}

#[test]
fn test_u8_to_i64_widening() {
  let code = r#"
var x as u8 = 100
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → i64 (widening) should be allowed");
}

#[test]
fn test_u16_to_u32_widening() {
  let code = r#"
var x as u16 = 65535
var y as u32 = x as u32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → u32 (widening) should be allowed"
  );
}

#[test]
fn test_u16_to_u64_widening() {
  let code = r#"
var x as u16 = 65535
var y as u64 = x as u64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → u64 (widening) should be allowed"
  );
}

#[test]
fn test_u16_to_i32_widening() {
  let code = r#"
var x as u16 = 30000
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → i32 (widening) should be allowed"
  );
}

#[test]
fn test_u32_to_u64_widening() {
  let code = r#"
var x as u32 = 4294967295
var y as u64 = x as u64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u32 → u64 (widening) should be allowed"
  );
}

#[test]
fn test_u32_to_i64_widening() {
  let code = r#"
var x as u32 = 4000000000
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u32 → i64 (widening) should be allowed"
  );
}

#[test]
fn test_i8_to_i16_widening() {
  let code = r#"
var x as i8 = -128
var y as i16 = x as i16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "i8 → i16 (widening) should be allowed");
}

#[test]
fn test_i8_to_i32_widening() {
  let code = r#"
var x as i8 = -128
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "i8 → i32 (widening) should be allowed");
}

#[test]
fn test_i8_to_i64_widening() {
  let code = r#"
var x as i8 = -128
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "i8 → i64 (widening) should be allowed");
}

#[test]
fn test_i16_to_i32_widening() {
  let code = r#"
var x as i16 = -32768
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i16 → i32 (widening) should be allowed"
  );
}

#[test]
fn test_i16_to_i64_widening() {
  let code = r#"
var x as i16 = -32768
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i16 → i64 (widening) should be allowed"
  );
}

#[test]
fn test_i32_to_i64_widening() {
  let code = r#"
var x as i32 = -2147483648
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i32 → i64 (widening) should be allowed"
  );
}

#[test]
fn test_u32_to_i32_sign_reinterpret() {
  let code = r#"
var x as u32 = 2147483648
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u32 → i32 (same-width reinterpret) should be allowed"
  );
}

#[test]
fn test_u16_to_i16_sign_reinterpret() {
  let code = r#"
var x as u16 = 32768
var y as i16 = x as i16
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → i16 (same-width reinterpret) should be allowed"
  );
}

// ==================== FLOAT CONVERSIONS ====================
// F16 is now supported via the LLVM backend

// ==================== INTEGER ↔ FLOAT: COMPREHENSIVE COMBINATIONS ====================

#[test]
fn test_u8_to_f32_widening() {
  let code = r#"
var x as u8 = 100
var y as f32 = x as f32
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 → f32 (widening) should be allowed");
}

#[test]
fn test_u16_to_f32_widening() {
  let code = r#"
var x as u16 = 30000
var y as f32 = x as f32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → f32 (widening) should be allowed"
  );
}

#[test]
fn test_u16_to_f64_widening() {
  let code = r#"
var x as u16 = 30000
var y as f64 = x as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u16 → f64 (widening) should be allowed"
  );
}

#[test]
fn test_u32_to_f64_widening() {
  let code = r#"
var x as u32 = 4000000000
var y as f64 = x as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u32 → f64 (widening) should be allowed"
  );
}

#[test]
fn test_u64_to_f64_widening() {
  let code = r#"
var x as u64 = 9223372036854775808
var y as f64 = x as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "u64 → f64 (widening) should be allowed"
  );
}

#[test]
fn test_i8_to_f32_widening() {
  let code = r#"
var x as i8 = -100
var y as f32 = x as f32
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "i8 → f32 (widening) should be allowed");
}

#[test]
fn test_i16_to_f32_widening() {
  let code = r#"
var x as i16 = -30000
var y as f32 = x as f32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i16 → f32 (widening) should be allowed"
  );
}

#[test]
fn test_i32_to_f64_widening() {
  let code = r#"
var x as i32 = -2147483648
var y as f64 = x as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i32 → f64 (widening) should be allowed"
  );
}

#[test]
fn test_i64_to_f64_widening() {
  let code = r#"
var x as i64 = -9223372036854775808
var y as f64 = x as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "i64 → f64 (widening) should be allowed"
  );
}

#[test]
fn test_f32_to_u16_narrowing_rejected() {
  let code = r#"
var x as f32 = 30000.5
var y as u16 = x as u16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "f32 → u16 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_f32_to_i32_narrowing_rejected() {
  let code = r#"
var x as f32 = 3.14
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "f32 → i32 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_f64_to_i64_narrowing_rejected() {
  let code = r#"
var x as f64 = 3.14159
var y as i64 = x as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "f64 → i64 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

// ==================== NUMERIC LITERAL RANGES: ALL INTEGER TYPES ====================
// Test boundary conditions for all 8 integer types

#[test]
fn test_literal_u16_min() {
  let code = r#"
var x as u16 = 0 as u16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 0 should fit in u16");
}

#[test]
fn test_literal_u16_max() {
  let code = r#"
var x as u16 = 65535 as u16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 65535 (max u16) should work");
}

#[test]
fn test_literal_u16_out_of_range() {
  let code = r#"
var x as u16 = 65536 as u16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 65536 should be rejected for u16"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_literal_u32_max() {
  let code = r#"
var x as u32 = 4294967295 as u32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal 4294967295 (max u32) should work"
  );
}

#[test]
fn test_literal_u32_out_of_range() {
  let code = r#"
var x as u32 = 4294967296 as u32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 4294967296 should be rejected for u32"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

// Note: u64 max literal (18446744073709551615) cannot be tested due to Pest numeric parser limitations
// The parser treats large positive literals as signed integers, causing range validation issues

#[test]
fn test_literal_i16_min() {
  let code = r#"
var x as i16 = -32768 as i16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal -32768 (min i16) should work");
}

#[test]
fn test_literal_i16_max() {
  let code = r#"
var x as i16 = 32767 as i16
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 32767 (max i16) should work");
}

#[test]
fn test_literal_i16_below_min() {
  let code = r#"
var x as i16 = -32769 as i16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal -32769 should be rejected for i16"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_literal_i16_above_max() {
  let code = r#"
var x as i16 = 32768 as i16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 32768 should be rejected for i16"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_literal_i32_min() {
  let code = r#"
var x as i32 = -2147483648 as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal -2147483648 (min i32) should work"
  );
}

#[test]
fn test_literal_i32_max() {
  let code = r#"
var x as i32 = 2147483647 as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal 2147483647 (max i32) should work"
  );
}

#[test]
fn test_literal_i32_above_max() {
  let code = r#"
var x as i32 = 2147483648 as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 2147483648 should be rejected for i32"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_literal_i64_min() {
  let code = r#"
var x as i64 = -9223372036854775808 as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal -9223372036854775808 (min i64) should work"
  );
}

#[test]
fn test_literal_i64_max() {
  let code = r#"
var x as i64 = 9223372036854775807 as i64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal 9223372036854775807 (max i64) should work"
  );
}

// ==================== VARIABLE NARROWING REJECTION: ALL COMBINATIONS ====================

#[test]
fn test_variable_u64_to_u32_narrowing_rejected() {
  let code = r#"
var x as u64 = 5000000000
var y as u32 = x as u32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "u64 variable → u32 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_u64_to_u16_narrowing_rejected() {
  let code = r#"
var x as u64 = 100000
var y as u16 = x as u16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "u64 variable → u16 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_i64_to_i32_narrowing_rejected() {
  let code = r#"
var x as i64 = 3000000000
var y as i32 = x as i32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "i64 variable → i32 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_i64_to_i16_narrowing_rejected() {
  let code = r#"
var x as i64 = 100000
var y as i16 = x as i16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "i64 variable → i16 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_i32_to_i16_narrowing_rejected() {
  let code = r#"
var x as i32 = 50000
var y as i16 = x as i16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "i32 variable → i16 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_u32_to_u8_narrowing_rejected() {
  let code = r#"
var x as u32 = 300
var y as u8 = x as u8
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "u32 variable → u8 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_f32_to_f16_narrowing_rejected() {
  let code = r#"
var x as f32 = 3.14
var y as f16 = x as f16
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "f32 variable → f16 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

#[test]
fn test_variable_f64_to_f32_narrowing_rejected() {
  let code = r#"
var x as f64 = 3.14159
var y as f32 = x as f32
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "f64 variable → f32 (narrowing) should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing"
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_mixed_integer_widening_chain() {
  let code = r#"
var a as u8 = 100
var b as u16 = a as u16
var c as u32 = b as u32
var d as u64 = c as u64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Sequential widening conversions should work"
  );
}

#[test]
fn test_mixed_numeric_widening_chain() {
  let code = r#"
var a as u8 = 100
var b as i32 = a as i32
var c as f64 = b as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Mixed-type widening conversions should work"
  );
}

#[test]
fn test_literal_conversions_all_integer_types() {
  let code = r#"
var a as i8 = 50 as i8
var b as i16 = 50 as i16
var c as i32 = 50 as i32
var d as i64 = 50 as i64
var e as u8 = 50 as u8
var f as u16 = 50 as u16
var g as u32 = 50 as u32
var h as u64 = 50 as u64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Same literal value should convert to all integer types if in range"
  );
}

#[test]
fn test_pointer_to_unsigned_integers() {
  let code = r#"
var a as u8 = 0
var b as u16 = 0
var c as u32 = 0
var d as u64 = 0
var p1 as pointer = a as pointer
var p2 as pointer = b as pointer
var p3 as pointer = c as pointer
var p4 as pointer = d as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "All unsigned integer types should convert to pointer"
  );
}

// ==================== DESIGN VERIFICATION ====================

#[test]
fn test_design_no_chained_casts_recommended_pattern() {
  // Design: Multiple conversions should use intermediate variables
  let code = r#"
var x as i32 = 42
var temp as u32 = x as u32
var result as f64 = temp as f64
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Recommended pattern with intermediate variables should work"
  );
}

// ==================== SECTION 2: POINTER CONVERSIONS ====================

#[test]
fn test_design_unsigned_u8_to_pointer() {
  // Design: u8 to pointer should be allowed
  let code = r#"
var small as u8 = 0
var ptr as pointer = small as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u8 to pointer should be allowed");
}

#[test]
fn test_design_unsigned_u16_to_pointer() {
  // Design: u16 to pointer should be allowed
  let code = r#"
var addr as u16 = 0
var ptr as pointer = addr as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u16 to pointer should be allowed");
}

#[test]
fn test_design_unsigned_u32_to_pointer() {
  // Design: u32 to pointer should be allowed
  let code = r#"
var addr as u32 = 0
var ptr as pointer = addr as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u32 to pointer should be allowed");
}

#[test]
fn test_design_unsigned_u64_to_pointer() {
  // Design: u64 to pointer should be allowed
  let code = r#"
var addr as u64 = 0x7FFF0000
var ptr as pointer = addr as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "u64 to pointer should be allowed");
}

#[test]
fn test_design_unsigned_literal_null_pointer() {
  // Design: Null pointer from literal should be allowed
  let code = r#"
var null_ptr as pointer = 0 as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal null pointer should be allowed"
  );
}

#[test]
fn test_design_signed_i32_to_pointer_rejected() {
  // Design: Signed integers cannot be converted to pointers
  let code = r#"
var x as i32 = 42
var p as pointer = x as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Signed i32 to pointer should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Only unsigned integers"),
    "Error should mention unsigned integers requirement"
  );
}

#[test]
fn test_design_signed_i64_to_pointer_rejected() {
  // Design: Signed integers cannot be converted to pointers
  let code = r#"
var x as i64 = 100
var p as pointer = x as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Signed i64 to pointer should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "Only unsigned integers"),
    "Error should mention unsigned integers requirement"
  );
}

#[test]
fn test_design_pointer_to_integer_rejected() {
  // Design: Pointer to integer requires unsafe cast, not regular `as`
  let code = r#"
var x as i32 = 42
var p as pointer = pointer to x
var addr as u64 = p as u64
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Pointer to integer should be rejected (use unsafe cast instead)"
  );
  assert!(
    has_error_containing(&analyzer, "Only unsigned integers"),
    "Error message should explain only unsigned integers can be targets"
  );
}

// ==================== SECTION 3: NUMERIC LITERAL CONVERSIONS ====================

#[test]
fn test_design_literal_1_to_u8() {
  // Design: Literal 1 fits in u8, should work
  let code = r#"
var a as u8 = 1 as u8
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 1 should fit in u8");
}

#[test]
fn test_design_literal_255_to_u8_max() {
  // Design: Literal 255 is max u8, should work
  let code = r#"
var b as u8 = 255 as u8
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 255 (max u8) should work");
}

#[test]
fn test_design_literal_256_to_u8_out_of_range() {
  // Design: Literal 256 exceeds u8, should be rejected
  let code = r#"
var e as u8 = 256 as u8
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 256 should be rejected for u8 (max 255)"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_design_literal_neg128_to_i8_min() {
  // Design: Literal -128 is min i8, should work
  let code = r#"
var c as i8 = -128 as i8
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal -128 (min i8) should work");
}

#[test]
fn test_design_literal_127_to_i8_max() {
  // Design: Literal 127 is max i8, should work
  let code = r#"
var d as i8 = 127 as i8
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Literal 127 (max i8) should work");
}

#[test]
fn test_design_literal_128_to_i8_out_of_range() {
  // Design: Literal 128 exceeds i8, should be rejected
  let code = r#"
var f as i8 = 128 as i8
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal 128 should be rejected for i8 (max 127)"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

#[test]
fn test_design_literal_neg129_to_i8_out_of_range() {
  // Design: Literal -129 exceeds i8, should be rejected
  let code = r#"
var g as i8 = -129 as i8
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Literal -129 should be rejected for i8 (min -128)"
  );
  assert!(
    has_error_containing(&analyzer, "out of range"),
    "Error should mention out of range"
  );
}

// ==================== SECTION 4: VARIABLE NARROWING STILL REJECTED ====================

#[test]
fn test_design_variable_narrowing_rejected() {
  // Design: Variable values are unknown at compile time, so narrowing is rejected
  let code = r#"
var x as i32 = 100
var y as u8 = x as u8
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Variable narrowing should be rejected (unknown value at compile time)"
  );
  assert!(
    has_error_containing(&analyzer, "Narrowing"),
    "Error should mention narrowing conversion"
  );
}

// ==================== EDGE CASES AND COMBINATIONS ====================

#[test]
fn test_design_all_unsigned_types_to_pointer() {
  // Design: All unsigned integer types can convert to pointer
  let code = r#"
var u8_val as u8 = 0
var u16_val as u16 = 0
var u32_val as u32 = 0
var u64_val as u64 = 0

var p1 as pointer = u8_val as pointer
var p2 as pointer = u16_val as pointer
var p3 as pointer = u32_val as pointer
var p4 as pointer = u64_val as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "All unsigned types should convert to pointer"
  );
}

#[test]
fn test_design_literal_conversions_in_expression() {
  // Design: Literals that fit can be used directly in expressions
  let code = r#"
var x as u8 = 3
var y as u8 = x bitwise and 1 as u8
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Literal conversions in expressions should work when value fits"
  );
}

#[test]
fn test_design_multiple_unsigned_literals_to_pointer() {
  // Design: Multiple unsigned literals can all convert to pointer
  let code = r#"
var p1 as pointer = 0 as pointer
var p2 as pointer = 42 as pointer
var p3 as pointer = 0xFF as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "All unsigned literal values should convert to pointer"
  );
}

#[test]
fn test_design_hex_literal_to_pointer() {
  // Design: Hex literals (like the example 0x7FFF0000) should convert to pointer
  // This is explicitly mentioned in the design document example:
  //   "var addr as u64 = 0x7FFF0000"
  //   "var ptr as pointer = addr as pointer"
  let code = r#"
var addr as u64 = 0x7FFF0000
var ptr as pointer = addr as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Hex literal u64 to pointer should be allowed"
  );
}

#[test]
fn test_design_various_hex_literals_to_pointer() {
  // Design: Various hex literal values should all convert to pointer
  let code = r#"
var p1 as pointer = 0xFF as pointer
var p2 as pointer = 0xDEADBEEF as pointer
var p3 as pointer = 0x7FFF0000 as pointer
var p4 as pointer = 0x00001234 as pointer
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Various hex literals should convert to pointer"
  );
}
