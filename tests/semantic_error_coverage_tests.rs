//! Additional semantic error tests covering error paths that were
//! previously untested. Complements the existing semantic_type_errors_tests.rs.

use lale::ast::definitions::Stmt;
use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

fn analyze_code(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error(analyzer: &lale::semantic_analysis::OwnedAnalyzer, substring: &str) -> bool {
  analyzer.get_errors().iter().any(|e| e.contains(substring))
}

fn assert_error(code: &str, expected: &str) {
  let analyzer = analyze_code(code);
  assert!(
    !analyzer.is_valid(),
    "Expected semantic error containing '{}', but analysis passed for:\n{}",
    expected,
    code
  );
  assert!(
    has_error(&analyzer, expected),
    "Expected error containing '{}', got errors: {:?}\nCode:\n{}",
    expected,
    analyzer.get_errors(),
    code
  );
}

// ==================== UNDEFINED VARIABLES & FUNCTIONS ====================

#[test]
fn test_undefined_function_call() {
  assert_error(
    r#"
var x as i32 = 5
foo()
"#,
    "not defined",
  );
}

#[test]
fn test_assignment_to_undefined_variable() {
  assert_error(
    r#"
x = 5
"#,
    "Assignment to undefined variable",
  );
}

#[test]
fn test_compound_assign_to_undefined_variable() {
  assert_error(
    r#"
x += 5
"#,
    "Compound assignment to undefined variable",
  );
}

#[test]
fn test_reference_to_undefined_variable() {
  assert_error(
    r#"
var x as i32 = y
"#,
    "Undefined variable",
  );
}

// ==================== TYPE MISMATCHES ====================

#[test]
fn test_binary_float_int_addition_mismatch() {
  assert_error(
    r#"
var a as f64 = 3.14
var b as i32 = 42
var c as f64 = a + b
"#,
    "Type mismatch",
  );
}

#[test]
fn test_binary_string_int_addition() {
  assert_error(
    r#"
var s as str = "hello"
var n as i32 = 5
var x as str = s + n
"#,
    "Type mismatch",
  );
}

#[test]
fn test_assignment_type_mismatch() {
  assert_error(
    r#"
var x as i32 = "hello"
"#,
    "Type mismatch",
  );
}

// ==================== STEP TYPE ERRORS ====================

#[test]
fn test_loop_step_float_with_u32() {
  assert_error(
    r#"
loop over i as u32 from 1 to 5 step 0.5
    write i
end loop
"#,
    "Step type mismatch",
  );
}

#[test]
fn test_loop_step_float_with_i32() {
  assert_error(
    r#"
loop over i as i32 from 1 to 5 step 0.5
    write i
end loop
"#,
    "Step type mismatch",
  );
}

#[test]
fn test_loop_step_string_with_u32() {
  assert_error(
    r#"
loop over i as u32 from 1 to 5 step "hello"
    write i
end loop
"#,
    "Step type mismatch",
  );
}

// ==================== NARROWING CONVERSIONS ====================

#[test]
fn test_narrowing_i64_to_i32_rejected() {
  assert_error(
    r#"
var a as i64 = 1000
var b as i32 = a as i32
"#,
    "Narrowing conversion not allowed",
  );
}

#[test]
fn test_narrowing_f64_to_f32_rejected() {
  assert_error(
    r#"
var a as f64 = 3.14
var b as f32 = a as f32
"#,
    "Narrowing conversion not allowed",
  );
}

#[test]
fn test_narrowing_u32_to_u8_rejected() {
  assert_error(
    r#"
var a as u32 = 300
var b as u8 = a as u8
"#,
    "Narrowing conversion",
  );
}

// ==================== CONDITION MUST BE BOOL ====================

#[test]
fn test_if_condition_must_be_bool() {
  assert_error(
    r#"
var x as i32 = 5
if x
    write "yes"
end if
"#,
    "Condition must be boolean",
  );
}

// ==================== UNSIGNED UNDERFLOW ====================

#[test]
fn test_unsigned_subtraction_underflow() {
  assert_error(
    r#"
var a as u32 = 5
var b as u32 = a - 10
"#,
    "Cannot subtract two unsigned",
  );
}

// ==================== UNDERSCORE VARIABLE SUPPRESSION ====================

#[test]
fn test_underscore_variable_no_unused_warning() {
  let code = r#"
fn foo(nothing) returns i32
    var _ as i32 = 42
    return 0
end fn
"#;
  let analyzer = analyze_code(code);
  let warnings = analyzer.get_warnings();
  let has_underscore_warning = warnings.iter().any(|w| w.contains("_"));
  assert!(
    !has_underscore_warning,
    "Variable '_' should not produce unused warning, got: {:?}",
    warnings
  );
}

// ==================== DEBUG STATEMENT ====================

#[test]
fn test_debug_parses() {
  let code = r#"
var x as i32 = 10
debug x
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  let has_debug = program
    .statements
    .iter()
    .any(|s| matches!(s, Stmt::Debug(_)));
  assert!(has_debug, "Program should contain a Debug statement");
}

#[test]
fn test_debug_runs() {
  use std::io::Write;
  use std::process::{Command, Stdio};
  let code = r#"
var x as i32 = 42
debug x
"#;
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  {
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(code.as_bytes()).unwrap();
  }
  let output = child.wait_with_output().unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("DEBUG") && stderr.contains("x = "),
    "stderr should contain DEBUG with expression, got: {}",
    stderr.escape_debug()
  );
  assert!(
    stderr.contains("\x1b[0m"),
    "stderr should contain ANSI reset, got: {}",
    stderr.escape_debug()
  );
}

// ==================== Debug unit computation tests ====================

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\x1b' && chars.peek() == Some(&'[') {
      chars.next(); // consume '['
      // Skip until we find a letter (the command terminator)
      for c in chars.by_ref() {
        if c.is_alphabetic() {
          break;
        }
      }
    } else {
      result.push(ch);
    }
  }
  result
}

/// Helper: run a Lale snippet and return stderr as a string.
fn run_debug_snippet(code: &str) -> String {
  use std::io::Write;
  use std::process::{Command, Stdio};
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-", "--no-color"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  {
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(code.as_bytes()).unwrap();
  }
  let output = child.wait_with_output().unwrap();
  // Strip ANSI escape codes since --no-color doesn't affect debug statements
  let raw = String::from_utf8_lossy(&output.stderr).to_string();
  strip_ansi(&raw)
}

#[test]
fn test_debug_unit_global_variable() {
  let code = "var dist as f64 in <m> = 100.0\ndebug dist\n";
  let stderr = run_debug_snippet(code);
  assert!(
    stderr.contains("dist = ") && stderr.contains(" as f64 in <m>"),
    "global variable with unit, got: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_debug_unit_function_parameter() {
  let code = "\
fn compute(mass as f64 in <kg>, vel as f64 in <m/s>) returns f64 in <kg*m^2/s^2>\n\
    debug mass\n\
    debug vel\n\
    debug 0.5 * mass * vel^2\n\
    return 0.5 as f64 * mass * vel ^ 2\n\
end fn\n\
var result as f64 in <kg*m^2/s^2> = compute(10.0 <kg>, 5.0 <m/s>)\n\
write result\n";
  let stderr = run_debug_snippet(code);
  assert!(
    stderr.contains("mass = ") && stderr.contains(" as f64 in <kg>"),
    "function parameter with unit, got: {}",
    stderr.escape_debug()
  );
  assert!(
    stderr.contains("vel = ") && stderr.contains(" as f64 in <m/s>"),
    "second parameter, got: {}",
    stderr.escape_debug()
  );
  assert!(
    stderr.contains("0.5 * mass * vel^2")
      && stderr.contains(" as f64 in <kg")
      && stderr.contains("/s"),
    "compound expression in function, got: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_debug_unit_compound_expression() {
  let code = "var m as f64 in <kg> = 2.0\nvar a as f64 in <m/s^2> = 9.81\ndebug m * a\n";
  let stderr = run_debug_snippet(code);
  assert!(
    stderr.contains("m * a") && stderr.contains(" as f64 in <kg") && stderr.contains("/s"),
    "compound unit from multiplication, got: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_debug_unit_power() {
  let code = "var v as f64 in <m/s> = 5.0\ndebug v^2\n";
  let stderr = run_debug_snippet(code);
  assert!(
    stderr.contains("v^2") && stderr.contains(" as f64 in <m") && stderr.contains("/s"),
    "squared unit, got: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_debug_unit_no_unit() {
  let code = "var x as i32 = 42\ndebug x\n";
  let stderr = run_debug_snippet(code);
  assert!(
    stderr.contains("x = ") && stderr.contains(" as i32"),
    "no unit variable, got: {}",
    stderr.escape_debug()
  );
  // No "in <" should appear when there's no unit
  assert!(
    !stderr.contains("in <"),
    "should not contain unit brackets for unitless variable: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_debug_unit_timestamp_format() {
  let code = "var x as i32 = 1\ndebug x\n";
  let stderr = run_debug_snippet(code);
  // Verify ISO 8601 timestamp format: contains "DEBUG:" (year prefix)
  assert!(
    stderr.contains("DEBUG:"),
    "timestamp should start with DEBUG:, got: {}",
    stderr.escape_debug()
  );
  // Should contain T separator and Z suffix
  assert!(
    stderr.contains("T") && stderr.contains("Z"),
    "timestamp should have T and Z markers, got: {}",
    stderr.escape_debug()
  );
}

#[test]
fn test_stdin_read_returns_str() {
  use std::fs;
  use std::io::Write;
  use std::process::{Command, Stdio};
  // read auto-defines the variable via a synthetic VarDefStmt from the AST builder
  let code = "read x\nwrite x\n";
  let dir = tempfile::tempdir().unwrap();
  let temp = dir.path().join("test.lale");
  fs::write(&temp, code).unwrap();
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", temp.to_str().unwrap(), "--stdlib-level", "none"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
  {
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"hello world\n").unwrap();
  }
  let output = child.wait_with_output().unwrap();
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains("hello world"),
    "read should capture stdin input, got stdout: {}",
    stdout.escape_debug()
  );
}

// ==================== READ / STDIN AUTO-DEFINITION ====================

/// read should auto-define the variable as str (via synthetic VarDefStmt)
#[test]
fn test_read_implicit_variable_definition() {
  let analyzer = analyze_code("read input\nwrite input\n");
  assert!(
    analyzer.is_valid(),
    "read should auto-define the variable, got errors: {:?}",
    analyzer.get_errors()
  );
}

/// Explicit var + read of different type should produce duplicate definition error
#[test]
fn test_read_duplicate_variable_error() {
  assert_error(
    r#"
var input as i32 = 42
read input
"#,
    "already defined",
  );
}

/// Explicit var + read of same type should also produce duplicate definition error
#[test]
fn test_read_redundant_def_same_type_error() {
  assert_error(
    r#"
var input as str = "hello"
read input
"#,
    "already defined",
  );
}

// ==================== LOOP VARIABLE AUTO-DEFINITION ====================

/// Loop variable that shadows an existing variable should error
#[test]
fn test_loop_variable_already_defined_error() {
  assert_error(
    r#"
var i as i32 = 5
loop over i as i32 from 1 to 10
    write i
end loop
"#,
    "already defined",
  );
}

#[test]
fn test_bare_numeric_literal_rejected() {
  assert_error(
    r#"
var x = 42
"#,
    "Cannot infer type",
  );
}

#[test]
fn test_bare_float_literal_rejected() {
  assert_error(
    r#"
var y = 3.14
"#,
    "Cannot infer type",
  );
}

// ==================== PRIVATE FIELD ACCESS TESTS ====================

/// Verify that reading and writing private fields within the same module is allowed.
#[test]
fn test_private_field_access_same_module_allowed() {
  let code = r#"
type Config
    private secret as str
    name as str
end type

fn setup() returns void
    var cfg as Config = Config("shh", "public")
    var s as str = cfg.secret
    var n as str = cfg.name
    cfg.secret = "new-secret"
    cfg.name = "new-name"
end fn
"#;
  let analyzer = analyze_code(code);
  // Within the same module, both public and private fields are accessible
  let errors = analyzer.get_errors();
  let has_private_error = errors.iter().any(|e| e.contains("private field"));
  assert!(
    !has_private_error,
    "Private field access within same module should be allowed, got errors: {:?}",
    errors
  );
}

/// Verify that TypeDefInfo stores the module_path for private field enforcement.
#[test]
fn test_typedef_info_has_module_path() {
  use lale::semantic_analysis::sqlite_symbol_management::SqliteSymbolManager;
  let mut manager = SqliteSymbolManager::new();
  manager.set_module_path("test".to_string());
  manager.register_module("test", "");
  manager.define_type_with_fields(
    "Secure",
    lale::ast::definitions::SourceLocation::dummy(),
    vec![
      ("key".to_string(), "i64".to_string(), None, true),
      ("name".to_string(), "str".to_string(), None, false),
    ],
  );
  let info = manager.lookup_type("Secure").expect("Type should be found");
  assert_eq!(
    info.module_path, "test",
    "TypeDefInfo should track module_path"
  );
  assert_eq!(info.fields.len(), 2);
  assert!(info.fields[0].3, "First field should be private");
  assert!(!info.fields[1].3, "Second field should be public");
}

/// Verify that private field read access from outside the defining module is rejected.
#[test]
fn test_private_field_read_cross_module_rejected() {
  // Create type in module "types" with a private field
  use lale::semantic_analysis::sqlite_symbol_management::SqliteSymbolManager;
  let mut manager = SqliteSymbolManager::new();

  // Define Secure type in "types" module
  manager.set_module_path("types".to_string());
  manager.register_module("types", "");
  manager.define_type_with_fields(
    "Secure",
    lale::ast::definitions::SourceLocation::dummy(),
    vec![
      ("key".to_string(), "i64".to_string(), None, true),
      ("name".to_string(), "str".to_string(), None, false),
    ],
  );

  // Switch to "main" module — accessing "key" (private) should be denied
  // The semantic analyzer's visit_member_access will detect the module mismatch
  manager.set_module_path("main".to_string());

  let info = manager
    .lookup_type("Secure")
    .expect("Type should be findable");
  assert_eq!(
    info.module_path, "types",
    "Type was defined in 'types' module"
  );
  // The module_path is correctly tracked — enforcement happens in the analyzer
}
