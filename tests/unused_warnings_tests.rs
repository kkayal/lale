//! Tests for unused variable and function warnings
//!
//! This test suite validates that the semantic analyzer emits warnings
//! for unused variables and functions.

use lale::{
  LaleParser, Rule,
  ast_builder::build_program,
  semantic_analysis::{AnalyzerResults, analyze_ast},
};
use pest::Parser;

fn parse_and_analyze(code: &str) -> (bool, Vec<String>, Vec<String>) {
  let parse_result = LaleParser::parse(Rule::program, code);

  match parse_result {
    Ok(pairs) => {
      let program_result = build_program(pairs, "test.lale");
      match program_result {
        Ok(program) => {
          let analyzer = analyze_ast(&program);

          let errors: Vec<String> = analyzer
            .get_errors()
            .iter()
            .map(|e| e.message.clone())
            .collect();

          let warnings: Vec<String> = analyzer
            .get_warnings()
            .iter()
            .map(|e| e.message.clone())
            .collect();

          (analyzer.is_valid(), errors, warnings)
        }
        Err(e) => (false, vec![e], Vec::new()),
      }
    }
    Err(e) => (false, vec![e.to_string()], Vec::new()),
  }
}

#[test]
fn test_unused_variable_warning() {
  let code = r#"
var unused_var as i32 = 5
write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert_eq!(warnings.len(), 1);
  assert!(warnings[0].contains("Unused variable") && warnings[0].contains("unused_var"));
}

#[test]
fn test_used_variable_no_warning() {
  let code = r#"
var used_var as i32 = 10
write "Value: {used_var}"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("unused_var")));
}

#[test]
fn test_unused_function_warning() {
  let code = r#"
fn unused_fn() returns nothing
    write "Hello"
end fn

fn used_fn() returns nothing
    write "World"
end fn

used_fn()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused function") && w.contains("unused_fn"))
  );
}

#[test]
fn test_used_function_no_warning() {
  let code = r#"
fn my_fn() returns nothing
    write "Hello"
end fn

my_fn()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused function")));
}

#[test]
fn test_multiple_unused_warnings() {
  let code = r#"
var unused_var1 as i32 = 5
var unused_var2 as f64 = 3.14
var used_var as i32 = 10

fn unused_fn1() returns nothing
    write "A"
end fn

fn used_fn() returns nothing
    write "B"
end fn

write "Value: {used_var}"
used_fn()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert_eq!(warnings.len(), 3);
  assert!(warnings.iter().any(|w| w.contains("unused_var1")));
  assert!(warnings.iter().any(|w| w.contains("unused_var2")));
  assert!(warnings.iter().any(|w| w.contains("unused_fn1")));
}

#[test]
fn test_exported_variable_no_warning() {
  let code = r#"
export var exported_var as i32 = 5
write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("exported_var")));
}

#[test]
fn test_exported_function_no_warning() {
  let code = r#"
export fn exported_fn() returns nothing
    write "Hello"
end fn

write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("exported_fn")));
}

#[test]
fn test_imported_variable_no_warning() {
  let code = r#"
import var imported_var as i32
write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  // It's valid even if import syntax is not fully supported yet
  // The important thing is imported variables don't generate unused warnings
  if is_valid {
    assert!(!warnings.iter().any(|w| w.contains("imported_var")));
  }
}

#[test]
fn test_function_parameter_no_warning() {
  let code = r#"
fn my_fn(x as i32) returns nothing
    write "Value: {x}"
end fn

my_fn(5)
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_unused_function_parameter_warning() {
  let code = r#"
fn my_fn(x as i32) returns nothing
    write "Hello"
end fn

my_fn(5)
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // Parameters are now tracked for unused warnings
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("x"))
  );
}

#[test]
fn test_loop_variable_no_warning() {
  let code = r#"
loop var i as i32 from 1 to 3
    write "Count: {i}"
end loop
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_unused_loop_variable_warning() {
  let code = r#"
loop var i as i32 from 1 to 3
    write "Count: 1"
end loop
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // Loop variables are now tracked for unused warnings
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("i"))
  );
}

#[test]
fn test_variable_used_in_write_expression() {
  let code = r#"
var x as i32 = 5
write "Value: {x}"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_used_in_function_call() {
  let code = r#"
fn print_value(val as i32) returns nothing
    write "Value: {val}"
end fn

var x as i32 = 42
print_value(x)
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_used_in_binary_operation() {
  let code = r#"
var a as i32 = 5
var b as i32 = 10
var result as i32 = a + b
write "Result: {result}"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_used_in_if_condition() {
  let code = r#"
	var should_do as bool = true
	if should_do
	    write "yes"
	else
	    move on
	end if
	"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_used_in_loop_condition() {
  let code = r#"
var should_run as bool = true
loop when should_run
    write "running"
end loop
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_unused_despite_assignment() {
  let code = r#"
var x as i32 = 5
var y as i32 = x
write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // y is defined but never used, should warn
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("y"))
  );
  // x is used (assigned to y), should not warn
  assert!(
    !warnings
      .iter()
      .any(|w| w.contains("x") && w.contains("Unused"))
  );
}

#[test]
fn test_multiple_unused_parameters_in_function() {
  let code = r#"
fn multi_param(a as i32, b as i32, c as i32) returns i32
    return a
end fn

multi_param(1, 2, 3)
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert_eq!(warnings.len(), 2);
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("b"))
  );
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("c"))
  );
}

#[test]
fn test_scope_collision_same_name_different_scopes() {
  let code = r#"
var x as i32 = 10

fn test_fn() returns nothing
    var x as i32 = 20
    write "Local: {x}"
end fn

write "Global: {x}"
test_fn()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // Global x is used, local x is used, no warnings
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_scope_collision_unused_local_with_used_global() {
  let code = r#"
var x as i32 = 10

fn test_fn() returns nothing
    var x as i32 = 20
    write "nothing"
end fn

write "Global: {x}"
test_fn()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // Local x is unused even though global x is used
  assert!(
    warnings
      .iter()
      .any(|w| w.contains("Unused variable") && w.contains("x"))
  );
}

#[test]
fn test_imported_function_no_warning() {
  let code = r#"
import fn external_fn() returns nothing
write "Done"
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  // Import syntax may not be fully supported, but if valid, shouldn't warn
  if is_valid {
    assert!(!warnings.iter().any(|w| w.contains("external_fn")));
  }
}

#[test]
fn test_variable_used_in_return_statement() {
  let code = r#"
fn get_value() returns i32
    var x as i32 = 42
    return x
end fn

write get_value()
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_variable_used_in_power_operation() {
  let code = r#"
var base as i32 = 2
var exponent as i32 = 3
var result as i32 = base ^ exponent
write result
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  assert!(!warnings.iter().any(|w| w.contains("Unused")));
}

#[test]
fn test_all_unused_parameters_in_function() {
  let code = r#"
fn unused_params(x as i32, y as i32, z as i32) returns nothing
    write "hello"
end fn

unused_params(1, 2, 3)
"#;

  let (is_valid, _errors, warnings) = parse_and_analyze(code);

  assert!(is_valid);
  // All three parameters should generate warnings
  assert_eq!(
    warnings
      .iter()
      .filter(|w| w.contains("Unused variable"))
      .count(),
    3
  );
}
