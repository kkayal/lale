//! Integration test for the platform-neutral `std.file_io` facade.
//!
//! Exercises the public API (`openFile` / `writeFile` / `closeFile` /
//! `readFile`) so the per-platform wrapper and the platform module are both
//! covered on the host that runs the test.

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
fn facade_round_trip() {
  let dir = std::env::temp_dir();
  let file = dir.join("lale_file_io_facade_test.txt");
  let _ = std::fs::remove_file(&file);
  let path = file.display().to_string();

  let source = concat!(
    "use all from std.full\n",
    "var path as text = \"@@PATH@@\"\n",
    "var fd_opt as i32? = openFile(path, \"w\")\n",
    "when fd_opt has no value\n",
    "    write \"open failed\"\n",
    "    exit program 1\n",
    "end when\n",
    "var fd as i32 = value of fd_opt\n",
    "var wrote as i64 = writeFile(fd, \"hello facade\")\n",
    "var closed as i32 = closeFile(fd)\n",
    "var rd_opt as i32? = openFile(path, \"r\")\n",
    "when rd_opt has no value\n",
    "    write \"reopen failed\"\n",
    "    exit program 1\n",
    "end when\n",
    "var rfd as i32 = value of rd_opt\n",
    "var content as text = readFile(rfd, 64 as i64)\n",
    "var closed2 as i32 = closeFile(rfd)\n",
    "write content\n"
  )
  .replace("@@PATH@@", &path);

  let (stdout, stderr, success) = run_lale(&source);
  let _ = std::fs::remove_file(&file);

  assert!(success, "file I/O through the facade failed: {stderr}");
  assert!(
    stdout.contains("hello facade"),
    "read-back mismatch. stdout: {stdout}"
  );
}
