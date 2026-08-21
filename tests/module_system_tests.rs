use lale::ast::builder::build_program;
use lale::ast::{Program, SourceLocation, Spanned, Stmt, UseImports};
use lale::semantic_analysis::{
  AnalyzerResults, ModuleError, ModuleGraph, ModuleId, ModuleResolver, ResolvedModule,
};
use lale::{LaleParser, Rule};
use pest::Parser;
use std::path::PathBuf;

/// Tests for the module system grammar rules.
///
/// This test suite covers:
/// - use_stmt: Module use statements
/// - module_path: Module path parsing
/// - use_import_list: Import list parsing
/// - ModuleResolver error handling
/// - Circular dependency detection

// ==================== MODULE PATH TESTS ====================

#[test]
fn test_module_path_simple() {
  let result = LaleParser::parse(Rule::qualified_identifier, "math");
  assert!(result.is_ok());
}

#[test]
fn test_module_path_nested() {
  let result = LaleParser::parse(Rule::qualified_identifier, "math -> lib");
  assert!(result.is_ok());
}

#[test]
fn test_module_path_deeply_nested() {
  let result = LaleParser::parse(Rule::qualified_identifier, "physics -> mechanics -> force");
  assert!(result.is_ok());
}

#[test]
fn test_module_path_parent_directory_rejected() {
  // Parent directory access is intentionally forbidden
  let result = LaleParser::parse(Rule::qualified_identifier, "../sibling");
  assert!(
    result.is_err(),
    "Parent directory access should be rejected"
  );
}

#[test]
fn test_module_path_parent_directory_nested_rejected() {
  // Parent directory access is intentionally forbidden
  let result = LaleParser::parse(Rule::qualified_identifier, "../../utils -> log");
  assert!(
    result.is_err(),
    "Parent directory access should be rejected"
  );
}

#[test]
fn test_use_stmt_slash_separator_rejected() {
  // Slash separator is not supported in use statements; use -> instead
  let result = LaleParser::parse(Rule::program, "use utils/log: helper");
  assert!(
    result.is_err(),
    "Slash separator should be rejected; use -> instead"
  );
}

#[test]
fn test_use_stmt_dot_separator_rejected() {
  // Dot separator is not supported in use statements; use -> instead
  let result = LaleParser::parse(Rule::program, "use utils.log: helper");
  assert!(
    result.is_err(),
    "Dot separator should be rejected; use -> instead"
  );
}

// ==================== USE IMPORT LIST TESTS ====================

#[test]
fn test_use_import_list_single() {
  let result = LaleParser::parse(Rule::use_import_list, "add");
  assert!(result.is_ok());
}

#[test]
fn test_use_import_list_multiple() {
  let result = LaleParser::parse(Rule::use_import_list, "add, subtract, multiply");
  assert!(result.is_ok());
}

// ==================== USE STATEMENT TESTS ====================

#[test]
fn test_use_stmt_simple() {
  let result = LaleParser::parse(Rule::use_stmt, "use math: add");
  assert!(result.is_ok());
}

#[test]
fn test_use_stmt_nested_module() {
  let result = LaleParser::parse(Rule::use_stmt, "use math -> lib: add, subtract");
  assert!(result.is_ok());
}

#[test]
fn test_use_stmt_glob_import() {
  let result = LaleParser::parse(Rule::use_stmt, "use utils");
  assert!(result.is_ok());
}

#[test]
fn test_use_stmt_parent_directory_rejected() {
  // Parent directory access is intentionally forbidden
  let result = LaleParser::parse(Rule::use_stmt, "use ../sibling: helper");
  assert!(
    result.is_err(),
    "Parent directory access should be rejected"
  );
}

#[test]
fn test_use_stmt_with_comment() {
  let result = LaleParser::parse(Rule::use_stmt, "use math: add // import math functions");
  assert!(result.is_ok());
}

// ==================== USE STATEMENT IN PROGRAM ====================

#[test]
fn test_use_stmt_in_program() {
  let source = "use math -> lib: add, subtract";
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");

  assert_eq!(program.statements.len(), 1);
  match &program.statements[0] {
    Stmt::Use(use_stmt) => {
      assert_eq!(use_stmt.module_path.len(), 2);
      assert_eq!(use_stmt.module_path[0].node, "math");
      assert_eq!(use_stmt.module_path[1].node, "lib");
      match &use_stmt.imports {
        UseImports::Named(names) => {
          assert_eq!(names.len(), 2);
          assert_eq!(names[0].node, "add");
          assert_eq!(names[1].node, "subtract");
        }
        _ => panic!("Expected Named imports"),
      }
    }
    _ => panic!("Expected Use statement"),
  }
}

#[test]
fn test_use_stmt_glob_in_program() {
  let source = "use utils";
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");

  assert_eq!(program.statements.len(), 1);
  match &program.statements[0] {
    Stmt::Use(use_stmt) => {
      assert_eq!(use_stmt.module_path.len(), 1);
      assert_eq!(use_stmt.module_path[0].node, "utils");
      match &use_stmt.imports {
        UseImports::All => {}
        _ => panic!("Expected All import"),
      }
    }
    _ => panic!("Expected Use statement"),
  }
}

#[test]
fn test_use_stmt_parent_directory_in_program_rejected() {
  // Parent directory access is intentionally forbidden at the grammar level
  let source = "use ../gravity: gravity_constant";
  let result = LaleParser::parse(Rule::program, source);
  assert!(
    result.is_err(),
    "Parent directory access should be rejected"
  );
}

#[test]
fn test_multiple_use_stmts_in_program() {
  let source = r#"use math -> lib: add
use physics -> gravity: constant
var x as i32 = 5"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");

  assert_eq!(program.statements.len(), 3);
  assert!(matches!(&program.statements[0], Stmt::Use(_)));
  assert!(matches!(&program.statements[1], Stmt::Use(_)));
  assert!(matches!(&program.statements[2], Stmt::VarDef(_)));
}

// ==================== SEMANTIC VALIDATION TESTS ====================

#[test]
fn test_use_stmt_in_function_rejected() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"fn test() returns nothing
  use math: add
end fn"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  assert!(
    !analyzer.is_valid(),
    "use inside function should be rejected"
  );
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.message.contains("module scope")));
}

// ==================== USE STATEMENT ORDERING TESTS ====================

#[test]
fn test_use_stmt_at_beginning_no_warning() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"use math: add
use physics: constant
var x as i32 = 5"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    !warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Should not warn when use statements are at the beginning"
  );
}

#[test]
fn test_use_stmt_after_def_warns() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"var x as i32 = 5
use math: add"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Should warn when use statement appears after var"
  );
}

#[test]
fn test_use_stmt_after_function_warns() {
  use lale::semantic_analysis::analyze_ast;

  let source = "fn foo() returns nothing\n  write 1\nend fn\nuse math: add";
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Should warn when use statement appears after function"
  );
}

#[test]
fn test_use_stmt_after_comment_no_warning() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"// This is a comment
use math: add
var x as i32 = 5"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    !warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Comments should not count as 'real' statements for ordering"
  );
}

#[test]
fn test_use_stmt_after_doc_comment_no_warning() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"/// Documentation comment
use math: add
var x as i32 = 5"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    !warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Doc comments should not count as 'real' statements for ordering"
  );
}

#[test]
fn test_multiple_misplaced_use_stmts_warn_each() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"var x as i32 = 5
use math: add
use physics: constant"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  let use_ordering_warnings: Vec<_> = warnings
    .iter()
    .filter(|w| w.message.contains("beginning of the file"))
    .collect();
  assert_eq!(
    use_ordering_warnings.len(),
    2,
    "Each misplaced use statement should generate a warning"
  );
}

#[test]
fn test_use_stmt_between_defs_warns() {
  use lale::semantic_analysis::analyze_ast;

  let source = r#"var x as i32 = 5
use math: add
var y as i32 = 10"#;
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let warnings = analyzer.get_warnings();
  assert!(
    warnings
      .iter()
      .any(|w| w.message.contains("beginning of the file")),
    "Should warn when use statement is sandwiched between definitions"
  );
}

// ==================== MODULE ERROR MESSAGE TESTS ====================

#[test]
fn test_module_not_found_error_message() {
  let error = ModuleError::ModuleNotFound {
    module_path: "math.lib".to_string(),
    search_path: PathBuf::from("/project"),
    location: SourceLocation::dummy(),
  };

  let msg = format!("{}", error);
  assert!(msg.contains("math.lib"), "Error should contain module path");
  assert!(msg.contains("not found"), "Error should indicate not found");
}

#[test]
fn test_circular_import_error_message() {
  let error = ModuleError::CircularImport {
    cycle: vec![
      ModuleId::new(PathBuf::from("a.lale")),
      ModuleId::new(PathBuf::from("b.lale")),
      ModuleId::new(PathBuf::from("a.lale")),
    ],
    location: SourceLocation::dummy(),
  };

  let msg = format!("{}", error);
  assert!(
    msg.contains("Circular import"),
    "Error should indicate circular import"
  );
  assert!(msg.contains("a.lale"), "Error should show modules in cycle");
  assert!(msg.contains("b.lale"), "Error should show modules in cycle");
}

#[test]
fn test_symbol_not_exported_error_message() {
  let error = ModuleError::SymbolNotExported {
    symbol: "private_fn".to_string(),
    module: ModuleId::new(PathBuf::from("utils.lale")),
    location: SourceLocation::dummy(),
  };

  let msg = format!("{}", error);
  assert!(
    msg.contains("private_fn"),
    "Error should contain symbol name"
  );
  assert!(
    msg.contains("not exported"),
    "Error should indicate not exported"
  );
  assert!(
    msg.contains("utils.lale"),
    "Error should contain module name"
  );
}

#[test]
fn test_symbol_conflict_error_message() {
  let error = ModuleError::SymbolConflict {
    symbol: "add".to_string(),
    first_module: ModuleId::new(PathBuf::from("math.lale")),
    second_module: ModuleId::new(PathBuf::from("utils.lale")),
    location: SourceLocation::dummy(),
  };

  let msg = format!("{}", error);
  assert!(msg.contains("add"), "Error should contain symbol name");
  assert!(msg.contains("math.lale"), "Error should show first module");
  assert!(
    msg.contains("utils.lale"),
    "Error should show second module"
  );
}

#[test]
fn test_use_not_at_top_level_error_message() {
  let error = ModuleError::UseNotAtTopLevel {
    location: SourceLocation::dummy(),
  };

  let msg = format!("{}", error);
  assert!(
    msg.contains("module scope"),
    "Error should indicate module scope requirement"
  );
}

#[test]
fn test_parse_error_message() {
  let error = ModuleError::ParseError {
    module: ModuleId::new(PathBuf::from("broken.lale")),
    message: "unexpected token".to_string(),
  };

  let msg = format!("{}", error);
  assert!(
    msg.contains("broken.lale"),
    "Error should contain module name"
  );
  assert!(
    msg.contains("unexpected token"),
    "Error should contain parse error message"
  );
}

// ==================== CIRCULAR DEPENDENCY DETECTION TESTS ====================

fn create_empty_program() -> Program {
  Program {
    statements: vec![],
    location: SourceLocation::dummy(),
    global_symbol_table: std::cell::RefCell::new(None),
  }
}

#[test]
fn test_circular_dependency_two_modules() {
  let mut graph = ModuleGraph::new();

  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("a.lale"))],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);

  let cycle = graph.detect_cycle();
  assert!(
    cycle.is_some(),
    "Should detect cycle between a.lale and b.lale"
  );

  let cycle = cycle.unwrap();
  assert!(cycle.len() >= 2, "Cycle should contain at least 2 modules");
}

#[test]
fn test_circular_dependency_three_modules() {
  let mut graph = ModuleGraph::new();

  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("c.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_c = ResolvedModule {
    id: ModuleId::new(PathBuf::from("c.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("a.lale"))],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);
  graph.add_module(mod_c);

  let cycle = graph.detect_cycle();
  assert!(
    cycle.is_some(),
    "Should detect cycle a.lale -> b.lale -> c.lale -> a.lale"
  );
}

#[test]
fn test_no_circular_dependency_chain() {
  let mut graph = ModuleGraph::new();

  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("c.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_c = ResolvedModule {
    id: ModuleId::new(PathBuf::from("c.lale")),
    program: create_empty_program(),
    dependencies: vec![],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);
  graph.add_module(mod_c);

  assert!(
    graph.detect_cycle().is_none(),
    "Should not detect cycle in linear chain"
  );
}

#[test]
fn test_no_circular_dependency_diamond() {
  let mut graph = ModuleGraph::new();

  // Diamond dependency: a -> b, a -> c, b -> d, c -> d
  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![
      ModuleId::new(PathBuf::from("b.lale")),
      ModuleId::new(PathBuf::from("c.lale")),
    ],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("d.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_c = ResolvedModule {
    id: ModuleId::new(PathBuf::from("c.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("d.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_d = ResolvedModule {
    id: ModuleId::new(PathBuf::from("d.lale")),
    program: create_empty_program(),
    dependencies: vec![],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);
  graph.add_module(mod_c);
  graph.add_module(mod_d);

  assert!(
    graph.detect_cycle().is_none(),
    "Diamond dependency is not a cycle"
  );
}

// ==================== TOPOLOGICAL SORT TESTS ====================

#[test]
fn test_topological_sort_linear_chain() {
  let mut graph = ModuleGraph::new();

  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("c.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_c = ResolvedModule {
    id: ModuleId::new(PathBuf::from("c.lale")),
    program: create_empty_program(),
    dependencies: vec![],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);
  graph.add_module(mod_c);

  let order = graph.topological_sort().expect("Should have valid order");

  let c_pos = order
    .iter()
    .position(|m| m.0.as_path() == std::path::Path::new("c.lale"))
    .unwrap();
  let b_pos = order
    .iter()
    .position(|m| m.0.as_path() == std::path::Path::new("b.lale"))
    .unwrap();
  let a_pos = order
    .iter()
    .position(|m| m.0.as_path() == std::path::Path::new("a.lale"))
    .unwrap();

  assert!(c_pos < b_pos, "c.lale should come before b.lale");
  assert!(b_pos < a_pos, "b.lale should come before a.lale");
}

#[test]
fn test_topological_sort_fails_on_cycle() {
  let mut graph = ModuleGraph::new();

  let mod_a = ResolvedModule {
    id: ModuleId::new(PathBuf::from("a.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
    use_statements: vec![],
    exports: None,
  };

  let mod_b = ResolvedModule {
    id: ModuleId::new(PathBuf::from("b.lale")),
    program: create_empty_program(),
    dependencies: vec![ModuleId::new(PathBuf::from("a.lale"))],
    use_statements: vec![],
    exports: None,
  };

  graph.add_module(mod_a);
  graph.add_module(mod_b);

  assert!(
    graph.topological_sort().is_none(),
    "Topological sort should fail on cycle"
  );
}

// ==================== MODULE RESOLVER TESTS ====================

#[test]
fn test_module_resolver_path_resolution() {
  let resolver = ModuleResolver::new(PathBuf::from("/project"));

  let path = resolver.resolve_module_path(
    &[
      Spanned::new("math".to_string(), SourceLocation::dummy()),
      Spanned::new("lib".to_string(), SourceLocation::dummy()),
    ],
    &PathBuf::from("/project/main.lale"),
  );

  assert_eq!(path, PathBuf::from("/project/math/lib.lale"));
}

#[test]
fn test_module_resolver_nested_path() {
  // Parent directory access is forbidden at grammar level, so the resolver
  // only handles forward-looking paths (subdirectories)
  let resolver = ModuleResolver::new(PathBuf::from("/project"));

  let path = resolver.resolve_module_path(
    &[
      Spanned::new("utils".to_string(), SourceLocation::dummy()),
      Spanned::new("helpers".to_string(), SourceLocation::dummy()),
    ],
    &PathBuf::from("/project/main.lale"),
  );

  assert_eq!(path, PathBuf::from("/project/utils/helpers.lale"));
}

#[test]
fn test_module_resolver_deeply_nested() {
  let resolver = ModuleResolver::new(PathBuf::from("/project"));

  let path = resolver.resolve_module_path(
    &[
      Spanned::new("physics".to_string(), SourceLocation::dummy()),
      Spanned::new("mechanics".to_string(), SourceLocation::dummy()),
      Spanned::new("force".to_string(), SourceLocation::dummy()),
    ],
    &PathBuf::from("/project/main.lale"),
  );

  assert_eq!(path, PathBuf::from("/project/physics/mechanics/force.lale"));
}

#[test]
fn test_module_resolver_is_valid_initially() {
  let resolver = ModuleResolver::new(PathBuf::from("/project"));
  assert!(resolver.is_valid(), "New resolver should have no errors");
}

#[test]
fn test_module_resolver_has_no_cycles_initially() {
  let resolver = ModuleResolver::new(PathBuf::from("/project"));
  assert!(!resolver.has_cycles(), "New resolver should have no cycles");
}
