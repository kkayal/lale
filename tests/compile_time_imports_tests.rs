//! Integration tests for `use` statements inside compile-time conditional blocks.
//!
//! Dependency discovery runs before compile-time conditions are evaluated, so it
//! scans every branch. The analyzer then visits only the taken branch. These
//! tests pin both halves of that contract.

use std::io::Write;
use std::process::{Command, Stdio};

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

#[test]
fn guarded_import_in_taken_branch_resolves() {
  // `abs` is used inside the branch, so this fails if the import is dropped.
  let (stdout, stderr, success) = run_lale(
    "#if #posix\n\
     use abs from std.full\n\
     write \"value {abs(-5)}\"\n\
     #else\n\
     write \"other platform\"\n\
     #end if\n",
  );
  assert!(success, "guarded import should compile: {stderr}");
  if cfg!(windows) {
    assert!(stdout.contains("other platform"), "stdout: {stdout}");
  } else {
    assert!(stdout.contains("value 5"), "stdout: {stdout}");
  }
}

#[test]
fn guarded_import_in_untaken_branch_is_ignored() {
  let (stdout, stderr, success) = run_lale(
    "#if #windows\n\
     use abs from std.full\n\
     write \"value {abs(-5)}\"\n\
     #else\n\
     write \"not windows\"\n\
     #end if\n",
  );
  assert!(success, "untaken branch must not break the build: {stderr}");
  if cfg!(windows) {
    assert!(stdout.contains("value 5"), "stdout: {stdout}");
  } else {
    assert!(stdout.contains("not windows"), "stdout: {stdout}");
  }
}

#[test]
fn guarded_import_in_else_branch_resolves() {
  let (stdout, stderr, success) = run_lale(
    "#if #windows\n\
     move on\n\
     #else\n\
     use abs from std.full\n\
     write \"value {abs(-7)}\"\n\
     #end if\n",
  );
  assert!(
    success,
    "guarded import in `#else` should compile: {stderr}"
  );
  if cfg!(windows) {
    assert!(stdout.trim().is_empty(), "stdout: {stdout}");
  } else {
    assert!(stdout.contains("value 7"), "stdout: {stdout}");
  }
}

#[test]
fn only_taken_branch_signature_is_registered() {
  if cfg!(windows) {
    return;
  }
  // Before the fix, both definitions registered and the `#else` one won,
  // leaving `sample()` typed as i32 and failing the assignment below.
  let (stdout, stderr, success) = run_lale(
    "#if #posix\n\
     fn sample() returns u64 in <ns>\n\
         var t as u64 in <ns> = 1000000000 as u64\n\
         return t\n\
     end fn\n\
     #else\n\
     fn sample() returns i32\n\
         return 5 as i32\n\
     end fn\n\
     #end if\n\
     var x as u64 in <ns> = sample()\n\
     write \"signature ok\"\n",
  );
  assert!(success, "the taken branch's signature must win: {stderr}");
  assert!(stdout.contains("signature ok"), "stdout: {stdout}");
}
