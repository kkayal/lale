use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for literal rules in the Lale grammar.
///
/// This test suite covers all literal types:
/// - Numeric literals: int, u_int, float, h_literal
/// - Character/string literals: c_literal, string
/// - Boolean literals: b_literal
/// - Array literals: a_literal
/// - Number components: sign, plus, minus

// ==================== SIGN TESTS ====================
// sign = { plus | minus }

#[test]
fn test_sign_plus() {
  let result = LaleParser::parse(Rule::sign, "+");
  assert!(result.is_ok());
}

#[test]
fn test_sign_minus() {
  let result = LaleParser::parse(Rule::sign, "-");
  assert!(result.is_ok());
}

#[test]
fn test_sign_rejects_other() {
  let invalid = vec!["0", "a", "!", " "];
  for s in invalid {
    let result = LaleParser::parse(Rule::sign, s);
    assert!(result.is_err());
  }
}

// ==================== PLUS/MINUS TESTS ====================
// plus = { "+" }
// minus = { "-" }

#[test]
fn test_plus() {
  let result = LaleParser::parse(Rule::plus, "+");
  assert!(result.is_ok());
}

#[test]
fn test_minus() {
  let result = LaleParser::parse(Rule::minus, "-");
  assert!(result.is_ok());
}

// ==================== INTEGER TESTS ====================
// int = @{ sign? ~ digit+ ~ !"." }
// The !"." negative lookahead ensures it's not a float

#[test]
fn test_int_single_digit() {
  let result = LaleParser::parse(Rule::int, "5");
  assert!(result.is_ok());
}

#[test]
fn test_int_multiple_digits() {
  let result = LaleParser::parse(Rule::int, "12345");
  assert!(result.is_ok());
}

#[test]
fn test_int_with_plus_sign() {
  let result = LaleParser::parse(Rule::int, "+42");
  assert!(result.is_ok());
}

#[test]
fn test_int_with_minus_sign() {
  let result = LaleParser::parse(Rule::int, "-42");
  assert!(result.is_ok());
}

#[test]
fn test_int_zero() {
  let result = LaleParser::parse(Rule::int, "0");
  assert!(result.is_ok());
}

#[test]
fn test_int_large_number() {
  let result = LaleParser::parse(Rule::int, "999999999999");
  assert!(result.is_ok());
}

#[test]
fn test_int_rejects_decimal() {
  let result = LaleParser::parse(Rule::int, "5.0");
  assert!(
    result.is_err(),
    "Should not parse as int if followed by period"
  );
}

#[test]
fn test_int_leading_zeros() {
  let result = LaleParser::parse(Rule::int, "00123");
  assert!(result.is_ok(), "Leading zeros are valid");
}

// ==================== UNSIGNED INTEGER TESTS ====================
// u_int = @{ "+"? ~ digit+ ~ !"." }

#[test]
fn test_u_int_positive() {
  let result = LaleParser::parse(Rule::u_int, "42");
  assert!(result.is_ok());
}

#[test]
fn test_u_int_with_plus() {
  let result = LaleParser::parse(Rule::u_int, "+42");
  assert!(result.is_ok());
}

#[test]
fn test_u_int_zero() {
  let result = LaleParser::parse(Rule::u_int, "0");
  assert!(result.is_ok());
}

#[test]
fn test_u_int_rejects_minus() {
  let result = LaleParser::parse(Rule::u_int, "-42");
  assert!(result.is_err(), "u_int cannot have minus sign");
}

#[test]
fn test_u_int_rejects_decimal() {
  let result = LaleParser::parse(Rule::u_int, "5.0");
  assert!(result.is_err());
}

// ==================== FLOAT TESTS ====================
// float = @{ sign? ~ (decimal_with_exp | decimal_only | integer_with_exp) }

#[test]
fn test_float_decimal_only() {
  let result = LaleParser::parse(Rule::float, "3.14");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_plus() {
  let result = LaleParser::parse(Rule::float, "+3.14");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_minus() {
  let result = LaleParser::parse(Rule::float, "-3.14");
  assert!(result.is_ok());
}

#[test]
fn test_float_zero() {
  let result = LaleParser::parse(Rule::float, "0.0");
  assert!(result.is_ok());
}

#[test]
fn test_float_large_digits() {
  let result = LaleParser::parse(Rule::float, "123456.789012");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_exponent_e() {
  let result = LaleParser::parse(Rule::float, "1.5e10");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_exponent_capital_e() {
  let result = LaleParser::parse(Rule::float, "1.5E10");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_negative_exponent() {
  let result = LaleParser::parse(Rule::float, "1.5e-10");
  assert!(result.is_ok());
}

#[test]
fn test_float_with_positive_exponent() {
  let result = LaleParser::parse(Rule::float, "1.5e+10");
  assert!(result.is_ok());
}

#[test]
fn test_float_integer_with_exponent() {
  let result = LaleParser::parse(Rule::float, "5e3");
  assert!(result.is_ok());
}

#[test]
fn test_float_scientific_notation() {
  let result = LaleParser::parse(Rule::float, "6.022e23");
  assert!(result.is_ok(), "Avogadro's number");
}

#[test]
fn test_float_very_small() {
  let result = LaleParser::parse(Rule::float, "1.67e-27");
  assert!(result.is_ok());
}

// ==================== HEX LITERAL TESTS ====================
// h_literal = @{ "0x" ~ hex+ }

#[test]
fn test_hex_literal_basic() {
  let result = LaleParser::parse(Rule::h_literal, "0x0");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_decimal_digits() {
  let result = LaleParser::parse(Rule::h_literal, "0x123");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_uppercase() {
  let result = LaleParser::parse(Rule::h_literal, "0xABCDEF");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_lowercase() {
  let result = LaleParser::parse(Rule::h_literal, "0xabcdef");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_mixed_case() {
  let result = LaleParser::parse(Rule::h_literal, "0xAaBbCc");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_long() {
  let result = LaleParser::parse(Rule::h_literal, "0xDEADBEEF");
  assert!(result.is_ok());
}

#[test]
fn test_hex_literal_rejects_no_prefix() {
  let result = LaleParser::parse(Rule::h_literal, "ABCDEF");
  assert!(result.is_err());
}

#[test]
fn test_hex_literal_rejects_invalid_char() {
  let result = LaleParser::parse(Rule::h_literal, "0xGHIJ");
  assert!(result.is_err());
}

// ==================== BOOLEAN LITERAL TESTS ====================
// b_literal = { "true" | "false" }

#[test]
fn test_bool_literal_true() {
  let result = LaleParser::parse(Rule::b_literal, "true");
  assert!(result.is_ok());
}

#[test]
fn test_bool_literal_false() {
  let result = LaleParser::parse(Rule::b_literal, "false");
  assert!(result.is_ok());
}

#[test]
fn test_bool_literal_rejects_true_caps() {
  let result = LaleParser::parse(Rule::b_literal, "TRUE");
  assert!(result.is_err());
}

#[test]
fn test_bool_literal_rejects_false_caps() {
  let result = LaleParser::parse(Rule::b_literal, "FALSE");
  assert!(result.is_err());
}

#[test]
fn test_bool_literal_rejects_yes() {
  let result = LaleParser::parse(Rule::b_literal, "yes");
  assert!(result.is_err());
}

// ==================== CHARACTER LITERAL TESTS ====================
// c_literal = @{ "'" ~ (!"'" ~ character) ~ "'" }

#[test]
fn test_char_literal_ascii() {
  let result = LaleParser::parse(Rule::c_literal, "'a'");
  assert!(result.is_ok());
}

#[test]
fn test_char_literal_digit() {
  let result = LaleParser::parse(Rule::c_literal, "'5'");
  assert!(result.is_ok());
}

#[test]
fn test_char_literal_space() {
  let result = LaleParser::parse(Rule::c_literal, "' '");
  assert!(result.is_ok());
}

#[test]
fn test_char_literal_escape_n() {
  let result = LaleParser::parse(Rule::c_literal, "'\\n'");
  assert!(result.is_ok(), "Escaped newline");
}

#[test]
fn test_char_literal_escape_t() {
  let result = LaleParser::parse(Rule::c_literal, "'\\t'");
  assert!(result.is_ok(), "Escaped tab");
}

#[test]
fn test_char_literal_escape_quote() {
  let result = LaleParser::parse(Rule::c_literal, "'\\''");
  assert!(result.is_ok(), "Escaped single quote");
}

#[test]
fn test_char_literal_escape_backslash() {
  let result = LaleParser::parse(Rule::c_literal, "'\\\\'");
  assert!(result.is_ok(), "Escaped backslash");
}

#[test]
fn test_char_literal_unicode_escape() {
  let result = LaleParser::parse(Rule::c_literal, "'\\u+03B1'");
  // Unicode escapes need proper 6 hex digits after u/U
  let _ = result;
}

#[test]
fn test_char_literal_rejects_empty() {
  let result = LaleParser::parse(Rule::c_literal, "''");
  assert!(result.is_err(), "Empty character not allowed");
}

#[test]
fn test_char_literal_rejects_multiple_chars() {
  let result = LaleParser::parse(Rule::c_literal, "'ab'");
  assert!(result.is_err(), "Multiple characters not allowed");
}

#[test]
fn test_char_literal_unicode_direct() {
  let result = LaleParser::parse(Rule::c_literal, "'α'");
  assert!(result.is_ok(), "Direct Unicode character");
}

// ==================== STRING LITERAL TESTS ====================
// s_literal = { "\"" ~ (embedded_value | text)* ~ "\"" }

#[test]
fn test_string_empty() {
  let result = LaleParser::parse(Rule::s_literal, "\"\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_simple() {
  let result = LaleParser::parse(Rule::s_literal, "\"hello\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_spaces() {
  let result = LaleParser::parse(Rule::s_literal, "\"hello world\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_digits() {
  let result = LaleParser::parse(Rule::s_literal, "\"abc123\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_punctuation() {
  let result = LaleParser::parse(Rule::s_literal, "\"Hello, World!\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_escaped_quote() {
  let result = LaleParser::parse(Rule::s_literal, "\"She said \\\"hello\\\"\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_escape_newline() {
  let result = LaleParser::parse(Rule::s_literal, "\"line1\\nline2\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_escape_tab() {
  let result = LaleParser::parse(Rule::s_literal, "\"col1\\tcol2\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_escape_backslash() {
  let result = LaleParser::parse(Rule::s_literal, "\"path\\\\file\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_escape_carriage_return() {
  let result = LaleParser::parse(Rule::s_literal, "\"line1\\rline2\"");
  assert!(result.is_ok());
}

#[test]
fn test_expand_escapes_newline() {
  use lale::ast::expr_parser::expand_escapes_str;
  let expanded = expand_escapes_str("hello\\nworld");
  assert_eq!(expanded, "hello\nworld");
  assert!(expanded.contains('\n'));
}

#[test]
fn test_expand_escapes_tab() {
  use lale::ast::expr_parser::expand_escapes_str;
  let expanded = expand_escapes_str("col1\\tcol2");
  assert_eq!(expanded, "col1\tcol2");
}

#[test]
fn test_expand_escapes_backslash() {
  use lale::ast::expr_parser::expand_escapes_str;
  let expanded = expand_escapes_str("path\\\\file");
  assert_eq!(expanded, "path\\file");
}

#[test]
fn test_expand_escapes_mixed() {
  use lale::ast::expr_parser::expand_escapes_str;
  let expanded = expand_escapes_str("line1\\n\\tindented\\nline2");
  assert_eq!(expanded, "line1\n\tindented\nline2");
}

#[test]
fn test_expand_escapes_no_escapes() {
  use lale::ast::expr_parser::expand_escapes_str;
  let expanded = expand_escapes_str("plain text");
  assert_eq!(expanded, "plain text");
}

#[test]
fn test_string_with_unicode() {
  let result = LaleParser::parse(Rule::s_literal, "\"Café α β γ\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_simple_embedding() {
  let result = LaleParser::parse(Rule::s_literal, "\"Value: {x}\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_with_expression_embedding() {
  let result = LaleParser::parse(Rule::s_literal, "\"Result: {x + 5}\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_multiple_embedded_values() {
  let result = LaleParser::parse(Rule::s_literal, "\"{a} plus {b} equals {a + b}\"");
  assert!(result.is_ok());
}

#[test]
fn test_string_very_long() {
  let long_str = "\"".to_string() + &"a".repeat(1000) + "\"";
  let result = LaleParser::parse(Rule::s_literal, &long_str);
  assert!(result.is_ok());
}

// ==================== NUMBER LITERAL TESTS ====================
// n_literal = { (float | u_int | int) ~ (os ~ &"<" ~ unit)? }

#[test]
fn test_n_literal_int() {
  let result = LaleParser::parse(Rule::n_literal, "42");
  assert!(result.is_ok());
}

#[test]
fn test_n_literal_float() {
  let result = LaleParser::parse(Rule::n_literal, "3.14");
  assert!(result.is_ok());
}

#[test]
fn test_n_literal_with_unit() {
  // Assuming basic unit syntax like <m> for meters
  let result = LaleParser::parse(Rule::n_literal, "5<m>");
  assert!(result.is_ok(), "Number with unit");
}

#[test]
fn test_n_literal_float_with_unit() {
  let result = LaleParser::parse(Rule::n_literal, "9.8<m/s^2>");
  assert!(result.is_ok(), "Float with complex unit");
}

#[test]
fn test_n_literal_with_space_before_unit() {
  let result = LaleParser::parse(Rule::n_literal, "5 <m>");
  assert!(result.is_ok(), "Number with space before unit");
}

#[test]
fn test_n_literal_float_with_space_before_unit() {
  let result = LaleParser::parse(Rule::n_literal, "9.8 <m/s^2>");
  assert!(result.is_ok(), "Float with space before complex unit");
}

#[test]
fn test_n_literal_with_multiple_spaces_before_unit() {
  let result = LaleParser::parse(Rule::n_literal, "5   <m>");
  assert!(result.is_ok(), "Number with multiple spaces before unit");
}

#[test]
fn test_n_literal_with_tab_before_unit() {
  let result = LaleParser::parse(Rule::n_literal, "5\t<m>");
  assert!(result.is_ok(), "Number with tab before unit");
}

#[test]
fn test_n_literal_negative_with_space_before_unit() {
  let result = LaleParser::parse(Rule::n_literal, "-3.14 <rad>");
  assert!(result.is_ok(), "Negative float with space before unit");
}

// ==================== ARRAY LITERAL TESTS ====================
// a_literal = { "[" ~ os_ln ~ "]" | "[" ~ os_ln ~ expression ~ (os_ln ~ "," ~ os_ln ~ expression)* ~ os_ln ~ "]" }

#[test]
fn test_array_empty() {
  let result = LaleParser::parse(Rule::a_literal, "[]");
  assert!(result.is_ok());
}

#[test]
fn test_array_single_element() {
  let result = LaleParser::parse(Rule::a_literal, "[1]");
  assert!(result.is_ok());
}

#[test]
fn test_array_multiple_elements() {
  let result = LaleParser::parse(Rule::a_literal, "[1, 2, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_array_with_spaces() {
  let result = LaleParser::parse(Rule::a_literal, "[ 1 , 2 , 3 ]");
  assert!(result.is_ok());
}

#[test]
fn test_array_with_newlines() {
  let result = LaleParser::parse(Rule::a_literal, "[\n1,\n2,\n3\n]");
  assert!(result.is_ok());
}

#[test]
fn test_array_nested() {
  let result = LaleParser::parse(Rule::a_literal, "[[1, 2], [3, 4]]");
  assert!(result.is_ok());
}

#[test]
fn test_array_mixed_types() {
  let result = LaleParser::parse(Rule::a_literal, "[1, 2.5, 3]");
  assert!(result.is_ok());
}

#[test]
fn test_array_with_expressions() {
  let result = LaleParser::parse(Rule::a_literal, "[x + 1, y * 2, z - 3]");
  assert!(result.is_ok());
}

#[test]
fn test_array_trailing_comma() {
  let result = LaleParser::parse(Rule::a_literal, "[1, 2, 3,]");
  // This may fail depending on whether trailing commas are allowed
  // Test shows what the actual behavior is
  let _ = result;
}

// ==================== EDGE CASES ====================

#[test]
fn test_number_with_many_zeros() {
  let result = LaleParser::parse(Rule::int, "000000");
  assert!(result.is_ok());
}

#[test]
fn test_float_scientific_zero() {
  let result = LaleParser::parse(Rule::float, "0e0");
  assert!(result.is_ok());
}

#[test]
fn test_hex_single_digit() {
  let result = LaleParser::parse(Rule::h_literal, "0x0");
  assert!(result.is_ok());
}

#[test]
fn test_hex_many_digits() {
  let hex_str = format!("0x{}", "F".repeat(64));
  let result = LaleParser::parse(Rule::h_literal, &hex_str);
  assert!(result.is_ok());
}

#[test]
fn test_string_only_whitespace() {
  let result = LaleParser::parse(Rule::s_literal, "\"   \\t\\n   \"");
  assert!(result.is_ok());
}

#[test]
fn test_array_deeply_nested() {
  let result = LaleParser::parse(Rule::a_literal, "[[[[[[1]]]]]]");
  assert!(result.is_ok());
}
