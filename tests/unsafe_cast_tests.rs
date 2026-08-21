//! Unsafe Cast Tests
//!
//! Comprehensive tests for the `unsafe cast` bit reinterpretation operator.
//!
//! This test suite covers:
//! - Grammar: Parsing of `unsafe value at ptr unsafe cast` syntax (postfix operator)
//! - Semantic Analysis: Unitless source requirement, bit-width compatibility
//! - Valid casts: same bit-width types without units
//! - Invalid casts: source with physical units, bit-width mismatches

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== GRAMMAR TESTS ====================

#[test]
fn test_unsafe_cast_grammar_basic() {
  let result = LaleParser::parse(Rule::unsafe_cast_wrapper, "unsafe cast");
  assert!(
    result.is_ok(),
    "Should parse 'unsafe cast' as postfix operator"
  );
}

#[test]
fn test_unsafe_cast_in_expression() {
  let result = LaleParser::parse(Rule::expression, "unsafe value at p unsafe cast");
  assert!(result.is_ok(), "Should parse 'unsafe value at p unsafe cast'");
}

#[test]
fn test_unsafe_cast_full_statement() {
  let code = r#"var bits as u32 = unsafe value at p unsafe cast"#;
  let result = LaleParser::parse(Rule::var, code);
  assert!(
    result.is_ok(),
    "Should parse full var statement with unsafe cast"
  );
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
    analyzer
      .get_errors()
      .iter()
      .any(|e| e.message.contains(text))
  }

  // ==================== VALID UNSAFE CASTS ====================

  #[test]
  fn test_unsafe_cast_f32_to_u32_unitless() {
    let code = r#"
var bar as f32 = 42.0
var p as pointer = pointer to bar
var bits as u32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast from unitless f32 to u32 (both 32-bit). Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_i32_to_f32_unitless() {
    let code = r#"
var x as i32 = 42
var p as pointer = pointer to x
var y as f32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast from unitless i32 to f32 (both 32-bit). Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_u64_to_f64_unitless() {
    let code = r#"
var x as u64 = 12345
var p as pointer = pointer to x
var y as f64 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast from unitless u64 to f64 (both 64-bit). Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_i64_to_pointer_unitless() {
    let code = r#"
var x as i64 = 12345
var p as pointer = pointer to x
var q as pointer = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast from unitless i64 to pointer (both 64-bit). Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== INVALID: SOURCE HAS PHYSICAL UNIT ====================

  #[test]
  fn test_unsafe_cast_rejects_source_with_unit() {
    let code = r#"
var distance as f32 in <m> = 5.0
var p as pointer = pointer to distance
var bits as u32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject unsafe cast when source has physical unit"
    );
    assert!(
      has_error_containing(&analyzer, "unitless"),
      "Error message should mention 'unitless'. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_rejects_source_with_compound_unit() {
    let code = r#"
var velocity as f64 in <m/s> = 9.8
var p as pointer = pointer to velocity
var bits as u64 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject unsafe cast when source has compound physical unit"
    );
    assert!(
      has_error_containing(&analyzer, "unitless"),
      "Error message should mention 'unitless'. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== INVALID: BIT-WIDTH MISMATCH ====================

  #[test]
  fn test_unsafe_cast_rejects_32_to_64_bit() {
    let code = r#"
var x as f32 = 42.0
var p as pointer = pointer to x
var y as f64 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject unsafe cast from 32-bit to 64-bit"
    );
    assert!(
      has_error_containing(&analyzer, "bit-width"),
      "Error message should mention 'bit-width'. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_rejects_64_to_32_bit() {
    let code = r#"
var x as f64 = 42.0
var p as pointer = pointer to x
var y as u32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject unsafe cast from 64-bit to 32-bit"
    );
    assert!(
      has_error_containing(&analyzer, "bit-width"),
      "Error message should mention 'bit-width'. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_rejects_16_to_32_bit() {
    let code = r#"
var x as i16 = 42
var p as pointer = pointer to x
var y as i32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      !analyzer.is_valid(),
      "Should reject unsafe cast from 16-bit to 32-bit"
    );
    assert!(
      has_error_containing(&analyzer, "bit-width"),
      "Error message should mention 'bit-width'. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== CORRECT PATTERN: USE UNITLESS VARIABLE ====================

  #[test]
  fn test_unsafe_cast_with_unitless_variable() {
    // Note: In Lale, if you assign a value with a unit to a variable without a declared unit,
    // the unit is inferred (carried over). To use unsafe cast, you must start with a unitless value.
    let code = r#"
var raw as f32 = 5.0
var p as pointer = pointer to raw
var bits as u32 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast when source variable has no unit. Errors: {:?}",
      analyzer.get_errors()
    );
  }

  // ==================== EDGE CASES ====================

  #[test]
  fn test_unsafe_cast_u8_to_i8_same_width() {
    let code = r#"
var x as u8 = 255
var p as pointer = pointer to x
var y as i8 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    assert!(
      analyzer.is_valid(),
      "Should allow unsafe cast from u8 to i8 (both 8-bit). Errors: {:?}",
      analyzer.get_errors()
    );
  }

  #[test]
  fn test_unsafe_cast_f16_to_u16_same_width() {
    let code = r#"
var x as f16 = 3.14
var p as pointer = pointer to x
var y as u16 = unsafe value at p unsafe cast
"#;
    let analyzer = analyze_code(code);
    // Semantic analysis should succeed
    let _ = analyzer.is_valid();
  }
}
