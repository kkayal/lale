// =============================================================================
// Logging and diagnostic output
// =============================================================================
// Tests for the consolidated logging statements (`log` / `warn` / `alert`),
// the runtime log-level filter (`LALE_LOG_LEVEL`), short-circuit evaluation,
// triple-quoted strings, and the removed `to var` / `inline` / error-stack
// drain operators.
//
// All output assertions use `--no-color` so ANSI escape codes are stripped.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run a Lale program via `lale run -`, returning `(stdout, stderr, success)`.
/// `env` entries are passed to the child process (e.g. `LALE_LOG_LEVEL`).
fn run_lale(code: &str, env: &[(&str, &str)], extra_args: &[&str]) -> (String, String, bool) {
  let mut cmd = Command::new(env!("CARGO_BIN_EXE_lale"));
  cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
  cmd.args(["run"]);
  cmd.args(extra_args);
  cmd.arg("-");
  cmd.stdin(Stdio::piped());
  cmd.stdout(Stdio::piped());
  cmd.stderr(Stdio::piped());
  for (key, value) in env {
    cmd.env(key, value);
  }

  let mut child = cmd.spawn().expect("spawn lale");
  child
    .stdin
    .take()
    .expect("stdin")
    .write_all(code.as_bytes())
    .expect("write stdin");
  let output = child.wait_with_output().expect("wait");

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  (stdout, stderr, output.status.success())
}

/// Check that a string is an ISO-8601 UTC timestamp of the form
/// `YYYY-MM-DDTHH:MM:SSZ` (as produced by `__lale_timestamp`).
fn is_iso8601_utc(s: &str) -> bool {
  let b = s.as_bytes();
  b.len() == 20
    && b.get(4) == Some(&b'-')
    && b.get(7) == Some(&b'-')
    && b.get(10) == Some(&b'T')
    && b.get(13) == Some(&b':')
    && b.get(16) == Some(&b':')
    && s.ends_with('Z')
}

// ==================== OUTPUT FORMAT ====================

#[test]
fn test_log_warn_alert_format_tab_separated() {
  // Release mode omits the file/line/function context, leaving
  // `<Level>\t<UTC>\t<abs s>\t<delta µs>\t<message>` per line.
  let code = "log \"hello\"\nwarn \"careful\"\nalert \"danger\"\n";
  let (stdout, stderr, ok) = run_lale(code, &[], &["--release", "--no-color"]);
  assert!(ok, "stderr: {stderr}");
  assert!(stdout.is_empty(), "log/warn/alert go to stderr, not stdout");

  let lines: Vec<&str> = stderr.lines().collect();
  assert_eq!(lines.len(), 3, "one line per statement, stderr: {stderr}");

  let expected: [(&str, &str); 3] = [("Log", "hello"), ("Warn", "careful"), ("Alert", "danger")];
  for (line, (level, msg)) in lines.iter().zip(expected.iter()) {
    let cols: Vec<&str> = line.split('\t').collect();
    assert_eq!(cols.len(), 5, "expected 5 columns, got: {line}");
    assert_eq!(cols[0], *level, "level column: {line}");
    assert!(
      is_iso8601_utc(cols[1]),
      "UTC column should be ISO-8601: {line}"
    );
    assert!(
      cols[2].parse::<f64>().is_ok(),
      "absolute-uptime column should be a float seconds value: {line}"
    );
    assert!(
      cols[3].parse::<i64>().is_ok(),
      "delta column should be an integer microsecond count: {line}"
    );
    assert_eq!(cols[4], *msg, "message column: {line}");
  }
}

#[test]
fn test_log_uptime_monotonic_and_delta_non_negative() {
  // Column 2 is the absolute uptime in seconds (monotonic non-decreasing);
  // column 3 is the delta in microseconds (non-negative single-threaded, and 0
  // for the first log).
  let code = "log \"a\"\nlog \"b\"\nlog \"c\"\n";
  let (_stdout, stderr, ok) = run_lale(code, &[], &["--release", "--no-color"]);
  assert!(ok, "stderr: {stderr}");

  let cols: Vec<Vec<&str>> = stderr.lines().map(|l| l.split('\t').collect()).collect();
  assert_eq!(cols.len(), 3, "stderr: {stderr}");

  let abs: Vec<f64> = cols
    .iter()
    .filter_map(|c| c[2].parse::<f64>().ok())
    .collect();
  assert_eq!(abs.len(), 3, "stderr: {stderr}");
  assert!(
    abs.windows(2).all(|w| w[0] <= w[1]),
    "absolute uptime must be non-decreasing: {abs:?}"
  );

  let deltas: Vec<i64> = cols
    .iter()
    .filter_map(|c| c[3].parse::<i64>().ok())
    .collect();
  assert_eq!(deltas.len(), 3, "stderr: {stderr}");
  assert_eq!(deltas[0], 0, "first log's delta must be 0: {deltas:?}");
  assert!(
    deltas.iter().all(|&d| d >= 0),
    "delta must be non-negative single-threaded: {deltas:?}"
  );
}

#[test]
fn test_write_goes_to_stdout_not_stderr() {
  let code = "write \"result\"\n";
  let (stdout, stderr, ok) = run_lale(code, &[], &["--release", "--no-color"]);
  assert!(ok);
  assert_eq!(stdout, "result\n");
  assert!(stderr.is_empty(), "write must not emit to stderr: {stderr}");
}

// ==================== LOG LEVEL FILTERING ====================

#[test]
fn test_log_level_default_emits_all() {
  let code = "log \"L\"\nwarn \"W\"\nalert \"A\"\n";
  let (_stdout, stderr, ok) = run_lale(code, &[], &["--release", "--no-color"]);
  assert!(ok);
  assert!(
    stderr.contains("L"),
    "default level should emit log: {stderr}"
  );
  assert!(
    stderr.contains("W"),
    "default level should emit warn: {stderr}"
  );
  assert!(
    stderr.contains("A"),
    "default level should emit alert: {stderr}"
  );
}

#[test]
fn test_log_level_warn_suppresses_log() {
  let code = "log \"L\"\nwarn \"W\"\nalert \"A\"\n";
  let (_stdout, stderr, ok) = run_lale(
    code,
    &[("LALE_LOG_LEVEL", "warn")],
    &["--release", "--no-color"],
  );
  assert!(ok);
  assert!(
    !stderr.contains("L"),
    "warn level should suppress log: {stderr}"
  );
  assert!(
    stderr.contains("W"),
    "warn level should emit warn: {stderr}"
  );
  assert!(
    stderr.contains("A"),
    "warn level should emit alert: {stderr}"
  );
}

#[test]
fn test_log_level_alert_suppresses_log_and_warn() {
  let code = "log \"L\"\nwarn \"W\"\nalert \"A\"\n";
  let (_stdout, stderr, ok) = run_lale(
    code,
    &[("LALE_LOG_LEVEL", "alert")],
    &["--release", "--no-color"],
  );
  assert!(ok);
  assert!(
    !stderr.contains("L"),
    "alert level should suppress log: {stderr}"
  );
  assert!(
    !stderr.contains("W"),
    "alert level should suppress warn: {stderr}"
  );
  assert!(
    stderr.contains("A"),
    "alert level should emit alert: {stderr}"
  );
}

#[test]
fn test_log_level_off_suppresses_all() {
  let code = "log \"L\"\nwarn \"W\"\nalert \"A\"\nwrite \"done\"\n";
  let (stdout, stderr, ok) = run_lale(
    code,
    &[("LALE_LOG_LEVEL", "off")],
    &["--release", "--no-color"],
  );
  assert!(ok);
  assert!(
    stderr.is_empty(),
    "off should suppress all diagnostics: {stderr}"
  );
  assert_eq!(stdout, "done\n", "off must not suppress write (stdout)");
}

#[test]
fn test_log_level_invalid_aborts() {
  let code = "log \"L\"\n";
  let (_stdout, stderr, ok) = run_lale(code, &[("LALE_LOG_LEVEL", "bogus")], &[]);
  assert!(!ok, "invalid LALE_LOG_LEVEL should abort");
  assert!(
    stderr.contains("invalid LALE_LOG_LEVEL value 'bogus'"),
    "stderr: {stderr}"
  );
}

// ==================== SHORT-CIRCUIT EVALUATION ====================

#[test]
fn test_suppressed_statement_does_not_evaluate_message() {
  // `noise()` writes "SIDE EFFECT" and returns a string. At level `off`, the
  // log statement must not call `noise()` at all.
  let code = r#"
fn noise() returns text
    write "SIDE EFFECT"
    return "noisy"
end fn
log noise()
write "after"
"#;
  let (stdout, _stderr, ok) = run_lale(code, &[("LALE_LOG_LEVEL", "off")], &["--no-color"]);
  assert!(ok);
  assert_eq!(
    stdout, "after\n",
    "suppressed log must not call noise(); got: {stdout}"
  );
}

// ==================== DEBUG IS NOT GATED ====================

#[test]
fn test_debug_ignores_log_level() {
  // `debug` is compile-time gated (debug builds only), never runtime gated.
  let code = "var x as i32 = 5\ndebug x\nlog \"L\"\n";
  let (_stdout, stderr, ok) = run_lale(code, &[("LALE_LOG_LEVEL", "off")], &["--no-color"]);
  assert!(ok);
  assert!(
    stderr.contains("DEBUG: x = 5"),
    "debug must ignore LALE_LOG_LEVEL: {stderr}"
  );
  assert!(
    !stderr.contains("L"),
    "log should still be suppressed: {stderr}"
  );
}

// ==================== TRIPLE-QUOTED STRINGS ====================

#[test]
fn test_triple_quoted_multiline_with_embedded_values() {
  let code = r#"
var date as text = "2026-08-30"
var count as i32 = 3
var report as text = """
Report for {date}
   Items processed: {count}
"""
write report
"#;
  let (stdout, _stderr, ok) = run_lale(code, &[], &["--no-color"]);
  assert!(ok);
  assert_eq!(
    stdout, "\nReport for 2026-08-30\n   Items processed: 3\n\n",
    "stdout: {stdout}"
  );
}

#[test]
fn test_triple_quoted_single_line() {
  let code = "write \"\"\"hello world\"\"\"\n";
  let (stdout, _stderr, ok) = run_lale(code, &[], &["--no-color"]);
  assert!(ok);
  assert_eq!(stdout, "hello world\n");
}

#[test]
fn test_triple_quoted_can_contain_unescaped_quotes() {
  let code = "write \"\"\"say \"hi\" now\"\"\"\n";
  let (stdout, _stderr, ok) = run_lale(code, &[], &["--no-color"]);
  assert!(ok);
  assert_eq!(stdout, "say \"hi\" now\n");
}

// ==================== ERROR STACK ====================

#[test]
fn test_alert_error_messages_format() {
  // `alert error messages` drains the error stack through the separate
  // `DrainErrors` IR instruction, not `generate_leveled_output`. Its line is
  // therefore `Alert\t<timestamp>\t<message>` (no uptime column) — a known
  // inconsistency with the `log`/`warn`/`alert` statement format.
  let code = "add error \"boom\"\nalert error messages\n";
  let (_stdout, stderr, ok) = run_lale(code, &[], &["--release", "--no-color"]);
  assert!(ok);
  let line = stderr.lines().next().unwrap_or("");
  assert!(
    line.starts_with("Alert\t") && line.ends_with("\tboom"),
    "alert error messages line: {line}"
  );
}

#[test]
fn test_error_stack_core_ops_still_work() {
  let code = r#"
add error "alpha"
add error "beta"
var first as text = last error
write first
var second as text = last error
write second
when not (errors has messages)
    write "drained"
end when
"#;
  let (stdout, _stderr, ok) = run_lale(code, &[], &["--no-color"]);
  assert!(ok);
  assert!(
    stdout.contains("beta") && stdout.contains("alpha") && stdout.contains("drained"),
    "stdout: {stdout}"
  );
}

// ==================== REMOVED FEATURES ARE PARSE ERRORS ====================

#[test]
fn test_removed_features_are_rejected() {
  // These constructs were removed in the logging consolidation; they must now
  // fail to parse rather than silently do something else.
  let removed: &[&str] = &[
    "write error messages\n",
    "warn error messages\n",
    "write \"x\" to var\n",
    "warn \"x\" to var\n",
    "warn inline \"x\"\n",
    "alert inline \"x\"\n",
  ];
  for code in removed {
    let (_stdout, stderr, ok) = run_lale(code, &[], &[]);
    assert!(
      !ok,
      "should reject removed construct: {code:?} (stderr: {stderr})"
    );
    assert!(
      stderr.contains("Parse error") || stderr.contains("grammar error"),
      "expected a parse error for {code:?}, got: {stderr}"
    );
  }
}
