//! Tests for the reserved built-in constant `π`.
//!
//! `π` is part of the language itself (not the standard library). It is a
//! reserved identifier: using it as an expression folds it to the float literal
//! `3.141592653589793` (context-typed, defaulting to `f64`), while defining it
//! (`var π`, a parameter, a function, a type, an enum, or a loop variable) is a
//! compile-time error in any scope.

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, OwnedAnalyzer, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

fn analyze(code: &str) -> OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("Parse failed");
  let program = build_program(pairs, "test.lale").expect("AST build failed");
  analyze_ast(&program)
}

fn has_error_containing(analyzer: &OwnedAnalyzer, substring: &str) -> bool {
  analyzer
    .get_errors()
    .iter()
    .any(|e| e.to_string().contains(substring))
}

fn errors_joined(analyzer: &OwnedAnalyzer) -> String {
  analyzer
    .get_errors()
    .iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>()
    .join(", ")
}

// ==================== Usage (folding) ====================

#[test]
fn test_pi_used_as_value_is_valid() {
  let code = r#"
var x as f64 = π
write x
"#;
  let analyzer = analyze(code);
  assert!(
    analyzer.is_valid(),
    "π should be a valid value expression. Errors: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_pi_infers_f64_by_default() {
  // No type annotation: π defaults to `f64` (the same default a float literal has).
  let code = r#"
var x = π
var twice = 2.0 ⋅ π
write x
write twice
"#;
  let analyzer = analyze(code);
  assert!(
    analyzer.is_valid(),
    "bare π should infer `f64`. Errors: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_pi_infers_f32_in_f32_context() {
  // A fixed-`f64` π would be a narrowing error here; context-typing lets π take
  // the `f32` width of the surrounding expression.
  let code = r#"
var quarter as f32 = π / 2.0
write quarter
"#;
  let analyzer = analyze(code);
  assert!(
    analyzer.is_valid(),
    "π should infer `f32` in an `f32` context. Errors: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_pi_is_unitless() {
  // π carries no physical unit, so multiplying a quantity by π preserves its unit.
  let code = r#"
var r as f64 in <m> = 2.0 <m>
var circumference as f64 in <m> = 2.0 ⋅ π ⋅ r
write circumference
"#;
  let analyzer = analyze(code);
  assert!(
    analyzer.is_valid(),
    "π must be unitless so `2.0 ⋅ π ⋅ r` stays in <m>. Errors: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_pi_is_constant_nonzero() {
  // π is a compile-time non-zero constant, so a division by it is provably safe.
  let code = r#"
var y as f64 = 1.0 / π
write y
"#;
  let analyzer = analyze(code);
  assert!(
    analyzer.is_valid(),
    "1.0 / π must be valid (π is provably non-zero). Errors: {}",
    errors_joined(&analyzer)
  );
}

// ==================== Reservation (definitions rejected) ====================

#[test]
fn test_var_pi_rejected() {
  let code = "var π as f64 = 3.0\n";
  let analyzer = analyze(code);
  assert!(!analyzer.is_valid(), "`var π` should be rejected");
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_function_pi_rejected() {
  let code = r#"
fn π() returns f64
  return 3.0
end fn
"#;
  let analyzer = analyze(code);
  assert!(!analyzer.is_valid(), "`fn π` should be rejected");
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_parameter_pi_rejected() {
  let code = r#"
fn f(π as i32) returns i32
  return 1
end fn
"#;
  let analyzer = analyze(code);
  assert!(
    !analyzer.is_valid(),
    "a parameter named π should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_type_pi_rejected() {
  let code = r#"
type π
  x as i32
end type
"#;
  let analyzer = analyze(code);
  assert!(!analyzer.is_valid(), "`type π` should be rejected");
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_enum_pi_rejected() {
  let code = "enum π red green end enum\n";
  let analyzer = analyze(code);
  assert!(!analyzer.is_valid(), "`enum π` should be rejected");
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

#[test]
fn test_loop_variable_pi_rejected() {
  let code = r#"
loop var π as i32 from 1 to 3
  write "x"
end loop
"#;
  let analyzer = analyze(code);
  assert!(
    !analyzer.is_valid(),
    "a loop variable named π should be rejected"
  );
  assert!(
    has_error_containing(&analyzer, "reserved constant"),
    "Error should mention π is reserved. Got: {}",
    errors_joined(&analyzer)
  );
}

// ==================== End-to-end value ====================

mod pi_e2e_tests {
  use std::process::Command;

  fn run_lale_program(source: &str) -> (bool, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
      .current_dir(env!("CARGO_MANIFEST_DIR"))
      .args(["run", "-", "--no-color"])
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()
      .expect("Failed to spawn lale");

    use std::io::Write;
    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin
      .write_all(source.as_bytes())
      .expect("Failed to write to stdin");

    let output = child.wait_with_output().expect("Failed to wait on child");
    (
      output.status.success(),
      String::from_utf8_lossy(&output.stdout).to_string(),
      String::from_utf8_lossy(&output.stderr).to_string(),
    )
  }

  #[test]
  fn test_write_pi_value() {
    let source = "write π\n";
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(success, "writing π should succeed. stderr: {}", stderr);
    assert_eq!(
      stdout.trim(),
      "3.141592653589793",
      "π should print as 3.141592653589793. stdout '{}', stderr '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_two_pi_value() {
    let source = "var c as f64 = 2.0 ⋅ π\nwrite c\n";
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(success, "2.0 ⋅ π should succeed. stderr: {}", stderr);
    assert_eq!(
      stdout.trim(),
      "6.283185307179586",
      "2.0 ⋅ π should be 6.283185307179586. stdout '{}', stderr '{}'",
      stdout,
      stderr
    );
  }
}
