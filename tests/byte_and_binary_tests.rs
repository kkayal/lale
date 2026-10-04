//! Integration tests for the `byte` and `binary` types.
//!
//! `byte` is a raw octet (not a number): hex literals, bitwise operations, and
//! explicit conversions to/from integers. `binary` is a compiler-managed byte
//! buffer ({ptr, bytes}) used for binary I/O.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program supplied on stdin and return (stdout, stderr, success).
fn run_lale(code: &str) -> (String, String, bool) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-", "--no-color"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  {
    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    stdin
      .write_all(code.as_bytes())
      .expect("Failed to write to stdin");
  }

  let output = child.wait_with_output().expect("Failed to wait on child");
  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status.success(),
  )
}

// ── byte: literals, display, range ────────────────────────────────

#[test]
fn test_write_bare_integer_is_semantic_error_not_ice() {
  let (stdout, stderr, success) = run_lale("write 5\n");
  assert!(!success, "bare literal write should fail");
  assert!(
    stderr.contains("semantic error"),
    "expected a semantic error, got: {stderr}"
  );
  assert!(
    !stderr.contains("internal compiler error"),
    "should not be an internal compiler error: {stderr}"
  );
  assert!(stdout.is_empty(), "no output expected, got: {stdout}");
}

#[test]
fn test_write_explicitly_typed_integer_succeeds() {
  let (stdout, stderr, success) = run_lale("write (5 as u32)\n");
  assert!(success, "explicitly typed literal should succeed: {stderr}");
  assert_eq!(stdout, "5\n");
}

#[test]
fn test_byte_hex_literal_and_display() {
  let (stdout, stderr, success) = run_lale("var a as byte = 0x50\nwrite a\n");
  assert!(success, "byte literal should compile: {stderr}");
  assert_eq!(stdout, "0x50\n");
}

#[test]
fn test_byte_display_two_digits_uppercase() {
  let (stdout, stderr, success) = run_lale("write (0x0a as byte)\n");
  assert!(success, "byte display should compile: {stderr}");
  assert_eq!(stdout, "0x0A\n");
}

#[test]
fn test_byte_range_validation_rejects_overflow() {
  let (_, stderr, success) = run_lale("var a as byte = 0x100\n");
  assert!(!success, "byte out-of-range literal should fail");
  assert!(
    stderr.contains("out of range"),
    "expected range error: {stderr}"
  );
}

// ── byte: operators ───────────────────────────────────────────────

#[test]
fn test_byte_arithmetic_rejected() {
  let (_, stderr, success) =
    run_lale("var a as byte = 0x01\nvar b as byte = 0x02\nvar c as byte = a + b\n");
  assert!(!success, "byte arithmetic should fail");
  assert!(
    stderr.contains("octet, not a number"),
    "expected byte arithmetic error: {stderr}"
  );
}

#[test]
fn test_byte_ordering_rejected() {
  let (_, stderr, success) =
    run_lale("var a as byte = 0x01\nvar b as byte = 0x02\nvar c as bool = a < b\n");
  assert!(!success, "byte ordering should fail");
  assert!(
    stderr.contains("octet, not a number"),
    "expected byte ordering error: {stderr}"
  );
}

#[test]
fn test_byte_bitwise_and_invert() {
  let (stdout, stderr, success) =
    run_lale("var a as byte = 0x0F\nvar b as byte = invert a\nwrite b\n");
  assert!(success, "byte bitwise should compile: {stderr}");
  assert_eq!(stdout, "0xF0\n");
}

#[test]
fn test_byte_equality() {
  let (stdout, stderr, success) =
    run_lale("var a as byte = 0x00\nvar b as byte = 0x00\nvar c as bool = a == b\nwrite c\n");
  assert!(success, "byte equality should compile: {stderr}");
  assert_eq!(stdout, "true\n");
}

// ── byte: conversions ─────────────────────────────────────────────

#[test]
fn test_byte_to_u8_and_back() {
  let (stdout, stderr, success) = run_lale(
    "var a as byte = 0x50\nvar n as u8 = a as u8\nvar b as byte = n as byte\nwrite n\nwrite b\n",
  );
  assert!(success, "byte conversions should compile: {stderr}");
  assert_eq!(stdout, "80\n0x50\n");
}

// ── integer bitwise operators (and/or/xor) ───────────────────────

#[test]
fn test_integer_bitwise_and_or_xor_execution() {
  // x = 3 (0b11): and 1 → 1, or 4 → 7, xor 1 → 2.
  let code = "var x as u8 = 3\n\
               var a as u8 = x bitwise and 1 as u8\n\
               var o as u8 = x bitwise or 4 as u8\n\
               var xo as u8 = x bitwise xor 1 as u8\n\
               write a\n\
               write o\n\
               write xo\n";
  let (stdout, stderr, success) = run_lale(code);
  assert!(
    success,
    "integer bitwise operators should compile: {stderr}"
  );
  assert_eq!(stdout, "1\n7\n2\n", "stderr: {stderr}");
}

// ── binary: construction, fields, I/O ─────────────────────────────

#[test]
fn test_binary_constructor_and_member_access() {
  let (stdout, stderr, success) = run_lale(
    "use all from std.full\nvar b as binary = binary(__lale_malloc(3 as u64), 3 as u64)\nwrite b.bytes\n",
  );
  assert!(success, "binary construction should compile: {stderr}");
  assert_eq!(stdout, "3\n");
}

#[test]
fn test_binary_read_bytes_write_bytes_roundtrip() {
  // Use stdlib file I/O to write a binary buffer to a file and read it back.
  let path = std::env::temp_dir().join("lale_binary_roundtrip.bin");
  let path_str = path.to_str().expect("temp path should be valid UTF-8");

  let write_code = format!(
    "use openFile, writeBytes, closeFile from std.file_io_posix\n\
     var fd as i32? = openFile(\"{path_str}\", \"w\")\n\
     var f as i32 = value of fd\n\
     var payload as binary = binary(__lale_malloc(4 as u64), 4 as u64)\n\
     unsafe value at payload.ptr = 65 as u8\n\
     var p1 as pointer = payload.ptr + 1 as u64\n\
     unsafe value at p1 = 66 as u8\n\
     var p2 as pointer = payload.ptr + 2 as u64\n\
     unsafe value at p2 = 67 as u8\n\
     var p3 as pointer = payload.ptr + 3 as u64\n\
     unsafe value at p3 = 68 as u8\n\
     var written as i64 = writeBytes(f, payload)\n\
     var c as i32 = closeFile(f)\n\
     write written\n"
  );
  let (stdout, stderr, success) = run_lale(&write_code);
  assert!(success, "writeBytes should succeed: {stderr}");
  assert_eq!(stdout, "4\n");

  let read_code = format!(
    "use openFile, readBytes, closeFile from std.file_io_posix\n\
     var fd as i32? = openFile(\"{path_str}\", \"r\")\n\
     var f as i32 = value of fd\n\
     var data as binary = readBytes(f, 4 as i64)\n\
     var p0 as pointer = data.ptr\n\
     var b0 as u8 = unsafe value at p0 unsafe bitcast\n\
     var p1 as pointer = data.ptr + 1 as u64\n\
     var b1 as u8 = unsafe value at p1 unsafe bitcast\n\
     var p2 as pointer = data.ptr + 2 as u64\n\
     var b2 as u8 = unsafe value at p2 unsafe bitcast\n\
     var p3 as pointer = data.ptr + 3 as u64\n\
     var b3 as u8 = unsafe value at p3 unsafe bitcast\n\
     write data.bytes\n\
     write b0\n\
     write b1\n\
     write b2\n\
     write b3\n\
     var c as i32 = closeFile(f)\n"
  );
  let (stdout, stderr, success) = run_lale(&read_code);
  assert!(success, "readBytes should succeed: {stderr}");
  assert_eq!(stdout, "4\n65\n66\n67\n68\n");

  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_binary_by_value_param_deep_copy() {
  // A by-value binary parameter must be deep-copied: mutating the callee's
  // copy must not affect the caller's buffer.
  let (stdout, stderr, success) = run_lale(
    "use all from std.full\n\
     fn mutate(b as binary) returns nothing\n\
       var p as pointer = b.ptr\n\
       unsafe value at p = 88 as u8\n\
     end fn\n\
     var data as binary = binary(__lale_malloc(1 as u64), 1 as u64)\n\
     unsafe value at data.ptr = 65 as u8\n\
     mutate(data)\n\
     var q as pointer = data.ptr\n\
     var v as u8 = unsafe value at q unsafe bitcast\n\
     write v\n",
  );
  assert!(success, "binary deep-copy should compile: {stderr}");
  // The caller's byte must remain 'A' (65), not the callee's 'X' (88).
  assert_eq!(stdout, "65\n");
}
