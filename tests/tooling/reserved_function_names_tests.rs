//! Tests for reserved function names in Lale
//!
//! Lale reserves certain function names like 'main' because they are required
//! as entry points for the C runtime. Users cannot define functions with these names.

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

fn analyze_code(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error_containing(
  analyzer: &lale::semantic_analysis::OwnedAnalyzer,
  substring: &str,
) -> bool {
  analyzer
    .get_errors()
    .iter()
    .any(|e| e.to_string().contains(substring))
}

#[test]
fn test_main_function_rejected() {
  let code = r#"
fn main() returns nothing
    write "Hello"
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should have errors for 'main' function"
  );

  assert!(
    has_error_containing(&analyzer, "'main' is a reserved function name"),
    "Error message should mention 'main' is reserved. Got: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_main_function_not_allowed_at_global_scope() {
  let code = r#"
fn main() returns nothing
    write "test"
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject 'main' function even with other global code"
  );
}

#[test]
fn test_other_function_names_allowed() {
  let code = r#"
fn my_main() returns nothing
    write "Hello"
end fn

fn entry_point() returns nothing
    write "Start"
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow functions named 'my_main' or 'entry_point'. Errors: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_main_with_parameters_still_rejected() {
  let code = r#"
fn main(x as i32) returns nothing
    write "Hello"
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject 'main' function even with parameters"
  );
}

#[test]
fn test_main_with_return_type_still_rejected() {
  let code = r#"
fn main() returns i32
    return 0
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject 'main' function even with return type"
  );
}

#[test]
fn test_main_export_still_rejected() {
  let code = r#"
export fn main() returns nothing
    write "Hello"
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject 'main' function even if exported"
  );
}

#[test]
fn test_top_level_code_still_works_without_main() {
  let code = r#"
var x as i32 = 5
var y as i32 = 10
write "x + y"
"#;

  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Top-level code should work fine without a user-defined 'main'. Errors: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_functions_with_main_in_name_allowed() {
  let code = r#"
fn is_main() returns bool
    return true
end fn

fn main_loop() returns nothing
    write "Loop"
end fn

fn get_main_value() returns i32
    return 42
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    analyzer.is_valid(),
    "Should allow functions with 'main' in the name (but not 'main' exactly). Errors: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_error_message_mentions_no_std_lib() {
  let code = r#"
fn main() returns nothing
    write "Hello"
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(!analyzer.is_valid());

  assert!(
    has_error_containing(&analyzer, "--no-std-lib"),
    "Error message should mention --no-std-lib as alternative. Got: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_calling_main_is_rejected() {
  let code = r#"
fn other_func() returns nothing
    main()
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject calling 'main' function"
  );

  assert!(
    has_error_containing(&analyzer, "'main' is the entry point"),
    "Error message should mention main is the entry point. Got: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_calling_main_in_expression_rejected() {
  let code = r#"
fn other_func() returns i32
    var result as i32 = main()
    return result
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject calling 'main' in expression"
  );
}

#[test]
fn test_start_function_rejected() {
  let code = r#"
fn _start() returns nothing
    write "Hello"
end fn
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should have errors for '_start' function"
  );

  assert!(
    has_error_containing(&analyzer, "'_start' is a reserved function name"),
    "Error message should mention '_start' is reserved. Got: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}

#[test]
fn test_calling_start_is_rejected() {
  let code = r#"
fn other_func() returns nothing
    _start()
end fn

var x as i32 = 5
"#;

  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Should reject calling '_start' function"
  );

  assert!(
    has_error_containing(&analyzer, "'_start' is the entry point"),
    "Error message should mention _start is the entry point. Got: {}",
    analyzer
      .get_errors()
      .iter()
      .map(|e| e.to_string())
      .collect::<Vec<_>>()
      .join(", ")
  );
}
