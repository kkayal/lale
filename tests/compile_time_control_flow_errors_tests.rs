//! Negative and edge-case tests for compile-time control flow
//! (`#if`, `#when`, `#match`, `#switch`).
//!
//! These cover the errors added when compile-time constructs were made strict:
//! non-boolean conditions, runtime-dependent (unevaluable) conditions/values,
//! and duplicate case/arm values.

use std::fs;
use std::process::Command;

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, OwnedAnalyzer, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Parse, build, and analyze the given Lale source.
fn analyze(code: &str) -> OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");
  analyze_ast(&program)
}

/// Assert that `code` fails analysis with an error containing `expected`.
fn assert_error(code: &str, expected: &str) {
  let analyzer = analyze(code);
  assert!(
    !analyzer.is_valid(),
    "Expected analysis to fail for:\n{}",
    code
  );
  let errors = analyzer.get_errors();
  assert!(
    errors.iter().any(|e| e.contains(expected)),
    "Expected an error containing {:?}, got:\n{:#?}",
    expected,
    errors
  );
}

/// Compile and run the given Lale code, returning (stdout, stderr, success).
fn run_lale(code: &str) -> (String, String, bool) {
  let dir = tempfile::tempdir().unwrap();
  let temp_file = dir.path().join("test.lale");
  fs::write(&temp_file, code).expect("Failed to write temp file");

  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", temp_file.to_str().unwrap()])
    .output()
    .expect("Failed to run lale compiler");

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();

  (stdout, stderr, success)
}

// ==================== Unevaluable conditions ====================

#[test]
fn test_ct_if_unevaluable_condition_is_error() {
  assert_error(
    "var flag as bool = true\n#if flag\n  write \"x\"\n#end if\n",
    "must be evaluable at compile time",
  );
}

#[test]
fn test_ct_when_unevaluable_condition_is_error() {
  assert_error(
    "var flag as bool = true\n#when flag\n  write \"x\"\n#end when\n",
    "must be evaluable at compile time",
  );
}

#[test]
fn test_ct_match_unevaluable_guard_is_error() {
  assert_error(
    "var flag as bool = true\n#match\n  #when flag:\n    write \"x\"\n  #else:\n    write \"y\"\n#end match\n",
    "must be evaluable at compile time",
  );
}

// ==================== Unevaluable switch values ====================

#[test]
fn test_ct_switch_unevaluable_value_is_error() {
  assert_error(
    "var mode as i32 = 1\n#switch mode\n  #case 1:\n    write \"one\"\n#end switch\n",
    "switch value must be a compile-time constant",
  );
}

#[test]
fn test_ct_switch_unevaluable_case_is_error() {
  assert_error(
    "var mode as i32 = 1\n#switch 5\n  #case mode:\n    write \"x\"\n#end switch\n",
    "switch case value must be a compile-time constant",
  );
}

// ==================== Non-boolean conditions ====================

#[test]
fn test_ct_if_non_boolean_condition_is_error() {
  assert_error(
    "#if 5\n  write \"x\"\n#end if\n",
    "must be a boolean expression",
  );
}

#[test]
fn test_ct_when_non_boolean_condition_is_error() {
  assert_error(
    "#when 5\n  write \"x\"\n#end when\n",
    "must be a boolean expression",
  );
}

#[test]
fn test_ct_match_non_boolean_guard_is_error() {
  assert_error(
    "#match\n  #when 5:\n    write \"x\"\n#end match\n",
    "must be a boolean expression",
  );
}

// ==================== Duplicate cases and arms ====================

#[test]
fn test_ct_switch_duplicate_case_is_error() {
  assert_error(
    "#switch 5\n  #case 5:\n    write \"a\"\n  #case 5:\n    write \"b\"\n#end switch\n",
    "Duplicate case value in '#switch'",
  );
}

#[test]
fn test_ct_switch_duplicate_case_unrelated_to_scrutinee_is_error() {
  assert_error(
    "#switch 3\n  #case 5:\n    write \"a\"\n  #case 5:\n    write \"b\"\n#end switch\n",
    "Duplicate case value in '#switch'",
  );
}

#[test]
fn test_ct_match_duplicate_arm_is_error() {
  assert_error(
    "#match\n  #when true:\n    write \"a\"\n  #when true:\n    write \"b\"\n#end match\n",
    "Duplicate '#match' arm",
  );
}

// ==================== Positive: #switch value kinds ====================

#[test]
fn test_ct_switch_bool_match() {
  let code = "#switch true\n  #case false:\n    write \"no\"\n  #case true:\n    write \"yes\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("yes"), "Expected 'yes', got: {}", stdout);
  assert!(
    !stdout.contains("no"),
    "Expected 'no' absent, got: {}",
    stdout
  );
}

#[test]
fn test_ct_switch_arithmetic_match() {
  let code =
    "#switch 2 + 3\n  #case 4:\n    write \"four\"\n  #case 5:\n    write \"five\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("five"), "Expected 'five', got: {}", stdout);
}

#[test]
fn test_ct_switch_negative_match() {
  let code =
    "#switch -5\n  #case 5:\n    write \"pos\"\n  #case -5:\n    write \"neg\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("neg"), "Expected 'neg', got: {}", stdout);
  assert!(
    !stdout.contains("pos"),
    "Expected 'pos' absent, got: {}",
    stdout
  );
}

// ==================== Nested compile-time control flow ====================

#[test]
fn test_nested_ct_if_inside_ct_when() {
  let code = "#when true\n  #if true\n    write \"nested\"\n  #end if\n#end when\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    stdout.contains("nested"),
    "Expected 'nested', got: {}",
    stdout
  );
}

#[test]
fn test_nested_ct_if_inside_ct_match() {
  let code =
    "#match\n  #when true:\n    #if true\n      write \"nested\"\n    #end if\n#end match\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(
    stdout.contains("nested"),
    "Expected 'nested', got: {}",
    stdout
  );
}

// ==================== Dead branches are not type-checked ====================

#[test]
fn test_ct_when_false_dead_branch_not_analyzed() {
  let code = "#when false\n  var x = 1\n#end when\nwrite \"ok\"\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("ok"), "Expected 'ok', got: {}", stdout);
}

#[test]
fn test_ct_if_false_dead_branch_not_analyzed() {
  let code = "#if false\n  var x = 1\n#end if\nwrite \"ok\"\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("ok"), "Expected 'ok', got: {}", stdout);
}

#[test]
fn test_ct_match_dead_arm_not_analyzed() {
  let code = "#match\n  #when false:\n    var x = 1\n  #else:\n    write \"ok\"\n#end match\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("ok"), "Expected 'ok', got: {}", stdout);
}

#[test]
fn test_ct_switch_dead_case_not_analyzed() {
  let code = "#switch 1\n  #case 2:\n    var x = 1\n  #default:\n    write \"ok\"\n#end switch\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success, stderr: {}", stderr);
  assert!(stdout.contains("ok"), "Expected 'ok', got: {}", stdout);
}

// ==================== Runtime dispatch duplicates ====================

#[test]
fn test_runtime_match_duplicate_arm_is_error() {
  let (_stdout, stderr, success) =
    run_lale("match\n  when true:\n    write \"a\"\n  when true:\n    write \"b\"\nend match\n");
  assert!(!success, "Expected duplicate match arm to fail");
  assert!(
    stderr.contains("Duplicate match arm"),
    "Expected duplicate error, got: {}",
    stderr
  );
}

#[test]
fn test_runtime_switch_duplicate_variant_is_error() {
  let code = "enum Color\n  Red\n  Green\n  Blue\nend enum\nvar c as Color = Red\nswitch c\n  case Red:\n    write \"r\"\n  case Red:\n    write \"r2\"\n  case Green:\n    write \"g\"\n  case Blue:\n    write \"b\"\nend switch\n";
  let (_stdout, stderr, success) = run_lale(code);
  assert!(!success, "Expected duplicate switch case to fail");
  assert!(
    stderr.contains("Duplicate case for variant 'Red'"),
    "Expected duplicate error, got: {}",
    stderr
  );
}
