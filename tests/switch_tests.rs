//! Tests for the `switch`/`case`/`default` statement in the Lale language.
//!
//! The `switch` statement compares a value against patterns. For enum types,
//! the compiler checks exhaustiveness (all variants must be covered or a
//! `default` arm must be present). For open values (strings/ints), `default`
//! is required.
//!
//! Syntax:
//! ```lale
//! switch <value>
//!     case <pattern>:
//!         <body>
//!     case <pattern>:
//!         <body>
//!     default:
//!         <body>
//! end switch
//! ```
//!
//! This test suite covers:
//! - Basic switch statements (parser-only)
//! - Multiple cases and `default`
//! - Enum patterns with field bindings and discards
//! - Exhaustive coverage (no default needed when all variants covered)
//! - Empty body, move on, missing code
//! - Execution tests (runtime behavior via the compiler binary)
//! - Negative tests (parser errors, semantic errors)

use lale::{LaleParser, Rule};
use pest::Parser;

// ==================== PARSER TESTS ====================

// --- Test 1: Minimal switch with default ---

#[test]
fn test_switch_minimal_with_default() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case One: write \"one\" default: move on end switch",
  );
  assert!(result.is_ok());
}

// --- Test 2: Multiple cases ---

#[test]
fn test_switch_multiple_cases() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case One: write \"one\" case Two: write \"two\" default: write \"other\" end switch",
  );
  assert!(result.is_ok());
}

// --- Test 3: Enum pattern with field binding ---

#[test]
fn test_switch_enum_pattern_with_field_binding() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch result case Ok(val): write val default: move on end switch",
  );
  assert!(result.is_ok());
}

// --- Test 4: Enum with multiple fields ---

#[test]
fn test_switch_enum_multiple_fields() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch shape case Circle(r): write r case Rect(w,h): write w end switch",
  );
  assert!(result.is_ok());
}

// --- Test 5: All variants covered, no default ---

#[test]
fn test_switch_all_variants_no_default() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch color case Red: write \"red\" case Green: write \"green\" case Blue: write \"blue\" end switch",
  );
  assert!(result.is_ok());
}

// --- Test 6: Empty body in case ---

#[test]
fn test_switch_empty_body_in_case() {
  // Unlike match (which allows empty arms), switch_case requires stmt_block
  // (at least one statement). An empty body is a parser error.
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case A: default: write \"other\" end switch",
  );
  assert!(
    result.is_err(),
    "switch case with empty body should be a parser error"
  );
}

// --- Test 7: move on in case/default body ---

#[test]
fn test_switch_move_on_in_body() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case A: move on default: move on end switch",
  );
  assert!(result.is_ok());
}

// --- Test 8: missing code in case/default body ---

#[test]
fn test_switch_missing_code_in_body() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case A: missing code default: missing code end switch",
  );
  assert!(result.is_ok());
}

// --- Additional parser tests for edge cases ---

#[test]
fn test_switch_multi_line() {
  let code = "switch val\n    case A:\n        write \"a\"\n    default:\n        write \"other\"\nend switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_with_discard_pattern() {
  let code = "switch shape case Rect(_, h): write h case Circle(_): write 0 end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_single_variant_exhaustive() {
  let code = "switch maybe case Just(x): write x case Nothing: move on end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_many_cases() {
  let code = "switch x case A: write 1 case B: write 2 case C: write 3 case D: write 4 case E: write 5 default: write 0 end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_default_only() {
  let code = "switch x default: write \"fallback\" end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_function_call_as_value() {
  let code = "switch getColor() case Red: write \"red\" case Green: write \"green\" end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_member_access_as_value() {
  let code = "switch obj.color case Red: write \"red\" default: move on end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_parse_literal_int_cases() {
  let code = "switch code case 200: write \"OK\" case 404: write \"Not Found\" default: write \"Other\" end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

#[test]
fn test_switch_parse_literal_string_cases() {
  let code = "switch day case \"Saturday\": write \"weekend\" default: write \"school\" end switch";
  let result = LaleParser::parse(Rule::switch_stmt, code);
  assert!(result.is_ok());
}

// --- Negative parser tests ---

// --- Test 17: Parser error: switch without end switch ---

#[test]
fn test_switch_without_end_switch_fails() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case A: write \"a\" default: move on\n",
  );
  assert!(
    result.is_err(),
    "switch without end switch should be a parser error"
  );
}

// --- Test 18: Parser error: case without colon ---

#[test]
fn test_switch_case_without_colon_fails() {
  let result = LaleParser::parse(Rule::switch_stmt, "switch x case A write \"a\" end switch");
  assert!(
    result.is_err(),
    "case without colon should be a parser error"
  );
}

#[test]
fn test_switch_missing_case_keyword_fails() {
  let result = LaleParser::parse(Rule::switch_stmt, "switch x A: write \"a\" end switch");
  assert!(
    result.is_err(),
    "switch missing case keyword should be a parser error"
  );
}

#[test]
fn test_switch_wrong_close_keyword_fails() {
  let result = LaleParser::parse(Rule::switch_stmt, "switch x case A: write \"a\" end match");
  assert!(
    result.is_err(),
    "switch closed with end match should be a parser error"
  );
}

#[test]
fn test_switch_without_switch_keyword_fails() {
  let result = LaleParser::parse(Rule::switch_stmt, "case A: write \"a\" end switch");
  assert!(
    result.is_err(),
    "missing switch keyword should be a parser error"
  );
}

#[test]
fn test_switch_case_with_extra_colon_fails() {
  let result = LaleParser::parse(
    Rule::switch_stmt,
    "switch x case A:: write \"a\" end switch",
  );
  assert!(
    result.is_err(),
    "double colon in case should be a parser error"
  );
}

// ==================== EXECUTION TESTS ====================

#[cfg(test)]
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

  // --- Test 9: Enum switch with matching case ---

  #[test]
  fn test_switch_enum_matching_case() {
    let code = r#"
enum Color Red Green Blue end enum
var c = Color->Green
switch c
    case Red: write "red"
    case Green: write "green"
    case Blue: write "blue"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("green"),
      "Expected 'green' in stdout, got: {}",
      stdout
    );
    assert!(
      !stdout.contains("red"),
      "Red case should not have executed, got: {}",
      stdout
    );
    assert!(
      !stdout.contains("blue"),
      "Blue case should not have executed, got: {}",
      stdout
    );
  }

  // --- Test 10: Enum switch with default fallback ---

  #[test]
  fn test_switch_enum_default_fallback() {
    let code = r#"
enum Color Red Green Blue end enum
var c = Color->Blue
switch c
    case Red: write "red"
    case Green: write "green"
    default: write "other"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("other"),
      "Expected default case to match (output 'other'), got: {}",
      stdout
    );
  }

  // --- Test 11: Enum switch with field binding (single field) ---

  #[test]
  fn test_switch_enum_field_binding() {
    let code = r#"
type Person
    name as str
    age as i32
end type
enum Color red(Person) green blue end enum
var p = Person("Alice", 30)
var c = Color->red(p)
switch c
    case red(person): write person.name
    case green: write "green"
    case blue: write "blue"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("Alice"),
      "Expected bound field 'Alice' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 12: Enum switch with multiple field bindings ---

  #[test]
  fn test_switch_enum_multiple_field_bindings() {
    let code = r#"
  type Point
      x as i32
      y as i32
  end type
  enum Shape Circle(Point) Rect(Point,Point) end enum
  var center = Point(5, 10)
  var s = Shape->Circle(center)
  switch s
      case Circle(pt): write pt.x
      case Rect(tl, br): write br.x
  end switch
  "#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("5"),
      "Expected bound field pt.x = 5 in stdout, got: {}",
      stdout
    );
  }

  // --- Test 13: Switch with all cases covered (exhaustive, no default needed) ---

  #[test]
  fn test_switch_exhaustive_no_default() {
    let code = r#"
enum TrafficLight Red Yellow Green end enum
var light = TrafficLight->Yellow
var result as str = ""
switch light
    case Red: result = "stop"
    case Yellow: result = "slow"
    case Green: result = "go"
end switch
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("slow"),
      "Expected 'slow' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 14: Short-circuit — first matching case wins ---

  #[test]
  fn test_switch_first_matching_case_wins() {
    let code = r#"
enum Val One Two end enum
var v = Val->One
var flag as i32 = 0
switch v
    case One: flag = 1 as i32
    case Two: flag = 2 as i32
end switch
write flag
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("1"),
      "Expected '1' in stdout, got: {}",
      stdout
    );
  }

  // --- Test 15: move on as case body ---

  #[test]
  fn test_switch_move_on_as_case_body() {
    let code = r#"
enum Val One Two end enum
var v = Val->One
var x as i32 = 0
switch v
    case One:
        move on
        x = 42 as i32
    case Two: move on
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("42"),
      "Expected '42' in stdout after move on, got: {}",
      stdout
    );
  }

  // --- Test 16: missing code in debug mode ---

  #[test]
  fn test_switch_missing_code_debug_mode() {
    let code = r#"
enum Val One Two end enum
var v = Val->One
var x as i32 = 1
switch v
    case One: missing code
    case Two: move on
end switch
write x
"#;
    let (stdout, stderr, success) = run_lale(code);
    // Program should still run; missing code is a warning, not fatal
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("1"),
      "Expected '1' in stdout, got: {}",
      stdout
    );
    // In debug mode, stderr should contain a warning about missing code
    assert!(
      stderr.contains("Warning") || stderr.contains("missing code"),
      "Expected 'missing code' warning in stderr, got: {}",
      stderr
    );
  }

  // --- Test: Enum switch inside a function (no parameter — works around
  //          known IR gen limitation with enum-typed parameters) ---

  #[test]
  fn test_switch_inside_function() {
    let code = r#"
enum Val One Two end enum

fn get_name() returns str
    var v = Val->One
    switch v
        case One: return "one"
        case Two: return "two"
    end switch
end fn

write get_name()
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("one"),
      "Expected 'one' in stdout, got: {}",
      stdout
    );
  }

  // --- Test: Enum switch with discard patterns ---

  #[test]
  fn test_switch_enum_discard_pattern() {
    let code = r#"
type Point
    x as i32
    y as i32
end type
enum Shape Circle(Point) Rect(Point,Point) end enum
var tl = Point(0, 0)
var br = Point(10, 20)
var s = Shape->Rect(tl, br)
var result as i32 = 0
switch s
    case Circle(_): result = 1 as i32
    case Rect(_, _): result = 2 as i32
end switch
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("2"),
      "Expected discard pattern to match Rect, got: {}",
      stdout
    );
  }

  // --- Test: Multiple statements per case body ---

  #[test]
  fn test_switch_multiple_statements_per_case() {
    let code = r#"
enum Val One Two end enum
var v = Val->One
var a as i32 = 0
var b as i32 = 0
switch v
    case One:
        a = 10 as i32
        b = 20 as i32
    case Two:
        a = 30 as i32
        b = 40 as i32
end switch
write a
write b
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("10"),
      "Expected '10' in stdout, got: {}",
      stdout
    );
    assert!(
      stdout.contains("20"),
      "Expected '20' in stdout, got: {}",
      stdout
    );
  }

  // --- Test: Nested switch ---

  #[test]
  fn test_switch_nested() {
    let code = r#"
enum Outer A B end enum
enum Inner X Y end enum
var outer = Outer->A
var inner = Inner->Y
var result as str = ""
switch outer
    case A:
        switch inner
            case X: result = "A-X"
            case Y: result = "A-Y"
        end switch
    case B: result = "B"
end switch
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("A-Y"),
      "Expected nested switch to produce 'A-Y', got: {}",
      stdout
    );
  }

  // --- Test: Enum variant with no fields, default present ---

  #[test]
  fn test_switch_data_less_variants_with_default() {
    let code = r#"
enum Status Ok Err Unknown end enum
var s = Status->Unknown
var result as str = ""
switch s
    case Ok: result = "ok"
    case Err: result = "error"
    default: result = "unknown"
end switch
write result
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("unknown"),
      "Expected default case 'unknown', got: {}",
      stdout
    );
  }

  // ==================== NEGATIVE TESTS ====================

  // --- Test: Switch without default when not exhaustive (should fail) ---

  #[test]
  fn test_switch_non_exhaustive_without_default_fails() {
    let code = r#"
enum Color Red Green Blue end enum
var c = Color->Red
switch c
    case Red: write "red"
    case Green: write "green"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Non-exhaustive switch without default should fail, but succeeded"
    );
    assert!(
      !stderr.is_empty(),
      "Expected error message for non-exhaustive switch, but stderr was empty"
    );
  }

  // ==================== VALUE DISPATCH TESTS ====================

  #[test]
  fn test_switch_value_int_dispatch() {
    let code = r#"
var code as i32 = 404
switch code
    case 200: write "OK"
    case 404: write "Not Found"
    default: write "Other"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("Not Found"),
      "Expected 'Not Found', got: {}",
      stdout
    );
  }

  #[test]
  fn test_switch_value_string_dispatch() {
    let code = r#"
var day as str = "Sunday"
switch day
    case "Saturday": write "Weekend"
    case "Sunday": write "Weekend"
    default: write "School day"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("Weekend"),
      "Expected 'Weekend', got: {}",
      stdout
    );
  }

  #[test]
  fn test_switch_value_bool_dispatch() {
    let code = r#"
var flag as bool = true
switch flag
    case true: write "yes"
    case false: write "no"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(stdout.contains("yes"), "Expected 'yes', got: {}", stdout);
  }

  #[test]
  fn test_switch_value_bool_missing_case_no_default_fails() {
    let code = r#"
var flag as bool = true
switch flag
    case true: write "yes"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Non-exhaustive bool switch without default should fail"
    );
    assert!(
      stderr.contains("missing case(s) 'false'"),
      "Expected missing-case error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_bool_unreachable_default_fails() {
    let code = r#"
var flag as bool = true
switch flag
    case true: write "yes"
    case false: write "no"
    default: write "unreachable"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Exhaustive bool switch plus default should fail");
    assert!(
      stderr.contains("Unreachable 'default'"),
      "Expected unreachable-default error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_char_dispatch() {
    let code = r#"
var ch as char = 'b'
switch ch
    case 'a': write "A"
    case 'b': write "B"
    default: write "?"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(stdout.contains("B"), "Expected 'B', got: {}", stdout);
  }

  #[test]
  fn test_switch_value_float_dispatch() {
    let code = r#"
var x as f64 = 3.5
switch x
    case 1.5: write "one point five"
    case 3.5: write "three point five"
    default: write "other"
end switch
"#;
    let (stdout, stderr, success) = run_lale(code);
    assert!(success, "Expected success, stderr: {}", stderr);
    assert!(
      stdout.contains("three point five"),
      "Expected 'three point five', got: {}",
      stdout
    );
  }

  #[test]
  fn test_switch_value_default_required() {
    let code = r#"
var x as i32 = 1
switch x
    case 1: write "one"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Value switch without default should fail");
    assert!(
      stderr.contains("requires a default case"),
      "Expected default-required error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_duplicate_case() {
    let code = r#"
var x as i32 = 1
switch x
    case 1: write "one"
    case 1: write "one again"
    default: write "other"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Duplicate value switch case should fail");
    assert!(
      stderr.contains("Duplicate case value"),
      "Expected duplicate error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_duplicate_hex_and_int() {
    let code = r#"
var x as i32 = 26
switch x
    case 26: write "twenty-six"
    case 0x1A: write "twenty-six again"
    default: write "other"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Equivalent integer spellings should be duplicates"
    );
    assert!(
      stderr.contains("Duplicate case value"),
      "Expected duplicate error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_duplicate_int_and_float() {
    let code = r#"
var x as f64 = 3.0
switch x
    case 3: write "three"
    case 3.0: write "three again"
    default: write "other"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Equivalent float spellings should be duplicates");
    assert!(
      stderr.contains("Duplicate case value"),
      "Expected duplicate error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_type_mismatch() {
    let code = r#"
var x as i32 = 1
switch x
    case "one": write "one"
    default: write "other"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Type-mismatched value switch should fail");
    assert!(
      stderr.contains("does not match scrutinee type"),
      "Expected type mismatch error, got: {}",
      stderr
    );
  }

  #[test]
  fn test_switch_value_unit_mismatch() {
    let code = r#"
var d as f64 in <m> = 5 <m>
switch d
    case 5 <s>: write "seconds"
    default: write "other"
end switch
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(!success, "Unit-mismatched value switch should fail");
    assert!(
      stderr.contains("does not match scrutinee unit"),
      "Expected unit mismatch error, got: {}",
      stderr
    );
  }

  // --- Test: Non-exhaustive enum switch in a function ---

  #[test]
  fn test_switch_non_exhaustive_in_function_fails() {
    let code = r#"
enum Color Red Green Blue end enum

fn describe(c as Color) returns str
    switch c
        case Red: return "red"
        case Green: return "green"
    end switch
end fn

write describe(Color->Red)
"#;
    let (_stdout, stderr, success) = run_lale(code);
    assert!(
      !success,
      "Non-exhaustive switch in function without default should fail"
    );
    assert!(
      !stderr.is_empty(),
      "Expected error for non-exhaustive switch in function"
    );
  }
}
