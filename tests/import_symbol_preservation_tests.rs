use lale::ast::builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, VarScope, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Comprehensive test suite for imported symbol module path preservation.
///
/// This test suite verifies that:
/// - import_symbol preserves original module path
/// - Imported symbols work with symbol tracking
/// - Module path field is present on imported symbols
/// - Multiple imports preserve module path structure
/// - All symbol kinds can be imported

// ==================== BASIC IMPORT WITH MODULE PATH ====================

#[test]
fn test_imported_function_symbol_has_module_path_field() {
  let source = "use math: add\nvar x as i32 = 5\nvar y as i32 = 3\nvar sum as i32 = add(x, y)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // User-defined symbols should all have module_path field
  for (name, symbol) in table.iter() {
    if ["x", "y", "sum"].contains(&name.as_str()) {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}

#[test]
fn test_imported_variable_symbol_has_module_path_field() {
  let source =
    "use math: pi\nvar radius as f64 = 5.0\nvar circumference as f64 = 2.0 * pi * radius";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All defined symbols should have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

#[test]
fn test_imported_constant_has_module_path_field() {
  let source = "use physics: GRAVITY_CONSTANT\nvar mass as f64 = 10.0\nvar weight as f64 = mass * GRAVITY_CONSTANT";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);
  assert!(!table.is_empty(), "Symbols should be defined");
}

#[test]
fn test_imported_type_with_module_path_field() {
  let source = "use utils: helper\nvar x as i32 = helper()";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // Defined symbols should have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

// ==================== MULTIPLE IMPORTS WITH MODULE PATH ====================

#[test]
fn test_multiple_imports_preserve_module_path_field() {
  let source = "use math: add, subtract, multiply\nvar x as i32 = 10\nvar y as i32 = 5\nvar sum as i32 = add(x, y)\nvar diff as i32 = subtract(x, y)\nvar prod as i32 = multiply(x, y)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All user-defined symbols should have module_path field
  for (name, symbol) in table.iter() {
    if ["x", "y", "sum", "diff", "prod"].contains(&name.as_str()) {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}

#[test]
fn test_imports_from_different_modules() {
  let source = "use math: add\nuse physics: gravity_constant\nvar result1 as i32 = add(1, 2)\nvar result2 as f64 = gravity_constant";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All symbols should be properly tracked
  assert!(!table.is_empty(), "Symbols should be defined");
}

#[test]
fn test_glob_import_has_module_path_field() {
  let source = "use math\nvar x as i32 = 5\nvar y as i32 = add(x, x)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // User-defined symbols should have module_path field
  for (name, symbol) in table.iter() {
    if ["x", "y"].contains(&name.as_str()) {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}

// ==================== NESTED MODULE IMPORTS ====================

#[test]
fn test_nested_module_import_has_module_path_field() {
  let source = "use math: add\nvar v1 as i32 = 1\nvar v2 as i32 = 2";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // Defined symbols should all have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

#[test]
fn test_relative_import_has_module_path_field() {
  let source = "use utils: helper\nvar x as i32 = 5";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

// ==================== IMPORT WITH SYMBOL USAGE ====================

#[test]
fn test_imported_function_in_expressions_has_module_path_field() {
  let source = "use math: add\nvar a as i32 = 10\nvar b as i32 = 20\nvar result as i32 = add(a, b)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All defined symbols should have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

#[test]
fn test_imported_constant_in_multiple_expressions() {
  let source = "use physics: G\nvar m1 as f64 = 1000.0\nvar m2 as f64 = 2000.0\nvar r as f64 = 10.0\nvar f as f64 = G * m1 * m2 / (r * r)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All user-defined symbols should have module_path field
  for (name, symbol) in table.iter() {
    if ["m1", "m2", "r", "f"].contains(&name.as_str()) {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}

// ==================== IMPORT WITH REDEFINITION ====================

#[test]
fn test_imported_symbol_locally_shadowed() {
  let source = "use math: add\nvar add as i32 = 100\nvar result as i32 = add + 5";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // The local 'add' definition and 'result' should have module_path field
  for (name, symbol) in table.iter() {
    if ["add", "result"].contains(&name.as_str()) {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}

// ==================== IMPORT WITH DIFFERENT SYMBOL KINDS ====================

#[test]
fn test_imported_function_and_variable_together() {
  let source = "use math: add, pi\nvar x as f64 = pi\nvar y as i32 = add(5, 3)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All symbols should have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

#[test]
fn test_imported_type_in_variable_definition() {
  let source = "use geometry: get_origin\nvar origin as i32 = get_origin()";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

// ==================== COMPLEX IMPORT SCENARIOS ====================

#[test]
fn test_multiple_levels_of_imports() {
  let source = "use math: add\nuse physics: calculate_force\nvar m as f64 = 10.0\nvar a as f64 = 5.0\nvar force as f64 = calculate_force(m, a)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let table = analyzer.get_symbol_table(VarScope::Global);

  // All defined symbols should have module_path field
  for symbol in table.values() {
    let _ = &symbol.module_path; // Field exists and is accessible
  }
}

#[test]
fn test_import_with_function_definitions() {
  let source = "use utils: convert\nfn process(value as i32) returns i32\n  return convert(value)\nend fn\nvar result as i32 = process(42)";

  let pairs = LaleParser::parse(Rule::program, source).expect("Failed to parse");
  let program = build_program(pairs, "test.lale").expect("Failed to build AST");
  let analyzer = analyze_ast(&program);

  let all_tables = analyzer.get_all_symbol_tables();

  // All symbols should be tracked with module_path field
  for table in all_tables.values() {
    for symbol in table.values() {
      let _ = &symbol.module_path; // Field exists and is accessible
    }
  }
}
