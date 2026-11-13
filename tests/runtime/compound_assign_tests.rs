use lale::ast::{CompoundOp, Stmt};
use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for compound assignment statement in the Lale grammar.
///
/// This test suite covers:
/// - All compound operators: +=, -=, *=, /=, %=
/// - Member access and array index compound assignment
/// - Semantic analysis (undefined variable, unit checking)

// ==================== GRAMMAR TESTS ====================

#[test]
fn test_compound_assign_add() {
  let result = LaleParser::parse(Rule::compound_assign, "x += 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_sub() {
  let result = LaleParser::parse(Rule::compound_assign, "x -= 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_mul() {
  let result = LaleParser::parse(Rule::compound_assign, "x *= 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_div() {
  let result = LaleParser::parse(Rule::compound_assign, "x /= 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_mod() {
  let result = LaleParser::parse(Rule::compound_assign, "x %= 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_complex_expression() {
  let result = LaleParser::parse(Rule::compound_assign, "x += a + b * c");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_member_access() {
  let result = LaleParser::parse(Rule::compound_assign, "obj.field += 10");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_nested_member() {
  let result = LaleParser::parse(Rule::compound_assign, "obj.sub.field -= 5");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_array_index() {
  let result = LaleParser::parse(Rule::compound_assign, "arr[0] += 100");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_variable_index() {
  let result = LaleParser::parse(Rule::compound_assign, "arr[i] *= 2");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_multi_dimensional() {
  let result = LaleParser::parse(Rule::compound_assign, "matrix[i][j] += 1");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_multiline_not_allowed_around_operator() {
  // Newlines are NOT allowed around compound operators
  // (the grammar uses `os` which only includes spaces/tabs, not newlines)
  let result = LaleParser::parse(Rule::compound_assign, "x\n    += 10");
  assert!(
    result.is_err(),
    "Compound assignment should not allow newlines around operator"
  );
}

#[test]
fn test_compound_assign_extra_whitespace() {
  let result = LaleParser::parse(Rule::compound_assign, "x   +=   42");
  assert!(result.is_ok());
}

// ==================== AST BUILDER TESTS ====================

fn parse_and_build(source: &str) -> Result<lale::ast::Program, String> {
  let pairs =
    LaleParser::parse(Rule::program, source).map_err(|e| format!("Parse error: {}", e))?;
  build_program(pairs, "test.lale")
}

#[test]
fn test_ast_compound_assign_add() {
  let source = "var x as i32 = 0\nx += 5";
  let program = parse_and_build(source).expect("Should parse");
  assert_eq!(program.statements.len(), 2);

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.target.node, vec!["x"]);
      assert_eq!(compound.operator.node, CompoundOp::AddAssign);
      assert!(compound.indices.is_empty());
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_sub() {
  let source = "var x as i32 = 10\nx -= 3";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.operator.node, CompoundOp::SubAssign);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_mul() {
  let source = "var x as i32 = 2\nx *= 5";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.operator.node, CompoundOp::MulAssign);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_div() {
  let source = "var x as i32 = 10\nx /= 2";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.operator.node, CompoundOp::DivAssign);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_mod() {
  let source = "var x as i32 = 10\nx %= 3";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.operator.node, CompoundOp::ModAssign);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_member_access() {
  let source = "var obj as i32 = 0\nobj.field += 10";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.target.node, vec!["obj", "field"]);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

#[test]
fn test_ast_compound_assign_array_index() {
  let source = "var arr as i32[3] = [1, 2, 3]\narr[0] += 42";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    Stmt::CompoundAssign(compound) => {
      assert_eq!(compound.target.node, vec!["arr"]);
      assert_eq!(compound.indices.len(), 1);
    }
    _ => panic!("Expected CompoundAssign statement"),
  }
}

// ==================== SEMANTIC ANALYSIS TESTS ====================

#[test]
fn test_semantic_compound_undefined_variable() {
  let source = "y += 10";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should detect undefined variable");
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("undefined variable")));
}

#[test]
fn test_semantic_compound_defined_variable() {
  let source = "var x as i32 = 0\nx += 10";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept compound assignment to defined variable"
  );
}

#[test]
fn test_semantic_compound_add_unit_match() {
  let source = "var x as f64 in <m> = 5.0\nx += 10.0 <m>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid(), "Should accept matching units for +=");
}

#[test]
fn test_semantic_compound_add_unit_mismatch() {
  let source = "var x as f64 in <m> = 5.0\nx += 10.0 <s>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should detect unit mismatch for +=");
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("Unit mismatch")));
}

#[test]
fn test_semantic_compound_mul_unitless() {
  let source = "var x as f64 in <m> = 5.0\nx *= 2.0";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept unitless multiplier for *="
  );
}

#[test]
fn test_semantic_compound_mul_with_unit() {
  let source = "var x as f64 in <m> = 5.0\nx *= 2.0 <s>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should reject unit operand for *=");
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("unitless")));
}

#[test]
fn test_semantic_compound_div_unitless() {
  let source = "var x as f64 in <m> = 10.0\nx /= 2.0";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid(), "Should accept unitless divisor for /=");
}

#[test]
fn test_semantic_compound_div_with_unit() {
  let source = "var x as f64 in <m> = 10.0\nx /= 2.0 <s>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should reject unit operand for /=");
}

#[test]
fn test_semantic_compound_mod_unitless() {
  let source = "var x as i32 = 10\nx %= 3";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept modulo on unitless variable"
  );
}

#[test]
fn test_semantic_compound_inside_function() {
  let source = r#"
fn test() returns nothing
    var x as i32 = 0
    x += 5
end fn
"#;
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept compound assignment inside function"
  );
}

#[test]
fn test_semantic_compound_global_from_function() {
  let source = r#"
var global as i32 = 0

fn test() returns nothing
    global += 5
end fn
"#;
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept compound assignment to global from function"
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_compound_assign_function_call_value() {
  let result = LaleParser::parse(Rule::compound_assign, "x += getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_parenthesized() {
  let result = LaleParser::parse(Rule::compound_assign, "x += (a + b) * c");
  assert!(result.is_ok());
}

#[test]
fn test_compound_assign_negative_value() {
  let result = LaleParser::parse(Rule::compound_assign, "x += -42");
  assert!(result.is_ok());
}
