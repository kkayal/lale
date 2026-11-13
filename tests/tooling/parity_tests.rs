use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write a temporary Lale source file and return its path.
fn write_temp_source(code: &str) -> std::path::PathBuf {
  let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
  let path = std::env::temp_dir().join(format!("lale_parity_{}_{}.lale", std::process::id(), id));
  let mut file = std::fs::File::create(&path).expect("create temp source");
  file.write_all(code.as_bytes()).expect("write temp source");
  path
}

fn run_parity(path: &std::path::Path) -> (String, String, std::process::ExitStatus) {
  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .arg("parity")
    .arg(path)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("spawn lale parity");

  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status,
  )
}

#[test]
fn parity_single_backend_reports_zero_diffs() {
  let path = write_temp_source(r#"write "hello from parity""#);
  let (stdout, _stderr, status) = run_parity(&path);

  assert!(status.success(), "parity failed with stderr: {}", _stderr);
  assert!(
    stdout.contains("backend interpreter (lale run): exit code 0"),
    "stdout: {}",
    stdout
  );
  assert!(
    stdout.contains("Parity: 1 backend registered — 0 diffs."),
    "stdout: {}",
    stdout
  );

  std::fs::remove_file(&path).ok();
}

#[test]
fn parity_captures_child_exit_code() {
  // `exit program 7` goes through `__lale_exit`, so the interpreter
  // subprocess exits non-zero. The parity harness must surface that exit code
  // without dying itself.
  let path = write_temp_source("exit program 7");
  let (stdout, _stderr, status) = run_parity(&path);

  assert!(
    status.success(),
    "parity harness itself failed: {}",
    _stderr
  );
  assert!(
    stdout.contains("backend interpreter (lale run): exit code 7"),
    "stdout: {}",
    stdout
  );

  std::fs::remove_file(&path).ok();
}
