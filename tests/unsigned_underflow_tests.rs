// =============================================================================
// Compile-time unsigned subtraction underflow detection
// =============================================================================
// The semantic analyzer rejects unsigned subtraction (`u8..u64 - u8..u64`) when
// the result could be negative. It now uses the SQLite constant-value store, so
// a subtraction of two *provably constant* variables is checked at compile time
// (rather than always rejected), while an unknown value is still conservatively
// rejected.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program through the interpreter binary, feeding `code` on stdin.
/// Returns stdout on success, or stderr on a non-zero exit (compile error/trap).
fn run_lale(code: &str) -> Result<String, String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {}", e))?;

  let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
  stdin
    .write_all(code.as_bytes())
    .map_err(|e| format!("Failed to write to stdin: {}", e))?;
  drop(stdin);

  let output = child
    .wait_with_output()
    .map_err(|e| format!("Failed to wait on child: {}", e))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

#[test]
fn test_constant_variable_subtraction_is_safe() {
  // `x` and `y` are provably constant (100 and 50), so `x - y` cannot underflow.
  let code = r#"
var x as u32 = 100
var y as u32 = 50
var z as u32 = x - y
write z
"#;
  let output = run_lale(code).expect("constant unsigned subtraction must not be rejected");
  assert!(output.contains("50"), "expected 50, got: {}", output);
}

#[test]
fn test_constant_variable_subtraction_underflows() {
  // `x` (5) < `y` (10): the subtraction provably underflows and is rejected.
  let code = r#"
var x as u32 = 5
var y as u32 = 10
var z as u32 = x - y
"#;
  let err = run_lale(code).expect_err("underflowing constant subtraction must be rejected");
  assert!(
    err.contains("Potential underflow in unsigned subtraction"),
    "expected underflow message, got: {}",
    err
  );
}

#[test]
fn test_unknown_variable_subtraction_is_rejected() {
  // Function parameters have unknown values, so the subtraction is conservatively
  // rejected (unchanged behavior).
  let code = r#"
fn f(a as u32, b as u32) returns u32
    return a - b
end fn
"#;
  let err = run_lale(code).expect_err("unknown unsigned subtraction must be rejected");
  assert!(
    err.contains("Cannot subtract two unsigned values"),
    "expected conservative rejection, got: {}",
    err
  );
}

#[test]
fn test_reassigned_constant_variable_subtraction_is_safe() {
  // Straight-line reassignment re-establishes the constant, so `x - y` is safe.
  let code = r#"
var x as u32 = 0
x = 100
var y as u32 = 50
var z as u32 = x - y
write z
"#;
  let output =
    run_lale(code).expect("reassigned constant unsigned subtraction must not be rejected");
  assert!(output.contains("50"), "expected 50, got: {}", output);
}

#[test]
fn test_u32_underflow_advises_i64_not_i32() {
  // `u32 as i64` is widening; `u32 as i32` would be rejected as narrowing.
  let code = r#"
var x as u32 = 5
var y as u32 = 10
var z as u32 = x - y
"#;
  let err = run_lale(code).expect_err("underflowing u32 subtraction must be rejected");
  assert!(err.contains("as i64"), "expected i64 advice, got: {}", err);
  assert!(
    !err.contains("as i32"),
    "should not advise i32 for u32, got: {}",
    err
  );
}

#[test]
fn test_u64_constant_underflow_advises_reorder_or_guard() {
  // `u64` has no wider signed type (no i128), so the advice must not suggest a cast.
  let code = r#"
var x as u64 = 5
var y as u64 = 10
var z as u64 = x - y
"#;
  let err = run_lale(code).expect_err("underflowing u64 subtraction must be rejected");
  assert!(
    err.contains("no wider signed type"),
    "expected no-wider-signed-type advice, got: {}",
    err
  );
}

#[test]
fn test_u64_unknown_underflow_advises_reorder_or_guard() {
  // The unknown-operand (function parameter) path must give the same u64 advice.
  let code = r#"
fn f(a as u64, b as u64) returns u64
    return a - b
end fn
"#;
  let err = run_lale(code).expect_err("u64 subtraction of unknown values must be rejected");
  assert!(
    err.contains("no wider signed type"),
    "expected no-wider-signed-type advice, got: {}",
    err
  );
}

#[test]
fn test_unsigned_sub_underflow_emits_single_error() {
  // Unsigned subtraction underflow must produce exactly one error (the
  // `check_unsigned_underflow` message), not a second `check_integer_overflow`
  // message for the same expression.
  let code = r#"
var x as u64 = 5
var y as u64 = 10
var z as u64 = x - y
"#;
  let err = run_lale(code).expect_err("underflowing u64 subtraction must be rejected");
  assert!(err.contains("Potential underflow"), "got: {}", err);
  assert!(
    !err.contains("integer overflow in constant expression"),
    "should not emit a second overflow error, got: {}",
    err
  );
}

#[test]
fn test_u64_expression_assigned_to_i64_is_type_mismatch() {
  // A `u64` arithmetic expression assigned to an `i64` variable is a genuine
  // type mismatch (u64 → i64 is a narrowing, not an inferable literal). It must
  // not be silently accepted via the old "any numeric expr → any numeric var"
  // exception.
  let code = r#"
var x as u64 = 5
var y as u64 = 10
var result as i64 = x + y
"#;
  let err = run_lale(code).expect_err("u64 expression into i64 variable must be rejected");
  assert!(err.contains("Type mismatch"), "got: {}", err);
}

#[test]
fn test_u64_expression_assigned_to_i64_via_assign_is_type_mismatch() {
  // Same check, but through the assignment (`=`) path rather than `var`.
  let code = r#"
var result as i64 = 0
var x as u64 = 5
var y as u64 = 10
result = x + y
"#;
  let err = run_lale(code).expect_err("u64 expression into i64 variable must be rejected");
  assert!(err.contains("Type mismatch in assignment"), "got: {}", err);
}

#[test]
fn test_numeric_literals_still_infer_from_context() {
  // The type-mismatch tightening must not break literal inference: a bare
  // integer literal infers from the declared type, and a float literal may
  // narrow to a smaller float type (precision loss is by design).
  let code = r#"
var a as u32 = 5
var b as f32 = 42.0
var c as f32 = 0.0
c = 42.0
write a
write b
write c
"#;
  let output = run_lale(code).expect("numeric literals must still infer from context");
  assert!(output.contains("5"), "got: {}", output);
  assert!(output.contains("42"), "got: {}", output);
}
