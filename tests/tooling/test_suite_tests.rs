use lale::ast::build_program;
use lale::ast::{Stmt, TestCaseStmt, TestSuiteItem, TestSuiteStmt};
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;
use std::io::Write;
use std::process::{Command, Stdio};

/// Phase 1.1: grammar and AST for `test suite` / `test case`.
#[test]
fn test_suite_rule_parses() {
  let code = r#"test suite S
    test case C
        assert true
    end test case
end test suite"#;
  let result = LaleParser::parse(Rule::test_suite, code);
  assert!(result.is_ok(), "parse failed: {:?}", result.err());
}

#[test]
fn test_case_rule_parses() {
  let code = r#"test case C
    assert true
end test case"#;
  let result = LaleParser::parse(Rule::test_case, code);
  assert!(result.is_ok(), "parse failed: {:?}", result.err());
}

#[test]
fn test_test_case_empty_body_rejected() {
  let result = LaleParser::parse(Rule::test_case, "test case C end test case");
  assert!(
    result.is_err(),
    "test case without any statement must fail to parse"
  );
}

#[test]
fn test_test_suite_empty_rejected() {
  let result = LaleParser::parse(Rule::test_suite, "test suite S end test suite");
  assert!(
    result.is_err(),
    "test suite without any item must fail to parse"
  );
}

#[test]
fn test_suite_builds_ast() {
  let code = r#"
test suite MatrixOperations
    test case Multiplication
        var a as f64 = 2.0
        assert a == 2.0
    end test case

    test case Inversion
        var b as f64 = 3.0
    end test case
end test suite
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");

  let suites: Vec<&TestSuiteStmt> = program
    .statements
    .iter()
    .filter_map(|s| match s {
      Stmt::TestSuite(suite) => Some(suite),
      _ => None,
    })
    .collect();

  assert_eq!(suites.len(), 1, "expected exactly one test suite");
  let suite = suites[0];
  assert_eq!(suite.name, "MatrixOperations");
  let cases: Vec<&TestCaseStmt> = suite.cases().collect();
  assert_eq!(cases.len(), 2);
  assert_eq!(cases[0].name, "Multiplication");
  assert_eq!(cases[1].name, "Inversion");
  assert_eq!(cases[0].body.len(), 2, "var + assert");
  assert_eq!(cases[1].body.len(), 1, "var only");
}

#[test]
fn test_test_case_leading_comments_attached_to_case() {
  // Comment lines before a `test case` become leading comments on that case's
  // AST node (the same way a suite's leading comments attach to the suite).
  // The comment after the last case becomes a standalone suite item.
  // Indentation between the comment block and the `test case` keyword is
  // tolerated.
  let code = r#"
test suite S
    // comment for case A
    test case A
        assert true
    end test case

    /// doc for case B
    test case B
        assert true
    end test case

    // trailing comment before end
end test suite
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");

  let suites: Vec<&TestSuiteStmt> = program
    .statements
    .iter()
    .filter_map(|s| match s {
      Stmt::TestSuite(suite) => Some(suite),
      _ => None,
    })
    .collect();
  assert_eq!(suites.len(), 1, "expected exactly one test suite");
  let suite = suites[0];
  let cases: Vec<&TestCaseStmt> = suite.cases().collect();
  assert_eq!(cases.len(), 2);

  assert_eq!(cases[0].comments.leading.len(), 1, "case A comment");
  match &cases[0].comments.leading[0] {
    lale::ast::AttachedComment::Comment(c) => assert_eq!(c.content, "comment for case A"),
    other => panic!("expected a // comment on case A, got {:?}", other),
  }

  assert_eq!(cases[1].comments.leading.len(), 1, "case B doc");
  match &cases[1].comments.leading[0] {
    lale::ast::AttachedComment::Doc(d) => assert_eq!(d.content, "doc for case B"),
    other => panic!("expected a /// doc on case B, got {:?}", other),
  }

  // The comment after the last case is preserved as a standalone suite item.
  let has_trailing = suite.items.iter().any(
    |item| matches!(item, TestSuiteItem::Comment(c) if c.content == "trailing comment before end"),
  );
  assert!(has_trailing, "trailing comment should be preserved");
}

#[test]
fn test_test_suite_standalone_comments_preserved() {
  // The suite body may contain standalone comments/docs: a comment separated
  // from the next case by a blank line is NOT attached to that case, and it is
  // preserved in source order as a `TestSuiteItem` (source fidelity).
  let code = r#"
test suite S
    // comment for case A
    test case A
        assert true
    end test case

    // standalone comment

    // comment for case B
    test case B
        assert true
    end test case

    // comment after last case
end test suite
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");

  let suites: Vec<&TestSuiteStmt> = program
    .statements
    .iter()
    .filter_map(|s| match s {
      Stmt::TestSuite(suite) => Some(suite),
      _ => None,
    })
    .collect();
  assert_eq!(suites.len(), 1, "expected exactly one test suite");
  let suite = suites[0];

  let cases: Vec<&TestCaseStmt> = suite.cases().collect();
  assert_eq!(cases.len(), 2);
  assert_eq!(cases[0].comments.leading.len(), 1, "case A comment");
  assert_eq!(cases[1].comments.leading.len(), 1, "case B comment");

  // Standalone comments are preserved in source order as suite items.
  let standalone: Vec<&str> = suite
    .items
    .iter()
    .filter_map(|item| match item {
      TestSuiteItem::Comment(c) => Some(c.content.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(
    standalone,
    vec!["standalone comment", "comment after last case"]
  );
}

#[test]
fn test_suite_after_user_code_parses() {
  // A test suite must appear after user definitions; parsing accepts it here
  // (position enforcement is semantic, Phase 1.2).
  let code = r#"
var x as i64 = 1
write "x = {x}"

test suite Smoke
    test case basic
        assert x == 1
    end test case
end test suite
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");

  let has_suite = program
    .statements
    .iter()
    .any(|s| matches!(s, Stmt::TestSuite(_)));
  assert!(has_suite);
}

// ==================== Semantic analysis (Phase 1.2) ====================

fn analyze(code: &str) -> lale::semantic_analysis::OwnedAnalyzer {
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");
  analyze_ast(&program)
}

fn has_error(errors: &[lale::semantic_analysis::SemanticError], needle: &str) -> bool {
  errors.iter().any(|e| e.message.contains(needle))
}

#[test]
fn test_suite_placement_valid() {
  let code = r#"
test suite S
    var x as i64 = 1
    test case C
        assert x == 1
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  let errors = analyzer.get_errors();
  assert!(
    !has_error(errors, "test suite"),
    "unexpected test-suite errors: {:?}",
    errors
  );
}

#[test]
fn test_case_cannot_write_global_variable() {
  // Module globals are invisible inside test suites, so writing one is an
  // undefined-variable error rather than a special write rejection.
  let code = r#"
var g as i32 = 0
test suite S
    test case C
        g = 5
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(
      analyzer.get_errors(),
      "Assignment to undefined variable 'g'"
    ),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_case_cannot_compound_assign_global_variable() {
  let code = r#"
var g as i32 = 0
test suite S
    test case C
        g += 1
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(
      analyzer.get_errors(),
      "Compound assignment to undefined variable 'g'"
    ),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_case_cannot_read_global_variable() {
  // Module globals are invisible inside test suites.
  let code = r#"
var g as i32 = 7
test suite S
    test case C
        write g
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "Undefined variable 'g'"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_suite_must_be_last() {
  let code = r#"
test suite S
    test case C
        assert true
    end test case
end test suite

var x as i64 = 1
"#;
  let analyzer = analyze(code);
  let errors = analyzer.get_errors();
  assert!(
    has_error(errors, "test suite must be the last top-level statement"),
    "expected placement error, got: {:?}",
    errors
  );
}

#[test]
fn test_suite_requires_at_least_one_case() {
  let code = r#"
test suite Empty
    var x as i32 = 1
end test suite
"#;
  let analyzer = analyze(code);
  let errors = analyzer.get_errors();
  assert!(
    has_error(errors, "test suite requires at least one test case"),
    "expected empty-suite error, got: {:?}",
    errors
  );
}

#[test]
fn test_suite_no_nesting() {
  let code = r#"
test suite S
    test case C
        test suite Nested
            test case N
                assert true
            end test case
        end test suite
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  let errors = analyzer.get_errors();
  assert!(
    has_error(errors, "test suites cannot be nested inside a test case"),
    "expected nesting error, got: {:?}",
    errors
  );
}

// ==================== #mode gating (Phase 1.3) ====================

fn run_lale(code: &str, extra_args: &[&str]) -> Result<String, String> {
  let (stdout, stderr, status) = run_lale_full(code, extra_args);
  if status.success() {
    Ok(stdout)
  } else {
    Err(stderr)
  }
}

/// Runs the `lale` binary and returns stdout, stderr, and the exit status.
///
/// Unlike [`run_lale`], this keeps stderr even on success, which is required
/// for `test`-mode assertions because `Pass:`/`Fail:` are written to stderr.
fn run_lale_full(code: &str, extra_args: &[&str]) -> (String, String, std::process::ExitStatus) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run"])
    .args(extra_args)
    .arg("-")
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
fn test_run_mode_skips_test_suites() {
  let code = r#"
write "RUN: hello"
test suite Smoke
    test case basic
        write "TEST: hello"
    end test case
end test suite
"#;
  let output = run_lale(code, &[]).expect("run mode must succeed");
  assert!(output.contains("RUN: hello"), "got: {}", output);
  assert!(!output.contains("TEST: hello"), "got: {}", output);
}

#[test]
fn test_test_mode_runs_test_suites() {
  let code = r#"
write "RUN: hello"
test suite Smoke
    test case basic
        write "TEST: hello"
    end test case
end test suite
"#;
  let output = run_lale(code, &["--test"]).expect("test mode must succeed");
  assert!(output.contains("TEST: hello"), "got: {}", output);
  assert!(!output.contains("RUN: hello"), "got: {}", output);
}

#[test]
fn test_lale_test_subcommand_runs_test_suites() {
  let code = r#"
write "RUN: hello"
test suite Smoke
    test case basic
        write "TEST: hello"
    end test case
end test suite
"#;
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["test", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("spawn lale test");

  let mut stdin = child.stdin.take().expect("open stdin");
  stdin.write_all(code.as_bytes()).expect("write stdin");
  drop(stdin);

  let output = child.wait_with_output().expect("wait");
  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("TEST: hello"), "got: {}", stdout);
  assert!(!stdout.contains("RUN: hello"), "got: {}", stdout);
}

// ==================== Per-case Pass/Fail reporting (Phase 1.5) ====================

#[test]
fn test_test_mode_reports_pass_and_fail_and_continues() {
  let code = r#"
test suite Smoke
    test case passes
        assert true
    end test case

    test case fails
        assert false
    end test case

    test case still_runs
        write "AFTER: ran"
    end test case
end test suite
"#;
  let (stdout, stderr, status) = run_lale_full(code, &["--test"]);

  assert!(
    !status.success(),
    "a failing test case must produce a non-zero exit"
  );
  assert!(
    stderr.contains("Pass: Smoke / passes"),
    "stderr: {}",
    stderr
  );
  assert!(stderr.contains("Fail: Smoke / fails"), "stderr: {}", stderr);
  assert!(
    stderr.contains("Pass: Smoke / still_runs"),
    "stderr: {}",
    stderr
  );
  assert!(stdout.contains("AFTER: ran"), "stdout: {}", stdout);
}

#[test]
fn test_run_roundtrip_flag_executes_reconstructed_ir() {
  let code = "write \"hello roundtrip\"";
  let output = run_lale(code, &["--roundtrip"]).expect("roundtrip run must succeed");
  assert!(output.contains("hello roundtrip"), "got: {}", output);
}

#[test]
fn test_lale_test_subcommand_reports_failures() {
  let code = r#"
test suite Smoke
    test case fails
        assert false
    end test case
end test suite
"#;
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["test", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("spawn lale test");

  let mut stdin = child.stdin.take().expect("open stdin");
  stdin.write_all(code.as_bytes()).expect("write stdin");
  drop(stdin);

  let output = child.wait_with_output().expect("wait");
  assert!(!output.status.success());
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(stderr.contains("Fail: Smoke / fails"), "stderr: {}", stderr);
}

#[test]
fn test_assert_equality_reports_expected_and_found() {
  // A failing `assert a == b` reports the right operand as `expected` and the
  // left operand as `found`, instead of a location-only message.
  let code = r#"
test suite S
    test case C
        var foo as i32 = 2
        var bar as i32 = 3
        assert foo == bar
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(!status.success(), "stderr: {}", stderr);
  assert!(
    stderr.contains("Fail: S / C: expected 3, found 2"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn test_assert_non_equality_keeps_location_only_message() {
  // Non-equality asserts (e.g. `assert a < b`) keep the location-only message.
  let code = r#"
test suite S
    test case C
        var a as i32 = 5
        var b as i32 = 3
        assert a < b
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(!status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Fail: S / C ("), "stderr: {}", stderr);
  assert!(
    !stderr.contains("expected"),
    "non-equality assert should not report expected/found, stderr: {}",
    stderr
  );
}

#[test]
fn test_test_mode_uninitialized_struct_local_does_not_crash() {
  // Regression: a struct-typed local created inside a run-mode-guarded block
  // (skipped in test mode) is uninitialized. The epilogue auto-free's nested
  // str-field extraction used to crash with "ExtractField: expected struct or
  // string value, got Int(0)" (src/interpreter.rs:3913).
  let code = r#"
type Person
    name as text
    age as i32
end type

enum Color red(Person) green end enum

var aaa = Color.red(Person("John", 30))
switch aaa
  case red(p_): write "The person in red is {p_}"
  case green: write "Green"
end switch

test suite Smoke
    test case basic
        write "TEST: ran"
    end test case
end test suite
"#;
  let (stdout, stderr, status) = run_lale_full(code, &["--test"]);

  assert!(
    status.success(),
    "test mode must not crash, stderr: {}",
    stderr
  );
  assert!(stdout.contains("TEST: ran"), "stdout: {}", stdout);
}

#[test]
fn test_case_variables_are_case_local() {
  // A variable defined in one test case must not be visible in another case.
  let code = r#"
test suite S
    test case C1
        var x as i32 = 1
    end test case
    test case C2
        write x
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "Undefined variable 'x'"),
    "C1's x must not be visible in C2, errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_case_same_var_name_across_cases_is_allowed() {
  // Two test cases may define a variable with the same name; each is case-local.
  let code = r#"
test suite S
    test case C1
        var x as i32 = 1
        write x
    end test case
    test case C2
        var x as i32 = 2
        write x
    end test case
end test suite
"#;
  let (stdout, _stderr, status) = run_lale_full(code, &["--test"]);
  assert!(status.success(), "stderr: {}", _stderr);
  assert!(stdout.contains('1'), "stdout: {}", stdout);
  assert!(stdout.contains('2'), "stdout: {}", stdout);
}

#[test]
fn test_case_cannot_read_globals() {
  // Test cases behave like a suite scope: module globals are NOT readable.
  let code = r#"
var g as i32 = 7
test suite S
    test case C
        write g
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "Undefined variable 'g'"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_case_assert_with_local_vars() {
  // assert works with case-local variables (scalar and str).
  let code = r#"
test suite S
    test case C
        var x as i32 = 42
        assert x == 42
        var s as text = "hi"
        assert s == "hi"
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_full(code, &["--test"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}

#[test]
fn test_case_same_str_var_name_no_leak() {
  // Same-named str variables in two cases are case-local; each case frees its
  // own string data on exit (no heap leak, no clash in the scope map).
  let code = r#"
test suite S
    test case C1
        var s as text = "hello"
    end test case
    test case C2
        var s as text = "world"
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_full(code, &["--test"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    !stderr.contains("HEAP LEAK"),
    "same-named str case vars must not leak, stderr: {}",
    stderr
  );
}

#[test]
fn test_duplicate_case_name_in_suite_rejected() {
  // Two test cases with the same name in one suite are an error (their scope
  // keys are the suite-qualified case name).
  let code = r#"
test suite S
    test case C
        write "one"
    end test case
    test case C
        write "two"
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(
      analyzer.get_errors(),
      "duplicate test case name 'C' in test suite 'S'"
    ),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_duplicate_suite_name_rejected() {
  let code = r#"
test suite S
    test case C
        assert true
    end test case
end test suite
test suite S
    test case D
        assert true
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "duplicate test suite name 'S'"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_same_case_name_in_different_suites_allowed() {
  // The same case name may appear in different suites; the suite-qualified
  // scope key keeps them distinct.
  let code = r#"
test suite S1
    test case basic
        write "one"
    end test case
end test suite
test suite S2
    test case basic
        write "two"
    end test case
end test suite
"#;
  let (stdout, _stderr, status) = run_lale_full(code, &["--test"]);
  assert!(status.success(), "stderr: {}", _stderr);
  assert!(stdout.contains("one"), "stdout: {}", stdout);
  assert!(stdout.contains("two"), "stdout: {}", stdout);
}

// ==================== Grammar: identifier names (Phase 1.1) ====================

#[test]
fn test_case_string_literal_name_rejected() {
  // Suite/case names must be identifiers, not string literals.
  let result = LaleParser::parse(Rule::test_case, "test case \"C\"\nend test case");
  assert!(
    result.is_err(),
    "string-literal case names must be rejected"
  );
}

#[test]
fn test_suite_string_literal_name_rejected() {
  let result = LaleParser::parse(Rule::test_suite, "test suite \"S\"\nend test suite");
  assert!(
    result.is_err(),
    "string-literal suite names must be rejected"
  );
}

#[test]
fn test_suite_underscore_identifier_accepted() {
  let code = r#"
test suite my_suite
    test case my_case
        assert true
    end test case
end test suite
"#;
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let program = build_program(pairs, "test.lale").expect("build");

  let suites: Vec<&TestSuiteStmt> = program
    .statements
    .iter()
    .filter_map(|s| match s {
      Stmt::TestSuite(suite) => Some(suite),
      _ => None,
    })
    .collect();
  assert_eq!(suites.len(), 1);
  assert_eq!(suites[0].name, "my_suite");
  let cases: Vec<&TestCaseStmt> = suites[0].cases().collect();
  assert_eq!(cases[0].name, "my_case");
}

#[test]
fn test_suite_empty_case_body_rejected() {
  let code = r#"
test suite S
    test case C
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_full(code, &["--test"]);
  assert!(
    !status.success(),
    "expected failure for empty test case body, stderr: {}",
    stderr
  );
}

// ==================== Semantic: function-like case scope ====================

#[test]
fn test_fn_def_in_test_case_rejected() {
  // Test cases behave like functions: nested function definitions are rejected.
  let code = r#"
test suite S
    test case C
        fn helper() returns i32
            return 42
        end fn
    end test case
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(
      analyzer.get_errors(),
      "Function definitions are only allowed at module or test-suite scope"
    ),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== Case-body behavior (control flow, auto-define) ====================

#[test]
fn test_case_with_control_flow() {
  let code = r#"
test suite S
    test case C
        var total as i32 = 0
        loop var n as i32 from 1 to 3
            total += n
        end loop
        if total > 5
            total = 10
        else move on
        end if
        assert total == 10
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_full(code, &["--test"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}

// ==================== `lale test <file>` subcommand ====================

#[test]
fn test_lale_test_file_argument() {
  // `lale test <file>` (as opposed to stdin `-`) runs the file's suites.
  let tmp = std::env::temp_dir().join(format!("lale_test_file_{}.lale", std::process::id()));
  let code = r#"
test suite S
    test case C
        write "FILE: ran"
    end test case
end test suite
"#;
  std::fs::write(&tmp, code).expect("write temp file");

  let child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .arg("test")
    .arg(&tmp)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("spawn lale test");

  let output = child.wait_with_output().expect("wait");
  assert!(
    output.status.success(),
    "stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(stdout.contains("FILE: ran"), "stdout: {}", stdout);

  let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_run_mode_suite_only_program() {
  // In run mode a suite-only program produces no output and exits 0.
  let code = r#"
test suite S
    test case C
        write "TEST: ran"
    end test case
end test suite
"#;
  let output = run_lale(code, &[]).expect("run mode must succeed");
  assert!(!output.contains("TEST: ran"), "got: {}", output);
}

// ==================== `lale test --filter <pattern>` ====================

/// Runs `lale test -` with extra args, feeding `code` on stdin.
/// Returns (stdout, stderr, exit status). Pass/Fail lines go to stderr.
fn run_lale_test_full(
  code: &str,
  extra_args: &[&str],
) -> (String, String, std::process::ExitStatus) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["test"])
    .args(extra_args)
    .arg("-")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale test");

  let mut stdin = child.stdin.take().expect("Failed to open stdin");
  stdin.write_all(code.as_bytes()).expect("write stdin");
  drop(stdin);

  let output = child.wait_with_output().expect("wait on lale test");
  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status,
  )
}

const FILTER_FIXTURE: &str = r#"
test suite Math
    test case Addition
        write "ADD"
    end test case
    test case Subtraction
        write "SUB"
    end test case
end test suite
test suite Strings
    test case Length
        write "LEN"
    end test case
end test suite
"#;

#[test]
fn test_lale_test_filter_by_case_name() {
  let (stdout, stderr, status) = run_lale_test_full(FILTER_FIXTURE, &["--filter", "Addition"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    stdout.contains("ADD") && !stdout.contains("SUB") && !stdout.contains("LEN"),
    "stdout: {}",
    stdout
  );
  assert!(
    stderr.contains("Pass: Math / Addition"),
    "stderr: {}",
    stderr
  );
  assert!(
    !stderr.contains("Pass: Math / Subtraction"),
    "stderr: {}",
    stderr
  );
  assert!(
    !stderr.contains("Pass: Strings / Length"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn test_lale_test_filter_by_suite_case_path() {
  let (stdout, stderr, status) =
    run_lale_test_full(FILTER_FIXTURE, &["--filter", "Math/Subtraction"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    stdout.contains("SUB") && !stdout.contains("ADD") && !stdout.contains("LEN"),
    "stdout: {}",
    stdout
  );
  assert!(
    stderr.contains("Pass: Math / Subtraction"),
    "stderr: {}",
    stderr
  );
  assert!(
    !stderr.contains("Pass: Math / Addition"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn test_lale_test_filter_multiple_patterns_any_match() {
  // Repeated --filter flags: a case runs if ANY pattern matches.
  let (stdout, stderr, status) = run_lale_test_full(
    FILTER_FIXTURE,
    &["--filter", "Addition", "--filter", "Length"],
  );
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    stdout.contains("ADD") && stdout.contains("LEN") && !stdout.contains("SUB"),
    "stdout: {}",
    stdout
  );
  assert!(
    stderr.contains("Pass: Math / Addition"),
    "stderr: {}",
    stderr
  );
  assert!(
    stderr.contains("Pass: Strings / Length"),
    "stderr: {}",
    stderr
  );
  assert!(
    !stderr.contains("Pass: Math / Subtraction"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn test_lale_test_filter_by_suite_name() {
  // A suite-name match runs every case of that suite.
  let (stdout, stderr, status) = run_lale_test_full(FILTER_FIXTURE, &["--filter", "Strings"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    stdout.contains("LEN") && !stdout.contains("ADD") && !stdout.contains("SUB"),
    "stdout: {}",
    stdout
  );
  assert!(
    stderr.contains("Pass: Strings / Length"),
    "stderr: {}",
    stderr
  );
  assert!(
    !stderr.contains("Pass: Math / Addition"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn test_lale_test_filter_no_match_runs_nothing() {
  let (stdout, stderr, status) = run_lale_test_full(FILTER_FIXTURE, &["--filter", "NoSuchTest"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(
    !stdout.contains("ADD") && !stdout.contains("SUB") && !stdout.contains("LEN"),
    "stdout: {}",
    stdout
  );
  assert!(!stderr.contains("Pass:"), "stderr: {}", stderr);
}

#[test]
fn test_lale_test_filter_skipped_case_leak_not_reported() {
  // A case that is filtered out never runs, so its allocation never happens.
  let code = r#"
test suite S
    test case Leaky
        var p as pointer = allocate(64 as u64)
        write p
    end test case
    test case Clean
        write "CLEAN"
    end test case
end test suite
"#;
  let (stdout, stderr, status) = run_lale_test_full(code, &["--filter", "Clean"]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stdout.contains("CLEAN"), "stdout: {}", stdout);
  assert!(!stderr.contains("HEAP LEAK"), "stderr: {}", stderr);
}

// ==================== Heap leaks fail test runs ====================

#[test]
fn test_lale_test_leak_fails_test_run() {
  // A test case that leaks heap memory must produce a non-zero exit.
  let code = r#"
test suite S
    test case Leaky
        var p as pointer = allocate(64 as u64)
        write p
    end test case
end test suite
"#;
  let (stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(
    !status.success(),
    "a leaking test must fail, stdout: {}",
    stdout
  );
  assert!(stderr.contains("HEAP LEAK"), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / Leaky"), "stderr: {}", stderr);
}

#[test]
fn test_lale_test_pass_without_leak_succeeds() {
  let code = r#"
test suite S
    test case Clean
        var p as pointer = allocate(64 as u64)
        release p
        write "CLEAN"
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(!stderr.contains("HEAP LEAK"), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / Clean"), "stderr: {}", stderr);
}

#[test]
fn test_write_numeric_variable_no_leak_in_test() {
  // Regression: writing a non-str variable (i32 here) used to leak the
  // type-to-str conversion temp string because the free was suppressed for
  // all variable references. The leak would fail the test run now, so this
  // asserts both: the output is correct and there is no leak.
  let code = r#"
test suite S
    test case C
        var g as i32 = 5
        write g
    end test case
end test suite
"#;
  let (stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(!stderr.contains("HEAP LEAK"), "stderr: {}", stderr);
  assert!(stdout.contains("5"), "stdout: {}", stdout);
}

#[test]
fn test_suite_var_visible_across_cases() {
  // Suite-level variables are shared by all cases (analogous to globals for a
  // function), so a mutation in one case is visible in the next.
  let code = r#"
test suite S
    var counter as i32 = 0

    test case C1
        counter = counter + 1
        assert counter == 1
    end test case

    test case C2
        counter = counter + 10
        assert counter == 11
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C1"), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C2"), "stderr: {}", stderr);
}

#[test]
fn test_suite_fn_callable_from_cases() {
  // Suite-level functions are callable from cases. `setup`/`teardown` are a
  // convention only (not auto-run hooks); each case invokes them manually.
  let code = r#"
test suite S
    var counter as i32 = 0

    fn setup() returns nothing
        counter = 0
    end fn

    test case C
        setup()
        counter = counter + 1
        assert counter == 1
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}

#[test]
fn test_suite_var_shadows_global() {
  // A suite var may reuse a global name; they are distinct storage.
  let code = r#"
var shared as i32 = 100
test suite S
    var shared as i32 = 5
    test case C
        assert shared == 5
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}

#[test]
fn test_suite_declaration_after_case_rejected() {
  // Suite-level declarations must appear before the first test case.
  let code = r#"
test suite S
    test case C
        assert 1 as i32 == 1 as i32
    end test case
    var late as i32 = 0
end test suite
"#;
  let analyzer = analyze(code);
  assert!(
    has_error(
      analyzer.get_errors(),
      "must appear before the first test case"
    ),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_global_fn_callable_from_cases() {
  // Module-level functions remain callable from test suites/cases.
  let code = r#"
fn answer() returns i32
    return 42
end fn

test suite S
    test case C
        var x as i32 = answer()
        assert x == 42
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}

#[test]
fn test_suite_str_var_no_leak() {
  // Suite-level `str` variables are mangled globals tracked separately from the
  // global scope map, so their heap-allocated string data must still be freed
  // at program exit (otherwise the leak detector fails the run).
  let code = r#"
test suite S
    var label as text = "fixture"

    test case C
        write label
    end test case
end test suite
"#;
  let (_stdout, stderr, status) = run_lale_test_full(code, &[]);
  assert!(status.success(), "stderr: {}", stderr);
  assert!(!stderr.contains("HEAP LEAK"), "stderr: {}", stderr);
  assert!(stderr.contains("Pass: S / C"), "stderr: {}", stderr);
}
