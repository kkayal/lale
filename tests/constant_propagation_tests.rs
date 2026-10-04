// =============================================================================
// IR constant propagation
// =============================================================================
// The IR generator folds uses of *effectively constant* globals (never reassigned,
// never shared) to IR constants instead of loads. This is a sound optimization:
// a reassigned or shared global must NOT be folded at earlier use sites.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program through the interpreter binary, feeding `code` on stdin.
fn run_lale(code: &str, extra_args: &[&str]) -> Result<String, String> {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run"])
    .args(extra_args)
    .arg("-")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| format!("Failed to spawn lale: {}", e))?;

  let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
  stdin
    .write_all(code.as_bytes())
    .map_err(|e| format!("Failed to write to stdin: {}", e))?;
  drop(stdin);

  let output = child
    .wait_with_output()
    .map_err(|e| format!("Failed to wait on child: {}", e))?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).to_string())
  }
}

#[test]
fn test_global_constant_is_folded_to_correct_output() {
  // `pi` is an effectively-constant global; folding it must still compute `area`
  // correctly.
  let code = r#"
var pi as f64 = 3.14
var area as f64 = pi * 2.0
write area
"#;
  let output = run_lale(code, &[]).expect("program should run");
  assert!(output.contains("6.28"), "expected 6.28, got: {}", output);
}

#[test]
fn test_reassigned_global_is_not_wrongly_propagated() {
  // `x` is reassigned, so it is NOT effectively constant. The first use must
  // read the value at that point (5), not the final value (20).
  let code = r#"
var x as i32 = 5
var first as i32 = x * 10
x = 20
var second as i32 = x * 10
write first
write second
"#;
  let output = run_lale(code, &[]).expect("program should run");
  assert!(output.contains("50"), "first should be 50, got: {}", output);
  assert!(
    output.contains("200"),
    "second should be 200, got: {}",
    output
  );
}
