use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for program structure rules in the Lale grammar.
///
/// This test suite covers:
/// - program: The top-level program rule that encompasses the entire file

// ==================== MINIMAL PROGRAMS ====================

#[test]
fn test_program_minimal_empty_rejected() {
  let result = LaleParser::parse(Rule::program, "");
  assert!(result.is_err(), "Empty program should be rejected");
}

#[test]
fn test_program_single_statement() {
  let result = LaleParser::parse(Rule::program, "var x = 5");
  assert!(result.is_ok());
}

#[test]
fn test_program_single_function_call() {
  let result = LaleParser::parse(Rule::program, "main()");
  assert!(result.is_ok());
}

// ==================== MULTI-STATEMENT PROGRAMS ====================

#[test]
fn test_program_two_statements() {
  let result = LaleParser::parse(Rule::program, "var x = 5\nvar y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_three_statements() {
  let result = LaleParser::parse(Rule::program, "var x = 5\nvar y = 10\nwrite x");
  assert!(result.is_ok());
}

#[test]
fn test_program_mixed_statements() {
  let result = LaleParser::parse(
    Rule::program,
    "var x = 5\nprocess(x)\nwrite x\nexit program 0",
  );
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_program_leading_whitespace() {
  let result = LaleParser::parse(Rule::program, "   var x = 5");
  assert!(result.is_ok());
}

#[test]
fn test_program_trailing_whitespace() {
  let result = LaleParser::parse(Rule::program, "var x = 5   ");
  assert!(result.is_ok());
}

#[test]
fn test_program_surrounding_whitespace() {
  let result = LaleParser::parse(Rule::program, "   var x = 5   ");
  assert!(result.is_ok());
}

#[test]
fn test_program_multiple_newlines() {
  let result = LaleParser::parse(Rule::program, "var x = 5\n\n\nvar y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_mixed_whitespace() {
  let result = LaleParser::parse(Rule::program, "  var x = 5  \n  var y = 10  ");
  assert!(result.is_ok());
}

// ==================== STATEMENT SEPARATION ====================

#[test]
fn test_program_semicolon_separator() {
  let result = LaleParser::parse(Rule::program, "var x = 5; var y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_newline_separator() {
  let result = LaleParser::parse(Rule::program, "var x = 5\nvar y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_mixed_separators() {
  let result = LaleParser::parse(Rule::program, "var x = 5; var y = 10\nvar z = 15");
  assert!(result.is_ok());
}

#[test]
fn test_program_multiple_semicolons() {
  let result = LaleParser::parse(Rule::program, "var x = 5;;; var y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_semicolon_with_whitespace() {
  let result = LaleParser::parse(Rule::program, "var x = 5 ; var y = 10");
  assert!(result.is_ok());
}

// ==================== VARIOUS STATEMENT TYPES ====================

#[test]
fn test_program_declarations_only() {
  let result = LaleParser::parse(Rule::program, "var a = 1\nvar b = 2\nvar c = 3");
  assert!(result.is_ok());
}

#[test]
fn test_program_function_calls() {
  let result = LaleParser::parse(Rule::program, "init()\nmain()\ncleanup()");
  assert!(result.is_ok());
}

#[test]
fn test_program_io_statements() {
  let result = LaleParser::parse(
    Rule::program,
    "write \"start\"\nread input as text\nwarn \"done\"",
  );
  assert!(result.is_ok());
}

#[test]
fn test_program_control_statements() {
  let result = LaleParser::parse(Rule::program, "exit program 0");
  assert!(result.is_ok());
}

// ==================== COMPLEX PROGRAMS ====================

#[test]
fn test_program_realistic_main() {
  let code = r#"var x as u32 = 0
var name as text = "test"
write name
var result = calculate(x)
write result
exit program 0"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_exports_and_imports() {
  let code = "export var API_VERSION = 1\nvar message = \"hello\"\nwrite message";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_with_various_types() {
  let code =
    "var i as u32 = 5\nvar f as f64 = 3.14\nvar b as bool = true\nvar s as text = \"text\"";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

// ==================== COMMENT AND DOCUMENTATION ====================

#[test]
fn test_program_with_comments() {
  let code = "// This is a comment\nvar x = 5\n// Another comment\nwrite x";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_with_documentation() {
  let code = "/// This is documentation\nvar x = 5\n/// More documentation\nwrite x";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_mixed_comments_and_code() {
  let code = "/// File documentation\n// Comment\nvar x = 5 // inline comment\nwrite x";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_program_single_character_statements() {
  let result = LaleParser::parse(Rule::program, "var x = 1\nvar y = 2\nvar z = 3");
  assert!(result.is_ok());
}

#[test]
fn test_program_very_long_statement() {
  let long_statement = "var result = ".to_string() + &"a + ".repeat(50) + "b";
  let result = LaleParser::parse(Rule::program, &long_statement);
  assert!(result.is_ok());
}

#[test]
fn test_program_many_statements() {
  let mut code = String::new();
  for i in 0..20 {
    code.push_str(&format!("var v{} = {}\n", i, i));
  }
  let result = LaleParser::parse(Rule::program, &code);
  assert!(result.is_ok());
}

#[test]
fn test_program_deep_expression_nesting() {
  let result = LaleParser::parse(Rule::program, "var x = ((((1 + 2) * 3) / 4) - 5)");
  assert!(result.is_ok());
}

#[test]
fn test_program_complex_expressions() {
  let code = "var a = 1 + 2 * 3\nvar b = (a - 1) / 2\nvar c = b > 0 and a < 10";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

// ==================== WHITESPACE EDGE CASES ====================

#[test]
fn test_program_tabs_as_whitespace() {
  let result = LaleParser::parse(Rule::program, "var x = 5\n\tvar y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_mixed_tabs_and_spaces() {
  let result = LaleParser::parse(Rule::program, "  var x = 5\n\tvar y = 10");
  assert!(result.is_ok());
}

#[test]
fn test_program_carriage_returns() {
  let result = LaleParser::parse(Rule::program, "var x = 5\r\nvar y = 10");
  assert!(result.is_ok());
}

// ==================== REALISTIC PROGRAMS ====================

#[test]
fn test_program_echo_program() {
  let code = r#"read message as text
write message
exit program 0"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_calculation() {
  let code = r#"var x as u32 = 10
var y as u32 = 20
var sum = x + y
write "Sum: "
write sum"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_with_unicode_identifiers() {
  let code = "var α as f64 = 3.14\nvar β as f64 = 2.71\nvar γ = α + β\nwrite γ";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_with_units() {
  let code = r#"var distance as f64 in <m> = 100
var time as f64 in <s> = 10
write distance"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_export_import() {
  let code = r#"export var PUBLIC_API = 1
var config = PUBLIC_API
write config"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

// ==================== PROGRAM STRUCTURE VALIDATION ====================

#[test]
fn test_program_requires_valid_statements() {
  // Valid individual statements
  let valid = vec![
    "var x = 5",
    "write x",
    "exit program 0",
    "read input as text",
  ];

  for stmt in valid {
    let result = LaleParser::parse(Rule::program, stmt);
    assert!(
      result.is_ok(),
      "Valid statement should work in program: {}",
      stmt
    );
  }
}

#[test]
fn test_program_allows_many_separators() {
  let code = "var x = 1;;\n\n;;var y = 2";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok(), "Multiple separators should be allowed");
}

#[test]
fn test_program_complete_declaration() {
  let code = r#"export var VERSION as u32 = 2
var buffer = 1
write VERSION"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

// ==================== BOUNDARY TESTS ====================

#[test]
fn test_program_statement_boundary() {
  // Test that statements are properly separated
  let code = "var a = 1\nvar b = 2";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_no_trailing_separator_needed() {
  let code = "var x = 5\nwrite x";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_trailing_newline_ok() {
  let code = "var x = 5\n";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_multiple_trailing_newlines() {
  let code = "var x = 5\n\n\n";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}

#[test]
fn test_program_leading_newline() {
  let code = "\nvar x = 5";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok());
}
