//! Build utilities for the Lale compiler.
//!
//! With the interpreter-only backend, the standard library is loaded from source
//! at compile time — no pre-compilation or archiving step is needed.
//! This binary is kept as a placeholder for future build tasks.

fn main() {
  eprintln!("xtask: No build tasks defined.");
  eprintln!("The standard library is loaded from source at compile time.");
  eprintln!("Use 'cargo build' to build the compiler, 'cargo test --all' to run tests.");
}
