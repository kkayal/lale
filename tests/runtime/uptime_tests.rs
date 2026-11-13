//! Integration tests for the `__lale_read_uptime()` builtin.
//!
//! `__lale_read_uptime()` returns nanoseconds since boot as a plain `u64 in <ns>`.
//! It is declared in `builtins.lale` (the `import fn` hook) and implemented in
//! `core.lale` (the `export fn` wrapper around `clock_gettime(CLOCK_BOOTTIME)`).

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
fn test_uptime_returns_non_negative_and_monotonic() {
  let (stdout, stderr, success) = run_lale(
    "use all from std.full\n\
     var a as u64 in <ns> = __lale_read_uptime()\n\
     var b as u64 in <ns> = __lale_read_uptime()\n\
     write a\n\
     write b\n",
  );
  assert!(success, "uptime program should run: {stderr}");

  let values: Vec<u64> = stdout
    .lines()
    .filter_map(|line| line.trim().parse::<u64>().ok())
    .collect();
  assert_eq!(
    values.len(),
    2,
    "expected two uptime readings, got: {stdout}"
  );
  assert!(values[0] <= values[1], "uptime must be non-decreasing");
}

#[test]
fn test_uptime_unit_mismatch_rejected() {
  let (_stdout, stderr, success) = run_lale(
    "use all from std.full\n\
     var bad as u64 in <m> = __lale_read_uptime()\n",
  );
  assert!(!success, "unit mismatch should be rejected");
  assert!(
    stderr.to_lowercase().contains("unit"),
    "error should mention the unit mismatch, got: {stderr}"
  );
}

#[test]
fn test_uptime_guarded_subtraction() {
  // The canonical interval-measurement pattern: guard the subtraction so the
  // compiler accepts the unsigned difference.
  let (stdout, stderr, success) = run_lale(
    "use all from std.full\n\
     var start as u64 in <ns> = __lale_read_uptime()\n\
     var end as u64 in <ns> = __lale_read_uptime()\n\
     when end >= start\n\
         write end - start\n\
     end when\n",
  );
  assert!(
    success,
    "guarded uptime subtraction should compile and run: {stderr}"
  );
  assert!(
    stdout.trim().parse::<u64>().is_ok(),
    "output should be a non-negative integer, got: {stdout}"
  );
}

// NOTE: One open finding surfaced while writing these tests. It is not covered by
// a `#[test]` here because it needs a decision first (see the implementation
// report), but it is recorded so it is not lost:
//
// In no-stdlib mode (`--stdlib-level none`), builtins `import fn` hooks are not
// visible to user code ("Function '...' is not defined") — pre-existing
// behaviour, identical to `__lale_malloc`. The proposal's "compiles without the
// standard library" acceptance criterion needs clarification.
