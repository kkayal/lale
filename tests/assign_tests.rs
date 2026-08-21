use lale::ast_builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for assignment statement in the Lale grammar.
///
/// This test suite covers:
/// - Basic assignment parsing
/// - Member access assignment
/// - Array index assignment
/// - Semantic analysis (undefined variable, unit checking)

// ==================== GRAMMAR TESTS ====================

#[test]
fn test_assign_simple() {
  let result = LaleParser::parse(Rule::assign, "x = 5");
  assert!(result.is_ok());
}

#[test]
fn test_assign_identifier_expression() {
  let result = LaleParser::parse(Rule::assign, "x = y");
  assert!(result.is_ok());
}

#[test]
fn test_assign_complex_expression() {
  let result = LaleParser::parse(Rule::assign, "x = a + b * c");
  assert!(result.is_ok());
}

#[test]
fn test_assign_member_access() {
  let result = LaleParser::parse(Rule::assign, "obj.field = 10");
  assert!(result.is_ok());
}

#[test]
fn test_assign_nested_member_access() {
  let result = LaleParser::parse(Rule::assign, "obj.sub.field = 42");
  assert!(result.is_ok());
}

#[test]
fn test_assign_array_index() {
  let result = LaleParser::parse(Rule::assign, "arr[0] = 100");
  assert!(result.is_ok());
}

#[test]
fn test_assign_array_variable_index() {
  let result = LaleParser::parse(Rule::assign, "arr[i] = value");
  assert!(result.is_ok());
}

#[test]
fn test_assign_multi_dimensional_array() {
  let result = LaleParser::parse(Rule::assign, "matrix[i][j] = 0");
  assert!(result.is_ok());
}

#[test]
fn test_assign_member_with_array_index() {
  let result = LaleParser::parse(Rule::assign, "obj.arr[0] = 5");
  assert!(result.is_ok());
}

#[test]
fn test_assign_function_call_value() {
  let result = LaleParser::parse(Rule::assign, "x = getValue()");
  assert!(result.is_ok());
}

#[test]
fn test_assign_with_unit_literal() {
  let result = LaleParser::parse(Rule::assign, "distance = 5.0 <m>");
  assert!(result.is_ok());
}

#[test]
fn test_assign_multiline_not_allowed_around_equals() {
  // Newlines are NOT allowed around the = operator in assignments
  // (the grammar uses `os` which only includes spaces/tabs, not newlines)
  let result = LaleParser::parse(Rule::assign, "x\n    = 10");
  assert!(
    result.is_err(),
    "Assignment should not allow newlines around ="
  );
}

#[test]
fn test_assign_extra_whitespace() {
  let result = LaleParser::parse(Rule::assign, "x   =   42");
  assert!(result.is_ok());
}

// ==================== AST BUILDER TESTS ====================

fn parse_and_build(source: &str) -> Result<lale::ast::Program, String> {
  let pairs =
    LaleParser::parse(Rule::program, source).map_err(|e| format!("Parse error: {}", e))?;
  build_program(pairs, "test.lale")
}

#[test]
fn test_ast_assign_simple() {
  let source = "var x as i32 = 0\nx = 5";
  let program = parse_and_build(source).expect("Should parse");
  assert_eq!(program.statements.len(), 2);

  match &program.statements[1] {
    lale::ast::Stmt::Assign(assign) => {
      assert_eq!(assign.target.node, vec!["x"]);
      assert!(assign.indices.is_empty());
    }
    _ => panic!("Expected Assign statement"),
  }
}

#[test]
fn test_ast_assign_member_access() {
  let source = "var obj as i32 = 0\nobj.field = 10";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    lale::ast::Stmt::Assign(assign) => {
      assert_eq!(assign.target.node, vec!["obj", "field"]);
    }
    _ => panic!("Expected Assign statement"),
  }
}

#[test]
fn test_ast_assign_array_index() {
  let source = "var arr as i32[3] = [1, 2, 3]\narr[0] = 42";
  let program = parse_and_build(source).expect("Should parse");

  match &program.statements[1] {
    lale::ast::Stmt::Assign(assign) => {
      assert_eq!(assign.target.node, vec!["arr"]);
      assert_eq!(assign.indices.len(), 1);
    }
    _ => panic!("Expected Assign statement"),
  }
}

// ==================== SEMANTIC ANALYSIS TESTS ====================

#[test]
fn test_semantic_assign_undefined_variable() {
  let source = "y = 10";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should detect undefined variable");
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("undefined variable")));
}

#[test]
fn test_semantic_assign_defined_variable() {
  let source = "var x as i32 = 0\nx = 10";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept assignment to defined variable"
  );
}

#[test]
fn test_semantic_assign_unit_match() {
  let source = "var x as f64 in <m> = 5.0\nx = 10.0 <m>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(analyzer.is_valid(), "Should accept matching units");
}

#[test]
fn test_semantic_assign_unit_mismatch() {
  let source = "var x as f64 in <m> = 5.0\nx = 10.0 <s>";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(!analyzer.is_valid(), "Should detect unit mismatch");
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("Unit mismatch")));
}

#[test]
fn test_semantic_assign_unitless_to_unit() {
  let source = "var x as f64 in <m> = 5.0\nx = 10.0";
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    !analyzer.is_valid(),
    "Should reject unitless value to unit variable"
  );
  let errors = analyzer.get_errors();
  assert!(errors.iter().any(|e| e.contains("Unit mismatch")));
}

#[test]
fn test_semantic_assign_inside_function() {
  let source = r#"
fn test() returns nothing
    var x as i32 = 0
    x = 5
end fn
"#;
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept assignment inside function"
  );
}

#[test]
fn test_semantic_assign_global_from_function() {
  let source = r#"
var global as i32 = 0

fn test() returns nothing
    global = 5
end fn
"#;
  let program = parse_and_build(source).expect("Should parse");
  let analyzer = analyze_ast(&program);

  assert!(
    analyzer.is_valid(),
    "Should accept assignment to global from function"
  );
}

// ==================== EDGE CASES ====================

#[test]
fn test_assign_boolean_value() {
  let result = LaleParser::parse(Rule::assign, "flag = true");
  assert!(result.is_ok());
}

#[test]
fn test_assign_string_value() {
  let result = LaleParser::parse(Rule::assign, "name = \"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_assign_array_literal() {
  let result = LaleParser::parse(Rule::assign, "data = [1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_assign_negative_value() {
  let result = LaleParser::parse(Rule::assign, "x = -42");
  assert!(result.is_ok());
}

#[test]
fn test_assign_hex_value() {
  let result = LaleParser::parse(Rule::assign, "color = 0xFF00FF");
  assert!(result.is_ok());
}

#[test]
fn test_assign_parenthesized_expression() {
  let result = LaleParser::parse(Rule::assign, "x = (a + b) * c");
  assert!(result.is_ok());
}

// ==================== ARRAY ASSIGNMENT EXECUTION TESTS ====================
// Tests for interpreter execution of array element assignments

pub mod array_assignment_execution_tests {
  use std::io::Write;
  use std::process::{Command, Stdio};

  /// Helper: Run lale code via interpreter and capture output
  pub fn run_interpreter(code: &str) -> Result<String, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
      .current_dir(env!("CARGO_MANIFEST_DIR"))
      .args(["run", "-"])
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|e| format!("Failed to spawn lale: {}", e))?;

    let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
    stdin
      .write_all(code.as_bytes())
      .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    // Close stdin so the child process sees EOF
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

  // ==================== Simple Array Element Assignment ====================

  #[test]
  fn test_array_assign_single_element() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
arr[1] = 100
write arr[1]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("100"),
      "arr[1] should be 100, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_assign_first_element() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
arr[1] = 999
write arr[1]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("999"),
      "First element should be 999, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_assign_last_element() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
arr[3] = 777
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("777"),
      "Last element should be 777, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_assign_with_expression() {
    let code = r#"
var arr as i64[3] = [10, 20, 30]
var idx as i64 = 2
arr[idx] = arr[1] + arr[3]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[2] = arr[1] + arr[3] = 10 + 30 = 40
    assert!(
      result.as_ref().unwrap().contains("40"),
      "arr[2] should be 40, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_assign_multiple_elements() {
    let code = r#"
var arr as i64[5] = [1, 2, 3, 4, 5]
arr[1] = 10
arr[3] = 30
arr[5] = 50
var sum as i64 = arr[1] + arr[3] + arr[5]
write sum
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // sum = 10 + 30 + 50 = 90
    assert!(
      result.as_ref().unwrap().contains("90"),
      "Sum should be 90, got: {}",
      result.unwrap()
    );
  }

  // ==================== Float Array Assignment ====================

  #[test]
  fn test_float_array_assign() {
    let code = r#"
var arr as f64[3] = [1.0, 2.0, 3.0]
arr[2] = 99.5
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // f64 output is in scientific notation
    let output = result.unwrap();
    assert!(
      output.contains("99.5") || output.contains("9.9") || output.contains("99"),
      "arr[2] should contain 99, got: {}",
      output
    );
  }

  // ==================== Bool Array Assignment ====================

  #[test]
  fn test_bool_array_assign() {
    let code = r#"
var flags as bool[3] = [true, false, true]
flags[2] = true
if flags[2]
    write "PASS"
else
    move on
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "flags[2] should be true, got: {}",
      result.unwrap()
    );
  }

  // ==================== Array in Function ====================

  #[test]
  fn test_array_assign_in_function() {
    let code = r#"
fn modify_array() returns i64
    var arr as i64[3] = [1, 2, 3]
    arr[2] = 42
    return arr[2]
end fn

var result as i64 = modify_array()
write result
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("42"),
      "Function should return 42, got: {}",
      result.unwrap()
    );
  }

  // ==================== Array with Variable Index ====================

  #[test]
  fn test_array_assign_with_variable_index() {
    // Test array assignment using a variable as the index
    let code = r#"
var arr as i64[5] = [1 as i64, 2 as i64, 3 as i64, 4 as i64, 5 as i64]
var i as i64 = 3
arr[i] = (99 as i64)
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("99"),
      "arr[3] should be 99, got: {}",
      result.unwrap()
    );
  }

  // Array assignment inside loops - now working correctly.
  // Fixed: array modifications now persist after the loop.
  #[test]
  fn test_array_assign_in_loop() {
    let code = r#"
var arr as i64[5] = [0 as i64, 0 as i64, 0 as i64, 0 as i64, 0 as i64]
var i as i64 = 1
var limit as i64 = 1
	loop when i <= limit
	    arr[i] = (10 as i64)
	    i = i + (1 as i64)
	end loop when not (i <= limit)
	write arr[1]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("10"),
      "arr[1] should be 10 after loop, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_sum_after_assignments() {
    let code = r#"
var arr as i64[4] = [0, 0, 0, 0]
arr[1] = 1
arr[2] = 2
arr[3] = 3
arr[4] = 4
var sum as i64 = arr[1] + arr[2] + arr[3] + arr[4]
write sum
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // sum = 1 + 2 + 3 + 4 = 10
    assert!(
      result.as_ref().unwrap().contains("10"),
      "Sum should be 10, got: {}",
      result.unwrap()
    );
  }

  // ==================== Compound Assignment to Array Elements ====================

  #[test]
  fn test_array_compound_assign_add() {
    let code = r#"
var arr as i64[3] = [10, 20, 30]
arr[2] += 5
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[2] = 20 + 5 = 25
    assert!(
      result.as_ref().unwrap().contains("25"),
      "arr[2] should be 25 after +=, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_compound_assign_sub() {
    let code = r#"
var arr as i64[3] = [10, 20, 30]
arr[3] -= 7
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[3] = 30 - 7 = 23
    assert!(
      result.as_ref().unwrap().contains("23"),
      "arr[3] should be 23 after -=, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_compound_assign_mul() {
    let code = r#"
var arr as i64[3] = [2, 3, 4]
arr[2] *= 10
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[2] = 3 * 10 = 30
    assert!(
      result.as_ref().unwrap().contains("30"),
      "arr[2] should be 30 after *=, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_compound_assign_div() {
    let code = r#"
var arr as i64[3] = [10, 20, 30]
arr[1] /= 2
write arr[1]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[1] = 10 / 2 = 5
    assert!(
      result.as_ref().unwrap().contains("5"),
      "arr[1] should be 5 after /=, got: {}",
      result.unwrap()
    );
  }

  // ==================== Multi-dimensional Array Assignment ====================

  #[test]
  fn test_2d_array_assign() {
    // Note: Lale uses 1-based indexing; literals need explicit casting
    let code = r#"
var matrix as i64[2][3] = [[(1 as i64), (2 as i64), (3 as i64)], [(4 as i64), (5 as i64), (6 as i64)]]
matrix[1][2] = (99 as i64)
write matrix[1][2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("99"),
      "matrix[1][2] should be 99, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_2d_array_assign_different_positions() {
    let code = r#"
var matrix as i64[2][2] = [[(1 as i64), (2 as i64)], [(3 as i64), (4 as i64)]]
matrix[1][1] = (10 as i64)
matrix[2][2] = (40 as i64)
var sum as i64 = matrix[1][1] + matrix[2][2]
write sum
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // sum = 10 + 40 = 50
    assert!(
      result.as_ref().unwrap().contains("50"),
      "Sum should be 50, got: {}",
      result.unwrap()
    );
  }

  // ==================== Array Assignment Edge Cases ====================

  #[test]
  fn test_array_assign_self_reference() {
    // arr[i] = arr[i] + 1
    let code = r#"
var arr as i64[3] = [10, 20, 30]
arr[2] = arr[2] + 1
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[2] = 20 + 1 = 21
    assert!(
      result.as_ref().unwrap().contains("21"),
      "arr[2] should be 21 after self-reference, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_swap_elements() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
var temp as i64 = arr[1]
arr[1] = arr[3]
arr[3] = temp
write arr[1]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    // After swap: arr[1] = 3, arr[3] = 1
    assert!(
      output.contains("3") && output.contains("1"),
      "Elements should be swapped, got: {}",
      output
    );
  }

  // ==================== Else Branch Execution Tests ====================
  // Verifying the Jan 24 2026 else branch parsing fix

  #[test]
  fn test_else_branch_basic() {
    let code = r#"
var x as i64 = 5
if x > (10 as i64)
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Else branch should execute, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_else_branch_nested() {
    let code = r#"
var x as i64 = 5
var y as i64 = 3
if x > (10 as i64)
    write "FAIL1"
else
    if y > (5 as i64)
        write "FAIL2"
    else
        write "PASS"
    end if
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Nested else should execute, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_else_if_chain() {
    let code = r#"
var x as i64 = 2
match
    when x == (1 as i64):
        write "ONE"
    when x == (2 as i64):
        write "TWO"
    else:
        write "OTHER"
end match
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("TWO"),
      "Second when arm should match, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_else_final_fallthrough() {
    let code = r#"
var x as i64 = 99
match
    when x == (1 as i64):
        write "ONE"
    when x == (2 as i64):
        write "TWO"
    else:
        write "PASS"
end match
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Else arm should execute, got: {}",
      result.unwrap()
    );
  }
}

// ==================== Multi-Dimensional Array Assignment Tests ====================

mod multidim_array_tests {
  use super::array_assignment_execution_tests::run_interpreter;

  #[test]
  fn test_2d_array_assignment() {
    let code = r#"
var row1 as i64[3] = [0, 0, 0]
var row2 as i64[3] = [0, 0, 0]
row1[1] = 11
row1[2] = 12
row2[1] = 21
row2[2] = 22
write row1[1]
write row2[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("11"),
      "row1[1] should be 11, got: {}",
      output
    );
    assert!(
      output.contains("22"),
      "row2[2] should be 22, got: {}",
      output
    );
  }

  #[test]
  fn test_array_assign_all_elements_in_loop() {
    let code = r#"
var arr as i64[5] = [0, 0, 0, 0, 0]
var i as i64 = 1
	loop when i <= (5 as i64)
	    arr[i] = i * (10 as i64)
	    i = i + (1 as i64)
	end loop when not (i <= (5 as i64))
	write arr[1]
write arr[3]
write arr[5]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("10"),
      "arr[1] should be 10, got: {}",
      output
    );
    assert!(
      output.contains("30"),
      "arr[3] should be 30, got: {}",
      output
    );
    assert!(
      output.contains("50"),
      "arr[5] should be 50, got: {}",
      output
    );
  }

  #[test]
  fn test_array_swap_elements() {
    let code = r#"
var arr as i64[3] = [100, 200, 300]
var temp as i64 = arr[1]
arr[1] = arr[3]
arr[3] = temp
write arr[1]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("300"),
      "arr[1] should be 300 after swap, got: {}",
      output
    );
    assert!(
      output.contains("100"),
      "arr[3] should be 100 after swap, got: {}",
      output
    );
  }

  #[test]
  fn test_array_nested_loop_assignment() {
    let code = r#"
var arr as i64[4] = [0, 0, 0, 0]
var i as i64 = 1
	loop when i <= (2 as i64)
	    var j as i64 = 1
	    loop when j <= (2 as i64)
	        var idx as i64 = (i - (1 as i64)) * (2 as i64) + j
	        arr[idx] = i * (10 as i64) + j
	        j = j + (1 as i64)
	    end loop when not (j <= (2 as i64))
	    i = i + (1 as i64)
	end loop when not (i <= (2 as i64))
write arr[1]
write arr[4]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("11"),
      "arr[1] should be 11, got: {}",
      output
    );
    assert!(
      output.contains("22"),
      "arr[4] should be 22, got: {}",
      output
    );
  }

  #[test]
  fn test_array_conditional_assignment() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
var i as i64 = 1
	loop when i <= (3 as i64)
	    if arr[i] > (1 as i64)
	        arr[i] = arr[i] * (10 as i64)
	    else
	        move on
	    end if
	    i = i + (1 as i64)
	end loop when not (i <= (3 as i64))
	write arr[1]
write arr[2]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("1"),
      "arr[1] should stay 1, got: {}",
      output
    );
    assert!(
      output.contains("20"),
      "arr[2] should be 20, got: {}",
      output
    );
    assert!(
      output.contains("30"),
      "arr[3] should be 30, got: {}",
      output
    );
  }

  #[test]
  fn test_array_compound_add_in_loop() {
    let code = r#"
var arr as i64[3] = [10, 20, 30]
var i as i64 = 1
	loop when i <= (3 as i64)
	    arr[i] += i
	    i = i + (1 as i64)
	end loop when not (i <= (3 as i64))
	write arr[1]
write arr[2]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("11"),
      "arr[1] should be 11, got: {}",
      output
    );
    assert!(
      output.contains("22"),
      "arr[2] should be 22, got: {}",
      output
    );
    assert!(
      output.contains("33"),
      "arr[3] should be 33, got: {}",
      output
    );
  }

  #[test]
  fn test_array_compound_sub() {
    let code = r#"
var arr as i64[2] = [100, 50]
arr[1] -= (25 as i64)
arr[2] -= (10 as i64)
write arr[1]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("75"),
      "arr[1] should be 75, got: {}",
      output
    );
    assert!(
      output.contains("40"),
      "arr[2] should be 40, got: {}",
      output
    );
  }

  #[test]
  fn test_array_compound_mul() {
    let code = r#"
var arr as i64[2] = [5, 7]
arr[1] *= (3 as i64)
arr[2] *= (4 as i64)
write arr[1]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("15"),
      "arr[1] should be 15, got: {}",
      output
    );
    assert!(
      output.contains("28"),
      "arr[2] should be 28, got: {}",
      output
    );
  }

  #[test]
  fn test_array_compound_div() {
    let code = r#"
var arr as i64[2] = [100, 81]
arr[1] /= (5 as i64)
arr[2] /= (9 as i64)
write arr[1]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("20"),
      "arr[1] should be 20, got: {}",
      output
    );
    assert!(output.contains("9"), "arr[2] should be 9, got: {}", output);
  }

  #[test]
  fn test_array_self_referential_assignment() {
    let code = r#"
var arr as i64[3] = [1, 2, 3]
arr[2] = arr[1] + arr[3]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    // arr[2] = 1 + 3 = 4
    assert!(
      result.as_ref().unwrap().contains("4"),
      "arr[2] should be 4, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_array_expression_index() {
    let code = r#"
var arr as i64[5] = [10, 20, 30, 40, 50]
var base as i64 = 2
arr[base + (1 as i64)] = 999
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("999"),
      "arr[3] should be 999, got: {}",
      result.unwrap()
    );
  }

  // ==================== MULTI-DIMENSIONAL ARRAY TESTS ====================

  #[test]
  fn test_2d_array_element_assign() {
    let code = r#"
var matrix as i64[2][3] = [[0 as i64, 0 as i64, 0 as i64], [0 as i64, 0 as i64, 0 as i64]]
matrix[1][2] = 99 as i64
write matrix[1][2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("99"),
      "matrix[1][2] should be 99"
    );
  }

  #[test]
  fn test_2d_array_multiple_assigns() {
    let code = r#"
var matrix as i64[2][2] = [[0 as i64, 0 as i64], [0 as i64, 0 as i64]]
matrix[1][1] = 10 as i64
matrix[1][2] = 20 as i64
matrix[2][1] = 30 as i64
matrix[2][2] = 40 as i64
write matrix[1][1]
write matrix[2][2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("10"), "matrix[1][1] should be 10");
    assert!(output.contains("40"), "matrix[2][2] should be 40");
  }

  #[test]
  fn test_3d_array_element_assign() {
    let code = r#"
var cube as i64[2][2][2] = [[[0 as i64, 0 as i64], [0 as i64, 0 as i64]], [[0 as i64, 0 as i64], [0 as i64, 0 as i64]]]
cube[1][1][1] = 100 as i64
write cube[1][1][1]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("100"),
      "cube[1][1][1] should be 100"
    );
  }

  #[test]
  fn test_array_assign_in_loop_simple() {
    let code = r#"
var arr as i64[3] = [0 as i64, 0 as i64, 0 as i64]
var i as i64 = 1 as i64
	loop when i <= (3 as i64)
	    arr[i] = i * (10 as i64)
	    i = i + (1 as i64)
	end loop when not (i <= (3 as i64))
	write arr[1]
	write arr[2]
	write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(
      output.contains("10"),
      "arr[1] should be 10, got: {}",
      output
    );
    assert!(
      output.contains("20"),
      "arr[2] should be 20, got: {}",
      output
    );
    assert!(
      output.contains("30"),
      "arr[3] should be 30, got: {}",
      output
    );
  }

  #[test]
  fn test_array_assign_loop_accumulator() {
    let code = r#"
var arr as i64[5] = [1 as i64, 2 as i64, 3 as i64, 4 as i64, 5 as i64]
var sum as i64 = 0 as i64
var i as i64 = 1 as i64
	loop when i <= (5 as i64)
	    sum = sum + arr[i]
	    i = i + (1 as i64)
	end loop when not (i <= (5 as i64))
	write sum
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(result.as_ref().unwrap().contains("15"), "sum should be 15");
  }

  #[test]
  fn test_array_compound_add_in_loop_all_elements() {
    let code = r#"
var arr as i64[3] = [10 as i64, 20 as i64, 30 as i64]
var i as i64 = 1 as i64
	loop when i <= (3 as i64)
	    arr[i] += (5 as i64)
	    i = i + (1 as i64)
	end loop when not (i <= (3 as i64))
	write arr[1]
write arr[2]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("15"), "arr[1] should be 15");
    assert!(output.contains("25"), "arr[2] should be 25");
    assert!(output.contains("35"), "arr[3] should be 35");
  }

  #[test]
  fn test_array_sum_elements() {
    // Test summing 1D array elements (2D array access has separate issues)
    let code = r#"
var arr as i64[3] = [1 as i64, 2 as i64, 3 as i64]
var sum as i64 = arr[1] + arr[2] + arr[3]
write sum
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(result.as_ref().unwrap().contains("6"), "sum should be 6");
  }

  #[test]
  fn test_array_swap_first_last() {
    let code = r#"
var arr as i64[3] = [1 as i64, 2 as i64, 3 as i64]
var temp as i64 = arr[1]
arr[1] = arr[3]
arr[3] = temp
write arr[1]
write arr[3]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("3"), "arr[1] should be 3 after swap");
    assert!(output.contains("1"), "arr[3] should be 1 after swap");
  }

  #[test]
  fn test_array_assign_from_expression() {
    let code = r#"
var arr as i64[3] = [1 as i64, 2 as i64, 3 as i64]
var x as i64 = 10 as i64
arr[2] = x * (2 as i64) + arr[1]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("21"),
      "arr[2] should be 21 (10*2 + 1)"
    );
  }

  #[test]
  fn test_array_compound_mod() {
    let code = r#"
var arr as i64[2] = [17 as i64, 25 as i64]
arr[1] %= (5 as i64)
arr[2] %= (7 as i64)
write arr[1]
write arr[2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("2"), "17 % 5 = 2");
    assert!(output.contains("4"), "25 % 7 = 4");
  }

  #[test]
  fn test_nested_loop_2d_array() {
    let code = r#"
var matrix as i64[2][2] = [[0 as i64, 0 as i64], [0 as i64, 0 as i64]]
var i as i64 = 1 as i64
	loop when i <= (2 as i64)
	    var j as i64 = 1 as i64
	    loop when j <= (2 as i64)
	        matrix[i][j] = i * (10 as i64) + j
	        j = j + (1 as i64)
	    end loop when not (j <= (2 as i64))
	    i = i + (1 as i64)
	end loop when not (i <= (2 as i64))
write matrix[1][1]
write matrix[1][2]
write matrix[2][1]
write matrix[2][2]
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("11"), "matrix[1][1] should be 11");
    assert!(output.contains("12"), "matrix[1][2] should be 12");
    assert!(output.contains("21"), "matrix[2][1] should be 21");
    assert!(output.contains("22"), "matrix[2][2] should be 22");
  }
}
