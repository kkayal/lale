// =============================================================================
// Typed-folding precision for div-zero / unsigned-underflow proofs
// =============================================================================
// The division-by-zero proof (`expr_is_constant_nonzero`) and the unsigned
// subtraction underflow check (`check_unsigned_underflow`) now fold the full
// operand expression with its declared type (via `expr_const_value_typed` +
// `const_eval`). This lets them prove facts like `(a + b) - c >= 0` or
// `a + b != 0` that the previous leaf-only proof had to reject conservatively.

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
fn test_sum_subtraction_underflow_is_precise() {
  // `(a + b)` folds to 150, so `150 - 100` cannot underflow. The leaf-only proof
  // would reject this conservatively.
  let code = r#"
var a as u32 = 100
var b as u32 = 50
var z as u32 = (a + b) - 100
write z
"#;
  let (stdout, _) = run_lale_full(code).expect("folded sum subtraction must not be rejected");
  assert!(stdout.contains("50"), "expected 50, got: {}", stdout);
}

#[test]
fn test_sum_subtraction_underflow_detected() {
  // `(a + b)` folds to 150, and `150 - 200` provably underflows.
  let code = r#"
var a as u32 = 100
var b as u32 = 50
var z as u32 = (a + b) - 200
write z
"#;
  let err = run_lale_full(code).expect_err("provably underflowing sum must be rejected");
  assert!(
    err.contains("Potential underflow in unsigned subtraction"),
    "expected underflow message, got: {}",
    err
  );
}

#[test]
fn test_sum_divisor_non_zero_suppresses_warning() {
  // `a + b` folds to 10 (non-zero), so `100 / (a + b)` must not warn.
  let code = r#"
var a as i32 = 5
var b as i32 = 5
var c as i32 = 100 / (a + b)
write c
"#;
  let (stdout, stderr) = run_lale_full(code).expect("program should run");
  assert!(stdout.contains("10"), "expected 10, got: {}", stdout);
  assert!(
    !stderr.contains("not provably non-zero"),
    "expected no division-by-zero warning, got: {}",
    stderr
  );
}

#[test]
fn test_zero_sum_divisor_warns() {
  // `a + b` folds to 0, so `100 / (a + b)` is a division by zero and must warn.
  let code = r#"
var a as i32 = 5
var b as i32 = -5
var c as i32 = 100 / (a + b)
write c
"#;
  let err = run_lale_full(code).expect_err("division by a zero sum must trap");
  assert!(
    err.contains("not provably non-zero"),
    "expected division-by-zero warning, got: {}",
    err
  );
}
