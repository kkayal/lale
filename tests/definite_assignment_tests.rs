//! Tests for definite-assignment analysis across control-flow branches.
//!
//! A `var` defined inside a branch is usable after the construct only if it is
//! defined on every path with a consistent type and unit; otherwise it is a
//! compile error (not a silent zero).

use lale::ast::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;
use std::io::Write;
use std::process::{Command, Stdio};

fn analyze(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");
  analyze_ast(&program)
}

fn has_error(errors: &[lale::semantic_analysis::SemanticError], needle: &str) -> bool {
  errors.iter().any(|e| e.message.contains(needle))
}

fn run_lale_full(code: &str) -> (String, String, std::process::ExitStatus) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  let mut stdin = child.stdin.take().expect("Failed to open stdin");
  stdin.write_all(code.as_bytes()).expect("write stdin");
  drop(stdin);

  let output = child.wait_with_output().expect("wait on lale");
  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status,
  )
}

#[test]
fn test_if_var_in_one_arm_rejected() {
  let code = r#"
if 2 as i8 == 3 as i8
  var yyy as i8 = 7
else
  move on
end if
debug yyy
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_if_var_in_both_arms_joined() {
  let code = r#"
if 1 as i8 == 1 as i8
  var yyy as i8 = 7
else
  var yyy as i8 = 8
end if
write yyy
"#;
  let (stdout, _stderr, status) = run_lale_full(code);
  assert!(status.success());
  assert!(stdout.contains("7"), "stdout: {}", stdout);
}

#[test]
fn test_if_var_type_mismatch_rejected() {
  let code = r#"
if 1 as i8 == 1 as i8
  var yyy as i8 = 7
else
  var yyy as i64 = 8
end if
debug yyy
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_when_body_var_rejected() {
  let code = r#"
when 1 as i8 == 1 as i8
  var v as i8 = 1
end when
debug v
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_loop_body_var_rejected() {
  let code = r#"
loop when 1 as i8 == 0 as i8
  var v as i8 = 1
end loop
debug v
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_loop_counter_still_accessible() {
  let code = r#"
loop over i as i32 from 1 to 3
  write i
end loop
write i
"#;
  let analyzer = analyze(code);
  assert!(
    !has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_match_var_in_one_arm_rejected() {
  let code = r#"
match
  when 1 as i8 == 1 as i8:
    var v as i8 = 1
  else:
    move on
end match
debug v
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_nested_if_var_rejected() {
  let code = r#"
if 1 as i8 == 1 as i8
  if 1 as i8 == 0 as i8
    var v as i8 = 1
  else
    move on
  end if
  debug v
else
  move on
end if
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_nested_if_inner_join_ok() {
  let code = r#"
if 1 as i8 == 1 as i8
  if 1 as i8 == 1 as i8
    var v as i8 = 1
  else
    var v as i8 = 2
  end if
  write v
else
  move on
end if
"#;
  let analyzer = analyze(code);
  assert!(
    !has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_match_var_in_all_arms_joined() {
  let code = r#"
match
  when 1 as i8 == 1 as i8:
    var v as i8 = 1
  when 1 as i8 == 0 as i8:
    var v as i8 = 2
  else:
    var v as i8 = 3
end match
write v
"#;
  let (stdout, _stderr, status) = run_lale_full(code);
  assert!(status.success());
  assert!(stdout.contains("1"), "stdout: {}", stdout);
}

#[test]
fn test_if_var_unit_mismatch_rejected() {
  let code = r#"
if 1 as i8 == 1 as i8
  var v as i32 in <m> = 1
else
  var v as i32 in <s> = 1
end if
debug v
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "is only defined in some branches"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}
