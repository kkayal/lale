// =============================================================================
// Compound-assignment constant folding (`+=`, `-=`, `*=`, `/=`, `%=`)
// =============================================================================
// The semantic analyzer folds the result of a compound assignment when both the
// variable's current value and the RHS are known constants. This keeps the
// flow-sensitive `const_value` store precise, so the division-by-zero and
// unsigned-subtraction underflow checks see the value *after* the assignment
// (rather than conservatively clearing it).

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
fn test_sub_assign_feeds_underflow_check() {
  // After `x -= 50`, `x` is provably 50, so `x - 20` cannot underflow. Without
  // folding the compound assignment, `x` would be unknown and rejected.
  let code = r#"
var x as u32 = 100
x -= 50
var z as u32 = x - 20
write z
"#;
  let (stdout, _) = run_lale_full(code).expect("folded subtraction must not be rejected");
  assert!(stdout.contains("30"), "expected 30, got: {}", stdout);
}

#[test]
fn test_mul_assign_feeds_underflow_check() {
  // `x *= 5` folds to 50; `x - 20` is safe.
  let code = r#"
var x as u32 = 10
x *= 5
var z as u32 = x - 20
write z
"#;
  let (stdout, _) = run_lale_full(code).expect("folded multiplication must not be rejected");
  assert!(stdout.contains("30"), "expected 30, got: {}", stdout);
}

#[test]
fn test_add_assign_suppresses_division_warning() {
  // `x += 5` folds `x` to 5 (non-zero), so `100 / x` must not warn.
  let code = r#"
var x as i32 = 0
x += 5
var y as i32 = 100 / x
write y
"#;
  let (stdout, stderr) = run_lale_full(code).expect("program should run");
  assert!(stdout.contains("20"), "expected 20, got: {}", stdout);
  assert!(
    !stderr.contains("division by zero"),
    "expected no division-by-zero warning, got: {}",
    stderr
  );
}

#[test]
fn test_overflowing_compound_assign_clears_constant() {
  // `x += 100` overflows u8 (200 + 100 = 300 > 255), so the constant is cleared
  // and the subsequent `x - 50` is conservatively rejected as a possible underflow.
  let code = r#"
var x as u8 = 200
x += 100
var z as u8 = x - 50
write z
"#;
  let err = run_lale_full(code).expect_err("unknown unsigned subtraction must be rejected");
  assert!(
    err.contains("Cannot subtract two unsigned values")
      || err.contains("Potential underflow in unsigned subtraction"),
    "expected conservative underflow rejection, got: {}",
    err
  );
}
