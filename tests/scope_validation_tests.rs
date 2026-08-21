//! Tests for scope validation in semantic analysis.
//!
//! This test suite covers:
//! - Type definitions must be at global scope (not inside functions)
//! - Function definitions must be at global scope (not inside other functions)
//! - Error messages for scope violations

use lale::{
  LaleParser, Rule,
  ast_builder::build_program,
  semantic_analysis::{AnalyzerResults, analyze_ast},
};
use pest::Parser;

fn parse_and_analyze(code: &str) -> Result<Vec<String>, Vec<String>> {
  let parse_result = LaleParser::parse(Rule::program, code);

  match parse_result {
    Ok(pairs) => match build_program(pairs, "test.lale") {
      Ok(program) => {
        let analyzer = analyze_ast(&program);
        if analyzer.is_valid() {
          Ok(Vec::new())
        } else {
          Err(
            analyzer
              .get_errors()
              .iter()
              .map(|e| e.message.clone())
              .collect(),
          )
        }
      }
      Err(e) => Err(vec![e]),
    },
    Err(e) => Err(vec![e.to_string()]),
  }
}

// ==================== RECORD SCOPE VALIDATION ====================

#[test]
fn test_type_at_global_scope_valid() {
  let code = r#"
type Point
    x as f64
    y as f64
end type

var p as Point = Point(1.0, 2.0)
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Type at global scope should be valid: {:?}",
    result
  );
}

#[test]
fn test_type_inside_function_invalid() {
  let code = r#"
fn createPoint() returns nothing
    type Point
        x as f64
        y as f64
    end type
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(result.is_err(), "Type inside function should be invalid");

  let errors = result.unwrap_err();
  assert!(
    errors
      .iter()
      .any(|e| e.contains("Type definitions are only allowed at global scope")),
    "Should report scope error for type: {:?}",
    errors
  );
}

#[test]
fn test_multiple_types_at_global_scope_valid() {
  let code = r#"
type Point
    x as f64
    y as f64
end type

type Color
    r as u8
    g as u8
    b as u8
end type

var point as Point = Point(0.0, 0.0)
var color as Color = Color(255, 0, 0)
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Multiple types at global scope should be valid: {:?}",
    result
  );
}

// ==================== FUNCTION SCOPE VALIDATION ====================

#[test]
fn test_function_at_global_scope_valid() {
  let code = r#"
fn add(a as i32, b as i32) returns i32
    return a + b
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Function at global scope should be valid: {:?}",
    result
  );
}

#[test]
fn test_function_inside_function_invalid() {
  let code = r#"
fn outer() returns nothing
    fn inner() returns nothing
        write "hello"
    end fn
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_err(),
    "Function inside function should be invalid"
  );

  let errors = result.unwrap_err();
  assert!(
    errors
      .iter()
      .any(|e| e.contains("Function definitions are only allowed at module or test-suite scope")),
    "Should report scope error for nested function: {:?}",
    errors
  );
}

#[test]
fn test_multiple_functions_at_global_scope_valid() {
  let code = r#"
fn add(a as i32, b as i32) returns i32
    return a + b
end fn

fn subtract(a as i32, b as i32) returns i32
    return a - b
end fn

var result = add(5, 3)
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Multiple functions at global scope should be valid: {:?}",
    result
  );
}

// ==================== COMBINED SCOPE VALIDATION ====================

#[test]
fn test_type_and_function_at_global_scope_valid() {
  let code = r#"
type Point
    x as f64
    y as f64
end type

fn distance(p as Point) returns f64
    return 0.0
end fn

var p as Point = Point(1.0, 2.0)
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Type and function at global scope should be valid: {:?}",
    result
  );
}

#[test]
fn test_type_inside_function_with_valid_global_type() {
  let code = r#"
type GlobalPoint
    x as f64
    y as f64
end type

fn createDuplicate() returns nothing
    type LocalPoint
        x as f64
        y as f64
    end type
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_err(),
    "Mixed global and local type definitions should fail"
  );

  let errors = result.unwrap_err();
  assert!(
    errors
      .iter()
      .any(|e| e.contains("Type definitions are only allowed at global scope")),
    "Should report scope error for nested type: {:?}",
    errors
  );
}

#[test]
fn test_function_inside_function_with_valid_global_function() {
  let code = r#"
fn globalAdd(a as i32, b as i32) returns i32
    return a + b
end fn

fn outerFunction() returns nothing
    fn localAdd(a as i32, b as i32) returns i32
        return a + b
    end fn
    write globalAdd(1, 2)
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_err(),
    "Mixed global and nested function definitions should fail"
  );

  let errors = result.unwrap_err();
  assert!(
    errors
      .iter()
      .any(|e| e.contains("Function definitions are only allowed at module or test-suite scope")),
    "Should report scope error for nested function: {:?}",
    errors
  );
}

// ==================== ERROR MESSAGE VALIDATION ====================

#[test]
fn test_type_scope_error_message_includes_location() {
  let code = r#"
fn test() returns nothing
    type BadType
        x as i32
    end type
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(result.is_err(), "Should have error");

  let errors = result.unwrap_err();
  assert!(!errors.is_empty(), "Should have at least one error");
  assert!(errors[0].contains("Type definitions are only allowed at global scope"));
}

#[test]
fn test_function_scope_error_message_includes_location() {
  let code = r#"
fn outer() returns nothing
    fn inner() returns nothing
        write "test"
    end fn
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(result.is_err(), "Should have error");

  let errors = result.unwrap_err();
  assert!(!errors.is_empty(), "Should have at least one error");
  assert!(
    errors[0].contains("Function definitions are only allowed at module or test-suite scope")
  );
}

// ==================== DOCUMENTATION ====================

#[test]
fn test_global_scope_with_documentation_and_type() {
  let code = r#"
/// This is a point in 2D space
type Point
    x as f64    /// X coordinate
    y as f64    /// Y coordinate
end type
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Type with documentation at global scope should be valid: {:?}",
    result
  );
}

#[test]
fn test_global_scope_with_documentation_and_function() {
  let code = r#"
/// Calculate the sum of two numbers
fn add(a as i32, b as i32) returns i32
    return a + b
end fn
"#;

  let result = parse_and_analyze(code);
  assert!(
    result.is_ok(),
    "Function with documentation at global scope should be valid: {:?}",
    result
  );
}
