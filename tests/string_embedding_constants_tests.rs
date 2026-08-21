use lale::{LaleParser, Rule};
use pest::Parser;

/// Test string embedding with compiler constants
#[test]
fn test_string_embedding_with_compiler_version() {
  let code = r#"
    write "compiler version is {#compiler_version}"
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse string embedding with compiler version"
  );
}

#[test]
fn test_string_embedding_multiple_constants() {
  let code = r#"
    write "{#compiler_version} at {#source_file}:{#source_line}"
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse string embedding with multiple constants"
  );
}

#[test]
fn test_string_embedding_with_variables_and_constants() {
  let code = r#"
    var name as str = "test"
    var msg as str = "{name} with version {#compiler_version}"
    write msg
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse string embedding with variables and constants"
  );
}

#[test]
fn test_string_embedding_compiler_version_syntax() {
  // Test the grammar can parse {#compiler_version} inside a string
  let code = r#"
    write "Version: {#compiler_version}"
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse string embedding with compiler version"
  );
}

#[test]
fn test_string_embedding_all_compiler_constants() {
  let code = r#"
    write "Version {#compiler_version} in {#function_name}"
    write "File {#source_file} line {#source_line}"
    write "Compiled at {#compile_time}"
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse all compiler constants in string embedding"
  );
}

#[test]
fn test_string_embedding_nested_expressions() {
  let code = r#"
    var x as i32 = 42
    write "Value {x + 10} Version {#compiler_version}"
  "#;

  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse nested expressions with compiler constants"
  );
}
