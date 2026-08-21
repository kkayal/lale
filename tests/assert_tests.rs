//! Tests for the `assert` statement.

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSING TESTS ====================

#[test]
fn test_assert_parses() {
  let code = "var x as bool = true\nassert x\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_assert_with_comparison() {
  let code = "var x as i32 = 5\nassert x > 0\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

#[test]
fn test_assert_with_comment() {
  let code = "var flag as bool = true\nassert flag // check flag\n";
  assert!(LaleParser::parse(Rule::program, code).is_ok());
}

// ==================== SEMANTIC TESTS ====================

use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};

fn analyze(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");
  analyze_ast(&program)
}

#[test]
fn test_assert_bool_ok() {
  let a = analyze("var flag as bool = true\nassert flag\n");
  assert!(a.is_valid(), "Errors: {:?}", a.get_errors());
}

#[test]
fn test_assert_non_bool_rejected() {
  let a = analyze("var x as i32 = 5\nassert x\n");
  assert!(!a.is_valid());
  assert!(a.get_errors().iter().any(|e| e.contains("boolean")));
}

// ==================== RUNTIME TESTS ====================

use std::fs;
use std::process::Command;

#[test]
fn test_assert_fails_in_debug() {
  let dir = tempfile::tempdir().unwrap();
  let f = dir.path().join("test.lale");
  let code = "var flag as bool = false\nassert flag\nwrite \"unreachable\"\n";
  fs::write(&f, code).unwrap();
  let out = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", f.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(!out.status.success());
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Assertion failed"), "stderr: {}", stderr);
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(!stdout.contains("unreachable"));
}

#[test]
fn test_assert_passes_when_true() {
  let dir = tempfile::tempdir().unwrap();
  let f = dir.path().join("test.lale");
  fs::write(&f, "var flag as bool = true\nassert flag\nwrite \"ok\"\n").unwrap();
  let out = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", f.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(out.status.success());
}

#[test]
fn test_assert_noop_in_release() {
  let dir = tempfile::tempdir().unwrap();
  let f = dir.path().join("test.lale");
  fs::write(&f, "var flag as bool = false\nassert flag\nwrite \"ok\"\n").unwrap();
  let out = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", f.to_str().unwrap(), "--release"])
    .output()
    .unwrap();
  assert!(out.status.success());
  assert!(String::from_utf8_lossy(&out.stdout).contains("ok"));
}
