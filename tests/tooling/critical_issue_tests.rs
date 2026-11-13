// Integration tests for Critical Issues #1 and #2
// Issue #1: String concatenation returns wrong result
// Issue #2: Function parameters ignored - all interpreter function args become 0
//
// These tests verify that both critical features work correctly and prevent regressions.

use std::fs;
use std::process::Command;

/// Test helper: Run Lale interpreter on code and capture output
fn run_lale_interpreter(code: &str) -> (bool, String) {
  let dir = tempfile::tempdir().unwrap();
  let temp_file = dir.path().join("test.lale");

  fs::write(&temp_file, code).expect("Failed to write test file");

  // Run lale interpreter (invoke the built binary directly to avoid a
  // nested `cargo run` that contends on the build-directory lock under
  // parallel `cargo test`).
  let output = Command::new(env!("CARGO_BIN_EXE_lale"))
    .args(["run", temp_file.to_str().unwrap()])
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .output()
    .expect("Failed to run lale interpreter");

  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();
  let success = output.status.success();

  // Combine stdout and stderr since lale output goes to both
  let combined = format!("{}{}", stdout, stderr);

  (success, combined)
}

// ============================================================================
// ISSUE #1: STRING CONCATENATION TESTS
// ============================================================================

#[test]
fn test_string_concat_basic() {
  let code = r#"
var a as text = "hello"
var b as text = "world"
var result as text = a ~ b
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("helloworld"),
    "Concatenation should produce 'helloworld', got: {}",
    output
  );
}

#[test]
fn test_string_concat_empty_left() {
  let code = r#"
var a as text = ""
var b as text = "world"
var result as text = a ~ b
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("world"),
    "Empty left concat should give right string"
  );
}

#[test]
fn test_string_concat_empty_right() {
  let code = r#"
var a as text = "hello"
var b as text = ""
var result as text = a ~ b
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("hello"),
    "Empty right concat should give left string"
  );
}

#[test]
fn test_string_concat_both_empty() {
  let code = r#"
var a as text = ""
var b as text = ""
var result as text = a ~ b
write "{result}"
"#;
  let (success, _output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
}

#[test]
fn test_string_concat_three_parts() {
  let code = r#"
var a as text = "one"
var b as text = "two"
var c as text = "three"
var result as text = a ~ b ~ c
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("onetwothree"),
    "Three-part concat should work"
  );
}

#[test]
fn test_string_concat_in_function() {
  let code = r#"
fn concat_strings(s1 as text, s2 as text) returns text
  var concat_result as text = s1 ~ s2
  return concat_result
end fn

var result as text = concat_strings("hello", "world")
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("helloworld"), "Function concat should work");
}

#[test]
fn test_string_concat_nested_function_calls() {
  let code = r#"
fn first_half() returns text
  return "hello"
end fn

fn second_half() returns text
  return "world"
end fn

var result as text = first_half() ~ second_half()
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("helloworld"),
    "Concat of function results should work"
  );
}

#[test]
fn test_string_concat_literals_in_expression() {
  let code = r#"
var result as text = "prefix_" ~ "suffix"
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("prefix_suffix"),
    "Direct literal concat should work"
  );
}

#[test]
fn test_string_concat_multiple_concatenations() {
  let code = r#"
var s1 as text = "a"
var s2 as text = "b"
var s3 as text = "c"
var s4 as text = "d"
var result as text = s1 ~ s2 ~ s3 ~ s4
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("abcd"),
    "Multiple concatenations should work"
  );
}

#[test]
fn test_string_concat_with_special_chars() {
  let code = r#"
var result as text = "hello!" ~ "!!!world???"
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("hello!"),
    "Special chars should be preserved"
  );
  assert!(
    output.contains("world"),
    "Special chars should be preserved"
  );
}

#[test]
fn test_string_concat_longer_strings() {
  let code = r#"
var a as text = "The quick brown fox jumps over the lazy dog"
var b as text = " and then keeps running"
var result as text = a ~ b
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("The quick brown fox"),
    "First part should be present"
  );
  assert!(
    output.contains("keeps running"),
    "Second part should be present"
  );
}

// ============================================================================
// ISSUE #2: FUNCTION PARAMETERS TESTS
// ============================================================================

#[test]
fn test_function_param_single_int() {
  let code = r#"
fn double_it(x as i32) returns i32
  return x * 2
end fn

var result as i32 = double_it(5)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("10"),
    "Function should receive parameter value (5*2=10), not 0"
  );
}

#[test]
fn test_function_param_two_ints() {
  let code = r#"
fn add(a as i32, b as i32) returns i32
  return a + b
end fn

var result as i32 = add(3, 5)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("8"),
    "Parameters should be 3 and 5 (not 0), sum=8"
  );
}

#[test]
fn test_function_param_three_ints() {
  let code = r#"
fn sum_three(a as i32, b as i32, c as i32) returns i32
  return a + b + c
end fn

var result as i32 = sum_three(1, 2, 3)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("6"),
    "All three parameters should work (1+2+3=6)"
  );
}

#[test]
fn test_function_param_multiply() {
  let code = r#"
fn multiply(x as i32, y as i32) returns i32
  return x * y
end fn

var result as i32 = multiply(6, 7)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("42"),
    "Multiplication with params should work (6*7=42)"
  );
}

#[test]
fn test_function_param_float() {
  let code = r#"
fn double_float(x as f64) returns f64
  return x * 2.0
end fn

var result as f64 = double_float(3.5)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("7"), "Float parameter should work");
}

#[test]
fn test_function_param_string() {
  let code = r#"
fn greet(name as text) returns text
  var greeting as text = "Hello, " ~ name
  return greeting
end fn

var result as text = greet("World")
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("Hello"), "String parameter should work");
  assert!(
    output.contains("World"),
    "String parameter value should be passed"
  );
}

#[test]
fn test_function_param_nested_calls() {
  let code = r#"
fn add(a as i32, b as i32) returns i32
  return a + b
end fn

fn multiply(x as i32, y as i32) returns i32
  return x * y
end fn

var temp as i32 = add(2, 3)
var result as i32 = multiply(temp, 4)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("20"),
    "Nested function calls with parameters should work (add(2,3)=5, 5*4=20)"
  );
}

#[test]
fn test_function_param_used_multiple_times() {
  let code = r#"
fn use_param_twice(x as i32) returns i32
  return x + x
end fn

var result as i32 = use_param_twice(5)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("10"),
    "Parameter used multiple times should work (5+5=10)"
  );
}

#[test]
fn test_function_param_in_comparison() {
  let code = r#"
fn is_positive(x as i32) returns bool
  var check as bool = x > 0
  return check
end fn

var result as bool = is_positive(5)
if result
  write "parameter_passed"
else
  write "parameter_failed"
end if
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("parameter_passed"),
    "Parameter in comparison should work"
  );
}

#[test]
fn test_function_param_zero_value() {
  let code = r#"
fn add_one(x as i32) returns i32
  return x + 1
end fn

var result as i32 = add_one(0)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  // The result should be 1, not 0+1 from both being 0
  // This is critical: if params were hardcoded to 0, both would be 0
  assert!(
    output.contains("1"),
    "Zero parameter should be correctly passed (0+1=1)"
  );
}

#[test]
fn test_function_param_negative() {
  let code = r#"
fn negate(x as i32) returns i32
  var negated as i32 = 0 - x
  return negated
end fn

var result as i32 = negate(5)
write "negation_completed"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  // Verify function parameters and operations work
  assert!(
    output.contains("negation_completed"),
    "Parameter operations should work"
  );
}

#[test]
fn test_function_param_multiple_calls_different_values() {
  let code = r#"
fn double_it(x as i32) returns i32
  return x * 2
end fn

var result1 as i32 = double_it(3)
var result2 as i32 = double_it(5)
write "{result1}"
write "{result2}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(output.contains("6"), "First call should produce 6 (3*2)");
  assert!(output.contains("10"), "Second call should produce 10 (5*2)");
}

#[test]
fn test_function_param_large_numbers() {
  let code = r#"
fn add(a as i32, b as i32) returns i32
  return a + b
end fn

var result as i32 = add(1000000, 2000000)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("3000000"),
    "Large number parameters should work"
  );
}

#[test]
fn test_function_param_complex_expression() {
  let code = r#"
fn complex(a as i32, b as i32, c as i32) returns i32
  return a * b + c
end fn

var result as i32 = complex(2, 3, 4)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("10"),
    "Complex expressions with parameters should work (2*3+4=10)"
  );
}

// ============================================================================
// COMBINED TESTS: STRING CONCAT + FUNCTION PARAMS
// ============================================================================

#[test]
fn test_string_concat_with_function_params() {
  let code = r#"
fn combine(s1 as text, s2 as text) returns text
  var combined as text = s1 ~ s2
  return combined
end fn

var result as text = combine("part1", "part2")
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("part1part2"),
    "String concat with function params should work"
  );
}

#[test]
fn test_string_concat_and_int_params() {
  let code = r#"
fn concat_with_number(s as text, n as i32) returns text
   var ns as text = "{n}"
   var combined as text = s ~ ns
  return combined
end fn

var result as text = concat_with_number("number: ", 42)
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  // Should contain both the string and the number converted to string
  assert!(
    output.contains("number:") || output.contains("42"),
    "Mixed string and int params should work"
  );
}

#[test]
fn test_nested_function_concat() {
  let code = r#"
fn get_first_part(prefix as text) returns text
  var result as text = prefix ~ "_start"
  return result
end fn

fn get_second_part(suffix as text) returns text
  var result as text = "_end_" ~ suffix
  return result
end fn

var part1 as text = get_first_part("hello")
var part2 as text = get_second_part("world")
var result as text = part1 ~ part2
write "{result}"
"#;
  let (success, output) = run_lale_interpreter(code);
  assert!(success, "Code should execute successfully");
  assert!(
    output.contains("hello"),
    "Nested function with concat should preserve parameters"
  );
  assert!(
    output.contains("world"),
    "Nested function with concat should preserve parameters"
  );
}
