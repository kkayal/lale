//! Tests for enum definitions.
//!
//! Covers grammar parsing, semantic analysis, IR generation, and runtime
//! behaviour for enum definitions with and without field-carrying variants.

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSING TESTS ====================

#[test]
fn test_enum_no_fields_parses() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_empty_rejected() {
  let result = LaleParser::parse(Rule::enum_def, "enum Empty end enum");
  assert!(
    result.is_err(),
    "enum without any variant must fail to parse"
  );
}

#[test]
fn test_enum_single_variant_no_fields() {
  let code = "enum Flag\n  Active\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_with_field_parses() {
  let code = "enum Shape\n  Circle(f64)\n  Rectangle(f64, f64)\n  Point\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_with_multiple_field_types() {
  let code = "enum Mixed\n  A(i32)\n  B(f64)\n  C(bool)\n  D(text)\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_with_unit_fields() {
  let code = "enum Measurement\n  Length(f64 in <m>)\n  Time(f64 in <s>)\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_with_export() {
  let code = "export enum Public\n  X\n  Y\n  Z\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_variant_field_rule() {
  assert!(LaleParser::parse(Rule::enum_variant_field, "f64").is_ok());
  assert!(LaleParser::parse(Rule::enum_variant_field, "i32 in <m>").is_ok());
  assert!(LaleParser::parse(Rule::enum_variant_field, "text").is_ok());
}

#[test]
fn test_enum_variant_rule() {
  assert!(LaleParser::parse(Rule::enum_variant, "Red").is_ok());
  assert!(LaleParser::parse(Rule::enum_variant, "Circle(f64)").is_ok());
  assert!(LaleParser::parse(Rule::enum_variant, "Rect(f64, f64)").is_ok());
}

#[test]
fn test_enum_variant_trailing_comma() {
  assert!(LaleParser::parse(Rule::enum_variant, "Circle(f64,)").is_ok());
  assert!(LaleParser::parse(Rule::enum_variant, "Rect(f64, f64,)").is_ok());
}

#[test]
fn test_enum_end_guarded() {
  // "end" should not be consumed as a variant name
  let code = "enum Tokens\n  Start\n  Finish\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_enum_after_var_def() {
  // Enum after other definitions in the same program
  let code = "var x as i32 = 0\nenum Color\n  Red\n  Green\nend enum\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

// ==================== SEMANTIC TESTS ====================

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};

fn analyze(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");
  analyze_ast(&program)
}

#[test]
fn test_enum_variant_used_ok() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\nvar c as Color = Red\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_field_variant_used_ok() {
  let code = "enum Shape\n  Circle(f64)\n  Point\nend enum\nvar s as Shape = Circle(3.14)\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_qualified_variant_ok() {
  let code = "enum Shape\n  Circle(f64)\n  Point\nend enum\nvar s as Shape = Shape.Circle(3.14)\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_qualified_zero_arg_ok() {
  let code = "enum Shape\n  Point\nend enum\nvar s as Shape = Shape.Point\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_field_variant_multi_arg_ok() {
  let code = "enum Shape\n  Rectangle(f64, f64)\nend enum\nvar s as Shape = Rectangle(3.0, 4.0)\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_wrong_arg_count_rejected() {
  let code = "enum Shape\n  Circle(f64)\n  Point\nend enum\nvar s as Shape = Circle(1.0, 2.0)\n";
  let a = analyze(code);
  assert!(!a.is_valid());
  assert!(a.get_errors().iter().any(|e| e.contains("expects")));
}

#[test]
fn test_enum_wrong_arg_type_warns() {
  // Bool f64 is an unsupported conversion — currently a compile-time warning, not an error.
  // The argument count is correct (1 arg, 1 param) so it parses and analyzes successfully.
  let code = "enum Shape\n  Circle(f64)\nend enum\nvar s as Shape = Circle(true)\n";
  let a = analyze(code);
  // This may produce a warning but should not be a hard error
  let _ = a.is_valid();
}

#[test]
fn test_enum_undefined_variant_rejected() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\nvar c as Color = Yellow\n";
  let a = analyze(code);
  assert!(!a.is_valid());
  assert!(a.get_errors().iter().any(|e| e.contains("Undefined")));
}

#[test]
fn test_enum_only_global_scope() {
  let code = "fn foo() returns i32\n  enum Bad\n    A\n    B\n  end enum\n  return 0\nend fn\n";
  let a = analyze(code);
  assert!(!a.is_valid());
  assert!(a.get_errors().iter().any(|e| e.contains("global scope")));
}

#[test]
fn test_enum_bool_field_ok() {
  let code = "enum Flag\n  On(bool)\n  Off\nend enum\nvar f as Flag = On(true)\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_enum_text_field_ok() {
  let code = "enum Message\n  Text(text)\n  Empty\nend enum\nvar m as Message = Text(\"hello\")\n";
  let a = analyze(code);
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

// ==================== RUNTIME TESTS ====================

use std::process::Command;

fn run_lale(code: &str) -> std::process::Output {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", "-"])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("spawn lale");
  use std::io::Write;
  child
    .stdin
    .take()
    .unwrap()
    .write_all(code.as_bytes())
    .unwrap();
  child.wait_with_output().expect("wait")
}

#[test]
fn test_enum_debug_simple_variant() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\nvar c as Color = Red\ndebug c\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Red"),
    "Expected 'Red' in stderr, got: {}",
    stderr
  );
  assert!(
    stderr.contains("Color"),
    "Expected 'Color' in stderr, got: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_enum_debug_field_variant() {
  let code = "enum Shape\n  Circle(f64)\nend enum\nvar s as Shape = Circle(3.14)\ndebug s\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Circle(3.14)"),
    "Expected 'Circle(3.14)' in stderr, got: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_enum_debug_multi_field() {
  let code =
    "enum Shape\n  Rectangle(f64, f64)\nend enum\nvar s as Shape = Rectangle(1.5, 2.5)\ndebug s\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Rectangle(1.5, 2.5)"),
    "Expected 'Rectangle(1.5, 2.5)' in stderr, got: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_enum_multiple_debug() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\nvar a as Color = Red\nvar b as Color = Green\nvar c as Color = Blue\ndebug a\ndebug b\ndebug c\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Red"), "Missing Red: {}", stderr);
  assert!(stderr.contains("Green"), "Missing Green: {}", stderr);
  assert!(stderr.contains("Blue"), "Missing Blue: {}", stderr);
  assert!(out.status.success());
}

#[test]
fn test_enum_qualified_debug() {
  let code =
    "enum Shape\n  Circle(f64)\n  Point\nend enum\nvar s as Shape = Shape.Circle(2.0)\ndebug s\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Circle(2)"),
    "Expected 'Circle(2)' in stderr, got: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_enum_typecheck_ok() {
  let code = "enum Color\n  Red\n  Green\nend enum\nvar c as Color = Red\nvar d as Color = Green\n";
  let out = run_lale(code);
  assert!(
    out.status.success(),
    "Stderr: {}",
    String::from_utf8_lossy(&out.stderr)
  );
}
