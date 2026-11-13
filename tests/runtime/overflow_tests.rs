use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program through the interpreter binary, feeding `code` on stdin.
/// Returns stdout on success, or stderr on a non-zero exit (including traps).
fn run_lale(code: &str, extra_args: &[&str]) -> Result<String, String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run"])
    .args(extra_args)
    .arg("-")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {}", e))?;

  let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
  stdin
    .write_all(code.as_bytes())
    .map_err(|e| format!("Failed to write to stdin: {}", e))?;
  // Close stdin so the child process sees EOF.
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

fn run_default(code: &str) -> Result<String, String> {
  run_lale(code, &[])
}

fn run_unchecked(code: &str) -> Result<String, String> {
  run_lale(code, &["--unchecked-overflow"])
}

// ============ Constant overflow is caught at compile time ============
// These use constant operands, so the semantic analyzer rejects the overflow at
// compile time (`check_integer_overflow`). Runtime trapping for non-constant
// operands is covered by `tests/compile_time_integer_overflow_tests.rs` and the
// `checked_*_arith` unit tests in `src/interpreter.rs`.

#[test]
fn test_signed_add_constant_overflow_is_compile_error() {
  let code = r#"
var a as i8 = 127
var b as i8 = a + 1
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("signed add constant overflow should be a compile error");
  assert!(
    err.contains("integer overflow"),
    "expected overflow message, got: {}",
    err
  );
}

#[test]
fn test_signed_sub_constant_underflow_is_compile_error() {
  let code = r#"
var a as i8 = -128
var b as i8 = a - 1
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("signed sub constant underflow should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_signed_mul_constant_overflow_is_compile_error() {
  let code = r#"
var a as i8 = 64
var b as i8 = a * 2
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("signed mul constant overflow should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_signed_neg_min_constant_is_compile_error() {
  // Negating a signed minimum overflows; expressed as `0 - MIN` because Lale's
  // unary minus currently only binds to literals/function calls, not variables.
  let code = r#"
var a as i8 = -128
var b as i8 = 0 as i8 - a
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("negating a signed minimum should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_unsigned_add_constant_overflow_is_compile_error() {
  let code = r#"
var a as u8 = 255
var b as u8 = a + 1
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("unsigned add constant overflow should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_unsigned_mul_constant_overflow_is_compile_error() {
  let code = r#"
var a as u8 = 16
var b as u8 = a * 16
write "b = {b}"
"#;
  let result = run_default(code);
  let err = result.expect_err("unsigned mul constant overflow should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_scalar_int_dot_constant_overflow_is_compile_error() {
  let code = "var a as i8 = 16\nvar b as i8 = a ⋅ a\nwrite \"b = {b}\"\n";
  let result = run_default(code);
  let err = result.expect_err("scalar integer dot constant overflow should be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_valid_arithmetic_does_not_error() {
  let code = r#"
var a as i8 = 100
var b as i8 = a + 20
write "b = {b}"
"#;
  let output = run_default(code).expect("valid arithmetic must not error");
  assert!(output.contains("120"), "got: {}", output);
}

// ==================== --unchecked-overflow wraps ====================

#[test]
fn test_unchecked_overflow_wraps_i64() {
  let code = r#"
var a as i64 = 9223372036854775807
var b as i64 = a + 1
write "b = {b}"
"#;
  let output = run_unchecked(code).expect("unchecked mode must not trap");
  assert!(
    output.contains("-9223372036854775808"),
    "i64 wrap should produce i64::MIN, got: {}",
    output
  );
}

// ==================== Deterministic wrapping for division/remainder ====================

#[test]
fn test_division_min_by_negative_one_is_deterministic() {
  // i64::MIN / -1 would panic in Rust debug builds if the interpreter used
  // plain `/`; it must use wrapping division instead.
  let code = r#"
var a as i64 = -9223372036854775808
var b as i64 = a / -1
write "b = {b}"
"#;
  let output = run_default(code).expect("division must not panic");
  assert!(output.contains("-9223372036854775808"), "got: {}", output);
}

#[test]
fn test_remainder_is_euclidean() {
  // `%` uses the Euclidean remainder: the result is always non-negative,
  // regardless of the sign of either operand.
  let code = r#"
fn rem(a as i32, b as i32) returns i32
    return a % b
end fn

write "{rem(-5, 3)} {rem(5, -3)} {rem(5, 3)} {rem(-5, -3)}"
"#;
  let output = run_default(code).expect("remainder must not trap");
  assert!(output.contains("1 2 2 1"), "got: {}", output);
}
