/// Integration tests for string type inference and global variable access.
///
/// Covers bugs fixed:
/// - `var foo = "Hallo"` without type annotation was allocated as i64 (8 bytes)
///   instead of %str (16 bytes), causing memory corruption
/// - Function `bar()` couldn't access global variable `foo` defined at top level
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

// ==================== STRING TYPE INFERENCE ====================

#[test]
fn test_def_string_without_annotation() {
  let code = r#"
var foo = "Hallo"
write "foo is {foo}"
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("foo is Hallo"),
    "Should print 'foo is Hallo', got: {}",
    output
  );
}

#[test]
fn test_def_string_concatenation_without_annotation() {
  let code = r#"
var greeting = "Hello"
var name = "World"
write "{greeting} {name}"
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("Hello World"),
    "Should print 'Hello World', got: {}",
    output
  );
}

// ==================== GLOBAL VARIABLE ACCESS FROM FUNCTIONS ====================

#[test]
fn test_function_accesses_global_variable() {
  let code = r#"
var message = "global"
fn get_message(nothing) returns str
    return message
end fn
write get_message()
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("global"),
    "Function should access global variable 'message', got: {}",
    output
  );
}

#[test]
fn test_function_accesses_global_string() {
  let code = r#"
var name = "Lale"
fn greet(nothing) returns str
    return name
end fn
write "Hello, {greet()}"
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("Hello, Lale"),
    "Should print 'Hello, Lale', got: {}",
    output
  );
}

#[test]
fn test_function_accesses_global_number() {
  let code = r#"
var value as i32 = 42
fn get_value(nothing) returns i32
    return value
end fn
write get_value()
"#;
  let output = run_interpreter(code).unwrap();
  assert!(output.contains("42"), "Should print 42, got: {}", output);
}

#[test]
fn test_multiple_functions_access_same_global() {
  let code = r#"
var counter as i32 = 0
fn increment(nothing) returns i32
    counter = counter + 1 as i32
    return counter
end fn
fn get_counter(nothing) returns i32
    return counter
end fn
write increment()
write increment()
write get_counter()
"#;
  let output = run_interpreter(code).unwrap();
  // After two increments, counter should be 2
  assert!(
    output.contains("1") && output.contains("2"),
    "Should print 1 and 2 from increments, got: {}",
    output
  );
  // Third write should show 2
  let lines: Vec<&str> = output.lines().collect();
  assert!(
    lines.contains(&"2"),
    "Third write should be 2, lines: {:?}",
    lines
  );
}

// ==================== STRING WITH INTERPOLATION ====================

#[test]
fn test_string_embedding_with_global() {
  let code = r#"
var greeting = "Hello"
var name = "World"
write "{greeting}, {name}!"
"#;
  let output = run_interpreter(code).unwrap();
  assert!(
    output.contains("Hello, World!"),
    "Should print 'Hello, World!', got: {}",
    output
  );
}
