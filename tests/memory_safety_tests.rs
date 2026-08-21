//! Tests for memory safety - escaping pointer detection.
//!
//! This test suite covers:
//! - Detection of returning pointers to local variables
//! - Allowed cases: returning parameter pointers, using pointers locally
//! - Symbol table fields: linkage, visibility, storage_class, is_definition, is_initialized
//! - Symbol table linkage to AST nodes

use lale::ast::{Stmt, SymbolKind};
use lale::ast_builder::build_program;
use lale::semantic_analysis::{
  AnalyzerResults, Linkage, StorageClass, VarScope, Visibility, analyze_ast,
};
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

// ==================== ESCAPING POINTER TESTS ====================

#[test]
fn test_return_pointer_to_local_is_error() {
  let code = r#"
fn bad() returns pointer
    var x as i32 = 42
    return pointer to x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(
    &analyzer,
    "Cannot return pointer to local variable"
  ));
  assert!(has_error_containing(&analyzer, "'x'"));
}

#[test]
fn test_return_pointer_to_parameter_is_ok() {
  let code = r#"
fn ok(p as pointer) returns pointer
    return p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

#[test]
fn test_use_pointer_to_local_within_function_is_ok() {
  let code = r#"
fn ok() returns i32
    var x as i32 = 42
    var p as pointer = pointer to x
    return unsafe value at p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

#[test]
fn test_return_pointer_to_local_in_expression_is_error() {
  let code = r#"
fn bad() returns pointer
    var x as i32 = 42
    var y as i32 = 10
    return pointer to x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(
    &analyzer,
    "Cannot return pointer to local variable"
  ));
}

#[test]
fn test_multiple_local_vars_return_pointer_to_first() {
  let code = r#"
fn bad() returns pointer
    var a as i32 = 1
    var b as i32 = 2
    var c as i32 = 3
    return pointer to a
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(&analyzer, "'a'"));
}

#[test]
fn test_return_nothing_is_ok() {
  let code = r#"
fn ok() returns nothing
    var x as i32 = 42
    var p as pointer = pointer to x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

#[test]
fn test_return_value_not_pointer_is_ok() {
  let code = r#"
fn ok() returns i32
    var x as i32 = 42
    return x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

// ==================== SYMBOL TABLE FIELD TESTS ====================

#[test]
fn test_symbol_linkage_internal_by_default() {
  let code = r#"
var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert_eq!(symbol.linkage, Linkage::Internal);
}

#[test]
fn test_symbol_linkage_export() {
  let code = r#"
export var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert_eq!(symbol.linkage, Linkage::Export);
}

#[test]
fn test_symbol_linkage_import() {
  let code = r#"
unsafe import decl x as i32
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert_eq!(symbol.linkage, Linkage::Import);
}

#[test]
fn test_function_linkage_internal_by_default() {
  let code = r#"
fn foo() returns nothing
    var x as i32 = 1
end fn
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("foo").expect("Symbol 'foo' not found");
  assert_eq!(symbol.linkage, Linkage::Internal);
}

#[test]
fn test_function_linkage_export() {
  let code = r#"
export fn foo() returns nothing
    var x as i32 = 1
end fn
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("foo").expect("Symbol 'foo' not found");
  assert_eq!(symbol.linkage, Linkage::Export);
}

#[test]
fn test_symbol_visibility_default_public() {
  let code = r#"
var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert_eq!(symbol.visibility, Visibility::Public);
}

#[test]
fn test_symbol_storage_class_default() {
  let code = r#"
var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert_eq!(symbol.storage_class, StorageClass::Default);
}

#[test]
fn test_symbol_is_definition_true_for_def() {
  let code = r#"
var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert!(symbol.is_definition);
}

#[test]
fn test_symbol_is_initialized_true_for_def() {
  let code = r#"
var x as i32 = 42
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert!(symbol.is_initialized);
}

#[test]
fn test_symbol_is_initialized_false_for_unsafe_decl() {
  let code = r#"
unsafe decl x as i32
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("x").expect("Symbol 'x' not found");
  assert!(!symbol.is_initialized);
}

#[test]
fn test_function_is_definition_true() {
  let code = r#"
fn foo() returns nothing
    var x as i32 = 1
end fn
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("foo").expect("Symbol 'foo' not found");
  assert!(symbol.is_definition);
}

#[test]
fn test_function_is_initialized_true() {
  let code = r#"
fn foo() returns nothing
    var x as i32 = 1
end fn
"#;
  let analyzer = analyze_code(code);
  let global_table = analyzer.get_symbol_table(VarScope::Global);
  let symbol = global_table.get("foo").expect("Symbol 'foo' not found");
  assert!(symbol.is_initialized);
}

// ==================== AST-SYMBOL TABLE LINKAGE TESTS ====================

#[test]
fn test_fn_def_has_symbol_table_after_analysis() {
  let code = r#"
fn example(x as i32, y as i32) returns i32
    var z as i32 = x + y
    return z
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");

  // Before analysis, symbol_table should be None
  if let Some(Stmt::FnDef(fn_def)) = program.statements.first() {
    assert!(
      fn_def.symbol_table.borrow().is_none(),
      "Symbol table should be None before analysis"
    );
  }

  // Run analysis
  let _analyzer = analyze_ast(&program);

  // After analysis, symbol_table should be populated
  if let Some(Stmt::FnDef(fn_def)) = program.statements.first() {
    let table = fn_def.symbol_table.borrow();
    assert!(
      table.is_some(),
      "Symbol table should be Some after analysis"
    );

    let table = table.as_ref().unwrap();
    assert!(
      table.contains_key("x"),
      "Symbol table should contain parameter 'x'"
    );
    assert!(
      table.contains_key("y"),
      "Symbol table should contain parameter 'y'"
    );
    assert!(
      table.contains_key("z"),
      "Symbol table should contain local 'z'"
    );
  } else {
    panic!("Expected FnDef statement");
  }
}

#[test]
fn test_fn_def_symbol_table_contains_correct_types() {
  let code = r#"
fn test(a as f64) returns f64
    var b as f64 = a * 2
    return b
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  let _analyzer = analyze_ast(&program);

  if let Some(Stmt::FnDef(fn_def)) = program.statements.first() {
    let table = fn_def.symbol_table.borrow();
    let table = table.as_ref().expect("Symbol table should exist");

    let sym_a = table.get("a").expect("Should have symbol 'a'");
    assert!(matches!(sym_a.kind, SymbolKind::Parameter));

    let sym_b = table.get("b").expect("Should have symbol 'b'");
    assert!(matches!(sym_b.kind, SymbolKind::Variable));
  } else {
    panic!("Expected FnDef statement");
  }
}

#[test]
fn test_multiple_functions_have_separate_symbol_tables() {
  let code = r#"
fn first(x as i32) returns i32
    var a as i32 = x
    return a
end fn

fn second(y as i32) returns i32
    var b as i32 = y
    return b
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  let _analyzer = analyze_ast(&program);

  let mut fn_count = 0;
  for stmt in &program.statements {
    if let Stmt::FnDef(fn_def) = stmt {
      let table = fn_def.symbol_table.borrow();
      let table = table.as_ref().expect("Symbol table should exist");

      if fn_def.name.node == "first" {
        assert!(table.contains_key("x"), "first should have 'x'");
        assert!(table.contains_key("a"), "first should have 'a'");
        assert!(!table.contains_key("y"), "first should not have 'y'");
        assert!(!table.contains_key("b"), "first should not have 'b'");
      } else if fn_def.name.node == "second" {
        assert!(table.contains_key("y"), "second should have 'y'");
        assert!(table.contains_key("b"), "second should have 'b'");
        assert!(!table.contains_key("x"), "second should not have 'x'");
        assert!(!table.contains_key("a"), "second should not have 'a'");
      }
      fn_count += 1;
    }
  }
  assert_eq!(fn_count, 2, "Should have analyzed 2 functions");
}

// ==================== GLOBAL SYMBOL TABLE LINKAGE TESTS ====================

#[test]
fn test_program_has_global_symbol_table_after_analysis() {
  let code = r#"
var globalX as i32 = 42
var globalY as f64 = 3.14

fn myFunc() returns i32
    return globalX
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");

  // Before analysis, global_symbol_table should be None
  assert!(
    program.global_symbol_table.borrow().is_none(),
    "Global symbol table should be None before analysis"
  );

  // Run analysis
  let _analyzer = analyze_ast(&program);

  // After analysis, global_symbol_table should be populated
  let table = program.global_symbol_table.borrow();
  assert!(
    table.is_some(),
    "Global symbol table should be Some after analysis"
  );

  let table = table.as_ref().unwrap();
  assert!(table.contains_key("globalX"), "Should contain 'globalX'");
  assert!(table.contains_key("globalY"), "Should contain 'globalY'");
  assert!(table.contains_key("myFunc"), "Should contain 'myFunc'");
}

#[test]
fn test_global_symbol_table_has_correct_kinds() {
  let code = r#"
var x as i32 = 1

fn foo() returns i32
    return x
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  let _analyzer = analyze_ast(&program);

  let table = program.global_symbol_table.borrow();
  let table = table.as_ref().expect("Global symbol table should exist");

  let sym_x = table.get("x").expect("Should have 'x'");
  assert!(matches!(sym_x.kind, SymbolKind::Variable));

  let sym_foo = table.get("foo").expect("Should have 'foo'");
  assert!(matches!(sym_foo.kind, SymbolKind::Function));
}

#[test]
fn test_codegen_can_access_all_symbols_from_ast() {
  let code = r#"
var globalVar as i32 = 100

fn compute(x as i32) returns i32
    var local as i32 = x + globalVar
    return local
end fn
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  let _analyzer = analyze_ast(&program);

  // Simulate what code generation would do:
  // 1. Access global symbols from program.global_symbol_table
  let global_table = program.global_symbol_table.borrow();
  let global_table = global_table.as_ref().expect("Need global symbols");
  assert!(global_table.contains_key("globalVar"));
  assert!(global_table.contains_key("compute"));

  // 2. Access function-local symbols from fn_def.symbol_table
  for stmt in &program.statements {
    if let Stmt::FnDef(fn_def) = stmt {
      let fn_table = fn_def.symbol_table.borrow();
      let fn_table = fn_table.as_ref().expect("Need function symbols");
      assert!(
        fn_table.contains_key("x"),
        "Parameter 'x' should be in function scope"
      );
      assert!(
        fn_table.contains_key("local"),
        "Local 'local' should be in function scope"
      );
    }
  }
}

// ==================== FIXED: RETURN POINTER TO PARAMETER ====================

#[test]
fn test_return_pointer_to_parameter_ref_is_ok() {
  // Parameters live in the caller frame — pointers to them are safe to return.
  // This was previously incorrectly rejected (is_local_variable included parameters).
  let code = r#"
fn ok(p as pointer) returns pointer
    return pointer to p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Returning pointer to parameter should be allowed. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_return_pointer_to_param_of_i32_is_ok() {
  let code = r#"
fn get_ref(x as i32) returns pointer
    return pointer to x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Returning pointer to i32 parameter should be allowed. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== FIXED: STORE POINTER-TO-LOCAL IN GLOBAL ====================

#[test]
fn test_assign_pointer_to_local_to_global_is_error() {
  // Store pointer to localVar in global: forbidden
  let code = r#"
unsafe decl global_ptr as pointer

fn bad() returns void
    var x as i32 = 42
    global_ptr = pointer to x
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject storing pointer-to-local in global"
  );
  assert!(
    has_error_containing(&analyzer, "Cannot assign pointer to local variable"),
    "Error should mention pointer to local for global assignment. Errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_assign_pointer_to_param_to_global_is_ok() {
  // Parameters live in caller frame — storing pointer-to-param in global is safe
  let code = r#"
unsafe decl global_ptr as pointer

fn ok(p as pointer) returns void
    global_ptr = pointer to p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Storing pointer to parameter in global should be allowed. Errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== POINTER-VARIABLE PROVENANCE TESTS ====================

fn has_warning_containing(
  analyzer: &lale::semantic_analysis::OwnedAnalyzer,
  substring: &str,
) -> bool {
  analyzer
    .get_warnings()
    .iter()
    .any(|w| w.contains(substring))
}

#[test]
fn test_return_pointer_variable_to_local_is_error() {
  let code = r#"
fn bad() returns pointer
    var x as i32 = 42
    var p as pointer = pointer to x
    return p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(
    &analyzer,
    "Cannot return pointer to local variable"
  ));
  assert!(has_error_containing(&analyzer, "'x'"));
  assert!(has_error_containing(&analyzer, "'p'"));
}

#[test]
fn test_assign_pointer_variable_to_global_is_error() {
  let code = r#"
unsafe decl global_ptr as pointer

fn bad() returns nothing
    var x as i32 = 42
    var p as pointer = pointer to x
    global_ptr = p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());
  assert!(has_error_containing(
    &analyzer,
    "Cannot assign pointer to local variable"
  ));
  assert!(has_error_containing(&analyzer, "'x'"));
  assert!(has_error_containing(&analyzer, "'global_ptr'"));
}

#[test]
fn test_reassign_pointer_to_global_clears_local_provenance() {
  let code = r#"
var g as i32 = 10

fn ok() returns pointer
    var x as i32 = 42
    var p as pointer = pointer to x
    p = pointer to g
    return p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

#[test]
fn test_reassign_pointer_to_raw_clears_local_provenance() {
  let code = r#"
fn ok() returns pointer
    var x as i32 = 42
    var p as pointer = pointer to x
    p = 0 as pointer
    return p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

#[test]
fn test_return_pointer_variable_to_parameter_is_ok() {
  let code = r#"
fn ok(x as i32) returns pointer
    var p as pointer = pointer to x
    return p
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
}

// ==================== RELEASE-MISUSE WARNING TESTS ====================

#[test]
fn test_double_release_warns() {
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(8 as u64)
    release buf
    release buf
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  assert!(has_warning_containing(
    &analyzer,
    "is released more than once (double free)"
  ));
}

#[test]
fn test_use_after_release_warns() {
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(8 as u64)
    release buf
    unsafe value at buf = 42 as u64
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  assert!(has_warning_containing(&analyzer, "use-after-free"));
}

#[test]
fn test_on_exit_release_then_deref_no_use_after_free_warning() {
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(8 as u64)
    on exit release buf
    unsafe value at buf = 42 as u64
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  assert!(!has_warning_containing(&analyzer, "use-after-free"));
}

#[test]
fn test_release_then_on_exit_release_warns_double_free() {
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(8 as u64)
    release buf
    on exit release buf
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  assert!(has_warning_containing(
    &analyzer,
    "is released more than once (double free)"
  ));
}

#[test]
fn test_on_exit_release_then_release_warns_double_free() {
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(8 as u64)
    on exit release buf
    release buf
end fn
"#;
  let analyzer = analyze_code(code);
  assert!(analyzer.is_valid(), "Errors: {:?}", analyzer.get_errors());
  assert!(has_warning_containing(
    &analyzer,
    "is released more than once (double free)"
  ));
}
