// =============================================================================
// Runtime NaN trapping
// =============================================================================
// Per Lale's "no silent errors" policy, a floating-point `NaN` or `±Inf`
// result is a runtime error that aborts execution — the float counterpart of
// the checked integer-overflow trap.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program through the interpreter binary, returning `(stdout, stderr)`.
fn run_lale_full(code: &str) -> Result<(String, String), String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
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
fn test_overflow_to_infinity_traps() {
  // `1e308 * 1e308` overflows to `Inf` and must abort execution.
  let code = r#"
var a as f64 = 1e308 * 1e308
write a
"#;
  let err = run_lale_full(code).expect_err("overflow to infinity must trap at runtime");
  assert!(
    err.contains("infinity"),
    "expected an infinity runtime error, got: {}",
    err
  );
}

#[test]
fn test_nan_traps() {
  // `(-1.0) ^ 0.5` is `NaN` (square root of a negative number) and must abort.
  let code = r#"
var a as f64 = (-1.0) ^ 0.5
write a
"#;
  let err = run_lale_full(code).expect_err("NaN must trap at runtime");
  assert!(
    err.contains("NaN"),
    "expected a NaN runtime error, got: {}",
    err
  );
}

#[test]
fn test_finite_float_computation_still_works() {
  // A finite float computation must not trap.
  let code = r#"
var x as f64 = 0.5 * 10.0
write x
"#;
  let (stdout, _) = run_lale_full(code).expect("finite float arithmetic must not trap");
  assert!(stdout.contains('5'), "expected 5, got: {}", stdout);
}
