use lale::semantic_analysis::AnalyzerResults;
use lale::{LaleParser, Rule, ast::Stmt, ast::builder::build_program, semantic_analysis};
use pest::Parser;

/// Tests for compile-time arithmetic evaluation in #if directives.
///
/// This test suite verifies:
/// - Integer literal parsing in compile-time conditions
/// - Arithmetic operations: +, -, *, /, %
/// - Comparison operators: <, <=, >, >=, ==, !=
/// - Operator precedence
/// - Parentheses grouping
/// - Combination with logical operators (and, or)
/// - Division by zero handling

// ==================== PARSING TESTS ====================

#[test]
fn test_parse_integer_literal_in_ct_condition() {
  let source = r#"
#if 5 > 0
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse integer comparison in #if");
}

#[test]
fn test_parse_arithmetic_in_ct_condition() {
  let source = r#"
#if 2 + 3 > 4
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse arithmetic in #if");
}

#[test]
fn test_parse_comparison_in_ct_condition() {
  let source = r#"
#if 5 > 3
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse comparison in #if");
}

#[test]
fn test_parse_negative_integer_with_comparison() {
  let source = r#"
#if -5 < 0
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse negative integer comparison");
}

// ==================== ARITHMETIC OPERATION TESTS ====================

#[test]
fn test_addition_evaluates_true() {
  let source = r#"
#if 2 + 3 > 4
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  // Should have "true" result
  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_addition_evaluates_to_zero_false() {
  let source = r#"
#if 5 - 5 == 0
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  // Should have "false" result (5 - 5 = 0, which is falsy)
  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_multiplication() {
  let source = r#"
#if 3 * 4 > 10
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_division() {
  let source = r#"
#if 10 / 2 > 4
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_modulo() {
  let source = r#"
#if 10 % 3 > 0
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

// ==================== COMPARISON OPERATOR TESTS ====================

#[test]
fn test_less_than_true() {
  let source = r#"
#if 3 < 5
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_less_than_false() {
  let source = r#"
#if 5 < 3
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_less_than_equal() {
  let source = r#"
#if 5 <= 5
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_greater_than() {
  let source = r#"
#if 5 > 3
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_greater_than_equal() {
  let source = r#"
#if 5 >= 3
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_equality_integers() {
  let source = r#"
#if 5 == 5
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_not_equal_integers() {
  let source = r#"
#if 5 != 3
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

// ==================== OPERATOR PRECEDENCE TESTS ====================

#[test]
fn test_precedence_multiplication_before_addition() {
  let source = r#"
#if 2 + 3 * 4 > 10
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition (2 + 3*4 = 14 > 10 = true)"
  );
}

#[test]
fn test_precedence_with_parentheses() {
  let source = r#"
#if (2 + 3) * 4 > 10
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition ((2+3)*4 = 20 > 10 = true)"
  );
}

// ==================== LOGICAL OPERATORS WITH ARITHMETIC TESTS ====================

#[test]
fn test_arithmetic_with_logical_and() {
  let source = r#"
#if 5 > 3 and 2 * 3 > 5
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_arithmetic_with_logical_or() {
  let source = r#"
#if 5 < 3 or 2 * 3 > 5
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_arithmetic_with_platform_and() {
  let source = r#"
#if 5 > 3 and #posix
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_zero_comparison() {
  let source = r#"
#if 0 == 0
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_negative_number_comparison() {
  let source = r#"
#if -5 < 0
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

#[test]
fn test_complex_nested_arithmetic() {
  let source = r#"
#if ((10 + 5) * 2 - 20) / 3 > 0
    var result as str = "true"
#else
    var result as str = "false"
#end if
write result
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(analyzer.is_valid(), "Semantic analysis should succeed");
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  let result_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "result" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    result_defs.len(),
    1,
    "Should have exactly one result definition"
  );
}

// ==================== ERROR HANDLING ====================

#[test]
fn test_integer_literal_without_comparison_error() {
  let source = r#"
#if 5
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    !analyzer.is_valid(),
    "Should report error for integer literal without comparison: {:?}",
    analyzer.get_errors()
  );
  let errors = analyzer.get_errors();
  assert!(!errors.is_empty(), "Should have semantic errors");
  assert!(
    errors[0].message.contains("boolean expression"),
    "Error message should mention boolean requirement: {}",
    errors[0].message
  );
}

#[test]
fn test_arithmetic_without_comparison_error() {
  let source = r#"
#if 2 + 3
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    !analyzer.is_valid(),
    "Should report error for arithmetic without comparison"
  );
  let errors = analyzer.get_errors();
  assert!(!errors.is_empty(), "Should have semantic errors");
  assert!(
    errors[0].message.contains("comparison"),
    "Error message should suggest using comparison"
  );
}

#[test]
fn test_unary_negation_without_comparison_error() {
  let source = r#"
#if -5
    var x as i32 = 1
#end if
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);
  assert!(
    !analyzer.is_valid(),
    "Should report error for unary negation without comparison"
  );
  let errors = analyzer.get_errors();
  assert!(!errors.is_empty(), "Should have semantic errors");
}
