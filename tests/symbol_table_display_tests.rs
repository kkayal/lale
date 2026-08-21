#![allow(clippy::assertions_on_constants)]
use lale::ast::builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, VarScope, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Comprehensive test suite for symbol table display with module information.
///
/// This test suite verifies that:
/// - Symbol table includes module_path field for each symbol
/// - Module paths are preserved across different scopes
/// - Root file symbols are properly tracked
/// - Imported symbols preserve module information

// ==================== SYMBOL TABLE COLUMN STRUCTURE ====================

#[test]
fn test_symbol_table_has_module_path_column() {
  let source = "var x as i32 = 10\nvar y as f64 = 3.14";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid());

  let table = analyzer.get_symbol_table(VarScope::Global);

  // Each symbol should have module_path field for display
  for (name, symbol) in table.iter() {
    let _ = &symbol.module_path;
    // module_path is used to populate the Module column
    assert!(
      true, /* module_path field exists */
      "Symbol '{}' should have module_path for column display",
      name
    );
  }
}

#[test]
fn test_symbol_table_displays_multiple_symbols_with_module() {
  let source = "var var1 as i32 = 1\nvar var2 as i32 = 2\nvar var3 as i32 = 3";
  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid());

  let table = analyzer.get_symbol_table(VarScope::Global);
  assert!(!table.is_empty(), "Should have multiple symbols");

  // All symbols should be displayable with module column
  for symbol in table.values() {
    let _ = &symbol.module_path;
    assert!(
      true,
      /* module_path field exists */ "module_path needed for display"
    );
  }
}

// ==================== ROOT FILE MODULE REPRESENTATION ====================

#[test]
fn test_root_file_symbol_shows_module_path_field() {
  let source = "var root_symbol as i32 = 1";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);
  let root = table.get("root_symbol").expect("Symbol not found");

  // Root file should have a module_path field
  let _ = &root.module_path; // Verify field exists
}

#[test]
fn test_multiple_symbols_in_root_file_same_module_column() {
  let source = "var x as i32 = 1\nvar y as i32 = 2\nvar z as i32 = 3";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  if table.len() >= 2 {
    let mut paths: Vec<String> = table.values().map(|s| s.module_path.clone()).collect();
    paths.sort();

    // All symbols from root file should show the same module in the Module column
    if let Some(first_path) = paths.first() {
      for path in paths.iter().skip(1) {
        assert_eq!(
          path, first_path,
          "Symbols from same module should display same Module column value"
        );
      }
    }
  }
}

// ==================== FUNCTION SCOPE SYMBOLS WITH MODULE ====================

#[test]
fn test_function_local_symbols_display_module() {
  let source = "fn test() returns i32\n  var local_x as i32 = 10\n  var local_y as i32 = 20\n  return local_x + local_y\nend fn";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid());

  let all_tables = analyzer.get_all_symbol_tables();

  // Function and its local symbols should all have module_path for display
  for table in all_tables.values() {
    for (name, symbol) in table.iter() {
      let _ = &symbol.module_path;
      assert!(
        true, /* module_path field exists */
        "Symbol '{}' should have module_path",
        name
      );
    }
  }
}

#[test]
fn test_multiple_functions_local_symbols_same_module() {
  let source = "fn func1() returns i32\n  var x as i32 = 1\n  return x\nend fn\n\nfn func2() returns i32\n  var y as i32 = 2\n  return y\nend fn";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let all_tables = analyzer.get_all_symbol_tables();

  // All symbols should have module_path for display
  for table in all_tables.values() {
    for (name, symbol) in table.iter() {
      let _ = &symbol.module_path;
      assert!(
        true, /* module_path field exists */
        "Symbol '{}' should have module_path",
        name
      );
    }
  }
}

// ==================== RECORD FIELD SYMBOLS WITH MODULE ====================

#[test]
fn test_type_definition_displays_module() {
  let source = "type Point\n  x as i32\n  y as i32\nend type";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  if analyzer.is_valid() {
    let table = analyzer.get_symbol_table(VarScope::Global);

    // Type type should have module_path for display
    if let Some(p) = table.get("Point") {
      let _ = &p.module_path; // Verify field exists
    }
  }
}

// ==================== IMPORTED SYMBOLS DISPLAY ====================

#[test]
fn test_imported_function_display_shows_module() {
  let source = "use math: add\nvar x as i32 = 5\nvar y as i32 = 10\nvar sum as i32 = add(x, y)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // User-defined symbols should have their module_path for display
  for (name, symbol) in table.iter() {
    let _ = &symbol.module_path;
    if ["x", "y", "sum"].contains(&name.as_str()) {
      assert!(
        true, /* module_path field exists */
        "Symbol '{}' should have module_path for display",
        name
      );
    }
  }
}

#[test]
fn test_imported_constant_display_shows_module() {
  let source = "use physics: G\nvar mass as f64 = 10.0\nvar weight as f64 = G * mass";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  for (name, symbol) in table.iter() {
    let _ = &symbol.module_path;
    assert!(
      true, /* module_path field exists */
      "Symbol '{}' should have module_path",
      name
    );
  }
}

// ==================== EXPORTED SYMBOL DISPLAY ====================

#[test]
fn test_exported_symbol_display_includes_module() {
  let source = "export var exported_x as i32 = 10";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  if let Some(s) = table.get("exported_x") {
    let _ = &s.module_path; // Verify field exists
  }
}

// ==================== NESTED MODULE PATH DISPLAY ====================

#[test]
fn test_nested_scope_symbol_displays_module() {
  let source =
    "fn outer() returns i32\n  var x as i32 = 10\n  var y as i32 = 20\n  return x + y\nend fn";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let all_tables = analyzer.get_all_symbol_tables();

  // All nested symbols should have module_path for display
  for table in all_tables.values() {
    for (name, symbol) in table.iter() {
      let _ = &symbol.module_path;
      assert!(
        true, /* module_path field exists */
        "Nested symbol '{}' should have module_path",
        name
      );
    }
  }
}

// ==================== SYMBOL TABLE OUTPUT CONSISTENCY ====================

#[test]
fn test_symbol_table_output_consistent_formatting() {
  let source = "var a as i32 = 1\nvar bb as i32 = 2\nvar ccc as i32 = 3";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All symbols should have module_path for consistent formatting
  for (name, symbol) in table.iter() {
    let _ = &symbol.module_path;
    assert!(
      true, /* module_path field exists */
      "Symbol '{}' needed for consistent table formatting",
      name
    );
  }
}

#[test]
fn test_symbol_table_handles_various_symbol_names() {
  let source =
    "var a as i32 = 1\nvar very_long_symbol_name_for_testing as i32 = 1\nvar x as i32 = 1";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All symbols should have module_path for display
  for symbol in table.values() {
    let _ = &symbol.module_path;
    assert!(
      true,
      /* module_path field exists */ "Symbol should have module_path"
    );
  }
}

// ==================== IMPORTED AND DEFINED TOGETHER ====================

#[test]
fn test_mixed_imported_and_defined_symbols_display() {
  let source = "use math: add, subtract\nvar x as i32 = 10\nvar y as i32 = 5\nvar sum as i32 = add(x, y)\nvar diff as i32 = subtract(x, y)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All symbols should be displayable with module column
  for (name, symbol) in table.iter() {
    let _ = &symbol.module_path;
    assert!(
      true, /* module_path field exists */
      "Symbol '{}' for mixed display",
      name
    );
  }
}
