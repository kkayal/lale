//! Tests for compile-time array bounds checking.
//!
//! This test suite covers array indexing validation:
//! - Lower bounds checking (indices must be >= 1)
//! - Upper bounds checking (indices must be <= array size)
//! - Multi-dimensional array checks
//!
//! NOTE: These tests verify the semantic analysis works correctly.
//! Currently, the grammar only supports array indexing in limited contexts
//! (assignments with identifiers directly). Once full postfix operator support
//! is implemented, additional patterns can be tested.

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

fn error_count(analyzer: &lale::semantic_analysis::OwnedAnalyzer) -> usize {
  analyzer.get_errors().len()
}

fn error_string(analyzer: &lale::semantic_analysis::OwnedAnalyzer) -> String {
  analyzer
    .get_errors()
    .iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>()
    .join("; ")
}

// ==================== LOWER BOUNDS TESTS (>= 1) ====================

#[test]
fn test_array_index_zero_error() {
  let code = r#"
var arr as u32[5] = [1, 2, 3, 4, 5]
var y as u32 = arr[0]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index must be >= 1"));
}

#[test]
fn test_array_index_negative_error() {
  let code = r#"
var arr as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var y as i32 = arr[-1]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index must be >= 1"));
}

#[test]
fn test_array_index_negative_large_error() {
  let code = r#"
var arr as i32[5] = [1, 2, 3, 4, 5]
var y as i32 = arr[-100]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index must be >= 1"));
}

// ==================== UPPER BOUNDS TESTS (>= size) ====================

#[test]
fn test_array_index_exceeds_size_error() {
  let code = r#"
var arr as u32[5] = [1, 2, 3, 4, 5]
var y as u32 = arr[6]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
  assert!(has_error_containing(&analyzer, "dimension 1 has size 5"));
}

#[test]
fn test_array_index_way_out_of_bounds_error() {
  let code = r#"
var arr as f64[3] = [1.0, 2.0, 3.0]
var y as f64 = arr[100]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
  assert!(has_error_containing(&analyzer, "dimension 1 has size 3"));
}

#[test]
fn test_array_index_one_past_boundary_error() {
  let code = r#"
var arr as i32[7] = [1, 2, 3, 4, 5, 6, 7]
var y as i32 = arr[8]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
}

// ==================== VALID ARRAY ACCESS TESTS ====================

#[test]
fn test_array_index_first_element_valid() {
  let code = r#"
var arr as u32[5] = [1, 2, 3, 4, 5]
var y as u32 = arr[1]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_array_index_last_element_valid() {
  let code = r#"
var arr as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var y as i32 = arr[10]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_array_index_middle_element_valid() {
  let code = r#"
var arr as f64[7] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
var y as f64 = arr[4]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_array_small_size_all_valid() {
  let code = r#"
var arr as u8[3] = [10, 20, 30]
var a as u8 = arr[1]
var b as u8 = arr[2]
var c as u8 = arr[3]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

// ==================== MULTI-DIMENSIONAL ARRAY TESTS ====================

#[test]
fn test_matrix_first_dimension_out_of_bounds() {
  let code = r#"
var matrix as i32[3][3] = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
var x as i32 = matrix[4][1]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
  assert!(has_error_containing(&analyzer, "dimension 1 has size 3"));
}

#[test]
fn test_matrix_second_dimension_out_of_bounds() {
  let code = r#"
var matrix as f64[2][4] = [[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]]
var x as f64 = matrix[1][5]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
  assert!(has_error_containing(&analyzer, "dimension 2 has size 4"));
}

#[test]
fn test_matrix_both_dimensions_out_of_bounds() {
  let code = r#"
var matrix as u32[2][2] = [[1, 2], [3, 4]]
var x as u32 = matrix[3][3]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
}

#[test]
fn test_matrix_first_dim_zero_error() {
  let code = r#"
var matrix as i32[3][3] = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
var x as i32 = matrix[0][1]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index must be >= 1"));
}

#[test]
fn test_matrix_second_dim_zero_error() {
  let code = r#"
var matrix as i32[3][3] = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
var x as i32 = matrix[1][0]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index must be >= 1"));
}

#[test]
fn test_matrix_valid_access_all_corners() {
  let code = r#"
var matrix as f64[3][4] = [[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0], [9.0, 10.0, 11.0, 12.0]]
var a as f64 = matrix[1][1]
var b as f64 = matrix[1][4]
var c as f64 = matrix[3][1]
var d as f64 = matrix[3][4]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

// ==================== VARIABLE INDEX TESTS (not checked at compile-time) ====================

#[test]
fn test_array_variable_index_no_error_at_compile_time() {
  let code = r#"
var arr as u32[5] = [1, 2, 3, 4, 5]
var idx as u32 = 3
var y as u32 = arr[idx]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_array_expression_index_no_error_at_compile_time() {
  let code = r#"
var arr as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var i as i32 = 2
var y as i32 = arr[i + 1]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

// ==================== EDGE CASES ====================

#[test]
fn test_array_size_one_valid() {
  let code = r#"
var arr as u32[1] = [42]
var y as u32 = arr[1]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_array_size_one_out_of_bounds() {
  let code = r#"
var arr as u32[1] = [42]
var y as u32 = arr[2]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
}

#[test]
fn test_large_array_boundary_valid() {
  let code = r#"
var arr as i32[1000] = [fill with 0]
var y as i32 = arr[1000]
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Error: {}", error_string(&analyzer));
}

#[test]
fn test_large_array_one_past_boundary() {
  let code = r#"
var arr as i32[1000] = [1, 2, 3, 4, 5]
var y as i32 = arr[1001]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
}

// ==================== COMBINED ERROR TESTS ====================

#[test]
fn test_multiple_array_errors_in_same_program() {
  let code = r#"
var arr1 as u32[5] = [1, 2, 3, 4, 5]
var arr2 as i32[3] = [10, 20, 30]
var x as u32 = arr1[0]
var y as i32 = arr2[4]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(
    error_count(&analyzer) >= 2,
    "Expected at least 2 errors, got {}",
    error_count(&analyzer)
  );
}

#[test]
fn test_multiple_sequential_array_accesses() {
  let code = r#"
var arr as f64[5] = [1.0, 2.0, 3.0, 4.0, 5.0]
var a as f64 = arr[1]
var b as f64 = arr[2]
var c as f64 = arr[6]
var d as f64 = arr[7]
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "Array index out of bounds"));
}
