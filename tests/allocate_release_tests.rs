//! Allocate/Release Keyword Tests
//!
//! Integration tests for the `allocate` and `release` Lale keywords.
//!
//! `allocate(size_expr)` returns a raw heap pointer.
//! `release pointer_expr` frees the heap memory at that pointer.
//!
//! These tests use the interpreter (`lale run -`) to verify end-to-end behavior
//! from parsing through code generation to runtime execution.

use std::io::Write;
use std::process::{Command, Stdio};

/// Helper: Run lale code via interpreter and capture stdout on success,
/// stderr on failure.
fn run_interpreter(code: &str) -> Result<String, String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {}", e))?;

  {
    let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
    stdin
      .write_all(code.as_bytes())
      .map_err(|e| format!("Failed to write to stdin: {}", e))?;
  }

  let output = child
    .wait_with_output()
    .map_err(|e| format!("Failed to wait on child: {}", e))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

/// Helper: Run lale code via interpreter and return both stdout and stderr
/// regardless of exit status. stdout/stderr are captured as strings even
/// on success (unlike `run_interpreter` which returns Err on non-zero).
fn run_interpreter_raw(code: &str) -> Result<(String, String, bool), String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {}", e))?;

  {
    let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
    stdin
      .write_all(code.as_bytes())
      .map_err(|e| format!("Failed to write to stdin: {}", e))?;
  }

  let output = child
    .wait_with_output()
    .map_err(|e| format!("Failed to wait on child: {}", e))?;

  let success = output.status.success();
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  Ok((stdout, stderr, success))
}

// =============================================================================
// 1. Basic allocate and release
// =============================================================================

#[test]
fn test_basic_allocate_and_release() {
  let code = r#"
var buf as pointer = allocate(8 as u64)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 2. Allocate without release (should still compile and run)
// =============================================================================

#[test]
fn test_allocate_without_release_compiles() {
  // Allocate without release — program should still compile and run.
  // The interpreter reports heap leaks on stderr in debug mode but
  // the exit status is still success.
  let code = r#"
var buf as pointer = allocate(8 as u64)
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(
    result.is_ok(),
    "Expected success despite leak, got: {:?}",
    result
  );
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_allocate_without_release_reports_leak() {
  // Verify that the heap leak is reported on stderr.
  let code = r#"
var buf as pointer = allocate(8 as u64)
write "OK"
"#;
  let (stdout, stderr, success) = run_interpreter_raw(code).unwrap();
  assert!(success, "Expected exit success");
  assert!(stdout.contains("OK"), "Expected 'OK' in stdout");
  // In debug builds, the interpreter reports heap leaks on stderr
  assert!(
    stderr.contains("HEAP LEAK") || stderr.contains("leak"),
    "Expected leak warning on stderr, got: {}",
    stderr
  );
}

// =============================================================================
// 3. Allocate with write through pointer
// =============================================================================

#[test]
fn test_allocate_write_and_release() {
  let code = r#"
var buf as pointer = allocate(8 as u64)
unsafe value at buf = 42 as u64
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_allocate_write_variable_and_release() {
  let code = r#"
var buf as pointer = allocate(8 as u64)
var val as u64 = 100 as u64
unsafe value at buf = val
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 4. Release with expression
// =============================================================================

#[test]
fn test_release_with_addition_expression() {
  let code = r#"
var buf as pointer = allocate(16 as u64)
release (buf + 0 as u64)
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_release_with_literal_pointer_expression() {
  // Release of null pointer — should not crash.
  let code = r#"
release 0 as pointer
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 5. Multiple allocates and releases
// =============================================================================

#[test]
fn test_multiple_allocates_and_releases() {
  let code = r#"
var buf1 as pointer = allocate(8 as u64)
var buf2 as pointer = allocate(16 as u64)
var buf3 as pointer = allocate(32 as u64)
release buf1
release buf2
release buf3
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_multiple_allocates_out_of_order_release() {
  // Release in different order than allocation — should still work.
  let code = r#"
var a as pointer = allocate(8 as u64)
var b as pointer = allocate(16 as u64)
var c as pointer = allocate(32 as u64)
release c
release a
release b
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 6. Allocate in function and release in caller
// =============================================================================

#[test]
fn test_allocate_in_function_release_in_caller() {
  let code = r#"
fn make_buf() returns pointer
    return allocate(8 as u64)
end fn

var buf as pointer = make_buf()
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_release_in_function() {
  // Pass allocated pointer to function that releases it.
  let code = r#"
fn cleanup(ptr as pointer) returns nothing
    release ptr
end fn

var buf as pointer = allocate(8 as u64)
cleanup(buf)
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 7. allocate() used in expression position
// =============================================================================

#[test]
fn test_allocate_in_var_initializer() {
  // `allocate()` in expression position: RHS of var definition.
  let code = r#"
var buf as pointer = allocate(8 as u64)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_allocate_directly_passed_to_function() {
  // `allocate()` used directly as a function argument (expression position).
  let code = r#"
fn use_ptr(p as pointer) returns pointer
    return p
end fn

var buf as pointer = use_ptr(allocate(8 as u64))
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 8. Compile error: release without argument
// =============================================================================

#[test]
fn test_release_without_argument_is_parse_error() {
  // `release` with no argument should be a parse error.
  let code = r#"
release
"#;
  let result = run_interpreter(code);
  assert!(result.is_err(), "Expected parse error for bare 'release'");
  let err = result.unwrap_err();
  assert!(
    err.contains("grammar error") || err.contains("Parse error") || err.contains("expected"),
    "Expected grammar/parse error, got: {}",
    err
  );
}

#[test]
fn test_release_undefined_variable_is_semantic_error() {
  // `release` with an undefined variable reference.
  let code = r#"
release nonexistent_ptr
"#;
  let result = run_interpreter(code);
  assert!(
    result.is_err(),
    "Expected semantic error for undefined variable"
  );
  let err = result.unwrap_err();
  assert!(
    err.contains("Undefined") || err.contains("not defined") || err.contains("semantic error"),
    "Expected semantic error about undefined variable, got: {}",
    err
  );
}

// =============================================================================
// 9. allocate with u64 literal
// =============================================================================

#[test]
fn test_allocate_with_u64_literal() {
  let code = r#"
var buf as pointer = allocate(8 as u64)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_allocate_with_large_u64_literal() {
  // Allocate a large block.
  let code = r#"
var buf as pointer = allocate(1024 as u64)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

#[test]
fn test_allocate_with_zero_size() {
  // Zero-size allocation — edge case, should not crash.
  let code = r#"
var buf as pointer = allocate(0 as u64)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// Additional tests: allocate with variable expression
// =============================================================================

#[test]
fn test_allocate_with_variable_size() {
  let code = r#"
var sz as u64 = 64 as u64
var buf as pointer = allocate(sz)
release buf
write "OK"
"#;
  let result = run_interpreter(code);
  assert!(result.is_ok(), "Expected success, got: {:?}", result);
  assert!(
    result.as_ref().unwrap().contains("OK"),
    "Expected 'OK' in output"
  );
}

// =============================================================================
// 10. Compile-time lint: allocate/release warnings
// =============================================================================

/// Helper: run lale code and return (stdout, stderr, success).
/// Wraps `run_interpreter_raw` for convenience.
fn run_lale(code: &str) -> (String, String, bool) {
  run_interpreter_raw(code).unwrap()
}

#[test]
fn test_allocate_without_release_warns() {
  // Allocate in a function without releasing — lint should warn.
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(1024 as u64)
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected compilation success (warning only)");
  assert!(
    stderr.contains("Compile-time warning:"),
    "Expected a compile-time warning on stderr, got: {}",
    stderr
  );
  assert!(
    stderr.contains("is allocated via allocate() but never released"),
    "Expected 'never released' warning, got: {}",
    stderr
  );
}

#[test]
fn test_allocate_with_on_exit_release_no_warn() {
  // Allocate with `on exit release` — no warning expected.
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(1024 as u64)
    on exit release buf
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success");
  assert!(
    !stderr.contains("is allocated via allocate() but never released"),
    "Expected no 'never released' warning, but got: {}",
    stderr
  );
}

#[test]
fn test_allocate_with_explicit_release_no_warn() {
  // Allocate with explicit `release` — no warning expected.
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(1024 as u64)
    release buf
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success");
  assert!(
    !stderr.contains("is allocated via allocate() but never released"),
    "Expected no 'never released' warning, but got: {}",
    stderr
  );
}

#[test]
fn test_reassign_allocate_without_release_warns() {
  // Reassigning via allocate() without releasing first — lint should warn.
  let code = r#"
fn do_work() returns nothing
    var buf as pointer = allocate(1024 as u64)
    buf = allocate(2048 as u64)
    release buf
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected compilation success (warning only)");
  assert!(
    stderr.contains("reassignment to 'buf' overwrites a previous allocate()"),
    "Expected reassignment warning, got: {}",
    stderr
  );
}

#[test]
fn test_allocate_multiple_vars() {
  // Two independent allocates, only one released.
  // Should warn for the unreleased one only.
  let code = r#"
fn do_work() returns nothing
    var buf1 as pointer = allocate(1024 as u64)
    var buf2 as pointer = allocate(2048 as u64)
    release buf1
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected compilation success (warning only)");
  assert!(
    stderr.contains("Compile-time warning:"),
    "Expected a compile-time warning for unreleased var"
  );
  // buf1 is released — no warning for it.
  assert!(
    !stderr.contains("'buf1'"),
    "Expected NO warning for buf1 (it was released), got: {}",
    stderr
  );
  // buf2 is NOT released — should warn.
  assert!(
    stderr.contains("'buf2'"),
    "Expected warning for buf2 (not released), got: {}",
    stderr
  );
  assert!(
    stderr.contains("is allocated via allocate() but never released"),
    "Expected 'never released' warning for buf2, got: {}",
    stderr
  );
}

#[test]
fn test_release_non_allocated_no_warn() {
  // Release a variable that was never allocated — should not crash or warn.
  let code = r#"
fn do_work() returns nothing
    var p as pointer = 0 as pointer
    release p
end fn
do_work()
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success");
  assert!(
    !stderr.contains("Compile-time warning:"),
    "Expected no compile-time warnings, got: {}",
    stderr
  );
}

#[test]
fn test_allocate_global_no_warn() {
  // Allocate at global scope (outside any function).
  // The lint only runs per-function, so global allocates should NOT warn.
  let code = r#"
var buf as pointer = allocate(1024 as u64)
"#;
  let (_stdout, stderr, success) = run_lale(code);
  assert!(success, "Expected success");
  assert!(
    !stderr.contains("is allocated via allocate() but never released"),
    "Expected no warning for global allocate, got: {}",
    stderr
  );
}
