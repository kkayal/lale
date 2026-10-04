//! Cross-module shared globals: by-reference (read-write) semantics and the
//! SQLite-backed single source of truth for module exports.
//!
//! Covers:
//! - Reading a stdlib `export var` through `use` (the imported name is the
//!   _same storage_, not a copy).
//! - Writing through an imported name and observing the write.
//! - The constant initializer traveling with an IR global (`§5.29`).
//! - `SqliteSymbolManager::get_exports` returning only the named module's
//!   exported symbols (variables + functions), disambiguated by module.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use lale::ast::SourceLocation;
use lale::semantic_analysis::{FnInfo, Linkage, SqliteSymbolManager, VarScope};

/// Run `lale run -` with `code` on stdin plus `args`, returning
/// `(stdout, stderr, success)`.
fn run_stdin(code: &str, args: &[&str]) -> (String, String, bool) {
  let mut cmd = Command::new(env!("CARGO_BIN_EXE_lale"));
  cmd
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .arg("run")
    .arg("-")
    .arg("--no-color")
    .args(args)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

  let mut child = cmd.spawn().expect("Failed to spawn lale");
  child
    .stdin
    .take()
    .expect("Failed to open stdin")
    .write_all(code.as_bytes())
    .expect("Failed to write stdin");

  let output = child.wait_with_output().expect("Failed to wait on child");
  (
    String::from_utf8_lossy(&output.stdout).to_string(),
    String::from_utf8_lossy(&output.stderr).to_string(),
    output.status.success(),
  )
}

// ==================== stdlib value import (by reference) ====================

#[test]
fn test_stdlib_imported_global_reads_initial_value() {
  let (stdout, stderr, ok) = run_stdin(
    "use O_RDONLY, O_CREAT from std.file_io_posix\nwrite O_RDONLY\nwrite O_CREAT\n",
    &[],
  );
  assert!(ok, "program should run cleanly, stderr: {stderr}");
  assert!(
    stdout.contains('0'),
    "O_RDONLY should read 0, got stdout: {stdout}"
  );
  assert!(
    stdout.contains("64"),
    "O_CREAT should read 64, got stdout: {stdout}"
  );
}

#[test]
fn test_stdlib_imported_global_is_writable() {
  let (stdout, stderr, ok) = run_stdin(
    "use O_RDONLY from std.file_io_posix\nO_RDONLY = 99\nwrite O_RDONLY\n",
    &[],
  );
  assert!(ok, "program should run cleanly, stderr: {stderr}");
  assert!(
    stdout.contains("99"),
    "write-through should print 99, got stdout: {stdout}"
  );
}

// ==================== constant initializer in IR ====================

#[test]
fn test_global_constant_initializer_emitted_in_ir() {
  let (_stdout, stderr, ok) = run_stdin("var y as u32 = 5\n", &["--print-ir"]);
  assert!(ok, "program should compile, stderr: {stderr}");
  assert!(
    stderr.contains("@y: u32 = 5u"),
    "IR should carry the constant initializer `@y: u32 = 5u`, got: {stderr}"
  );
}

// ==================== SQLite single source of truth for exports ============

#[test]
fn test_get_exports_filters_by_module_and_linkage() {
  let mut m = SqliteSymbolManager::new();
  let loc = SourceLocation::dummy();

  // Module "math.lale" exports `e` (and keeps `hidden` internal).
  m.set_module_path("math.lale".to_string());
  m.define_variable(
    VarScope::Global,
    "e",
    &loc,
    "u32",
    None,
    Linkage::Export,
    true,
  )
  .unwrap();
  m.define_variable(
    VarScope::Global,
    "hidden",
    &loc,
    "u32",
    None,
    Linkage::Internal,
    true,
  )
  .unwrap();

  // Module "physics.lale" exports a distinct `g`.
  m.set_module_path("physics.lale".to_string());
  m.define_variable(
    VarScope::Global,
    "g",
    &loc,
    "f64",
    None,
    Linkage::Export,
    true,
  )
  .unwrap();

  let math_exports = m.get_exports("math.lale").expect("query should succeed");
  assert_eq!(math_exports.len(), 1, "only `e` is exported from math.lale");
  assert!(
    !math_exports.contains_key("hidden"),
    "internal symbols must not leak"
  );
  assert!(
    !math_exports.contains_key("g"),
    "another module's export must not leak"
  );
  let e = math_exports.get("e").expect("e should be present");
  assert_eq!(e.data_type, "u32");
  assert_eq!(e.linkage, Linkage::Export);
  assert_eq!(e.module_path, "math.lale");

  let physics_exports = m.get_exports("physics.lale").expect("query should succeed");
  assert_eq!(physics_exports.len(), 1);
  assert!(
    !physics_exports.contains_key("e"),
    "math.lale's export must not leak"
  );
  assert_eq!(physics_exports.get("g").unwrap().data_type, "f64");
  assert_eq!(
    physics_exports.get("g").unwrap().module_path,
    "physics.lale"
  );
}

#[test]
fn test_get_exports_includes_functions() {
  let mut m = SqliteSymbolManager::new();
  let loc = SourceLocation::dummy();

  m.set_module_path("math.lale".to_string());
  m.define_function(
    "double",
    &loc,
    "i32",
    None,
    FnInfo {
      parameters: vec![],
      return_unit: None,
      body: vec![],
    },
    Linkage::Export,
  )
  .unwrap();

  let exports = m.get_exports("math.lale").expect("query should succeed");
  assert_eq!(exports.len(), 1);
  let double = exports.get("double").expect("double should be exported");
  assert!(
    matches!(double.kind, lale::semantic_analysis::SymbolKind::Function),
    "double should be a Function symbol"
  );
  assert_eq!(double.data_type, "i32");
  assert_eq!(double.linkage, Linkage::Export);
}

// ==================== module-aware uniqueness ====================

#[test]
fn test_define_symbol_allows_same_name_in_different_modules() {
  let mut m = SqliteSymbolManager::new();
  let loc = SourceLocation::dummy();

  m.set_module_path("math.lale".to_string());
  m.define_variable(
    VarScope::Global,
    "e",
    &loc,
    "u32",
    None,
    Linkage::Export,
    true,
  )
  .unwrap();

  // Same name in a *different* module is allowed (module-aware uniqueness).
  m.set_module_path("physics.lale".to_string());
  m.define_variable(
    VarScope::Global,
    "e",
    &loc,
    "f64",
    None,
    Linkage::Export,
    true,
  )
  .unwrap();

  // A redefinition in the *same* module is still rejected.
  let err = m
    .define_variable(
      VarScope::Global,
      "e",
      &loc,
      "f64",
      None,
      Linkage::Export,
      true,
    )
    .unwrap_err();
  assert!(err.contains("already defined"), "got: {err}");
}

// ==================== cross-module ambiguity (end-to-end) ====================

fn run_main_file(dir: &Path) -> (String, String, bool) {
  let main = dir.join("main.lale");
  let out = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", main.to_str().unwrap(), "--no-color"])
    .output()
    .expect("Failed to run lale");
  (
    String::from_utf8_lossy(&out.stdout).to_string(),
    String::from_utf8_lossy(&out.stderr).to_string(),
    out.status.success(),
  )
}

#[test]
fn test_two_modules_can_export_same_named_global() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var e as u32 = 1\nexport var m as u32 = 2\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("physics.lale"),
    "export var e as f64 = 3.0\nexport var p as f64 = 4.0\n",
  )
  .unwrap();
  // Import only the non-colliding names; both modules still define `e`.
  fs::write(
    dir.path().join("main.lale"),
    "use m from local.math\nuse p from local.physics\nwrite \"ok\"\n",
  )
  .unwrap();

  let (_stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "should compile cleanly, stderr: {stderr}");
}

#[test]
fn test_importing_same_name_from_two_modules_is_ambiguous() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(dir.path().join("math.lale"), "export var e as u32 = 1\n").unwrap();
  fs::write(
    dir.path().join("physics.lale"),
    "export var e as f64 = 3.0\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use e from local.math\nuse e from local.physics\n",
  )
  .unwrap();

  let (_stdout, stderr, ok) = run_main_file(dir.path());
  assert!(!ok, "importing the same name from two modules should fail");
  assert!(
    stderr.contains("imported multiple times"),
    "expected an 'imported multiple times' error, got: {stderr}"
  );
}

// ==================== user-module IR generation (end-to-end) ====================

#[test]
fn test_user_module_function_import() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export fn double(x as i32) returns i32\n    return x * 2\nend fn\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use double from local.math\nwrite double(21)\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("42"),
    "double(21) should print 42, got: {stdout}"
  );
}

#[test]
fn test_user_module_value_import_read_and_write() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("config.lale"),
    "export var retries as u32 = 3\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use retries from local.config\nwrite retries\nretries = 5\nwrite retries\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("3") && stdout.contains("5"),
    "should read 3 then write-through 5, got: {stdout}"
  );
}

#[test]
fn test_user_module_function_references_own_export_global() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var base as u32 = 10\nexport fn add_base(x as u32) returns u32\n    return x + base\nend fn\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use add_base from local.math\nwrite add_base(5)\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("15"),
    "add_base(5) should print 15, got: {stdout}"
  );
}

#[test]
fn test_user_module_transitive_import() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(dir.path().join("b.lale"), "export var g as u32 = 7\n").unwrap();
  fs::write(
    dir.path().join("a.lale"),
    "use g from local.b\nexport fn get_g() returns u32\n    return g\nend fn\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use get_g from local.a\nwrite get_g()\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("7"),
    "get_g() should print 7, got: {stdout}"
  );
}

// ==================== module-qualified access (`math.e`) ====================

#[test]
fn test_module_qualified_value_access_read_and_write() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var e as u32 = 42\nexport var m as u32 = 1\n",
  )
  .unwrap();
  // `use m from local.math` brings math into the compilation; `math.e` is then reachable
  // by its qualified name without importing `e`.
  fs::write(
    dir.path().join("main.lale"),
    "use m from local.math\nwrite math.e\nmath.e = 99\nwrite math.e\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("42") && stdout.contains("99"),
    "should read 42 then write-through 99, got: {stdout}"
  );
}

#[test]
fn test_same_named_exports_disambiguated_by_qualification() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var e as u32 = 42\nexport var m as u32 = 1\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("physics.lale"),
    "export var e as f64 = 3.5\nexport var p as f64 = 2.0\n",
  )
  .unwrap();
  // Import `e` from math; access `physics.e` by its qualified name.
  fs::write(
    dir.path().join("main.lale"),
    "use e from local.math\nuse p from local.physics\nwrite e\nwrite physics.e\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("42") && stdout.contains("3.5"),
    "should read math.e=42 and physics.e=3.5, got: {stdout}"
  );
}

#[test]
fn test_module_qualified_function_call() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var e as u32 = 42\nexport fn double(x as i32) returns i32\n    return x * 2\nend fn\n",
  )
  .unwrap();
  // `use e from local.math` brings math into the compilation; `math.double` is then
  // reachable by its qualified name without importing `double`.
  fs::write(
    dir.path().join("main.lale"),
    "use e from local.math\nwrite math.double(21)\n",
  )
  .unwrap();

  let (stdout, stderr, ok) = run_main_file(dir.path());
  assert!(ok, "program should run, stderr: {stderr}");
  assert!(
    stdout.contains("42"),
    "math.double(21) should print 42, got: {stdout}"
  );
}

#[test]
fn test_module_qualified_function_call_unknown_module_rejected() {
  let dir = tempfile::tempdir().unwrap();
  fs::write(
    dir.path().join("math.lale"),
    "export var e as u32 = 42\nexport fn double(x as i32) returns i32\n    return x * 2\nend fn\n",
  )
  .unwrap();
  fs::write(
    dir.path().join("main.lale"),
    "use e from local.math\nwrite typo.double(21)\n",
  )
  .unwrap();

  let (_stdout, stderr, ok) = run_main_file(dir.path());
  assert!(!ok, "a qualified call to an unknown module should fail");
  assert!(
    stderr.contains("not exported from module"),
    "expected a 'not exported from module' error, got: {stderr}"
  );
}
