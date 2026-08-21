//! Integration tests for write statements with type conversions
//! These tests verify that write statements correctly convert values to strings
//! before passing them to the POSIX write() extern (fd=1 stdout, fd=2 stderr).
#![allow(clippy::assertions_on_constants)]

#[cfg(test)]
mod write_output_tests {
  use std::fs;
  use std::process::Command;

  fn get_ir(code: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let temp_file = dir.path().join("test.lale");
    fs::write(&temp_file, code).expect("Failed to write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_lale"))
      .current_dir(env!("CARGO_MANIFEST_DIR"))
      .args([
        "run",
        temp_file.to_str().unwrap(),
        "--print-ir",
        "--no-color",
      ])
      .output()
      .expect("Failed to run lale compiler");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // tempdir auto-cleans up

    // Combine stdout and stderr to capture the IR before build failure
    stdout + &stderr
  }

  #[test]
  fn test_write_float_generates_f64_to_str_call() {
    let code = r#"
var x as f64 = 42.5
write x
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_f64_to_str_f64"),
      "IR should contain __lale_f64_to_str call for float write"
    );
    assert!(
      ir.contains("call @write"),
      "IR should contain write call after conversion"
    );
  }

  #[test]
  fn test_write_integer_generates_i64_to_str_call() {
    let code = r#"
var x as i64 = 42
write x
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_i64_to_str_i64"),
      "IR should contain __lale_i64_to_str call for i64 write"
    );
    assert!(
      ir.contains("call @write"),
      "IR should contain write call after conversion"
    );
  }

  #[test]
  fn test_write_uint_generates_u64_to_str_call() {
    let code = r#"
var x as u64 = 42
write x
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_u64_to_str_u64"),
      "IR should contain __lale_u64_to_str call for u64 write"
    );
    assert!(
      ir.contains("call @write"),
      "IR should contain write call after conversion"
    );
  }

  #[test]
  fn test_write_bool_generates_bool_to_str_call() {
    let code = r#"
var x as bool = true
write x
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_bool_to_str_bool"),
      "IR should contain __lale_bool_to_str call for bool write"
    );
    assert!(
      ir.contains("call @write"),
      "IR should contain write call after conversion"
    );
  }

  #[test]
  fn test_write_string_no_conversion() {
    let code = r#"
var x as str = "hello"
write x
        "#;
    let ir = get_ir(code);
    assert!(ir.contains("call @write"), "IR should contain write call");

    // Find the main function and check it doesn't call conversion functions
    if let Some(main_start) = ir.find("export func @main") {
      // Find the end of main - look for closing brace followed by newline and "func @"
      let rest = &ir[main_start..];
      // Main ends at the next function definition (stdlib helpers come after main)
      let main_end = rest.find("\n}\n").map(|i| i + 2).unwrap_or(rest.len());
      let main_body = &rest[..main_end];
      assert!(
        !main_body.contains("call @__lale_i64_to_str_i64"),
        "Should not convert string to i64_to_str"
      );
      assert!(
        !main_body.contains("call @__lale_u64_to_str_u64"),
        "Should not convert string to u64_to_str"
      );
      assert!(
        !main_body.contains("call @__lale_f64_to_str_f64"),
        "Should not convert string to f64_to_str"
      );
    }
  }

  #[test]
  fn test_write_function_result_converts() {
    let code = r#"
fn get_value() returns f64
    return 3.14
end fn

var result as f64 = get_value()
write result
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_f64_to_str_f64"),
      "Should convert f64 function result to string"
    );
    assert!(
      ir.contains("call @write"),
      "Should call write after conversion"
    );
  }

  #[test]
  fn test_warn_generates_put_str_err_call() {
    let code = r#"
var x as i64 = 42
warn x
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @__lale_i64_to_str_i64"),
      "IR should contain __lale_i64_to_str call for warn"
    );
    assert!(
      ir.contains("call @write"),
      "IR should contain write call for stderr"
    );
  }

  #[test]
  fn test_warn_inline_generates_put_str_err_call() {
    let code = r#"
var msg as str = "error message"
warn inline msg
        "#;
    let ir = get_ir(code);
    assert!(
      ir.contains("call @write"),
      "IR should contain write call for warn inline"
    );
  }
}
