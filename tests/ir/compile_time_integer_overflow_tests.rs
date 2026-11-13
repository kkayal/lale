// =============================================================================
// Compile-time integer overflow detection for constant arithmetic
// =============================================================================
// The semantic analyzer rejects `+`/`-`/`*`/`⋅` on integer operands when both
// operands fold to known constants and the result would overflow/underflow. This
// mirrors the runtime `Checked*` trap, but fires at compile time. Non-constant
// operands stay the responsibility of the runtime trap (no compile error).

use std::io::Write;
use std::process::{Command, Stdio};

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

#[test]
fn test_signed_add_constant_overflow_is_compile_error() {
  let code = r#"
var a as i8 = 127
var b as i8 = a + 1
"#;
  let err = run_default(code).expect_err("constant signed add overflow must be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_unsigned_add_constant_overflow_is_compile_error() {
  let code = r#"
var a as u8 = 255
var b as u8 = a + 1
"#;
  let err = run_default(code).expect_err("constant unsigned add overflow must be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_unsigned_mul_constant_overflow_is_compile_error() {
  let code = r#"
var a as u8 = 200
var b as u8 = a * 2
"#;
  let err = run_default(code).expect_err("constant unsigned mul overflow must be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_signed_sub_constant_underflow_is_compile_error() {
  // Negating a signed minimum expressed as `0 - MIN` (unary minus does not bind
  // to variables); `0 - (-128)` = 128 overflows i8.
  let code = r#"
var a as i8 = -128
var b as i8 = 0 as i8 - a
"#;
  let err = run_default(code).expect_err("constant signed sub underflow must be a compile error");
  assert!(err.contains("integer overflow"), "got: {}", err);
}

#[test]
fn test_non_constant_operands_do_not_error_at_compile_time() {
  // Unknown parameter values: no compile-time error; the runtime trap remains.
  let code = r#"
fn add(a as i8, b as i8) returns i8
    return a + b
end fn

write "ok"
"#;
  let output = run_default(code).expect("non-constant overflow must compile and run");
  assert!(output.contains("ok"), "got: {}", output);
}

#[test]
fn test_valid_constant_arithmetic_does_not_error() {
  let code = r#"
var a as i8 = 100
var b as i8 = a + 20
write "b = {b}"
"#;
  let output = run_default(code).expect("valid constant arithmetic must compile and run");
  assert!(output.contains("120"), "got: {}", output);
}

#[test]
fn test_unchecked_overflow_disables_compile_time_check() {
  // `--unchecked-overflow` opts out of both the runtime trap and the compile-time
  // check; the constant add wraps instead (i64 is the native width, so wrapping
  // is exact).
  let code = r#"
var a as i64 = 9223372036854775807
var b as i64 = a + 1
write "b = {b}"
"#;
  let output = run_unchecked(code).expect("unchecked overflow must wrap, not error");
  assert!(output.contains("-9223372036854775808"), "got: {}", output);
}
