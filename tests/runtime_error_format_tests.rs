// =============================================================================
// Runtime-error output format (golden tests)
// =============================================================================
// The runtime-error message is a color-free render spec carried by the IR trap
// instructions. The interpreter renders it and the output layer applies ANSI
// color (controlled by `--color`) plus a trailing newline. These tests assert
// the exact bytes so that a future AOT backend can be diffed against the same
// golden output.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run `lale run <args> -` with `code` on stdin, returning raw captured bytes
/// and whether the child exited successfully.
fn run_lale_stderr(args: &[&str], code: &str) -> (Vec<u8>, Vec<u8>, bool) {
  let mut cmd = Command::new(env!("CARGO_BIN_EXE_lale"));
  cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
  cmd.args(["run"]);
  cmd.args(args);
  cmd.arg("-");
  cmd.stdin(Stdio::piped());
  cmd.stdout(Stdio::piped());
  cmd.stderr(Stdio::piped());

  let mut child = cmd.spawn().expect("spawn lale");
  child
    .stdin
    .take()
    .expect("stdin")
    .write_all(code.as_bytes())
    .expect("write stdin");
  let output = child.wait_with_output().expect("wait");

  (output.stdout, output.stderr, output.status.success())
}

/// Assert that `code` aborts with exactly `expected` on stderr.
fn assert_abort_stderr(code: &str, args: &[&str], expected: &str) {
  let (stdout, stderr, success) = run_lale_stderr(args, code);
  assert!(!success, "expected a runtime abort");
  assert!(stdout.is_empty(), "runtime abort must not write stdout");
  assert_eq!(stderr, expected.as_bytes());
}

const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

// ==================== BoundsCheck (mixed Text + Value parts) ====================

#[test]
fn bounds_check_always_colors() {
  let code = "var arr as i32[3] = [10, 20, 30]\nvar idx as i32 = 5\nvar _ as i32 = arr[idx]\n";
  let expected =
    format!("{RED}ERROR at -:3:19: array index out of bounds (index=5, length=3){RESET}\n");
  assert_abort_stderr(code, &["--color", "always"], &expected);
}

#[test]
fn bounds_check_never_is_plain() {
  let code = "var arr as i32[3] = [10, 20, 30]\nvar idx as i32 = 5\nvar _ as i32 = arr[idx]\n";
  let expected = "ERROR at -:3:19: array index out of bounds (index=5, length=3)\n";
  assert_abort_stderr(code, &["--color", "never"], expected);
}

#[test]
fn bounds_check_auto_is_plain_when_piped() {
  // The child's stderr is a pipe (not a TTY), so `auto` resolves to no color.
  let code = "var arr as i32[3] = [10, 20, 30]\nvar idx as i32 = 5\nvar _ as i32 = arr[idx]\n";
  let expected = "ERROR at -:3:19: array index out of bounds (index=5, length=3)\n";
  assert_abort_stderr(code, &["--color", "auto"], expected);
}

// ==================== Checked overflow (single Text part + call stack) ====================

#[test]
fn checked_overflow_always_colors_with_call_stack() {
  let code = "fn inc(x as i8) returns i8\n  return x + 1\nend fn\n\nvar a as i8 = 127\nvar _ as i8 = inc(a)\n";
  let expected = format!(
    "{RED}ERROR at -:2:12: integer overflow in signed addition (i8){RESET}\n\n\
     Call stack (most recent first):\n  1. inc_i8  at -:6\n"
  );
  assert_abort_stderr(code, &["--color", "always"], &expected);
}

#[test]
fn checked_overflow_never_is_plain() {
  let code = "fn inc(x as i8) returns i8\n  return x + 1\nend fn\n\nvar a as i8 = 127\nvar _ as i8 = inc(a)\n";
  let expected = "ERROR at -:2:12: integer overflow in signed addition (i8)\n\n\
     Call stack (most recent first):\n  1. inc_i8  at -:6\n";
  assert_abort_stderr(code, &["--color", "never"], expected);
}

// ==================== Non-finite float traps (reject_non_finite path) ====================

#[test]
fn nan_always_colors() {
  let code = "var a as f64 = (-1.0) ^ 0.5\nwrite a\n";
  let expected =
    format!("{RED}ERROR: float power produced NaN — Lale treats NaN as an error{RESET}\n");
  assert_abort_stderr(code, &["--color", "always"], &expected);
}

#[test]
fn nan_never_is_plain() {
  let code = "var a as f64 = (-1.0) ^ 0.5\nwrite a\n";
  let expected = "ERROR: float power produced NaN — Lale treats NaN as an error\n";
  assert_abort_stderr(code, &["--color", "never"], expected);
}

#[test]
fn infinity_always_colors() {
  let code = "var a as f64 = 1e308 * 1e308\nwrite a\n";
  let expected = format!(
    "{RED}ERROR: float multiplication produced infinity — Lale treats infinity as an error{RESET}\n"
  );
  assert_abort_stderr(code, &["--color", "always"], &expected);
}

#[test]
fn infinity_never_is_plain() {
  let code = "var a as f64 = 1e308 * 1e308\nwrite a\n";
  let expected =
    "ERROR: float multiplication produced infinity — Lale treats infinity as an error\n";
  assert_abort_stderr(code, &["--color", "never"], expected);
}
