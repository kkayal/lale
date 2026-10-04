//! `#function_name` Compiler Constant Tests
//!
//! Tests for the `#function_name` compiler constant that returns the current function name.
//! This constant provides debugging and logging capabilities by exposing the current function
//! name as a compile-time constant.
//!
//! The `#function_name` constant:
//! - Returns the current function name as a string
//! - Returns "<global>" when used at module scope
//! - Useful for debugging, logging, and diagnostic frameworks

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== BASIC TESTS ====================

#[test]
fn test_function_name_in_expression() {
  let code = "var func_name as text = #function_name";
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok(), "Should parse #function_name in expression");
}

#[test]
fn test_function_name_in_function() {
  let code = r#"
fn get_function_name() returns text
    return #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok(), "Should parse #function_name in function");
}

#[test]
fn test_function_name_in_write() {
  let code = r#"
fn test_function() returns nothing
    write #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse #function_name in write statement"
  );
}

#[test]
fn test_function_name_in_assignment() {
  let code = r#"
fn report() returns nothing
    var fn_name as text = #function_name
    write fn_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(result.is_ok(), "Should parse #function_name in assignment");
}

#[test]
fn test_function_name_at_global_scope() {
  let code = "var func_name as text = #function_name";
  let result = LaleParser::parse(Rule::program, code);
  // Should parse - returns "<global>" or similar at module scope
  assert!(
    result.is_ok(),
    "Should parse #function_name at global scope"
  );
}

// ==================== MULTIPLE FUNCTIONS ====================

#[test]
fn test_function_name_multiple_functions() {
  let code = r#"
fn func_a() returns text
    return #function_name
end fn

fn func_b() returns text
    return #function_name
end fn

fn func_c() returns text
    return #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse #function_name in multiple functions"
  );
}

#[test]
fn test_function_name_in_nested_calls() {
  let code = r#"
fn helper(name as text) returns nothing
    write name
end fn

fn wrapper() returns nothing
    helper(#function_name)
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse #function_name as function argument"
  );
}

// ==================== COMPILER CONSTANTS ====================

#[test]
fn test_function_name_with_compiler_constants() {
  let code = r#"
fn debug_info() returns nothing
    var file as text = #source_file
    var line as i32 = #source_line
    var func as text = #function_name
    write file
    write line
    write func
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse #function_name mixed with legacy constants"
  );
}

#[test]
fn test_legacy_constants_still_work() {
  let code = r#"
fn test() returns nothing
    var line as i32 = #source_line
    var file as text = #source_file
    var version as text = #compiler_version
    write line
    write file
    write version
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Legacy constants should still parse without #function_name"
  );
}

// ==================== USE CASES ====================

#[test]
fn test_debug_logging_framework() {
  let code = r#"
fn log_debug(msg as text) returns nothing
    write "DEBUG: "
    write #function_name
    write " - "
    write msg
end fn

fn process() returns nothing
    log_debug("Starting")
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse debug logging using #function_name"
  );
}

#[test]
fn test_assertion_framework() {
  let code = r#"
fn assert_true(condition as bool, message as text) returns nothing
    if not condition
        write "Assertion failed in "
        write #function_name
        write ": "
        write message
    end if
end fn

fn test_logic() returns nothing
    var x as i32 = 42
    assert_true(x > 0, "x should be positive")
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse assertion framework using #function_name"
  );
}

#[test]
fn test_error_reporting() {
  let code = r#"
fn report_error(code as i32, message as text) returns nothing
    write "Error in "
    write #function_name
    write " (code "
    write code
    write "): "
    write message
end fn

fn risky_operation() returns nothing
    var result as i32 = perform_task()
    if result < 0
        report_error(result, "Task failed")
    end if
end fn

fn perform_task() returns i32
    return -1
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse error reporting using #function_name"
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_function_name_with_spaces() {
  let code = r#"
fn test() returns text
    return   #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should handle whitespace around #function_name"
  );
}

#[test]
fn test_function_name_multiline() {
  let code = r#"
fn func1() returns text
    return #function_name
end fn

fn func2() returns text
    return #function_name
end fn

fn func3() returns text
    return #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse #function_name across multiple lines"
  );
}

#[test]
fn test_function_name_is_primary() {
  // #function_name should be recognized as a primary expression
  let code = r#"
fn get_name() returns text
    return #function_name
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "#function_name should be a valid primary expression"
  );
}

#[test]
fn test_function_name_in_variable_chain() {
  let code = r#"
fn chain_test() returns nothing
    var name1 as text = #function_name
    var name2 as text = #function_name
    var name3 as text = #function_name
    write name1
    write name2
    write name3
end fn
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should handle multiple #function_name in same scope"
  );
}

// ==================== REALISTIC SCENARIO ====================

#[test]
fn test_complete_logging_framework() {
  let code = r#"
fn log_info(message as text) returns nothing
    write "INFO ["
    write #function_name
    write "]: "
    write message
end fn

fn log_warn(message as text) returns nothing
    write "WARN ["
    write #function_name
    write "]: "
    write message
end fn

fn log_error(message as text) returns nothing
    write "ERROR ["
    write #function_name
    write "]: "
    write message
end fn

fn initialize_system() returns nothing
    log_info("System starting")
    log_warn("Low memory detected")
end fn

fn process_data() returns nothing
    log_info("Processing begins")
    var items as i32 = 100
    if items > 50
        log_warn("Large dataset")
    end if
    log_info("Processing complete")
end fn

fn shutdown_system() returns nothing
    log_info("Shutting down")
end fn

initialize_system()
process_data()
shutdown_system()
"#;
  let result = LaleParser::parse(Rule::program, code);
  assert!(
    result.is_ok(),
    "Should parse complete realistic logging framework"
  );
}
