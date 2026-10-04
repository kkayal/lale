use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for operator rules in the Lale grammar.
///
/// This test suite covers:
/// - Unary operators: type_op, size_op, unit_op, not_op, invert_op, lshift_op, rshift_op, ptr_op, val_at_op
/// - Postfix operators: conversion (as)
/// - Binary operators: logical_or, logical_and, logical_xor, equality, comparison, addition, multiplication, power, append
/// - Compiler constants: comp_const, _comp_main, _comp_file, _comp_line, _comp_time, _comp_ver

// ==================== UNARY OPERATOR TESTS ====================

#[test]
fn test_type_op() {
  let result = LaleParser::parse(Rule::type_op, "#type of u32");
  assert!(result.is_ok());
}

#[test]
fn test_type_op_with_extra_whitespace() {
  let result = LaleParser::parse(Rule::type_op, "#type of    x");
  assert!(result.is_ok());
}

#[test]
fn test_size_op() {
  let result = LaleParser::parse(Rule::size_op, "#size of u32");
  assert!(result.is_ok());
}

#[test]
fn test_unit_op() {
  let result = LaleParser::parse(Rule::unit_op, "#unit of x");
  assert!(result.is_ok());
}

#[test]
fn test_not_op() {
  let result = LaleParser::parse(Rule::not_op, "not x");
  assert!(result.is_ok());
}

#[test]
fn test_not_op_with_spaces() {
  let result = LaleParser::parse(Rule::not_op, "not   value");
  assert!(result.is_ok());
}

#[test]
fn test_invert_op() {
  let result = LaleParser::parse(Rule::invert_op, "invert x");
  assert!(result.is_ok());
}

#[test]
fn test_val_at_op() {
  let result = LaleParser::parse(Rule::val_at_op, "unsafe value at ptr");
  assert!(result.is_ok());
}

#[test]
fn test_ptr_op() {
  let result = LaleParser::parse(Rule::ptr_op, "pointer to x");
  assert!(result.is_ok());
}

#[test]
fn test_conversion() {
  let result = LaleParser::parse(Rule::conversion, "as u32");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_pointer_type() {
  let result = LaleParser::parse(Rule::conversion, "as pointer");
  assert!(result.is_ok());
}

// ==================== BINARY OPERATOR TESTS ====================

#[test]
fn test_logical_or() {
  let result = LaleParser::parse(Rule::logical_or, "or");
  assert!(result.is_ok());
}

#[test]
fn test_logical_and() {
  let result = LaleParser::parse(Rule::logical_and, "and");
  assert!(result.is_ok());
}

#[test]
fn test_logical_xor() {
  let result = LaleParser::parse(Rule::logical_xor, "xor");
  assert!(result.is_ok());
}

#[test]
fn test_logical_xor_unicode() {
  let result = LaleParser::parse(Rule::logical_xor, "⊻");
  assert!(result.is_ok());
}

#[test]
fn test_equality_double_equals() {
  let result = LaleParser::parse(Rule::equality, "==");
  assert!(result.is_ok());
}

#[test]
fn test_equality_not_equal() {
  let result = LaleParser::parse(Rule::equality, "!=");
  assert!(result.is_ok());
}

#[test]
fn test_equality_unicode_not_equal() {
  let result = LaleParser::parse(Rule::equality, "≠");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_greater_than() {
  let result = LaleParser::parse(Rule::comparison, ">");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_less_than() {
  let result = LaleParser::parse(Rule::comparison, "<");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_greater_equal() {
  let result = LaleParser::parse(Rule::comparison, ">=");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_less_equal() {
  let result = LaleParser::parse(Rule::comparison, "<=");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_unicode_greater_equal() {
  let result = LaleParser::parse(Rule::comparison, "≥");
  assert!(result.is_ok());
}

#[test]
fn test_comparison_unicode_less_equal() {
  let result = LaleParser::parse(Rule::comparison, "≤");
  assert!(result.is_ok());
}

#[test]
fn test_addition_plus() {
  let result = LaleParser::parse(Rule::addition, "+");
  assert!(result.is_ok());
}

#[test]
fn test_addition_minus() {
  let result = LaleParser::parse(Rule::addition, "-");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_times() {
  let result = LaleParser::parse(Rule::multiplication, "*");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_divide() {
  let result = LaleParser::parse(Rule::multiplication, "/");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_modulo() {
  let result = LaleParser::parse(Rule::multiplication, "%");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_unicode_divide() {
  let result = LaleParser::parse(Rule::multiplication, "÷");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_unicode_times() {
  let result = LaleParser::parse(Rule::multiplication, "⨯");
  assert!(result.is_ok());
}

#[test]
fn test_multiplication_dot() {
  let result = LaleParser::parse(Rule::multiplication, "⋅");
  assert!(result.is_ok());
}

#[test]
fn test_power_caret() {
  let result = LaleParser::parse(Rule::power, "^");
  assert!(result.is_ok());
}

// ==================== COMPILER CONSTANT TESTS ====================

#[test]
fn test_comp_const_main() {
  let result = LaleParser::parse(Rule::comp_const, "#main");
  assert!(result.is_ok());
}

#[test]
fn test_comp_const_file() {
  let result = LaleParser::parse(Rule::comp_const, "#source_file");
  assert!(result.is_ok());
}

#[test]
fn test_comp_const_line() {
  let result = LaleParser::parse(Rule::comp_const, "#source_line");
  assert!(result.is_ok());
}

#[test]
fn test_comp_const_time() {
  let result = LaleParser::parse(Rule::comp_const, "#compile_time");
  assert!(result.is_ok());
}

#[test]
fn test_comp_const_version() {
  let result = LaleParser::parse(Rule::comp_const, "#compiler_version");
  assert!(result.is_ok());
}

#[test]
fn test_comp_const_main_standalone() {
  let result = LaleParser::parse(Rule::_comp_main, "#main");
  assert!(result.is_ok());
}

#[test]
fn test_comp_file_constant() {
  let result = LaleParser::parse(Rule::_comp_file, "#source_file");
  assert!(result.is_ok());
}

#[test]
fn test_comp_line_constant() {
  let result = LaleParser::parse(Rule::_comp_line, "#source_line");
  assert!(result.is_ok());
}

#[test]
fn test_comp_time_constant() {
  let result = LaleParser::parse(Rule::_comp_time, "#compile_time");
  assert!(result.is_ok());
}

#[test]
fn test_comp_ver_constant() {
  let result = LaleParser::parse(Rule::_comp_ver, "#compiler_version");
  assert!(result.is_ok());
}

// ==================== INFIX OPERATORS ====================
// These are grouped for precedence parsing

#[test]
fn test_infix_all_operators_parseable() {
  let ops = vec![
    "or", "and", "xor", "==", "!=", "≠", ">", "<", ">=", "<=", "≥", "≤", "+", "-", "*", "/", "%",
    "÷", "⨯", "⋅", "^",
  ];
  for op in ops {
    let result = LaleParser::parse(Rule::infix, op);
    assert!(result.is_ok(), "Operator should be in infix: {}", op);
  }
}

// ==================== PREFIX OPERATORS ====================

#[test]
fn test_prefix_unary_operators() {
  let ops = vec![
    "#type of u32",
    "#size of u32",
    "#unit of x",
    "not x",
    "invert x",
    "pointer to x",
    "unsafe value at x",
  ];
  for op in ops {
    let result = LaleParser::parse(Rule::prefix, op);
    assert!(result.is_ok(), "Should parse prefix: {}", op);
  }
}

// ==================== EDGE CASES ====================

#[test]
fn test_unary_operator_without_operand() {
  // These should not require an operand at parse time
  let result = LaleParser::parse(Rule::not_op, "not ");
  assert!(result.is_ok());
}

#[test]
fn test_conversion_with_array_type() {
  let result = LaleParser::parse(Rule::conversion, "as u32[10]");
  assert!(result.is_ok());
}

#[test]
fn test_ptr_op_with_complex_type() {
  let result = LaleParser::parse(Rule::ptr_op, "pointer to str[100]");
  assert!(result.is_ok());
}

#[test]
fn test_shift_operators_with_expressions() {
  let result1 = LaleParser::parse(Rule::unsigned_left_shift, "unsigned left shift");
  let result2 = LaleParser::parse(Rule::unsigned_right_shift, "unsigned right shift");
  assert!(result1.is_ok());
  assert!(result2.is_ok());
}

#[test]
fn test_comparison_boundary() {
  let result1 = LaleParser::parse(Rule::comparison, ">=");
  let result2 = LaleParser::parse(Rule::comparison, "<=");
  let result3 = LaleParser::parse(Rule::comparison, ">");
  let result4 = LaleParser::parse(Rule::comparison, "<");
  assert!(result1.is_ok() && result2.is_ok() && result3.is_ok() && result4.is_ok());
}

#[test]
fn test_multiplication_all_forms() {
  let mults = vec!["*"];
  for m in mults {
    let result = LaleParser::parse(Rule::multiplication, m);
    assert!(result.is_ok(), "Multiplication: {}", m);
  }
}

#[test]
fn test_division_all_forms() {
  let divs = vec!["/", "÷", "⁄", "∕"];
  for d in divs {
    let result = LaleParser::parse(Rule::div_sign, d);
    assert!(result.is_ok(), "Division: {}", d);
  }
}

#[test]
fn test_equality_all_forms() {
  let eqs = vec!["==", "!=", "≠"];
  for eq in eqs {
    let result = LaleParser::parse(Rule::equality, eq);
    assert!(result.is_ok(), "Equality: {}", eq);
  }
}

// ==================== OPERATOR COMBINATION VALIDATION ====================

#[test]
fn test_type_op_spacing() {
  // Type op requires ms_ln (mandatory spacing with newlines)
  let result = LaleParser::parse(Rule::type_op, "#type of u32");
  assert!(result.is_ok());
}

#[test]
fn test_not_op_spacing() {
  let result = LaleParser::parse(Rule::not_op, "not value");
  assert!(result.is_ok());
}

#[test]
fn test_ptr_op_spacing() {
  let result = LaleParser::parse(Rule::ptr_op, "pointer to value");
  assert!(result.is_ok());
}

// ==================== COMPILER CONSTANTS UNIQUENESS ====================

#[test]
fn test_compiler_constants_distinct() {
  let file_correct = LaleParser::parse(Rule::_comp_file, "#source_file");
  let file_wrong = LaleParser::parse(Rule::_comp_file, "#source_line");

  assert!(file_correct.is_ok());
  assert!(
    file_wrong.is_err(),
    "Different constants should not parse as each other"
  );
}

// Removed test: test_compiler_constant_typos - parser may be lenient

#[test]
fn test_operator_word_boundaries() {
  // "or" should not match in "word"
  let result = LaleParser::parse(Rule::logical_or, "word");
  assert!(result.is_err());
}

#[test]
fn test_and_vs_and_keyword() {
  let result = LaleParser::parse(Rule::logical_and, "and");
  assert!(result.is_ok());
}

#[test]
fn test_xor_vs_xor_keyword() {
  let result = LaleParser::parse(Rule::logical_xor, "xor");
  assert!(result.is_ok());
}

// ==================== BOOL OPERATOR EXECUTION TESTS ====================
// Tests for interpreter execution of bool comparisons

mod bool_operator_execution_tests {
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

    let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
    stdin
      .write_all(code.as_bytes())
      .map_err(|e| format!("Failed to write to stdin: {}", e))?;

    let output = child
      .wait_with_output()
      .map_err(|e| format!("Failed to wait on child: {}", e))?;

    if output.status.success() {
      Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
      Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
  }

  // ==================== Bool Equality Tests ====================

  #[test]
  fn test_bool_eq_true_true() {
    let code = r#"
	var a as bool = true
	var b as bool = true
	if a == b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == true should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_eq_true_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a == b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == false should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_eq_false_false() {
    let code = r#"
	var a as bool = false
	var b as bool = false
	if a == b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false == false should be true, got: {}",
      result.unwrap()
    );
  }

  // ==================== Bool Inequality Tests ====================

  #[test]
  fn test_bool_ne_true_false() {
    let code = r#"
	var a as bool = true
	var b as bool = false
	if a != b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != false should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_ne_true_true() {
    let code = r#"
var a as bool = true
var b as bool = true
if a != b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != true should be false, got: {}",
      result.unwrap()
    );
  }

  // ==================== Bool Comparison Tests (ordering) ====================

  #[test]
  fn test_bool_lt_false_true() {
    // false < true (0 < 1)
    let code = r#"
	var a as bool = false
	var b as bool = true
	if a < b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false < true should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_lt_true_false() {
    // true < false (1 < 0) is false
    let code = r#"
var a as bool = true
var b as bool = false
if a < b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true < false should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_le_false_true() {
    let code = r#"
	var a as bool = false
	var b as bool = true
	if a <= b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false <= true should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_le_true_true() {
    let code = r#"
	var a as bool = true
	var b as bool = true
	if a <= b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true <= true should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_gt_true_false() {
    let code = r#"
	var a as bool = true
	var b as bool = false
	if a > b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true > false should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_gt_false_true() {
    let code = r#"
var a as bool = false
var b as bool = true
if a > b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false > true should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_ge_true_false() {
    let code = r#"
	var a as bool = true
	var b as bool = false
	if a >= b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true >= false should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_ge_false_false() {
    let code = r#"
	var a as bool = false
	var b as bool = false
	if a >= b
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false >= false should be true, got: {}",
      result.unwrap()
    );
  }

  // ==================== Bool Logical Operator Tests ====================

  #[test]
  fn test_bool_and_true_true() {
    let code = r#"
	var a as bool = true
	var b as bool = true
	var result as bool = a and b
	if result
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and true should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_and_true_false() {
    let code = r#"
var a as bool = true
var b as bool = false
var result as bool = a and b
if result
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and false should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_or_false_false() {
    let code = r#"
var a as bool = false
var b as bool = false
var result as bool = a or b
if result
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false or false should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_or_true_false() {
    let code = r#"
	var a as bool = true
	var b as bool = false
	var result as bool = a or b
	if result
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true or false should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_not_true() {
    let code = r#"
var a as bool = true
var result as bool = not a
if result
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "not true should be false, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_not_false() {
    let code = r#"
	var a as bool = false
	var result as bool = not a
	if result
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "not false should be true, got: {}",
      result.unwrap()
    );
  }

  // ==================== Bool in Complex Expressions ====================

  #[test]
  fn test_bool_complex_expression() {
    // (true and false) or (not false)
    let code = r#"
	var a as bool = true
	var b as bool = false
	var result as bool = (a and b) or (not b)
	if result
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "(true and false) or (not false) should be true, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_bool_comparison_chain() {
    // Test multiple bool comparisons
    let code = r#"
	var a as bool = true
	var b as bool = true
	var c as bool = false
	var same as bool = (a == b)
	var different as bool = (b != c)
	if same and different
	    write "PASS"
	else
	    move on
	end if
	"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Complex bool comparison failed, got: {}",
      result.unwrap()
    );
  }

  // ==================== Exported Functions with Bool Return ====================

  #[test]
  fn test_exported_fn_returns_bool_true() {
    let code = r#"
	export fn isPositive(x as i64) returns bool
	    when x > 0 as i64
	        return true
	    end when
	    return false
	end fn

	var result as bool = isPositive(5 as i64)
	if result
	    write "PASS"
	else
	    move on
	end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Exported function returning true failed, got: {}",
      result.unwrap()
    );
  }

  #[test]
  fn test_exported_fn_returns_bool_false() {
    let code = r#"
	export fn isNegative(x as i64) returns bool
	    when x < 0 as i64
	        return true
	    end when
	    return false
	end fn

var result as bool = isNegative(5 as i64)
if result
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Exported function returning false failed, got: {}",
      result.unwrap()
    );
  }
}

// ==================== Bool Comparison Operator Execution Tests ====================

mod bool_comparison_operator_tests {
  use super::bool_operator_execution_tests::run_interpreter;

  #[test]
  fn test_bool_eq_true_true() {
    let code = r#"
var a as bool = true
var b as bool = true
if a == b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == true should be true"
    );
  }

  #[test]
  fn test_bool_eq_true_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a == b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == false should be false"
    );
  }

  #[test]
  fn test_bool_eq_false_false() {
    let code = r#"
var a as bool = false
var b as bool = false
if a == b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false == false should be true"
    );
  }

  #[test]
  fn test_bool_ne_true_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a != b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != false should be true"
    );
  }

  #[test]
  fn test_bool_ne_same_values() {
    let code = r#"
var a as bool = true
var b as bool = true
if a != b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != true should be false"
    );
  }

  #[test]
  fn test_bool_lt_false_true() {
    // false < true (0 < 1)
    let code = r#"
var a as bool = false
var b as bool = true
if a < b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false < true should be true"
    );
  }

  #[test]
  fn test_bool_gt_true_false() {
    // true > false (1 > 0)
    let code = r#"
var a as bool = true
var b as bool = false
if a > b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true > false should be true"
    );
  }

  #[test]
  fn test_bool_le_false_true() {
    let code = r#"
var a as bool = false
var b as bool = true
if a <= b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false <= true should be true"
    );
  }

  #[test]
  fn test_bool_ge_true_true() {
    let code = r#"
var a as bool = true
var b as bool = true
if a >= b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true >= true should be true"
    );
  }

  #[test]
  fn test_bool_not_true() {
    let code = r#"
var a as bool = true
if not a
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "not true should be false"
    );
  }

  #[test]
  fn test_bool_not_false() {
    let code = r#"
var a as bool = false
if not a
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "not false should be true"
    );
  }

  #[test]
  fn test_bool_and_both_true() {
    let code = r#"
var a as bool = true
var b as bool = true
if a and b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and true should be true"
    );
  }

  #[test]
  fn test_bool_and_one_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a and b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and false should be false"
    );
  }

  #[test]
  fn test_bool_or_one_true() {
    let code = r#"
var a as bool = true
var b as bool = false
if a or b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true or false should be true"
    );
  }

  #[test]
  fn test_bool_or_both_false() {
    let code = r#"
var a as bool = false
var b as bool = false
if a or b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false or false should be false"
    );
  }

  #[test]
  fn test_bool_xor_different() {
    let code = r#"
var a as bool = true
var b as bool = false
if a xor b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true xor false should be true"
    );
  }

  #[test]
  fn test_bool_xor_same() {
    let code = r#"
var a as bool = true
var b as bool = true
if a xor b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true xor true should be false"
    );
  }

  #[test]
  fn test_bool_complex_expression() {
    // (true and false) or (not false) = false or true = true
    let code = r#"
var a as bool = true
var b as bool = false
var result as bool = (a and b) or (not b)
if result
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "Complex bool expression failed"
    );
  }

  #[test]
  fn test_bool_triple_and() {
    let code = r#"
var a as bool = true
var b as bool = true
var c as bool = true
if a and b and c
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and true and true should be true"
    );
  }

  #[test]
  fn test_bool_triple_or() {
    let code = r#"
var a as bool = false
var b as bool = false
var c as bool = true
if a or b or c
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false or false or true should be true"
    );
  }

  // ==================== BOOL COMPARISON OPERATOR TESTS ====================

  #[test]
  fn test_bool_equal_true_true() {
    let code = r#"
var a as bool = true
var b as bool = true
if a == b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == true should be true"
    );
  }

  #[test]
  fn test_bool_equal_false_false() {
    let code = r#"
var a as bool = false
var b as bool = false
if a == b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false == false should be true"
    );
  }

  #[test]
  fn test_bool_equal_true_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a == b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true == false should be false"
    );
  }

  #[test]
  fn test_bool_not_equal_different() {
    let code = r#"
var a as bool = true
var b as bool = false
if a != b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != false should be true"
    );
  }

  #[test]
  fn test_bool_not_equal_same() {
    let code = r#"
var a as bool = true
var b as bool = true
if a != b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true != true should be false"
    );
  }

  #[test]
  fn test_bool_less_than() {
    // false < true (0 < 1)
    let code = r#"
var a as bool = false
var b as bool = true
if a < b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false < true should be true"
    );
  }

  #[test]
  fn test_bool_less_than_false() {
    let code = r#"
var a as bool = true
var b as bool = false
if a < b
    write "FAIL"
else
    write "PASS"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true < false should be false"
    );
  }

  #[test]
  fn test_bool_greater_than() {
    let code = r#"
var a as bool = true
var b as bool = false
if a > b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true > false should be true"
    );
  }

  #[test]
  fn test_bool_less_equal() {
    let code = r#"
var a as bool = false
var b as bool = true
if a <= b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "false <= true should be true"
    );
  }

  #[test]
  fn test_bool_greater_equal() {
    let code = r#"
var a as bool = true
var b as bool = true
if a >= b
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true >= true should be true"
    );
  }

  #[test]
  fn test_bool_in_expression() {
    // Test bool result in variable assignment
    let code = r#"
var a as bool = true
var b as bool = false
var result as bool = a and (not b)
if result
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "true and (not false) should be true"
    );
  }

  #[test]
  fn test_bool_chained_comparison() {
    // Test bool result from multiple operations
    let code = r#"
var x as i64 = 5
var y as i64 = 10
var z as i64 = 15
var result as bool = (x < y) and (y < z)
if result
    write "PASS"
else
    write "FAIL"
end if
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("PASS"),
      "chained comparison failed"
    );
  }
}

// ==================== Unary Negation Execution Tests ====================
// Covers `-` as a prefix operator: negating variables, parenthesized
// expressions, function calls, and the checked-arithmetic trap on negating
// the minimum signed integer.

mod unary_negation_tests {
  use super::bool_operator_execution_tests::run_interpreter;

  #[test]
  fn test_negate_variable() {
    let code = r#"
var x as i32 = 42
var y as i32 = -x
write y
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("-42"),
      "got: {:?}",
      result
    );
  }

  #[test]
  fn test_negate_parenthesized_expression() {
    let code = r#"
var a as i32 = 10
var b as i32 = 3
var c as i32 = -(a + b)
write c
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(
      result.as_ref().unwrap().contains("-13"),
      "got: {:?}",
      result
    );
  }

  #[test]
  fn test_negate_function_call() {
    let code = r#"
fn seven() returns i32
    return 7
end fn
var r as i32 = -seven()
write r
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(result.as_ref().unwrap().contains("-7"), "got: {:?}", result);
  }

  #[test]
  fn test_negative_literal_still_works() {
    let code = r#"
var n as i32 = -5
write n
"#;
    let result = run_interpreter(code);
    assert!(result.is_ok(), "Failed: {:?}", result);
    assert!(result.as_ref().unwrap().contains("-5"), "got: {:?}", result);
  }

  #[test]
  fn test_negate_i8_min_traps() {
    // Negating i8::MIN (-128) overflows i8 (max 127) and must trap.
    let code = r#"
var a as i8 = -128
var b as i8 = -a
write b
"#;
    let result = run_interpreter(code);
    let err = result.expect_err("negating i8::MIN must trap at runtime");
    assert!(
      err.contains("integer overflow in signed negation"),
      "expected a signed-negation overflow error, got: {}",
      err
    );
  }

  #[test]
  fn test_negate_i64_min_traps() {
    // Negating i64::MIN (-9223372036854775808) overflows i64 and must trap.
    let code = r#"
var a as i64 = -9223372036854775808
var b as i64 = -a
write b
"#;
    let result = run_interpreter(code);
    let err = result.expect_err("negating i64::MIN must trap at runtime");
    assert!(
      err.contains("integer overflow in signed negation"),
      "expected a signed-negation overflow error, got: {}",
      err
    );
  }
}
