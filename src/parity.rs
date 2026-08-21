//! Parity harness for the interpreter and future Lale backends.
//!
//! The harness treats every backend as an **executable command**. It spawns
//! that command with the same source file and options, captures the child's
//! stdout/stderr/exit code, and diffs the results. This avoids any in-process
//! output redirection in the interpreter and matches how a cross-language AOT
//! backend (`lale exec`) will eventually be wired in.
//!
//! Today only the interpreter (`lale run`) is registered, so parity reports
//! zero diffs by construction.

use std::error::Error;
use std::process::{Command, Stdio};

/// A backend executable and the CLI subcommand that runs it.
pub struct BackendRunner {
  pub name: &'static str,
  pub command: &'static str,
}

/// Canonical output captured from one backend run.
#[derive(Debug, PartialEq, Eq)]
pub struct BackendOutput {
  pub stdout: Vec<u8>,
  pub stderr: Vec<u8>,
  pub exit_code: i32,
}

/// Options forwarded to every backend subprocess.
pub struct ParityOptions {
  pub source_file: String,
  pub no_color: bool,
  pub stdlib: String,
  pub release: bool,
  pub unchecked_overflow: bool,
  pub show_exit_paths: bool,
}

/// Run the parity harness. Registered backends are executed as subprocesses and
/// their `(stdout, stderr, exit_code)` triples are compared.
pub fn run_parity(options: &ParityOptions) -> Result<(), Box<dyn Error>> {
  let runners = registered_backends();
  let mut outputs: Vec<(&BackendRunner, BackendOutput)> = Vec::with_capacity(runners.len());

  for runner in &runners {
    let output = run_backend(runner, options)?;
    println!(
      "backend {} (lale {}): exit code {}",
      runner.name, runner.command, output.exit_code
    );
    outputs.push((runner, output));
  }

  if outputs.len() < 2 {
    println!("Parity: {} backend registered — 0 diffs.", outputs.len());
    return Ok(());
  }

  let reference = &outputs[0].1;
  let mut diffs = 0usize;
  for (runner, output) in outputs.iter().skip(1) {
    if output.stdout != reference.stdout {
      diffs += 1;
      eprintln!(
        "Parity diff: {} stdout differs from {}",
        runner.name, outputs[0].0.name
      );
    }
    if output.stderr != reference.stderr {
      diffs += 1;
      eprintln!(
        "Parity diff: {} stderr differs from {}",
        runner.name, outputs[0].0.name
      );
    }
    if output.exit_code != reference.exit_code {
      diffs += 1;
      eprintln!(
        "Parity diff: {} exit code {} differs from {} exit code {}",
        runner.name, output.exit_code, outputs[0].0.name, reference.exit_code
      );
    }
  }

  if diffs == 0 {
    println!("Parity: {} backends — 0 diffs.", outputs.len());
  } else {
    eprintln!("Parity: {} diff(s) found.", diffs);
    std::process::exit(1);
  }

  Ok(())
}

/// The current registry of runnable backends.
///
/// When the AOT branch lands, add a `BackendRunner { name: "aot", command: "exec" }`
/// entry here. No parity-harness changes should be required beyond that.
fn registered_backends() -> Vec<BackendRunner> {
  vec![BackendRunner {
    name: "interpreter",
    command: "run",
  }]
}

fn run_backend(
  runner: &BackendRunner,
  options: &ParityOptions,
) -> Result<BackendOutput, Box<dyn Error>> {
  let exe = std::env::current_exe()?;

  let mut cmd = Command::new(exe);
  cmd.arg(runner.command);

  if options.no_color {
    cmd.arg("--no-color");
  }
  cmd.arg("--stdlib-level").arg(&options.stdlib);
  if options.release {
    cmd.arg("--release");
  }
  if options.unchecked_overflow {
    cmd.arg("--unchecked-overflow");
  }
  if options.show_exit_paths {
    cmd.arg("--show-exit-paths");
  }
  cmd.arg(&options.source_file);

  let output = cmd
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()?;

  let exit_code = output
    .status
    .code()
    .ok_or("backend terminated by a signal and did not produce an exit code")?;

  Ok(BackendOutput {
    stdout: output.stdout,
    stderr: output.stderr,
    exit_code,
  })
}
