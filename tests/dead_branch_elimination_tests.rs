// =============================================================================
// Dead-branch elimination
// =============================================================================
// The IR generator folds the condition of `if`/`when`/`match`/`loop` to a
// compile-time boolean when every operand is a literal or an *effectively
// constant* module-global (never reassigned, never shared). A provably-true or
// provably-false condition emits only the live branch; the dead branch's code
// (including any side effects or string constants) is never generated.
//
// Each test runs with `--print-ir` so we can assert both halves:
//   1. correctness  — the program output matches the live branch only;
//   2. elimination  — the dead branch's marker string is absent from the IR.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program through the interpreter binary, returning `(stdout, stderr)`.
fn run_lale_full(code: &str, extra_args: &[&str]) -> Result<(String, String), String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run"])
    .args(extra_args)
    .arg("-")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {e}"))?;

  let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
  stdin
    .write_all(code.as_bytes())
    .map_err(|e| format!("Failed to write to stdin: {e}"))?;
  drop(stdin);

  let output = child
    .wait_with_output()
    .map_err(|e| format!("Failed to wait on child: {e}"))?;

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();

  if output.status.success() {
    Ok((stdout, stderr))
  } else {
    Err(format!("stdout:\n{stdout}\nstderr:\n{stderr}"))
  }
}

#[test]
fn test_when_false_eliminates_body() {
  let code = r#"
when false
    write "DEAD_MARKER"
end when
write "AFTER"
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    !stdout.contains("DEAD_MARKER"),
    "dead body must not run: {stdout}"
  );
  assert!(
    stdout.contains("AFTER"),
    "code after `when` must run: {stdout}"
  );
  assert!(
    !ir.contains("DEAD_MARKER"),
    "dead body must be eliminated from the IR"
  );
}

#[test]
fn test_when_true_inlines_body() {
  let code = r#"
when true
    write "LIVE_MARKER"
end when
write "AFTER"
"#;
  let (stdout, _) = run_lale_full(code, &[]).expect("program should run");
  assert!(stdout.contains("LIVE_MARKER"), "body must run: {stdout}");
  assert!(
    stdout.contains("AFTER"),
    "code after `when` must run: {stdout}"
  );
}

#[test]
fn test_if_true_eliminates_else() {
  let code = r#"
if true
    write "LIVE_MARKER"
else
    write "DEAD_MARKER"
end if
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    stdout.contains("LIVE_MARKER"),
    "then branch must run: {stdout}"
  );
  assert!(
    !stdout.contains("DEAD_MARKER"),
    "else branch must not run: {stdout}"
  );
  assert!(
    !ir.contains("DEAD_MARKER"),
    "else branch must be eliminated"
  );
}

#[test]
fn test_if_false_eliminates_then() {
  let code = r#"
if false
    write "DEAD_MARKER"
else
    write "LIVE_MARKER"
end if
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    stdout.contains("LIVE_MARKER"),
    "else branch must run: {stdout}"
  );
  assert!(
    !stdout.contains("DEAD_MARKER"),
    "then branch must not run: {stdout}"
  );
  assert!(
    !ir.contains("DEAD_MARKER"),
    "then branch must be eliminated"
  );
}

#[test]
fn test_match_leading_true_eliminates_rest() {
  let code = r#"
match
    when true:
        write "LIVE_MARKER"
    when false:
        write "DEAD_MARKER"
end match
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    stdout.contains("LIVE_MARKER"),
    "first arm must run: {stdout}"
  );
  assert!(
    !stdout.contains("DEAD_MARKER"),
    "second arm must not run: {stdout}"
  );
  assert!(!ir.contains("DEAD_MARKER"), "second arm must be eliminated");
}

#[test]
fn test_loop_when_false_skips_body() {
  let code = r#"
loop when false
    write "DEAD_MARKER"
end loop
write "AFTER"
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    !stdout.contains("DEAD_MARKER"),
    "loop body must not run: {stdout}"
  );
  assert!(
    stdout.contains("AFTER"),
    "code after loop must run: {stdout}"
  );
  assert!(!ir.contains("DEAD_MARKER"), "loop body must be eliminated");
}

#[test]
fn test_comparison_with_constant_global_folds() {
  let code = r#"
var x as i32 = 5
if x < 10
    write "LIVE_MARKER"
else
    write "DEAD_MARKER"
end if
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    stdout.contains("LIVE_MARKER"),
    "then branch must run: {stdout}"
  );
  assert!(
    !ir.contains("DEAD_MARKER"),
    "else branch must be eliminated"
  );
}

#[test]
fn test_reassigned_variable_is_not_eliminated() {
  // `x` is reassigned, so it is NOT effectively constant. Dead-branch
  // elimination must NOT fold `x < 10`; both branches remain, and the runtime
  // value (100) selects the else branch.
  let code = r#"
var x as i32 = 5
x = 100
if x < 10
    write "LIVE_MARKER"
else
    write "DEAD_MARKER"
end if
"#;
  let (stdout, ir) = run_lale_full(code, &["--print-ir"]).expect("program should run");
  assert!(
    !stdout.contains("LIVE_MARKER"),
    "then branch must not run: {stdout}"
  );
  assert!(
    stdout.contains("DEAD_MARKER"),
    "else branch must run: {stdout}"
  );
  assert!(
    ir.contains("DEAD_MARKER"),
    "else branch must NOT be eliminated (soundness)"
  );
}
