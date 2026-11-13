// =============================================================================
// Optional Type Tests
// =============================================================================
// Tests for optional types (T?): return nothing, has value, has no value, value of.
// Represents T? as struct {is_present: bool, value: T} in IR.

use std::fs;
use std::process::Command;

/// Test helper: Write Lale source to temp file, run the built binary directly,
/// capture output.
fn run_lale(code: &str) -> (bool, String) {
  let dir = tempfile::tempdir().unwrap();
  let temp_file = dir.path().join("test.lale");

  fs::write(&temp_file, code).expect("Failed to write test file");

  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", temp_file.to_str().unwrap()])
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .output()
    .expect("Failed to run lale interpreter");

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();
  let combined = format!("{}{}", stdout, stderr);

  (success, combined)
}

// =============================================================================
// Basic Optional Function: return nothing + has value + value of
// =============================================================================

#[test]
fn test_optional_basic_some() {
  let code = r#"
fn maybe_value(flag as bool) returns i64?
    if flag
        return 42
    else
        return nothing
    end if
end fn

var result as bool = maybe_value(true) has value
if result
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_optional_basic_none() {
  let code = r#"
fn maybe_value(flag as bool) returns i64?
    if flag
        return 42
    else
        return nothing
    end if
end fn

var result as bool = maybe_value(false) has no value
if result
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_optional_value_of_present() {
  let code = r#"
fn maybe_value(flag as bool) returns i64?
    if flag
        return 42
    else
        return nothing
    end if
end fn

var val as i64 = value of maybe_value(true)
write "{val}"
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("42"), "Expected '42', got: {}", output);
}

#[test]
fn test_optional_value_of_absent_should_abort() {
  let code = r#"
fn maybe_value(flag as bool) returns i64?
    if flag
        return 42
    else
        return nothing
    end if
end fn

var val as i64 = value of maybe_value(false)
write "{val}"
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected abort due to absent unwrap");
  assert!(
    output.contains("absent optional") || output.contains("ERROR"),
    "Expected error message, got: {}",
    output
  );
}

// =============================================================================
// Nothing Expression in Variable Definition
// =============================================================================

#[test]
fn test_optional_nothing_in_var_def() {
  let code = r#"
var maybe_val as i64? = nothing
if maybe_val has no value
    write "absent"
else
    write "present"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("absent"),
    "Expected 'absent', got: {}",
    output
  );
}

#[test]
fn test_optional_concrete_in_var_def() {
  let code = r#"
var maybe_val as i64? = 42 as i64
if maybe_val has value
    write "present"
else
    write "absent"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("present"),
    "Expected 'present', got: {}",
    output
  );
}

// =============================================================================
// Optional Types with Different Inner Types
// =============================================================================

#[test]
fn test_optional_f64() {
  let code = r#"
	fn safe_divide(a as f64, b as f64) returns f64?
	    when b == 0.0
	        return nothing
	    end when
	    return a / b
	end fn

var result as f64? = safe_divide(10.0, 2.0)
if result has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_optional_bool() {
  let code = r#"
fn maybe_flag(flag as bool) returns bool?
    if flag
        return flag
    else
        return nothing
    end if
end fn

var result as bool? = maybe_flag(true)
if result has value
    write "present"
else
    write "absent"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("present"),
    "Expected 'present', got: {}",
    output
  );
}

// =============================================================================
// Semantic Error: value of on non-optional type
// =============================================================================

#[test]
fn test_value_of_on_non_optional_errors() {
  let code = r#"
var x as i64 = 42
var y as i64 = value of x
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected compilation error");
  assert!(
    output.contains("value of") && output.contains("optional"),
    "Expected error about value of requiring optional, got: {}",
    output
  );
}

// =============================================================================
// Chained Optional Operations
// =============================================================================

#[test]
fn test_optional_chained_checks() {
  let code = r#"
fn get_value(x as i64) returns i64?
    if x > 0
        return x
    else
        return nothing
    end if
end fn

var a as i64? = get_value(10)
var b as i64? = get_value(0)

if a has value
    if b has no value
        write "ok"
    else
        write "fail_b"
    end if
else
    write "fail_a"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

// =============================================================================
// return nothing in returns nothing function (should be no-op)
// =============================================================================

#[test]
fn test_return_nothing_in_void_function() {
  let code = r#"
fn do_stuff() returns nothing
    return nothing
end fn

do_stuff()
write "ok"
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

// =============================================================================
// Parse Function Input Validation
// =============================================================================

#[test]
fn test_parse_float_valid() {
  let code = r#"
use all from std.full
var val as f64? = parse_float("3.14")
if val has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_parse_float_invalid_abc() {
  let code = r#"
use all from std.full
var val as f64? = parse_float("abc")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_float_empty() {
  let code = r#"
use all from std.full
var val as f64? = parse_float("")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_float_trailing_garbage() {
  let code = r#"
use all from std.full
var val as f64? = parse_float("123abc")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_int_valid() {
  let code = r#"
use all from std.full
var val as i64? = parse_int("42")
if val has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_parse_int_rejects_float() {
  let code = r#"
use all from std.full
var val as i64? = parse_int("3.14")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_int_rejects_abc() {
  let code = r#"
use all from std.full
var val as i64? = parse_int("abc")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_int_negative() {
  let code = r#"
use all from std.full
var val as i64? = parse_int("-5")
if val has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_parse_uint_valid() {
  let code = r#"
use all from std.full
var val as u64? = parse_uint("99")
if val has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_parse_uint_rejects_negative() {
  let code = r#"
use all from std.full
var val as u64? = parse_uint("-1")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_uint_rejects_float() {
  let code = r#"
use all from std.full
var val as u64? = parse_uint("1.5")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_uint_rejects_abc() {
  let code = r#"
use all from std.full
var val as u64? = parse_uint("abc")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

// =============================================================================
// strto* whitespace rejection and base handling
// =============================================================================
// The interpreter's strtod/strtol/strtoul emulation is strict: the whole string
// must be a valid number. Leading and trailing whitespace are rejected (no
// libc-style prefix parsing), and strtol/strtoul honor the base argument (2–36).

#[test]
fn test_parse_int_rejects_leading_whitespace() {
  let code = r#"
use all from std.full
var val as i64? = parse_int(" 42")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_int_rejects_trailing_whitespace() {
  let code = r#"
use all from std.full
var val as i64? = parse_int("42 ")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_float_rejects_leading_whitespace() {
  let code = r#"
use all from std.full
var val as f64? = parse_float(" 3.14")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_parse_uint_rejects_trailing_whitespace() {
  let code = r#"
use all from std.full
var val as u64? = parse_uint("99 ")
if val has no value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("ok"),
    "Expected 'ok' (absent), got: {}",
    output
  );
}

#[test]
fn test_strtol_honors_base() {
  let code = r#"
import fn signature strtol(nptr as pointer, endptr as pointer, base as i32) returns i64
var s as text = "ff"
var endptr as pointer = 0 as pointer
var val as i64 = strtol(s.ptr, pointer to endptr, 16 as i32)
if val == 255 as i64
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_strtoul_honors_base() {
  let code = r#"
import fn signature strtoul(nptr as pointer, endptr as pointer, base as i32) returns u64
var s as text = "ff"
var endptr as pointer = 0 as pointer
var val as u64 = strtoul(s.ptr, pointer to endptr, 16 as i32)
if val == 255 as u64
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

// =============================================================================
// Error Stack Tests
// =============================================================================

#[test]
fn test_add_error_and_last_error() {
  let code = r#"
use all from std.full
add error "something broke"
var msg as text = last error
write msg
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("something broke"),
    "Expected 'something broke', got: {}",
    output
  );
}

#[test]
fn test_errors_has_messages_present() {
  let code = r#"
use all from std.full
add error "test"
if errors has messages
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_errors_has_messages_empty() {
  let code = r#"
use all from std.full
if errors has messages
    write "fail"
else
    write "ok"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_last_error_consumes() {
  let code = r#"
use all from std.full
add error "alpha"
add error "beta"
var first as text = last error
// first should be "beta" (LIFO) — just write it to verify
write first
var second as text = last error
// second should be "alpha"
write second
	if not (errors has messages)
	    write "drained"
	else
	    move on
	end if
	"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("beta") && output.contains("alpha") && output.contains("drained"),
    "Expected beta, alpha, drained. Got: {}",
    output
  );
}

#[test]
fn test_last_error_empty_stack_returns_nothing() {
  let code = r#"
use all from std.full
var msg as text = last error
write msg
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("Nothing"),
    "Expected 'Nothing', got: {}",
    output
  );
}

#[test]
fn test_drain_clears_stack() {
  let code = r#"
use all from std.full
add error "msg1"
add error "msg2"
alert error messages
// stack should be empty now
	if not (errors has messages)
	    write "cleared"
	else
	    move on
	end if
	"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    output.contains("cleared"),
    "Expected 'cleared', got: {}",
    output
  );
}

#[test]
fn test_alert_error_messages() {
  let code = r#"
use all from std.full
add error "problem"
alert error messages
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("problem"),
    "Expected 'problem', got: {}",
    output
  );
}

#[test]
fn test_alert_error_messages_empty() {
  let code = r#"
use all from std.full
alert error messages
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("Nothing"),
    "Expected 'Nothing', got: {}",
    output
  );
}

#[test]
fn test_alert_statement() {
  let code = r#"
use all from std.full
alert "something went wrong"
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("something went wrong"),
    "Expected 'something went wrong', got: {}",
    output
  );
}

// =============================================================================
// Optional Lint Tests: unguarded value of + unchecked optional warnings
// =============================================================================

#[test]
fn test_warn_unguarded_value_of() {
  // value of x without has value/has no value check should emit a warning
  let code = r#"
fn maybe() returns i64?
    return 42
end fn

var opt as i64? = maybe()
var val as i64 = value of opt
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    output.contains("without a visible"),
    "Expected unguarded value-of warning, got: {}",
    output
  );
}

#[test]
fn test_no_warn_guarded_value_of() {
  // value of x WITH has value check should NOT emit a warning
  let code = r#"
fn maybe() returns i64?
    return 42
end fn

	var result as i64? = maybe()
	if result has value
	    var val as i64 = value of result
	    write "{val}"
	else
	    move on
	end if
	"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    !output.contains("without a visible"),
    "Should NOT warn when guarded, got: {}",
    output
  );
}

#[test]
fn test_warn_unchecked_optional() {
  // T? variable defined but never checked should emit a warning
  let code = r#"
fn maybe() returns i64?
    return nothing
end fn

fn wrapper() returns nothing
    var val as i64? = maybe()
    write "done"
end fn

wrapper()
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    output.contains("never checked with"),
    "Expected unchecked-optional warning, got: {}",
    output
  );
}

#[test]
fn test_no_warn_checked_optional() {
  // T? variable checked with has value should NOT emit a warning
  let code = r#"
fn maybe() returns i64?
    return nothing
end fn

var val as i64? = maybe()
if val has value
    write "present"
else
    write "absent"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    !output.contains("never checked with"),
    "Should NOT warn when checked, got: {}",
    output
  );
}

#[test]
fn test_no_warn_checked_optional_has_no_value() {
  // T? variable checked with has no value should NOT emit a warning
  let code = r#"
fn maybe() returns i64?
    return nothing
end fn

	var val as i64? = maybe()
	if val has no value
	    write "absent"
	else
	    move on
	end if
	"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should compile successfully");
  assert!(
    !output.contains("never checked with"),
    "Should NOT warn when checked with has no value, got: {}",
    output
  );
}

#[test]
fn test_warn_unchecked_optional_param() {
  // T? function parameter never checked should emit a warning
  let code = r#"
fn process(val as i64?) returns nothing
    write "processing"
end fn

var n as i64? = nothing
process(n)
"#;
  let (success, output) = run_lale(code);
  if !success {
    panic!("Compilation failed. Output: {}", output);
  }
  assert!(
    output.contains("never checked with"),
    "Expected unchecked-optional warning for parameter, got: {}",
    output
  );
}

#[test]
fn test_no_warn_checked_optional_param() {
  // T? function parameter checked should NOT emit a warning
  let code = r#"
fn process(val as i64?) returns nothing
    if val has value
        write "present"
    else
        write "absent"
    end if
end fn

var n as i64? = nothing
process(n)
"#;
  let (success, output) = run_lale(code);
  if !success {
    panic!("Compilation failed. Output: {}", output);
  }
  assert!(
    !output.contains("never checked with"),
    "Should NOT warn when param is checked, got: {}",
    output
  );
}

// =============================================================================
// ? Operator Tests (try-propagate)
// =============================================================================

#[test]
fn test_try_propagate_present() {
  let code = r#"
fn maybe(val as f64) returns f64?
    if val > 0.0
        return val
    else
        return nothing
    end if
end fn

fn process(val as f64) returns f64?
    var x as f64 = maybe(val)?
    return x
end fn

var r as f64? = process(1.0)
if r has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_try_propagate_absent() {
  let code = r#"
fn maybe(val as f64) returns f64?
    if val > 0.0
        return val
    else
        return nothing
    end if
end fn

fn process(val as f64) returns f64?
    var x as f64 = maybe(val)?
    return x
end fn

var r as f64? = process(-1.0)
if r has no value
    write "absent"
else
    write "present"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("absent"),
    "Expected 'absent', got: {}",
    output
  );
}

#[test]
fn test_try_propagate_chained() {
  let code = r#"
fn maybe(val as f64) returns f64?
    if val > 0.0
        return val
    else
        return nothing
    end if
end fn

fn double(val as f64) returns f64?
    var x as f64 = maybe(val)?
    return x * 2.0
end fn

fn process(val as f64) returns f64?
    var x as f64 = double(val)?
    return x + 1.0
end fn

var r as f64? = process(5.0)
if r has value
    write "ok"
else
    write "fail"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("ok"), "Expected 'ok', got: {}", output);
}

#[test]
fn test_try_propagate_top_level_absent_should_exit() {
  let code = r#"
fn maybe(val as f64) returns f64?
    if val > 0.0
        return val
    else
        return nothing
    end if
end fn

var x as f64 = maybe(-1.0)?
write "should not reach"
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected abort due to propagate at top level");
  assert!(
    output.contains("maybe"),
    "Expected error mentioning function name, got: {}",
    output
  );
}

#[test]
fn test_try_propagate_semantic_error_non_optional() {
  let code = r#"
var x as f64 = 42.0
var y as f64 = x?
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected compilation error");
  assert!(
    output.contains("optional"),
    "Expected error about ? requiring optional, got: {}",
    output
  );
}

#[test]
fn test_try_propagate_semantic_error_wrong_context() {
  let code = r#"
fn maybe(val as f64) returns f64?
    if val > 0.0
        return val
    else
        return nothing
    end if
end fn

fn wrong_context(val as f64) returns f64
    var x as f64 = maybe(val)?
    return x
end fn
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected compilation error");
  assert!(
    output.contains("function that returns T?"),
    "Expected error about wrong function context, got: {}",
    output
  );
}

#[test]
fn test_try_propagate_then_equality() {
  // `expr? == other` must parse as propagate-then-compare, not confuse `?` with `==`.
  let code = r#"
fn get_value() returns u64?
    return 7 as u64
end fn

if (get_value())? == 7 as u64
    write "equal"
else
    write "not equal"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(success, "Expected successful execution, got: {}", output);
  assert!(
    output.contains("equal"),
    "Expected 'equal', got: {}",
    output
  );
}

#[test]
fn test_qmark_equals_is_rejected() {
  // `?=` is no longer an operator; it should be a parse error.
  let code = r#"
var x as u64 = 0
var y as u64 = 1
if x ?= y
    write "equal"
else
    write "not equal"
end if
"#;
  let (success, output) = run_lale(code);
  assert!(!success, "Expected ?= to be rejected, got: {}", output);
}
