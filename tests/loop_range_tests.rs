/// Integration tests for loop range and step behavior.
///
/// Covers bugs fixed in the interpreter and IR generator:
/// - Loop variable starts at correct `from` value (not 0)
/// - Step expression is actually used (was silently ignored)
/// - Step type must match loop variable type (semantic error)
/// - Loop range check is in header, not after body execution
/// - Float literals infer correctly for integer loop variables
use std::io::Write;
use std::process::{Command, Stdio};

/// Helper: Run lale code via interpreter and capture stdout
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
    .map_err(|e| format!("Failed to wait for lale: {}", e))?;

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  Ok(stdout)
}

// ==================== LOOP VARIABLE CORRECTLY INITIALIZED ====================

#[test]
fn test_loop_u32_starts_at_from_value() {
  let code = r#"
loop over i as u32 from 5 to 10
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("5"),
    "Should print 5 as first value, got: {}",
    output
  );
  // Should not print standalone 0 — but 10 contains a '0' character,
  // so check that there's no line with just "0"
  let has_zero_line = output.lines().any(|l| l.trim() == "0");
  assert!(
    !has_zero_line,
    "Should not have a standalone 0 line, got: {}",
    output
  );
}

#[test]
fn test_loop_i32_starts_at_from_value() {
  let code = r#"
loop over i as i32 from -3 to 3
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("-3"),
    "Should print -3 as first value, got: {}",
    output
  );
}

#[test]
fn test_loop_f64_starts_at_from_value() {
  let code = r#"
loop over i as f64 from 2.5 to 5.0
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("2.5"),
    "Should print 2.5 as first value, got: {}",
    output
  );
}

// ==================== STEP WITH UNSIGNED INTEGER ====================

#[test]
fn test_loop_u32_step_2() {
  let code = r#"
loop over i as u32 from 1 to 10 step 2
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Should print: 1, 3, 5, 7, 9
  assert!(output.contains("1"), "Should print 1, got: {}", output);
  assert!(output.contains("3"), "Should print 3, got: {}", output);
  assert!(output.contains("5"), "Should print 5, got: {}", output);
  assert!(output.contains("7"), "Should print 7, got: {}", output);
  assert!(output.contains("9"), "Should print 9, got: {}", output);
  // Should NOT print 11 (step 2 overshoots 10 after 9)
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    5,
    "Expected exactly 5 iterations, got {}: {:?}",
    lines.len(),
    lines
  );
}

#[test]
fn test_loop_u32_step_7_overshoots_range() {
  let code = r#"
loop over i as u32 from 1 to 5 step 7
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Should print only 1 (1+7=8 > 5, second iteration skipped)
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    1,
    "Expected exactly 1 iteration, got {}: {:?}",
    lines.len(),
    lines
  );
  assert!(output.contains("1"), "Should print 1, got: {}", output);
}

// ==================== STEP WITH FLOAT ====================

#[test]
fn test_loop_f64_step_half() {
  let code = r#"
loop over i as f64 from 1.0 to 3.0 step 0.5
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Should print: 1, 1.5, 2, 2.5, 3 (mainstream float formatting)
  let lines: Vec<&str> = output
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty())
    .collect();
  assert_eq!(
    lines,
    vec!["1", "1.5", "2", "2.5", "3"],
    "Unexpected loop output: {}",
    output
  );
}

#[test]
fn test_loop_f32_step_half() {
  let code = r#"
loop over i as f32 from 1.0 to 3.0 step 0.5
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // f32 with step 0.5: the float literal 0.5 should infer to f32 via context
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    5,
    "Expected 5 iterations for f32 step 0.5, got {}: {:?}",
    lines.len(),
    lines
  );
}

// ==================== STEP FROM INTEGER LITERAL WITH FLOAT LOOP VAR ====================

#[test]
fn test_loop_f64_with_integer_from_and_step() {
  let code = r#"
loop over i as f64 from 1 to 5 step 2
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Integer literals 1 and 2 should be promoted to f64 for the loop variable
  assert!(
    output.contains("1\n") || output.contains("1.0"),
    "Should print 1, got: {}",
    output
  );
  assert!(
    output.contains("3\n") || output.contains("3.0"),
    "Should print 3, got: {}",
    output
  );
  assert!(
    output.contains("5\n") || output.contains("5.0"),
    "Should print 5, got: {}",
    output
  );
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    3,
    "Expected 3 iterations, got {}: {:?}",
    lines.len(),
    lines
  );
}

// ==================== STEP TYPE MISMATCH ERRORS ====================

/// Helper: Run and capture stderr for semantic errors
fn run_interpreter_stderr(code: &str) -> Result<String, String> {
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
    .map_err(|e| format!("Failed to wait for lale: {}", e))?;

  Ok(String::from_utf8_lossy(&output.stderr).to_string())
}

#[test]
fn test_step_float_with_u32_rejected() {
  let code = r#"
loop over i as u32 from 1 to 5 step 0.5
    write i
end loop
"#;
  let stderr = run_interpreter_stderr(code).unwrap();
  assert!(
    stderr.contains("Step type mismatch"),
    "Should report step type mismatch, got: {}",
    stderr
  );
  assert!(
    stderr.contains("u32") && stderr.contains("f64"),
    "Should mention u32 and f64 types, got: {}",
    stderr
  );
}

#[test]
fn test_step_float_with_i32_rejected() {
  let code = r#"
loop over i as i32 from 1 to 5 step 0.5
    write i
end loop
"#;
  let stderr = run_interpreter_stderr(code).unwrap();
  assert!(
    stderr.contains("Step type mismatch"),
    "Should report step type mismatch for i32 + f64 step, got: {}",
    stderr
  );
}

// ==================== LOOP RANGE CHECK IN HEADER ====================

#[test]
fn test_loop_step_larger_than_range_runs_once() {
  let code = r#"
loop over i as u32 from 1 to 3 step 5
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Step 5 from 1: i=1, then 6 > 3 → exit. Only one iteration.
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    1,
    "Expected 1 iteration, got {}: {:?}",
    lines.len(),
    lines
  );
  assert!(output.contains("1"), "Should only print 1, got: {}", output);
}

#[test]
fn test_loop_step_exact_range_last_value() {
  let code = r#"
loop over i as u32 from 1 to 10 step 3
    write i
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // Step 3: 1, 4, 7, 10 — 10 is within range (inclusive)
  // After 10: 10+3=13 > 10 → exit
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    4,
    "Expected 4 iterations (1,4,7,10), got {}: {:?}",
    lines.len(),
    lines
  );
  assert!(
    output.contains("10"),
    "Should include 10 (inclusive bound), got: {}",
    output
  );
}

// ==================== LOOP WITH EXPRESSIONS USING LOOP VARIABLE ====================

#[test]
fn test_loop_variable_in_expression() {
  let code = r#"
var v as f64 = 2.0
loop over i as u32 from 1 to 3
    var vel as f64 = v * (i as f64)
    write vel
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // i=1 → vel=2, i=2 → vel=4, i=3 → vel=6
  assert!(
    output.contains("2\n") || output.contains("2.0"),
    "Should print 2, got: {}",
    output
  );
  assert!(
    output.contains("4\n") || output.contains("4.0"),
    "Should print 4, got: {}",
    output
  );
  assert!(
    output.contains("6\n") || output.contains("6.0"),
    "Should print 6, got: {}",
    output
  );
}

#[test]
fn test_loop_variable_u32_to_f64_conversion() {
  let code = r#"
loop over i as u32 from 1 to 3
    write i as f64
end loop
"#;
  let output = run_interpreter(code).unwrap();
  // u32 to f64 conversion should produce correct float values
  let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines.len(),
    3,
    "Expected 3 iterations, got {}: {:?}",
    lines.len(),
    lines
  );
  // None of the lines should be "0" or "0.0"
  let has_zero = lines.iter().any(|l| *l == "0" || *l == "0.0");
  assert!(!has_zero, "Should not print 0 or 0.0, got: {}", output);
}
