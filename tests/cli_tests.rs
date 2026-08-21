use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn test_stdin_compilation() {
  // Test reading source from stdin with "-" argument
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  let stdin = child.stdin.as_mut().expect("Failed to open stdin");
  stdin
    .write_all(b"var x as i32 = 42\nwrite \"hello\"\n")
    .expect("Failed to write to stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");

  // Should succeed (no semantic errors in "var x = 42; write hello")
  assert!(
    output.status.success(),
    "Should succeed with valid stdin input. stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

#[test]
fn test_stdin_with_error() {
  // Test that errors from stdin input show <stdin> as the file name
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  let stdin = child.stdin.as_mut().expect("Failed to open stdin");
  stdin
    .write_all(b"write undefined_variable\n")
    .expect("Failed to write to stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");

  assert!(
    !output.status.success(),
    "Should fail with undefined variable"
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("<stdin>"),
    "Error should reference <stdin>, got: {}",
    stderr
  );
  assert!(
    stderr.contains("Undefined variable"),
    "Should report undefined variable error, got: {}",
    stderr
  );
}

#[test]
fn test_no_args_piped_shows_hint() {
  // When no command is provided and stdin is piped, show usage hint
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  let stdin = child.stdin.as_mut().expect("Failed to open stdin");
  stdin
    .write_all(b"write \"hello\"\n")
    .expect("Failed to write to stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");

  assert!(
    !output.status.success(),
    "Should fail when no args provided"
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("lale run -"),
    "Should show hint about using 'lale run -', got: {}",
    stderr
  );
}

#[test]
fn test_file_with_piped_stdin_works() {
  // Test that having a file argument with piped stdin works
  // (stdin is ignored, file is read)
  let dir = tempfile::tempdir().expect("Failed to create temp dir");
  let temp_file = dir.path().join("test.lale");
  std::fs::write(&temp_file, "write \"Hello from file\"\n").expect("Failed to write temp file");

  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", temp_file.to_str().unwrap()])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  let stdin = child.stdin.as_mut().expect("Failed to open stdin");
  stdin
    .write_all(b"this should be ignored\n")
    .expect("Failed to write to stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");

  // tempdir cleans up automatically on drop, even on panic

  // Should succeed because the file is valid
  assert!(
    output.status.success(),
    "Should succeed reading from file even with piped stdin. stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

// ==================== PRIVATE FIELD TESTS ====================

#[test]
fn test_private_field_read_cross_module_rejected() {
  // Test that accessing a private field from outside the defining module
  // produces a compile error.
  let dir = tempfile::tempdir().expect("Failed to create temp dir");

  // Module 'types' defines Secure with a private key field
  let types_file = dir.path().join("types.lale");
  std::fs::write(
    &types_file,
    r#"
type Secure
    private key as i64
    name as str
end type
"#,
  )
  .expect("Failed to write types.lale");

  // Module 'main' imports Secure and tries to access the private field
  let main_file = dir.path().join("main.lale");
  std::fs::write(
    &main_file,
    r#"
use types: Secure

var s as Secure = Secure(42, "public")
var k as i64 = s.key
"#,
  )
  .expect("Failed to write main.lale");

  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", main_file.to_str().unwrap()])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("Failed to run lale");

  assert!(
    !output.status.success(),
    "Should fail when accessing private field from another module.
stdout: {}",
    String::from_utf8_lossy(&output.stdout)
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("private field"),
    "Error should mention 'private field', got: {}",
    stderr
  );
  assert!(
    stderr.contains("key"),
    "Error should mention the field name 'key', got: {}",
    stderr
  );
}

#[test]
fn test_private_field_write_cross_module_rejected() {
  // Test that assigning to a private field from outside the defining module
  // produces a compile error.
  let dir = tempfile::tempdir().expect("Failed to create temp dir");

  let types_file = dir.path().join("types.lale");
  std::fs::write(
    &types_file,
    r#"
type Secure
    private key as i64
    name as str
end type
"#,
  )
  .expect("Failed to write types.lale");

  let main_file = dir.path().join("main.lale");
  std::fs::write(
    &main_file,
    r#"
use types: Secure

var s as Secure = Secure(42, "public")
s.key = 99
"#,
  )
  .expect("Failed to write main.lale");

  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", main_file.to_str().unwrap()])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("Failed to run lale");

  assert!(
    !output.status.success(),
    "Should fail when writing to private field from another module.
stdout: {}",
    String::from_utf8_lossy(&output.stdout)
  );
  let stderr = String::from_utf8_lossy(&output.stderr);
  assert!(
    stderr.contains("private field"),
    "Error should mention 'private field', got: {}",
    stderr
  );
  assert!(
    stderr.contains("key"),
    "Error should mention the field name 'key', got: {}",
    stderr
  );
}
