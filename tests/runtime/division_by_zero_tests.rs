// =============================================================================
// Division by Zero Detection Tests
// =============================================================================
// Tests for the semantic analyzer's ability to detect unguarded divisions
// and modulo operations. The analyzer tracks structurally identical expressions
// in surrounding guard conditions to prove non-zeroness.

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

  (success, format!("{}{}", stdout, stderr))
}

// =============================================================================
// Basic warning tests
// =============================================================================

#[test]
fn test_warn_unguarded_division() {
  let code = r#"
var x as i32 = 42
var y as i32 = 0
var z as i32 = x / y
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected division-by-zero warning, got: {}",
    output
  );
  // Code should still compile and run (runtime will abort though)
  assert!(
    !success,
    "Code with division by zero should fail at runtime"
  );
}

#[test]
fn test_warn_unguarded_modulo() {
  let code = r#"
var x as i32 = 42
var y as i32 = 0
var z as i32 = x % y
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected division-by-zero warning for modulo, got: {}",
    output
  );
  assert!(!success, "Code with modulo by zero should fail at runtime");
}

// =============================================================================
// Guarded division tests (should NOT warn)
// =============================================================================

#[test]
fn test_no_warn_guarded_with_not_eq_zero() {
  let code = r#"
var x as i32 = 42
var y as i32 = 10
		if y != 0
		    var z as i32 = x / y
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guarded with !=, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_no_warn_guarded_with_gt_zero() {
  let code = r#"
var x as i32 = 42
var y as i32 = 10
		if y > 0
		    var z as i32 = x / y
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guarded with > 0, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_no_warn_guarded_with_lt_zero() {
  let code = r#"
var x as i32 = 42
var y as i32 = -5
		if y < 0
		    var z as i32 = x / y
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guarded with < 0, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_no_warn_constant_divisor() {
  let code = r#"
var x as i32 = 42
var z as i32 = x / 7
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn for constant non-zero divisor, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// Conversion-wrapped divisor tests
// =============================================================================

#[test]
fn test_no_warn_conversion_divisor_guarded() {
  // Guard is on bare x, divisor is x as f64 — conversion is transparent for zero check
  let code = r#"
var x as i32 = 42
var n as f64 = 100.0
		if x != 0
		    var z as f64 = n / x as f64
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when divisor is x as T guarded by bare x, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_no_warn_guard_numeric_literal_equivalence() {
  // `4 ⋅ c` and `4.0 ⋅ c` are the same divisor; the integer and float literal
  // spellings of the same value should match the guard.
  let code = r#"
var c as f64 = 3.141592653589793
var n as f64 = 42.0
		if (4 ⋅ c) != 0
		    var z as f64 = n / (4.0 ⋅ c)
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when the divisor is numerically equal to the guard, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// else-branch guard tests
// =============================================================================

#[test]
fn test_warn_inside_then_branch_of_eq_zero() {
  // Division inside "if y == 0" — y IS zero here
  let code = r#"
var x as i32 = 42
var y as i32 = 0
		if y == 0
		    var z as i32 = x / y   // Definitely dangerous
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning for division inside y==0 branch, got: {}",
    output
  );
  assert!(!success, "Code with guaranteed div-by-zero should fail");
}

#[test]
fn test_warn_else_and_then_both_checked() {
  // y is a function PARAMETER — its value is unknown at compile time.
  // In the THEN branch (y == 0), y IS zero → division by y warns.
  // In the ELSE branch (y == 0 false), y is non-zero → no warning.
  let code = r#"
fn test(x as i32, y as i32) returns nothing
  if y == 0
      var z as i32 = x / y   // THEN: warned (y could be 0)
  else
      var w as i32 = x / y   // ELSE: guarded
  end if
end fn
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning for unguarded division in then-branch, got: {}",
    output
  );
  // Program has no main — execution failure expected, warning verified above
}

#[test]
fn test_no_warn_non_zero_initializer_never_reassigned() {
  // y is initialized to non-zero and never reassigned — provably safe.
  let code = r#"
var x as i32 = 42
var y as i32 = 10
var z as i32 = x / y
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should not warn: y = 10 and never reassigned, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_warn_non_zero_initializer_then_reassigned() {
  // y is initialized to 10 but then reassigned to 0 — should warn.
  let code = r#"
var x as i32 = 42
var y as i32 = 10
y = 0
var z as i32 = x / y
write z
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: y was reassigned after initialization, got: {}",
    output
  );
}

// =============================================================================
// Not-negated condition tests
// =============================================================================

#[test]
fn test_no_warn_guarded_with_not_expr() {
  let code = r#"
var x as i32 = 42
var y as i32 = 10
		if not (y == 0)
		    var z as i32 = x / y
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guarded with not (y == 0), got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// else-if tests
// =============================================================================

#[test]
fn test_no_warn_guarded_in_match_when() {
  let code = r#"
var x as i32 = 42
var y as i32 = 10
match
    when y == 0:
        var a as i32 = 0
    when y > 0:
        var z as i32 = x / y   // Guarded: y > 0 in this branch
    else:
        move on
end match
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guarded in match when arm, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// Division outside guard should warn
// =============================================================================

#[test]
fn test_warn_division_outside_guard() {
  // Guard does NOT cover the division because division is outside the if block
  let code = r#"
var x as i32 = 42
var y as i32 = 0
		if y != 0
		    var dummy as i32 = 1
		else move on
		end if
var z as i32 = x / y   // Division outside guard
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning for division outside guard, got: {}",
    output
  );
  assert!(!success, "Code with unguarded div-by-zero should fail");
}

// =============================================================================
// Compound assignment tests
// =============================================================================

#[test]
fn test_warn_compound_div_assign_unguarded() {
  let code = r#"
var x as i32 = 42
var y as i32 = 0
x /= y
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning for compound /=, got: {}",
    output
  );
  assert!(!success, "Code with compound div-by-zero should fail");
}

#[test]
fn test_no_warn_compound_div_assign_guarded() {
  let code = r#"
var x as i32 = 42
var y as i32 = 10
		if y != 0
		    x /= y
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn for guarded compound /=, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// Structurally identical expression tests
// =============================================================================

#[test]
fn test_no_warn_function_call_divisor_guarded() {
  // foo(x) appears in both guard and divisor — structurally identical
  let code = r#"
fn foo(a as f64) returns f64
    return a
end fn

var x as f64 = 1.0
var n as f64 = 42.0
		if foo(x) != 0 as f64
		    var z as f64 = n / foo(x)
		else move on
		end if
		"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when guard and divisor are structurally identical function calls, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

// =============================================================================
// Conservative false-positive tests (should still warn)
// =============================================================================

#[test]
fn test_warn_different_expression_divisor() {
  // foo(x) != 0 does NOT guard 2*foo(x) — different structure
  let code = r#"
fn foo(a as f64) returns f64
    return a
end fn

var x as f64 = 1.0
var n as f64 = 42.0
		if foo(x) != 0 as f64
		    var z as f64 = n / (2 as f64 * foo(x))
		else move on
		end if
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning when divisor differs structurally from guard, got: {}",
    output
  );
  // Still succeeds at runtime because the divisor IS actually non-zero
  assert!(
    success,
    "Code should compile and run (divisor actually non-zero at runtime)"
  );
}

// =============================================================================
// match-arm guard tests (when inside a match statement)
// =============================================================================

#[test]
fn test_match_arm_guard_suppresses_warning() {
  let code = r#"
var x as i32 = 42
var y as i32 = 0
y = 10
match
    when y != 0:
        var z as i32 = x / y
        write "OK"
end match
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn when a match arm guard proves the divisor non-zero, got: {}",
    output
  );
  assert!(success, "Code should compile and run: {}", output);
  assert!(output.contains("OK"), "Expected OK output: {}", output);
}

#[test]
fn test_match_arm_unrelated_guard_still_warns() {
  let code = r#"
var x as i32 = 42
var y as i32 = 0
match
    when true:
        var z as i32 = x / y
end match
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected warning when a match arm guard does not prove non-zero, got: {}",
    output
  );
  assert!(!success, "Division by zero should abort at runtime");
}

// =============================================================================
// Compound assignment runtime ZeroCheck
// =============================================================================

#[test]
fn test_compound_div_assign_emits_runtime_zero_check() {
  let code = r#"
var x as i32 = 10
var y as i32 = 0
x /= y
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected compile-time warning for /=, got: {}",
    output
  );
  assert!(
    output.contains("ERROR at"),
    "Expected runtime ZeroCheck abort for /=, got: {}",
    output
  );
  assert!(
    output.contains("division by zero"),
    "Expected division-by-zero runtime message, got: {}",
    output
  );
  assert!(!success, "Division by zero should abort at runtime");
}

#[test]
fn test_compound_mod_assign_emits_runtime_zero_check() {
  let code = r#"
var x as i32 = 10
var y as i32 = 0
x %= y
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Expected compile-time warning for %=, got: {}",
    output
  );
  assert!(
    output.contains("ERROR at"),
    "Expected runtime ZeroCheck abort for %=, got: {}",
    output
  );
  assert!(
    output.contains("modulo by zero"),
    "Expected modulo-by-zero runtime message, got: {}",
    output
  );
  assert!(!success, "Modulo by zero should abort at runtime");
}

// =============================================================================
// Constant-expression divisor proof (non-zero compile-time constants)
// =============================================================================

#[test]
fn test_no_warn_constant_product_divisor() {
  // `c` is a global initialized to a non-zero literal, never reassigned, and
  // never shared (not export/import, address-taken, or ref-passed). The product
  // `4.0 ⋅ c` is therefore provably non-zero and must not warn.
  let code = r#"
var c as f64 = 3.141592653589793
var n as f64 = 42.0
var z as f64 = n / (4.0 ⋅ c)
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn for a constant non-zero product divisor, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}

#[test]
fn test_warn_pointer_to_invalidates_constant() {
  // Taking the address of `x` makes it mutable through the pointer, so it can
  // no longer be treated as a constant — the division must warn.
  let code = r#"
var x as f64 = 5.0
var p as pointer = pointer to x
var n as f64 = 42.0
var z as f64 = n / x
write z
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: `pointer to x` invalidates x as a constant, got: {}",
    output
  );
}

#[test]
fn test_warn_ref_argument_invalidates_constant() {
  // Passing `x` to a `ref` parameter lets the callee mutate it, so after the
  // call `x` is no longer a constant — the division must warn.
  let code = r#"
fn f(ref a as f64) returns nothing
    a = a * 2.0
end fn

var x as f64 = 5.0
f(x)
var n as f64 = 42.0
var z as f64 = n / x
write z
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: `f(x)` passes x by ref and invalidates it, got: {}",
    output
  );
}

#[test]
fn test_warn_export_variable_not_constant() {
  // An `export`ed variable is shared with other modules and may be mutated out
  // of sight, so it is never treated as a compile-time constant.
  let code = r#"
export var x as f64 = 5.0
var n as f64 = 42.0
var z as f64 = n / x
write z
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: exported variable x is not a constant, got: {}",
    output
  );
}

#[test]
fn test_warn_import_variable_not_constant() {
  // An `import`ed variable originates outside this module and may change at any
  // time, so it is never treated as a compile-time constant.
  let code = r#"
import var x as f64 = 5.0
var n as f64 = 42.0
var z as f64 = n / x
write z
"#;
  let (_success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: imported variable x is not a constant, got: {}",
    output
  );
}

#[test]
fn test_no_warn_sum_of_nonzero_constants() {
  // `a + b` folds exactly to 8.0 (non-zero), so `n / (a + b)` no longer warns.
  // (Step 5: the typed fold replaces the old leaf-only proof, which had to treat
  // any `+` conservatively because `5 + -5 == 0`.)
  let code = r#"
var a as f64 = 5.0
var b as f64 = 3.0
var n as f64 = 42.0
var z as f64 = n / (a + b)
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn: a + b = 8.0 is provably non-zero, got: {}",
    output
  );
  assert!(
    success,
    "Code should compile and run (a + b is actually non-zero)"
  );
}

// =============================================================================
// Join-point soundness (a variable written in sibling branches is not constant)
// =============================================================================

#[test]
fn test_warn_branch_join_var_not_constant() {
  // `x` is defined in both branches with different values. After the join it
  // could be either, so it must NOT be treated as a constant.
  let code = r#"
var n as f64 = 42.0
if true
    var x as f64 = 5.0
else
    var x as f64 = 0.0
end if
var z as f64 = n / x
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: x is a branch join, not a constant, got: {}",
    output
  );
  assert!(success, "Code should compile and run (then-branch x = 5.0)");
}

#[test]
fn test_warn_branch_reassignment_not_constant() {
  // `x` is reassigned in each branch. After the join it could be 7.0 or 3.0,
  // so even though both are non-zero the compiler cannot prove it and must warn.
  let code = r#"
var n as f64 = 42.0
var x as f64 = 5.0
if true
    x = 7.0
else
    x = 3.0
end if
var z as f64 = n / x
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    output.contains("not provably non-zero"),
    "Should warn: x is reassigned in branches, not a constant, got: {}",
    output
  );
  assert!(
    success,
    "Code should compile and run (x is non-zero on both paths)"
  );
}

#[test]
fn test_no_warn_straight_line_reassignment_reestablishes_constant() {
  // Straight-line reassignment (no branches) re-establishes the constant.
  let code = r#"
var n as f64 = 42.0
var x as f64 = 0.0
x = 7.0
var z as f64 = n / x
write z
"#;
  let (success, output) = run_lale(code);
  assert!(
    !output.contains("not provably non-zero"),
    "Should NOT warn: x was reassigned to 7.0 in straight-line code, got: {}",
    output
  );
  assert!(success, "Code should compile and run");
}
