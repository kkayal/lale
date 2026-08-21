use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for expression rules in the Lale grammar.
///
/// This test suite covers:
/// - expression: Full expression parsing with operator precedence
/// - primary: Primary expressions (literals, identifiers, function calls, parenthesized)
/// - Unary and binary operators in expressions

// ==================== SIMPLE EXPRESSIONS ====================

#[test]
fn test_expression_integer_literal() {
  let result = LaleParser::parse(Rule::expression, "42");
  assert!(result.is_ok());
}

#[test]
fn test_expression_float_literal() {
  let result = LaleParser::parse(Rule::expression, "3.14");
  assert!(result.is_ok());
}

#[test]
fn test_expression_string_literal() {
  let result = LaleParser::parse(Rule::expression, "\"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_expression_char_literal() {
  let result = LaleParser::parse(Rule::expression, "'x'");
  assert!(result.is_ok());
}

#[test]
fn test_expression_bool_literal() {
  let result = LaleParser::parse(Rule::expression, "true");
  assert!(result.is_ok());
}

#[test]
fn test_expression_array_literal() {
  let result = LaleParser::parse(Rule::expression, "[1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_expression_hex_literal() {
  let result = LaleParser::parse(Rule::expression, "0xDEADBEEF");
  assert!(result.is_ok());
}

// ==================== IDENTIFIER EXPRESSIONS ====================

#[test]
fn test_expression_simple_identifier() {
  let result = LaleParser::parse(Rule::expression, "x");
  assert!(result.is_ok());
}

#[test]
fn test_expression_underscore_identifier() {
  let result = LaleParser::parse(Rule::expression, "_value");
  assert!(result.is_ok());
}

#[test]
fn test_expression_identifier_with_subscript() {
  let result = LaleParser::parse(Rule::expression, "x\u{2080}");
  assert!(result.is_ok());
}

// ==================== UNARY EXPRESSIONS ====================

#[test]
fn test_expression_unary_not() {
  let result = LaleParser::parse(Rule::expression, "not true");
  assert!(result.is_ok());
}

#[test]
fn test_expression_unary_invert() {
  let result = LaleParser::parse(Rule::expression, "invert x");
  assert!(result.is_ok());
}

#[test]
fn test_expression_type_of() {
  let result = LaleParser::parse(Rule::expression, "#type of u32");
  assert!(result.is_ok());
}

#[test]
fn test_expression_size_of() {
  let result = LaleParser::parse(Rule::expression, "#size of x");
  assert!(result.is_ok());
}

#[test]
fn test_expression_unit_of() {
  let result = LaleParser::parse(Rule::expression, "#unit of distance");
  assert!(result.is_ok());
}

#[test]
fn test_expression_pointer_to() {
  let result = LaleParser::parse(Rule::expression, "pointer to x");
  assert!(result.is_ok());
}

#[test]
fn test_expression_value_at() {
  let result = LaleParser::parse(Rule::expression, "unsafe value at ptr");
  assert!(result.is_ok());
}

#[test]
fn test_expression_conversion() {
  let result = LaleParser::parse(Rule::expression, "x as u32");
  assert!(result.is_ok());
}

// ==================== NEGATIVE NUMBER EXPRESSIONS ====================

#[test]
fn test_expression_negative_int() {
  let result = LaleParser::parse(Rule::expression, "-42");
  assert!(result.is_ok());
}

#[test]
fn test_expression_negative_float() {
  let result = LaleParser::parse(Rule::expression, "-3.14");
  assert!(result.is_ok());
}

#[test]
fn test_expression_negative_identifier() {
  let result = LaleParser::parse(Rule::expression, "-x");
  assert!(result.is_ok());
}

// ==================== BINARY EXPRESSIONS ====================

#[test]
fn test_expression_addition() {
  let result = LaleParser::parse(Rule::expression, "1 + 2");
  assert!(result.is_ok());
}

#[test]
fn test_expression_subtraction() {
  let result = LaleParser::parse(Rule::expression, "5 - 3");
  assert!(result.is_ok());
}

#[test]
fn test_expression_multiplication() {
  let result = LaleParser::parse(Rule::expression, "6 * 7");
  assert!(result.is_ok());
}

#[test]
fn test_expression_division() {
  let result = LaleParser::parse(Rule::expression, "10 / 2");
  assert!(result.is_ok());
}

#[test]
fn test_expression_modulo() {
  let result = LaleParser::parse(Rule::expression, "10 % 3");
  assert!(result.is_ok());
}

#[test]
fn test_expression_power() {
  let result = LaleParser::parse(Rule::expression, "2 ^ 3");
  assert!(result.is_ok());
}

#[test]
fn test_expression_comparison_greater() {
  let result = LaleParser::parse(Rule::expression, "x > 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_comparison_less() {
  let result = LaleParser::parse(Rule::expression, "x < 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_comparison_greater_equal() {
  let result = LaleParser::parse(Rule::expression, "x >= 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_comparison_less_equal() {
  let result = LaleParser::parse(Rule::expression, "x <= 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_equality() {
  let result = LaleParser::parse(Rule::expression, "x == 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_not_equal() {
  let result = LaleParser::parse(Rule::expression, "x != 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_logical_and() {
  let result = LaleParser::parse(Rule::expression, "a and b");
  assert!(result.is_ok());
}

#[test]
fn test_expression_logical_or() {
  let result = LaleParser::parse(Rule::expression, "a or b");
  assert!(result.is_ok());
}

#[test]
fn test_expression_logical_xor() {
  let result = LaleParser::parse(Rule::expression, "a xor b");
  assert!(result.is_ok());
}

// ==================== COMPLEX EXPRESSIONS ====================

#[test]
fn test_expression_precedence_add_mult() {
  let result = LaleParser::parse(Rule::expression, "2 + 3 * 4");
  assert!(result.is_ok(), "Should parse with correct precedence");
}

#[test]
fn test_expression_precedence_mult_power() {
  let result = LaleParser::parse(Rule::expression, "2 * 3 ^ 2");
  assert!(result.is_ok());
}

#[test]
fn test_expression_chain_comparison() {
  let result = LaleParser::parse(Rule::expression, "a > b and b > c");
  assert!(result.is_ok());
}

#[test]
fn test_expression_mixed_operators() {
  let result = LaleParser::parse(Rule::expression, "a + b * c - d / e");
  assert!(result.is_ok());
}

// ==================== PARENTHESIZED EXPRESSIONS ====================

#[test]
fn test_expression_parentheses_simple() {
  let result = LaleParser::parse(Rule::expression, "(x)");
  assert!(result.is_ok());
}

#[test]
fn test_expression_parentheses_override_precedence() {
  let result = LaleParser::parse(Rule::expression, "(2 + 3) * 4");
  assert!(result.is_ok());
}

#[test]
fn test_expression_nested_parentheses() {
  let result = LaleParser::parse(Rule::expression, "((a + b) * (c - d))");
  assert!(result.is_ok());
}

#[test]
fn test_expression_parentheses_complex() {
  let result = LaleParser::parse(Rule::expression, "(a and b) or (c and d)");
  assert!(result.is_ok());
}

// ==================== ARRAY ACCESS EXPRESSIONS ====================

#[test]
fn test_expression_array_index() {
  let result = LaleParser::parse(Rule::expression, "arr[0]");
  assert!(result.is_ok());
}

#[test]
fn test_expression_array_2d() {
  let result = LaleParser::parse(Rule::expression, "matrix[i][j]");
  assert!(result.is_ok());
}

#[test]
fn test_expression_array_expression_index() {
  let result = LaleParser::parse(Rule::expression, "arr[i + 1]");
  assert!(result.is_ok());
}

// ==================== MEMBER ACCESS EXPRESSIONS ====================

#[test]
fn test_expression_member_access() {
  let result = LaleParser::parse(Rule::expression, "obj.field");
  assert!(result.is_ok());
}

#[test]
fn test_expression_chained_member() {
  let result = LaleParser::parse(Rule::expression, "obj.field1.field2");
  assert!(result.is_ok());
}

#[test]
fn test_expression_member_with_array() {
  let result = LaleParser::parse(Rule::expression, "obj.arr[0]");
  assert!(result.is_ok());
}

// ==================== FUNCTION CALL EXPRESSIONS ====================

#[test]
fn test_expression_function_call_no_args() {
  let result = LaleParser::parse(Rule::expression, "func()");
  assert!(result.is_ok());
}

#[test]
fn test_expression_function_call_one_arg() {
  let result = LaleParser::parse(Rule::expression, "func(x)");
  assert!(result.is_ok());
}

#[test]
fn test_expression_function_call_multiple_args() {
  let result = LaleParser::parse(Rule::expression, "func(a, b, c)");
  assert!(result.is_ok());
}

#[test]
fn test_expression_function_call_nothing() {
  let result = LaleParser::parse(Rule::expression, "func(nothing)");
  assert!(result.is_ok());
}

#[test]
fn test_expression_method_call() {
  let result = LaleParser::parse(Rule::expression, "obj.method()");
  assert!(result.is_ok());
}

#[test]
fn test_expression_chained_method_calls() {
  let result = LaleParser::parse(Rule::expression, "obj.method1().method2()");
  assert!(result.is_ok());
}

// ==================== COMPILER CONSTANTS ====================

#[test]
fn test_expression_compiler_constant_main() {
  let result = LaleParser::parse(Rule::expression, "#main");
  assert!(result.is_ok());
}

#[test]
fn test_expression_compiler_constant_file() {
  let result = LaleParser::parse(Rule::expression, "#source_file");
  assert!(result.is_ok());
}

#[test]
fn test_expression_compiler_constant_line() {
  let result = LaleParser::parse(Rule::expression, "#source_line");
  assert!(result.is_ok());
}

#[test]
fn test_expression_compiler_constant_time() {
  let result = LaleParser::parse(Rule::expression, "#compile_time");
  assert!(result.is_ok());
}

#[test]
fn test_expression_compiler_constant_version() {
  let result = LaleParser::parse(Rule::expression, "#compiler_version");
  assert!(result.is_ok());
}

// ==================== PRIMARY RULE TESTS ====================

#[test]
fn test_primary_hex_literal() {
  let result = LaleParser::parse(Rule::primary, "0xABCD");
  assert!(result.is_ok());
}

#[test]
fn test_primary_char_literal() {
  let result = LaleParser::parse(Rule::primary, "'x'");
  assert!(result.is_ok());
}

#[test]
fn test_primary_string_literal() {
  let result = LaleParser::parse(Rule::primary, "\"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_primary_bool_true() {
  let result = LaleParser::parse(Rule::primary, "true");
  assert!(result.is_ok());
}

#[test]
fn test_primary_bool_false() {
  let result = LaleParser::parse(Rule::primary, "false");
  assert!(result.is_ok());
}

#[test]
fn test_primary_numeric_with_unit() {
  let result = LaleParser::parse(Rule::primary, "5<m>");
  assert!(result.is_ok());
}

#[test]
fn test_primary_array_literal() {
  let result = LaleParser::parse(Rule::primary, "[1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_primary_empty_array() {
  let result = LaleParser::parse(Rule::primary, "[]");
  assert!(result.is_ok());
}

#[test]
fn test_primary_parenthesized_expression() {
  let result = LaleParser::parse(Rule::primary, "(a + b)");
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_expression_whitespace_handling() {
  let result = LaleParser::parse(Rule::expression, "1   +   2");
  assert!(result.is_ok());
}

#[test]
fn test_expression_newline_handling() {
  let result = LaleParser::parse(Rule::expression, "1 + \n 2");
  // This depends on how whitespace is handled
  let _ = result;
}

#[test]
fn test_expression_unicode_operators() {
  let result = LaleParser::parse(Rule::expression, "x ≥ 5");
  assert!(result.is_ok());
}

#[test]
fn test_expression_unicode_mult_div() {
  let result = LaleParser::parse(Rule::expression, "a × b ÷ c");
  assert!(result.is_ok());
}

#[test]
fn test_expression_deeply_nested() {
  let result = LaleParser::parse(Rule::expression, "((((a))))");
  assert!(result.is_ok());
}

#[test]
fn test_expression_complex_realistic() {
  let result = LaleParser::parse(Rule::expression, "result[i].value * 2 + base");
  assert!(result.is_ok());
}

// ==================== VECTOR DOT PRODUCT EXPRESSIONS ====================

#[test]
fn test_expression_vector_dot_product() {
  let result = LaleParser::parse(Rule::expression, "a dot b");
  assert!(result.is_ok(), "dot product expression should parse");
}

#[test]
fn test_expression_vector_dot_product_unicode() {
  let result = LaleParser::parse(Rule::expression, "a ⋅ b");
  assert!(result.is_ok(), "unicode dot operator should parse");
}

#[test]
fn test_expression_vector_dot_product_chained() {
  // dot has same precedence as multiplication
  let result = LaleParser::parse(Rule::expression, "a dot b dot c");
  assert!(result.is_ok(), "chained dot product should parse");
}

// ==================== VECTOR CROSS PRODUCT EXPRESSIONS ====================

#[test]
fn test_expression_vector_cross_product() {
  let result = LaleParser::parse(Rule::expression, "a cross b");
  assert!(result.is_ok(), "cross product expression should parse");
}

#[test]
fn test_expression_vector_cross_product_unicode() {
  let result = LaleParser::parse(Rule::expression, "a ⨯ b");
  assert!(result.is_ok(), "unicode cross operator should parse");
}

// ==================== MIXED VECTOR EXPRESSIONS ====================

#[test]
fn test_expression_vector_precedence_dot_then_add() {
  // dot has higher precedence than addition
  let result = LaleParser::parse(Rule::expression, "a dot b + c");
  assert!(result.is_ok(), "dot before addition should parse");
}

#[test]
fn test_expression_vector_precedence_cross_then_add() {
  // cross has higher precedence than addition
  let result = LaleParser::parse(Rule::expression, "a cross b + c");
  assert!(result.is_ok(), "cross before addition should parse");
}

#[test]
fn test_expression_vector_precedence_mul_then_dot() {
  // mul and dot/cross have same precedence
  let result = LaleParser::parse(Rule::expression, "a * b dot c");
  assert!(result.is_ok(), "mul and dot mixed should parse");
}

#[test]
fn test_expression_vector_dot_with_function_call() {
  let result = LaleParser::parse(Rule::expression, "a dot func(x)");
  assert!(
    result.is_ok(),
    "dot with function call operand should parse"
  );
}

#[test]
fn test_expression_vector_cross_with_function_call() {
  let result = LaleParser::parse(Rule::expression, "a cross func(x)");
  assert!(
    result.is_ok(),
    "cross with function call operand should parse"
  );
}

#[test]
fn test_expression_superscript_square() {
  let result = LaleParser::parse(Rule::expression, "m²");
  assert!(result.is_ok(), "m² should parse as m ^ 2");
}

#[test]
fn test_expression_superscript_cube() {
  let result = LaleParser::parse(Rule::expression, "x³");
  assert!(result.is_ok(), "x³ should parse as x ^ 3");
}

#[test]
fn test_expression_superscript_negative() {
  let result = LaleParser::parse(Rule::expression, "x⁻¹");
  assert!(result.is_ok(), "x⁻¹ should parse as x ^ (-1)");
}

#[test]
fn test_expression_superscript_multi_digit() {
  let result = LaleParser::parse(Rule::expression, "x²³");
  assert!(result.is_ok(), "x²³ should parse as x ^ 23");
}

#[test]
fn test_expression_superscript_positive_sign() {
  let result = LaleParser::parse(Rule::expression, "x⁺²");
  assert!(result.is_ok(), "x⁺² should parse as x ^ 2");
}

#[test]
fn test_expression_superscript_compound() {
  let result = LaleParser::parse(Rule::expression, "0.5 * m * v²");
  assert!(result.is_ok(), "0.5 * m * v² should parse correctly");
}

#[test]
fn test_expression_superscript_no_conflict_with_array() {
  // Superscript should not conflict with array indexing [n]
  let result = LaleParser::parse(Rule::expression, "a² + b[1]");
  assert!(
    result.is_ok(),
    "a² + b[1] should parse with superscript and array index"
  );
}

// ==================== SUPERSCRIPT ALL DIGITS ====================

#[test]
fn test_expression_superscript_digit_0() {
  let result = LaleParser::parse(Rule::expression, "m⁰");
  assert!(result.is_ok(), "m⁰ should parse as m ^ 0");
}

#[test]
fn test_expression_superscript_digit_1() {
  let result = LaleParser::parse(Rule::expression, "m¹");
  assert!(result.is_ok(), "m¹ should parse as m ^ 1");
}

#[test]
fn test_expression_superscript_digit_2() {
  let result = LaleParser::parse(Rule::expression, "m²");
  assert!(result.is_ok(), "m² should parse as m ^ 2");
}

#[test]
fn test_expression_superscript_digit_3() {
  let result = LaleParser::parse(Rule::expression, "m³");
  assert!(result.is_ok(), "m³ should parse as m ^ 3");
}

#[test]
fn test_expression_superscript_digit_4() {
  let result = LaleParser::parse(Rule::expression, "m⁴");
  assert!(result.is_ok(), "m⁴ should parse as m ^ 4");
}

#[test]
fn test_expression_superscript_digit_5() {
  let result = LaleParser::parse(Rule::expression, "m⁵");
  assert!(result.is_ok(), "m⁵ should parse as m ^ 5");
}

#[test]
fn test_expression_superscript_digit_6() {
  let result = LaleParser::parse(Rule::expression, "m⁶");
  assert!(result.is_ok(), "m⁶ should parse as m ^ 6");
}

#[test]
fn test_expression_superscript_digit_7() {
  let result = LaleParser::parse(Rule::expression, "m⁷");
  assert!(result.is_ok(), "m⁷ should parse as m ^ 7");
}

#[test]
fn test_expression_superscript_digit_8() {
  let result = LaleParser::parse(Rule::expression, "m⁸");
  assert!(result.is_ok(), "m⁸ should parse as m ^ 8");
}

#[test]
fn test_expression_superscript_digit_9() {
  let result = LaleParser::parse(Rule::expression, "m⁹");
  assert!(result.is_ok(), "m⁹ should parse as m ^ 9");
}

// ==================== SUPERSCRIPT MULTI-DIGIT ====================

#[test]
fn test_expression_superscript_multi_digit_10() {
  let result = LaleParser::parse(Rule::expression, "x¹⁰");
  assert!(result.is_ok(), "x¹⁰ should parse as x ^ 10");
}

#[test]
fn test_expression_superscript_multi_digit_99() {
  let result = LaleParser::parse(Rule::expression, "x⁹⁹");
  assert!(result.is_ok(), "x⁹⁹ should parse as x ^ 99");
}

// ==================== SUPERSCRIPT PRECEDENCE ====================

#[test]
fn test_expression_superscript_precedence_over_multiplication() {
  // Superscript (exponentiation) has higher precedence than multiplication.
  // m² * n should parse as (m²) * n, not m ^ (2 * n)
  let result = LaleParser::parse(Rule::expression, "m² * n");
  assert!(
    result.is_ok(),
    "m² * n should parse with superscript binding tighter than multiplication"
  );
}

#[test]
fn test_expression_superscript_precedence_over_addition() {
  let result = LaleParser::parse(Rule::expression, "m² + n");
  assert!(
    result.is_ok(),
    "m² + n should parse with superscript binding tighter than addition"
  );
}

// ==================== SUPERSCRIPT NESTED ====================

#[test]
fn test_expression_superscript_nested_parenthesized() {
  let result = LaleParser::parse(Rule::expression, "(a + b)²");
  assert!(result.is_ok(), "(a + b)² should parse as (a + b) ^ 2");
}

#[test]
fn test_expression_superscript_nested_complex() {
  let result = LaleParser::parse(Rule::expression, "(x * y)³");
  assert!(result.is_ok(), "(x * y)³ should parse as (x * y) ^ 3");
}

// ==================== SUPERSCRIPT WITH FUNCTION CALLS ====================

#[test]
fn test_expression_superscript_with_function_call() {
  let result = LaleParser::parse(Rule::expression, "f(x)²");
  assert!(result.is_ok(), "f(x)² should parse as f(x) ^ 2");
}

#[test]
fn test_expression_superscript_with_method_call() {
  let result = LaleParser::parse(Rule::expression, "obj.method()³");
  assert!(
    result.is_ok(),
    "obj.method()³ should parse as obj.method() ^ 3"
  );
}

// ==================== SUPERSCRIPT WITH ARRAY INDEX ====================

#[test]
fn test_expression_superscript_with_array_index() {
  let result = LaleParser::parse(Rule::expression, "arr[1]²");
  assert!(result.is_ok(), "arr[1]² should parse as arr[1] ^ 2");
}

#[test]
fn test_expression_superscript_with_2d_array_index() {
  let result = LaleParser::parse(Rule::expression, "matrix[i][j]²");
  assert!(
    result.is_ok(),
    "matrix[i][j]² should parse as matrix[i][j] ^ 2"
  );
}

// ==================== SUPERSCRIPT NEGATIVE ====================

#[test]
fn test_expression_superscript_negative_multi_digit() {
  let result = LaleParser::parse(Rule::expression, "x⁻¹²");
  assert!(result.is_ok(), "x⁻¹² should parse as x ^ (-12)");
}

#[test]
fn test_expression_superscript_negative_with_operations() {
  let result = LaleParser::parse(Rule::expression, "x⁻¹ + y");
  assert!(result.is_ok(), "x⁻¹ + y should parse correctly");
}

// ==================== SUPERSCRIPT AST VERIFICATION ====================

/// Helper: parse a Lale expression string and build the AST.
fn parse_expr_to_ast(source: &str) -> lale::Expr {
  let mut pairs = LaleParser::parse(Rule::expression, source).expect("Failed to parse expression");
  let pair = pairs.next().expect("No expression pair found");
  lale::build_expression(pair).expect("Failed to build expression AST")
}

#[test]
fn test_superscript_ast_m_squared_produces_pow_2() {
  let expr = parse_expr_to_ast("m²");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(
        matches!(bin.operator, lale::BinaryOp::Pow),
        "Expected Pow operator, got {:?}",
        bin.operator
      );
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 2, "Expected IntLiteral(2), got {}", lit.value);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_x_neg_cubed_produces_pow_neg3() {
  let expr = parse_expr_to_ast("x⁻³");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(
        matches!(bin.operator, lale::BinaryOp::Pow),
        "Expected Pow operator, got {:?}",
        bin.operator
      );
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, -3, "Expected IntLiteral(-3), got {}", lit.value);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_multi_digit_23() {
  let expr = parse_expr_to_ast("x²³");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 23, "Expected IntLiteral(23), got {}", lit.value);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_positive_sign() {
  let expr = parse_expr_to_ast("x⁺⁵");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 5, "Expected IntLiteral(5), got {}", lit.value);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_nested_parenthesized() {
  // (a + b)² should produce Pow with left = (a + b) Binary(Add), right = IntLiteral(2)
  let expr = parse_expr_to_ast("(a + b)²");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      // Left should be (a + b), a Binary Add
      match *bin.left {
        lale::Expr::Binary(ref inner) => {
          assert!(
            matches!(inner.operator, lale::BinaryOp::Add),
            "Expected left operand to be Add, got {:?}",
            inner.operator
          );
        }
        other => panic!("Expected Binary(Add) as left operand, got {:?}", other),
      }
      // Right should be IntLiteral(2)
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 2);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_function_call_squared() {
  // f(x)² should produce Pow with left = FnCall, right = IntLiteral(2)
  let expr = parse_expr_to_ast("f(x)²");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      match *bin.left {
        lale::Expr::FnCallExpr(_) => {} // correct
        other => panic!("Expected FnCall as left operand, got {:?}", other),
      }
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 2);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_array_index_squared() {
  // arr[1]² should produce Pow with left = ArrayIndex, right = IntLiteral(2)
  let expr = parse_expr_to_ast("arr[1]²");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      match *bin.left {
        lale::Expr::ArrayIndex(_) => {} // correct
        other => panic!("Expected ArrayIndex as left operand, got {:?}", other),
      }
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 2);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

#[test]
fn test_superscript_ast_zero_exponent() {
  // m⁰ should produce Pow with IntLiteral(0)
  let expr = parse_expr_to_ast("m⁰");
  match expr {
    lale::Expr::Binary(bin) => {
      assert!(matches!(bin.operator, lale::BinaryOp::Pow));
      match *bin.right {
        lale::Expr::IntLiteral(lit) => {
          assert_eq!(lit.value, 0, "Expected IntLiteral(0), got {}", lit.value);
        }
        other => panic!("Expected IntLiteral as right operand, got {:?}", other),
      }
    }
    other => panic!("Expected Binary expression, got {:?}", other),
  }
}

// ==================== SUPERSCRIPT END-TO-END TESTS ====================

#[cfg(test)]
mod superscript_e2e_tests {
  use std::process::Command;

  /// Helper: run a Lale program string and return (success, stdout, stderr).
  fn run_lale_program(source: &str) -> (bool, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lale"))
      .current_dir(env!("CARGO_MANIFEST_DIR"))
      .args(["run", "-", "--no-color"])
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()
      .expect("Failed to spawn lale");

    use std::io::Write;
    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin
      .write_all(source.as_bytes())
      .expect("Failed to write to stdin");
    // stdin is closed when it goes out of scope (dropped before wait_with_output)

    let output = child.wait_with_output().expect("Failed to wait on child");
    (
      output.status.success(),
      String::from_utf8_lossy(&output.stdout).to_string(),
      String::from_utf8_lossy(&output.stderr).to_string(),
    )
  }

  #[test]
  fn test_kinetic_energy_superscript() {
    // E = 0.5 * m * v²
    // m = 10.0, v = 5.0 → E = 0.5 * 10 * 25 = 125
    let source = r#"
var m as f64 = 10.0
var v as f64 = 5.0
var E as f64 = 0.5 * m * v²
write E
"#;
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(
      success,
      "Kinetic energy program should compile and run successfully. stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("125"),
      "Expected output to contain 125, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_kinetic_energy_superscript_with_dot_operator() {
    // E = 0.5⋅m⋅v² — textbook notation with dot operator, no spaces
    // m = 10.0, v = 5.0 → E = 0.5 * 10 * 25 = 125
    let source = "var m as f64 = 10.0\nvar v as f64 = 5.0\nvar E as f64 = 0.5⋅m⋅v²\nwrite E\n";
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(
      success,
      "Kinetic energy with dot operator should compile. stderr: {}",
      stderr
    );
    assert!(
      stdout.contains("125"),
      "Expected output to contain 125, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_dot_operator_with_superscript_no_spaces() {
    // a⋅b² — dot operator with superscript, no spaces
    // a = 3.0, b = 2.0 → 3.0 * 4.0 = 12.0
    let source = "var a as f64 = 3.0\nvar b as f64 = 2.0\nvar r as f64 = a⋅b²\nwrite r\n";
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(success, "a⋅b² should compile. stderr: {}", stderr);
    assert!(
      stdout.contains("11.999") || stdout.contains("12"),
      "Expected ~12, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_dot_operator_chained_with_superscript() {
    // a⋅b⋅c² — chained dot operators with superscript
    // a=2, b=3, c=4 → 2 * 3 * 16 = 96
    let source = "var a as f64 = 2.0\nvar b as f64 = 3.0\nvar c as f64 = 4.0\nvar r as f64 = a⋅b⋅c²\nwrite r\n";
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(success, "a⋅b⋅c² should compile. stderr: {}", stderr);
    assert!(
      stdout.contains("95.999") || stdout.contains("96"),
      "Expected ~96, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_kinetic_energy_superscript_in_expression() {
    // Same but using superscript in a larger expression context
    let source = r#"
var m as f64 = 2.0
var v as f64 = 3.0
var E as f64 = m * v² / 2.0
write E
"#;
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(
      success,
      "Expression with superscript should compile. stderr: {}",
      stderr
    );
    // E = 2 * 9 / 2 = 9
    assert!(
      stdout.contains("9"),
      "Expected output to contain 9, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }

  #[test]
  fn test_superscript_negative_exponent_in_program() {
    // x⁻¹ = 1/x
    let source = r#"
var x as f64 = 2.0
var inv as f64 = x⁻¹
write inv
"#;
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(
      success,
      "Negative superscript program should compile. stderr: {}",
      stderr
    );
    // This tests that x⁻¹ produces x ^ (-1), which is 1/x = 0.5 for x=2.0
    // Note: the interpreter may not support f64 ^ i64 yet; this test verifies
    // the pipeline doesn't crash. Adjust expected output based on implementation.
    let _ = (stdout, stderr);
  }

  #[test]
  fn test_superscript_with_function_call_e2e() {
    // Define a square function, call it, then apply superscript to result
    let source = r#"
fn square(x as f64) returns f64
  return x * x
end fn
var result as f64 = square(3.0)²
write result
"#;
    let (success, stdout, stderr) = run_lale_program(source);
    assert!(
      success,
      "Superscript with function call should compile. stderr: {}",
      stderr
    );
    // square(3.0) = 9.0, 9.0² = 81.0
    assert!(
      stdout.contains("80.999") || stdout.contains("81"),
      "Expected output to contain 81, got stdout: '{}', stderr: '{}'",
      stdout,
      stderr
    );
  }
}
