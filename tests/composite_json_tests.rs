use std::process::Command;

fn run_lale(code: &str) -> std::process::Output {
  let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", "-"])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("spawn lale");
  use std::io::Write;
  child
    .stdin
    .take()
    .unwrap()
    .write_all(code.as_bytes())
    .unwrap();
  child.wait_with_output().expect("wait")
}

#[test]
fn test_struct_debug_has_type_name() {
  let code = "type Point\n  x as f64\n  y as f64\nend type\nvar p = Point(1.5, 2.5)\ndebug p\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Point"), "Missing Point: {}", stderr);
  assert!(out.status.success());
}

#[test]
fn test_struct_embedding_has_type_name() {
  let code =
    "type Point\n  x as f64\n  y as f64\nend type\nvar p = Point(3.0, 4.0)\nwrite \"{p}\"\n";
  let out = run_lale(code);
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.contains("Point"), "Missing Point: {}", stdout);
  assert!(out.status.success());
}

#[test]
fn test_array_embedding_shows_elements() {
  let code = "var arr as i32[3] = [1, 2, 3]\nwrite \"{arr}\"\n";
  let out = run_lale(code);
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(
    stdout.contains("[1, 2, 3]"),
    "Array elements missing from output: {}",
    stdout
  );
  assert!(out.status.success());
}

#[test]
fn test_array_debug_shows_elements() {
  let code = "var arr as i32[4] = [10, 20, 30, 40]\ndebug arr\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("[10, 20, 30, 40]"),
    "Array elements missing from debug output: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_struct_with_text_value() {
  let code =
    "type Person\n  name as text\n  age as i32\nend type\nvar p = Person(\"Alice\", 25)\ndebug p\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Alice"), "Missing Alice: {}", stderr);
  assert!(out.status.success());
}

#[test]
fn test_struct_no_as_type_suffix() {
  let code = "type Point\n  x as f64\nend type\nvar p = Point(1.0)\ndebug p\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    !stderr.contains("as Point"),
    "Should not have as Point: {}",
    stderr
  );
  assert!(out.status.success());
}

#[test]
fn test_simple_type_keeps_as_suffix() {
  let code = "var x as u8 = 42\ndebug x\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("as u8"), "Should keep as u8: {}", stderr);
  assert!(out.status.success());
}

#[test]
fn test_json_balanced_braces() {
  let code =
    "type Point\n  x as f64\n  y as f64\nend type\nvar p = Point(1.0, 2.0)\nwrite \"{p}\"\n";
  let out = run_lale(code);
  let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
  let opens = stdout.chars().filter(|c| *c == '{').count();
  let closes = stdout.chars().filter(|c| *c == '}').count();
  assert_eq!(opens, closes, "Braces unbalanced: {}", stdout);
  assert!(stdout.starts_with("{"), "Should start with bra: {}", stdout);
  assert!(stdout.ends_with("}"), "Should end with ket: {}", stdout);
}

#[test]
fn test_struct_with_units() {
  let code = "type Measurement\n  value as f64 in <m>\n  tolerance as f64 in <mm>\nend type\nvar m = Measurement(5.0 <m>, 0.1 <mm>)\ndebug m\n";
  let out = run_lale(code);
  let stderr = String::from_utf8_lossy(&out.stderr);
  // The unit is stored with angle brackets: "<m>", "<mm>"
  assert!(stderr.contains("<m>"), "Missing unit <m>: {}", stderr);
  assert!(stderr.contains("<mm>"), "Missing unit <mm>: {}", stderr);
  assert!(out.status.success());
}

#[test]
fn test_struct_embedding_no_type_key() {
  let code = "type Point\n  x as f64\nend type\nvar p = Point(1.0)\nwrite \"{p}\"\n";
  let out = run_lale(code);
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(
    !stdout.contains("type"),
    "Should not have type key: {}",
    stdout
  );
  assert!(out.status.success());
}
