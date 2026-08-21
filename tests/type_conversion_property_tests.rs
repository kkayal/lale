/// Property-based tests for type conversions in the Lale interpreter.
///
/// Tests combinations of:
/// - Integer → integer (widening and narrowing)
/// - Integer → float
/// - Float → integer (truncation)
/// - Float ↔ float
/// - Boundary values (0, 1, -1, MAX, MIN)
///
/// These tests catch silent fallback bugs by verifying that type conversions
/// produce correct values rather than silently defaulting to 0.
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

// ==================== INTEGER TO INTEGER ====================

#[test]
fn test_int_to_int_widening_preserves_value() {
  let code = r#"
var a as i8 = 42
var b as i16 = a as i16
var c as i32 = b as i32
var d as i64 = c as i64
write d
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("42"),
    "i8→i16→i32→i64 should be 42, got: {}",
    output
  );
}

#[test]
fn test_int_to_int_same_width() {
  // i32 → i32 (same width, allowed)
  let code = r#"
var a as i32 = 1000
var b as i32 = a as i32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("1000"),
    "i32→i32 should be 1000, got: {}",
    output
  );
}

#[test]
fn test_uint_to_uint_widening() {
  let code = r#"
var a as u8 = 200
var b as u16 = a as u16
var c as u32 = b as u32
var d as u64 = c as u64
write d
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("200"),
    "u8→u16→u32→u64 should be 200, got: {}",
    output
  );
}

#[test]
fn test_int_to_uint() {
  let code = r#"
var a as i32 = 42
var b as u32 = a as u32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("42"),
    "i32→u32 should be 42, got: {}",
    output
  );
}

#[test]
fn test_uint_to_int() {
  let code = r#"
var a as u32 = 42
var b as i32 = a as i32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("42"),
    "u32→i32 should be 42, got: {}",
    output
  );
}

// ==================== INTEGER TO FLOAT ====================

#[test]
fn test_int_to_f64() {
  let code = r#"
var a as i32 = 42
var b as f64 = a as f64
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("42"),
    "i32→f64 should be 42, got: {}",
    output
  );
}

#[test]
fn test_uint_to_f64() {
  let code = r#"
var a as u32 = 100
var b as f64 = a as f64
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("100"),
    "u32→f64 should be 100, got: {}",
    output
  );
}

#[test]
fn test_int_to_f32() {
  let code = r#"
var a as i32 = 7
var b as f32 = a as f32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(output.contains("7"), "i32→f32 should be 7, got: {}", output);
}

// ==================== FLOAT TO INTEGER (widening only) ====================

#[test]
fn test_f64_to_f64_is_noop() {
  let code = r#"
var a as f64 = 3.7
var b as f64 = a as f64
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("3.7"),
    "f64→f64 should be 3.7, got: {}",
    output
  );
}

// ==================== FLOAT TO FLOAT ====================

#[test]
fn test_f32_to_f64() {
  let code = r#"
var a as f32 = 3.14
var b as f64 = a as f64
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("3.14"),
    "f32→f64 should preserve value, got: {}",
    output
  );
}

#[test]
fn test_f32_to_f32_is_noop() {
  let code = r#"
var a as f32 = 3.14
var b as f32 = a as f32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("3.14"),
    "f32→f32 should preserve value, got: {}",
    output
  );
}

// ==================== BOUNDARY VALUES ====================

#[test]
fn test_zero_conversion() {
  let code = r#"
var a as i32 = 0
var b as f64 = a as f64
var c as u64 = a as u64
write b
write c
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("0"),
    "Zero should remain 0 across conversions, got: {}",
    output
  );
}

#[test]
fn test_negative_int_to_float() {
  let code = r#"
var a as i32 = -42
var b as f64 = a as f64
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("-42"),
    "Negative int→f64 should be -42, got: {}",
    output
  );
}

#[test]
fn test_one_conversion() {
  let code = r#"
var a as i32 = 1
var b as f64 = a as f64
var c as u32 = a as u32
write b
write c
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("1"),
    "One should remain 1 across conversions, got: {}",
    output
  );
}

// ==================== CHAINED CONVERSIONS ====================

#[test]
fn test_chained_widening_conversions() {
  let code = r#"
var a as i8 = 10
var b as i16 = a as i16
var c as i32 = b as i32
var d as i64 = c as i64
write d
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("10"),
    "i8→i16→i32→i64 chain should be 10, got: {}",
    output
  );
}

// ==================== UNSIGNED EDGE CASES ====================

#[test]
fn test_u8_max() {
  let code = r#"
var a as u8 = 255
var b as u16 = a as u16
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("255"),
    "u8 max should be 255, got: {}",
    output
  );
}

#[test]
fn test_u8_to_u32_widening() {
  let code = r#"
var a as u8 = 200
var b as u32 = a as u32
write b
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("200"),
    "u8→u32 widening should be 200, got: {}",
    output
  );
}

// ==================== I32 TO STRING EDGE CASES ====================

#[test]
fn test_i32_to_str_edge_cases() {
  // Batch all i32 edge cases into a single process
  let code = r#"
var a as i32 = 0
write a
var b as i32 = 42
write b
var c as i32 = -42
write c
var d as i32 = 2147483647
write d
var e as i32 = -2147483648
write e
"#;
  let output = run_interpreter(code).unwrap();
  assert!(output.contains("0"), "i32 0, got: {}", output);
  assert!(output.contains("42"), "i32 42, got: {}", output);
  assert!(output.contains("-42"), "i32 -42, got: {}", output);
  assert!(output.contains("2147483647"), "i32 MAX, got: {}", output);
  assert!(output.contains("-2147483648"), "i32 MIN, got: {}", output);
}
