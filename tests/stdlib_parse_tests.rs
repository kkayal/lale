//! Tests for the availability of the stdlib `parse_*` functions.
//!
//! `parse_float`, `parse_int`, and `parse_uint` live in the `core` module
//! (`stdlib/src/core.lale`), not in `builtins.lale`. They are therefore **not**
//! implicitly available: a program must import them (e.g. `use all from
//! std.core`, or `use parse_float, parse_int, parse_uint from std.core`) before
//! calling them. This mirrors `doc/stdlib.md` and `doc/lale.md`.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale source string through the CLI and return (success, stdout, stderr).
fn run_source(source: &str) -> (bool, String, String) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .args(["run", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("Failed to spawn lale");

  child
    .stdin
    .as_mut()
    .expect("Failed to open stdin")
    .write_all(source.as_bytes())
    .expect("Failed to write to stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");
  (
    output.status.success(),
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
  )
}

#[test]
fn test_parse_functions_require_import() {
  // Without an import, `parse_float` is not in scope and compilation fails.
  let source = "var val as f64? = parse_float(\"42.5\")\nwrite \"ok\"\n";
  let (success, _stdout, stderr) = run_source(source);

  assert!(
    !success,
    "parse_float must require an import; expected failure but compilation succeeded"
  );
  assert!(
    stderr.contains("not defined"),
    "Error should report an undefined function, got: {}",
    stderr
  );
}

#[test]
fn test_parse_functions_available_via_core_import() {
  // `use all from std.core` brings all three parse functions into scope.
  let source = r#"use all from std.core
var a as f64? = parse_float("42.5")
var b as i64? = parse_int("-17")
var c as u64? = parse_uint("99")
when a has value
    write value of a
end when
when b has value
    write value of b
end when
when c has value
    write value of c
end when
"#;

  let (success, stdout, stderr) = run_source(source);

  assert!(
    success,
    "parse functions should work with `use all from std.core`. stderr: {}",
    stderr
  );
  assert_eq!(stdout, "42.5\n-17\n99\n", "unexpected parse output");
}

#[test]
fn test_parse_functions_available_via_full_import() {
  // `std.full` re-exports `std.core`, so `use all from std.full` also works.
  let source = r#"use all from std.full
var a as f64? = parse_float("3.25")
when a has value
    write value of a
end when
"#;

  let (success, stdout, stderr) = run_source(source);

  assert!(
    success,
    "parse_float should work with `use all from std.full`. stderr: {}",
    stderr
  );
  assert_eq!(stdout, "3.25\n", "unexpected parse output");
}
