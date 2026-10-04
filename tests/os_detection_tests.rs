use lale::semantic_analysis::AnalyzerResults;
use lale::{LaleParser, Rule, ast::Stmt, ast::builder::build_program, semantic_analysis};
use pest::Parser;

/// Tests for OS detection compiler constants (#posix and #windows).
///
/// This test suite verifies:
/// - Parsing of #posix and #windows constants
/// - Semantic analysis and evaluation of these constants
/// - Integration with compile-time conditional directives

// ==================== PARSING TESTS ====================

#[test]
fn test_parse_posix_in_expression() {
  let source = "write \"{#posix}\"";
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse #posix in expression");
}

#[test]
fn test_parse_windows_in_expression() {
  let source = "write \"{#windows}\"";
  let pairs = LaleParser::parse(Rule::program, source);
  assert!(pairs.is_ok(), "Should parse #windows in expression");
}

// ==================== COMPILE-TIME CONDITIONAL TESTS ====================

#[test]
fn test_posix_condition_true_on_posix() {
  let source = r#"
#if #posix
    var platform as text = "posix"
#else
    var platform as text = "windows"
#end if
write platform
"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("Build failed");
  let analyzer = semantic_analysis::analyze_ast_with_stdlib(&program);

  // Should have no errors
  assert!(
    analyzer.is_valid(),
    "Semantic analysis should succeed: {:?}",
    analyzer.get_errors()
  );

  // Process compile-time directives
  let processed = semantic_analysis::process_ct_directives(&program, &analyzer);

  // Check that the program has the expected definition
  // On POSIX systems, platform should be defined; on Windows, it should be different
  let platform_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "platform" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    platform_defs.len(),
    1,
    "Should have exactly one platform definition after processing"
  );

  #[cfg(not(target_os = "windows"))]
  {
    // On POSIX, the then branch should be included
    assert_eq!(
      platform_defs.len(),
      1,
      "POSIX system should have platform definition from then branch"
    );
  }

  #[cfg(target_os = "windows")]
  {
    // On Windows, the else branch should be included
    assert_eq!(
      platform_defs.len(),
      1,
      "Windows system should have platform definition from else branch"
    );
  }
}

#[test]
fn test_windows_condition_true_on_windows() {
  let source = r#"
#if #windows
    var os_type as text = "windows"
#else
    var os_type as text = "posix"
#end if
write os_type
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

  let os_type_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "os_type" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    os_type_defs.len(),
    1,
    "Should have exactly one os_type definition after processing"
  );
}

#[test]
fn test_mutually_exclusive_posix_windows() {
  let source = r#"
#if #posix
    #if #windows
        var error as text = "both true"
    #else
        var result as text = "posix only"
    #end if
#else
    #if #windows
        var result as text = "windows only"
    #else
        var error as text = "neither true"
    #end if
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

  // Should have exactly one "result" definition, not "error"
  let error_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "error" {
          return Some(def);
        }
      None
    })
    .collect();

  assert!(
    error_defs.is_empty(),
    "Should not have 'error' definition (invalid state)"
  );

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
    "Should have exactly one 'result' definition"
  );
}

#[test]
fn test_posix_windows_with_source_file() {
  let source = r#"
#if #posix and #source_file == "test.lale"
    var config as text = "posix+test"
#else
    var config as text = "other"
#end if
write config
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

  let config_defs: Vec<_> = processed
    .statements
    .iter()
    .filter_map(|stmt| {
      if let Stmt::VarDef(def) = stmt
        && def.name.node == "config" {
          return Some(def);
        }
      None
    })
    .collect();

  assert_eq!(
    config_defs.len(),
    1,
    "Should have exactly one config definition after processing"
  );

  #[cfg(not(target_os = "windows"))]
  {
    // On POSIX with matching source_file, the then branch should be included
    assert_eq!(
      config_defs.len(),
      1,
      "Should have config definition from then branch on POSIX"
    );
  }

  #[cfg(target_os = "windows")]
  {
    // On Windows, the else branch should be included
    assert_eq!(
      config_defs.len(),
      1,
      "Should have config definition from else branch on Windows"
    );
  }
}
