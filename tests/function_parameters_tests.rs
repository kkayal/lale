use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for function parameter passing: by-value (default), `copy`, and `ref`.
///
/// These verify the grammar accepts `ref` as a parameter modifier, and that the
/// runtime semantics match `doc/ARCHITECTURE.md` §5.14d (see also
/// `doc/lale.md` §Functions, "Parameter Passing"):
/// - the default (no modifier) and `copy` copy the argument by value, so
///   mutation inside the function is not visible to the caller;
/// - `ref` passes a mutable reference, so mutation is visible to the caller;
/// - an aggregate parameter with no modifier warns at the definition site,
///   while `copy`, `ref`, `pointer`, and primitive-vector parameters do not.

// ==================== GRAMMAR ====================

#[test]
fn test_fn_parameter_ref() {
  let result = LaleParser::parse(Rule::fn_parameter, "ref x as u32");
  assert!(result.is_ok());
}

#[test]
fn test_fn_parameter_ref_with_unit() {
  let result = LaleParser::parse(Rule::fn_parameter, "ref distance as f64 in <m>");
  assert!(result.is_ok());
}

#[test]
fn test_parameters_mixed_copy_and_ref() {
  let result = LaleParser::parse(Rule::parameters, "copy a as u32, ref b as f64");
  assert!(result.is_ok());
}

#[test]
fn test_ref_keyword_is_distinct_from_copy() {
  assert!(LaleParser::parse(Rule::kw_ref, "ref").is_ok());
  assert!(LaleParser::parse(Rule::kw_ref, "copy").is_err());
  assert!(LaleParser::parse(Rule::kw_copy, "ref").is_err());
}

// ==================== EXECUTION ====================

mod execution_tests {
  use std::fs;
  use std::process::Command;

  /// Helper: compiles and runs the given Lale code, returning (stdout, stderr, success).
  fn run_lale(code: &str) -> (String, String, bool) {
    let dir = tempfile::tempdir().unwrap();
    let temp_file = dir.path().join("test.lale");
    fs::write(&temp_file, code).expect("Failed to write temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_lale"))
      .args(["run", temp_file.to_str().unwrap()])
      .output()
      .expect("Failed to run lale compiler");

    // tempdir auto-cleans up on drop

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
  }

  #[test]
  fn test_copy_by_default_mutation_not_visible() {
    let code = r#"
fn bump(x as i32) returns nothing
    x = x + 1
end fn

var m as i32 = 5
bump(m)
write m
"#;
    let (stdout, _stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "5");
  }

  #[test]
  fn test_copy_keyword_mutation_not_visible_and_no_warning() {
    let code = r#"
fn bump(copy x as i32) returns nothing
    x = x + 1
end fn

var m as i32 = 5
bump(m)
write m
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "5");
    assert!(
      !stderr.contains("copied by value"),
      "`copy` should not warn, got: {stderr}"
    );
  }

  #[test]
  fn test_ref_mutation_visible() {
    let code = r#"
fn double(ref x as i32) returns nothing
    x = x * 2
end fn

var n as i32 = 21
double(n)
write n
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "42");
    assert!(
      !stderr.contains("copied by value"),
      "`ref` should not warn, got: {stderr}"
    );
  }

  #[test]
  fn test_aggregate_param_warns_at_definition() {
    let code = r#"
type Point
    x as i32
    y as i32
end type

fn show(p as Point) returns nothing
    write "point"
end fn

var p as Point = Point(1, 2)
show(p)
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert!(
      stderr.contains("parameter 'p' of type 'Point' is copied by value at every call"),
      "expected aggregate copy warning, got: {stderr}"
    );
  }

  #[test]
  fn test_text_param_warns() {
    let code = r#"
fn echo(s as text) returns nothing
    write s
end fn

var greeting as text = "hi"
echo(greeting)
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert!(
      stderr.contains("parameter 's' of type 'text' is copied by value at every call"),
      "expected str copy warning, got: {stderr}"
    );
  }

  #[test]
  fn test_pointer_param_does_not_warn() {
    let code = r#"
fn use_ptr(p as pointer) returns nothing
    write "ptr"
end fn

var q as pointer = 0 as pointer
use_ptr(q)
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert!(
      !stderr.contains("copied by value"),
      "pointer should not warn, got: {stderr}"
    );
  }

  #[test]
  fn test_primitive_vector_param_does_not_warn() {
    let code = r#"
fn use_vec(v as vec3 of f64) returns nothing
    write "vec"
end fn

var v as vec3 of f64 = vec3(1.0, 2.0, 3.0)
use_vec(v)
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert!(
      !stderr.contains("copied by value"),
      "primitive vector should not warn, got: {stderr}"
    );
  }

  #[test]
  fn test_ref_struct_mutation_visible() {
    let code = r#"
type Point
    x as i32
    y as i32
end type

fn set_x(ref p as Point) returns nothing
    p = Point(99, 2)
end fn

var p as Point = Point(1, 2)
set_x(p)
write p.x
"#;
    let (stdout, _stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "99");
  }

  #[test]
  fn test_array_by_value_mutation_not_visible() {
    let code = r#"
fn mutate(arr as i32[3]) returns nothing
    arr[1] = 99
end fn

var a as i32[3] = [1, 2, 3]
mutate(a)
write a[1]
"#;
    let (stdout, _stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "1");
  }

  #[test]
  fn test_array_ref_mutation_visible() {
    let code = r#"
fn mutate(ref arr as i32[3]) returns nothing
    arr[1] = 99
end fn

var a as i32[3] = [1, 2, 3]
mutate(a)
write a[1]
"#;
    let (stdout, _stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "99");
  }

  #[test]
  fn test_2d_array_by_value_read() {
    let code = r#"
fn read_corner(m as i32[2][2]) returns i32
    return m[2][2]
end fn

var a as i32[2][2] = [[1, 2], [3, 4]]
write read_corner(a)
"#;
    let (stdout, _stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "4");
  }

  #[test]
  fn test_array_param_return_does_not_leak() {
    // Regression: a non-void function whose return-slot `ValueId` collided with
    // the caller's array-literal temporary used to leak the temporary.
    let code = r#"
fn read2(arr as i32[3]) returns i32
    return arr[2]
end fn

var a as i32[3] = [1, 2, 3]
write read2(a)
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "2");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_struct_text_field_deep_copy_isolated() {
    let code = r#"
type Person
    name as text
    age as i32
end type

fn mutate(p as Person) returns nothing
    unsafe value at p.name.ptr = 88 as u8
end fn

var bob as Person = Person("Bob", 30)
mutate(bob)
write bob.name
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "Bob");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_array_of_str_init_and_read_no_leak() {
    let code = r#"
var a as text[2] = ["Hi", "Bye"]
write a[1]
write a[2]
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "Hi\nBye");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_array_of_str_by_value_deep_copy_isolated() {
    let code = r#"
fn mutate(arr as text[2]) returns nothing
    unsafe value at arr[1].ptr = 88 as u8
end fn

var a as text[2] = ["Hi", "Bye"]
mutate(a)
write a[1]
write a[2]
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "Hi\nBye");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_optional_str_by_value_deep_copy_no_leak() {
    let code = r#"
fn echo_opt(x as text?) returns nothing
    when x has value
        write value of x
    end when
end fn

var n as text? = "Bobby"
echo_opt(n)
when n has value
    write value of n
end when
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "Bobby\nBobby");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_vec2_of_str_by_value_deep_copy_no_leak() {
    let code = r#"
fn take_vec(v as vec2 of text) returns nothing
    write "ok"
end fn

var v as vec2 of text = vec2("a", "b")
take_vec(v)
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "ok");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }

  #[test]
  fn test_struct_optional_text_field_by_value_deep_copy_no_leak() {
    let code = r#"
type Person
    name as text
    nickname as text?
end type

fn show_person(p as Person) returns nothing
    write p.name
end fn

var p as Person = Person("Bob", "Bobby")
show_person(p)
write p.name
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success);
    assert_eq!(stdout.trim(), "Bob\nBob");
    assert!(
      !stderr.contains("HEAP LEAK"),
      "unexpected heap leak: {stderr}"
    );
  }
}
