use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for identifier rules in the Lale grammar.
///
/// This test suite covers:
/// - identifier: Complete identifier rule (starts with alpha, followed by identifier_continue)
/// - identifier_continue: Characters valid in identifiers after the first character

// ==================== SIMPLE IDENTIFIERS ====================

#[test]
fn test_identifier_single_ascii_letter() {
  let result = LaleParser::parse(Rule::single_identifier, "a");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_lowercase_word() {
  let result = LaleParser::parse(Rule::single_identifier, "variable");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_uppercase_word() {
  let result = LaleParser::parse(Rule::single_identifier, "CONSTANT");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_mixed_case() {
  let result = LaleParser::parse(Rule::single_identifier, "myVariable");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_underscore_start() {
  let result = LaleParser::parse(Rule::single_identifier, "_private");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_multiple_underscores() {
  let result = LaleParser::parse(Rule::single_identifier, "__dunder__");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_underscore_only() {
  let result = LaleParser::parse(Rule::single_identifier, "_");
  assert!(result.is_ok());
}

// ==================== IDENTIFIERS WITH DIGITS ====================

#[test]
fn test_identifier_with_trailing_digit() {
  let result = LaleParser::parse(Rule::single_identifier, "var1");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_with_multiple_digits() {
  let result = LaleParser::parse(Rule::single_identifier, "x123");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_with_digits_interspersed() {
  let result = LaleParser::parse(Rule::single_identifier, "v1a2r3");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_underscore_with_digits() {
  let result = LaleParser::parse(Rule::single_identifier, "_var123");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_rejects_starting_digit() {
  let result = LaleParser::parse(Rule::single_identifier, "1variable");
  assert!(result.is_err(), "Identifier cannot start with digit");
}

#[test]
fn test_identifier_rejects_starting_special() {
  let invalid = vec!["$var", "@name", "#hash", "!bang", "+plus"];
  for id in invalid {
    let result = LaleParser::parse(Rule::single_identifier, id);
    assert!(
      result.is_err(),
      "Should reject identifier starting with special char: {}",
      id
    );
  }
}

// ==================== UNICODE IDENTIFIERS ====================

#[test]
fn test_identifier_hiragana_start() {
  let result = LaleParser::parse(Rule::single_identifier, "あ");
  assert!(result.is_ok(), "Should accept hiragana character");
}

#[test]
fn test_identifier_hiragana_word() {
  let result = LaleParser::parse(Rule::single_identifier, "あいうえお");
  assert!(result.is_ok(), "Should accept Japanese hiragana word");
}

#[test]
fn test_identifier_greek_start() {
  let result = LaleParser::parse(Rule::single_identifier, "α");
  assert!(result.is_ok(), "Should accept Greek alpha");
}

#[test]
fn test_identifier_greek_word() {
  let result = LaleParser::parse(Rule::single_identifier, "αβγδ");
  assert!(result.is_ok(), "Should accept Greek letters");
}

#[test]
fn test_identifier_mixed_unicode_ascii() {
  let result = LaleParser::parse(Rule::single_identifier, "α_var");
  assert!(result.is_ok(), "Should mix Greek and ASCII");
}

#[test]
fn test_identifier_latin1_chars() {
  let result = LaleParser::parse(Rule::single_identifier, "À");
  assert!(result.is_ok(), "Should accept Latin-1 accented character");
}

#[test]
fn test_identifier_latin1_word() {
  let result = LaleParser::parse(Rule::single_identifier, "café");
  assert!(result.is_ok(), "Should accept café");
}

// ==================== SUBSCRIPT DIGITS IN IDENTIFIERS ====================

#[test]
fn test_identifier_with_subscript_digit() {
  // x₀ - x with subscript 0 (U+2080)
  let result = LaleParser::parse(Rule::single_identifier, "x\u{2080}");
  assert!(result.is_ok(), "Should accept subscript digit");
}

#[test]
fn test_identifier_with_multiple_subscripts() {
  // x₁₂₃
  let result = LaleParser::parse(Rule::single_identifier, "x\u{2081}\u{2082}\u{2083}");
  assert!(result.is_ok(), "Should accept multiple subscript digits");
}

#[test]
fn test_identifier_alpha_subscript_alpha() {
  let result = LaleParser::parse(Rule::single_identifier, "a\u{2080}b");
  assert!(result.is_ok(), "Should mix regular and subscript");
}

#[test]
fn test_identifier_rejects_subscript_start() {
  let result = LaleParser::parse(Rule::single_identifier, "\u{2080}var");
  assert!(result.is_err(), "Cannot start with subscript digit");
}

#[test]
fn test_identifier_underscore_subscript() {
  let result = LaleParser::parse(Rule::single_identifier, "_x\u{2085}");
  assert!(
    result.is_ok(),
    "Should accept subscript after underscore start"
  );
}

// ==================== LONG IDENTIFIERS ====================

#[test]
fn test_identifier_very_long() {
  let long_id = "a".repeat(100);
  let result = LaleParser::parse(Rule::single_identifier, &long_id);
  assert!(result.is_ok(), "Should accept very long identifiers");
}

#[test]
fn test_identifier_mixed_case_long() {
  let long_id = "aAbBcCdDeEfF".repeat(10);
  let result = LaleParser::parse(Rule::single_identifier, &long_id);
  assert!(result.is_ok(), "Should accept long mixed-case identifiers");
}

// ==================== IDENTIFIER BOUNDARIES ====================

#[test]
fn test_identifier_consumed_only_valid_chars() {
  let result = LaleParser::parse(Rule::single_identifier, "var!");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(
    pair.as_str(),
    "var",
    "Should consume only valid identifier chars"
  );
}

#[test]
fn test_identifier_stops_at_whitespace() {
  let result = LaleParser::parse(Rule::single_identifier, "var ");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "var", "Should stop at whitespace");
}

#[test]
fn test_identifier_stops_at_operator() {
  let result = LaleParser::parse(Rule::single_identifier, "var+5");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "var", "Should stop at operator");
}

#[test]
fn test_identifier_stops_at_paren() {
  let result = LaleParser::parse(Rule::single_identifier, "func()");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "func", "Should stop at parenthesis");
}

#[test]
fn test_identifier_stops_at_bracket() {
  let result = LaleParser::parse(Rule::single_identifier, "arr[0]");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "arr", "Should stop at bracket");
}

#[test]
fn test_identifier_stops_at_dot() {
  let result = LaleParser::parse(Rule::single_identifier, "obj.field");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "obj", "Should stop at dot (member access)");
}

// ==================== IDENTIFIER_CONTINUE TESTS ====================

#[test]
fn test_identifier_continue_single_alpha() {
  let result = LaleParser::parse(Rule::identifier_continue, "a");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_continue_single_digit() {
  let result = LaleParser::parse(Rule::identifier_continue, "5");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_continue_subscript() {
  let result = LaleParser::parse(Rule::identifier_continue, "\u{2085}");
  assert!(result.is_ok(), "Should accept subscript digit");
}

#[test]
fn test_identifier_continue_underscore() {
  let result = LaleParser::parse(Rule::identifier_continue, "_");
  assert!(result.is_ok(), "Underscore is alpha, so it's valid");
}

#[test]
fn test_identifier_continue_sequence() {
  let result = LaleParser::parse(Rule::identifier_continue, "a5_");
  assert!(result.is_ok(), "Should parse sequence");
}

#[test]
fn test_identifier_continue_with_subscripts() {
  let result = LaleParser::parse(Rule::identifier_continue, "a\u{2080}\u{2081}5");
  assert!(result.is_ok(), "Should mix digit and subscript");
}

#[test]
fn test_identifier_continue_rejects_special() {
  let invalid = vec!["!", "@", "#", "$", "%", "&", "*", "(", ")"];
  for ch in invalid {
    let result = LaleParser::parse(Rule::identifier_continue, ch);
    assert!(result.is_err(), "Should reject: {}", ch);
  }
}

// ==================== REAL-WORLD EXAMPLES ====================

#[test]
fn test_identifier_mathematical_variable() {
  let result = LaleParser::parse(Rule::single_identifier, "Δx");
  assert!(result.is_ok(), "Should accept mathematical variable");
}

#[test]
fn test_identifier_snake_case() {
  let result = LaleParser::parse(Rule::single_identifier, "my_variable_name");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_camel_case() {
  let result = LaleParser::parse(Rule::single_identifier, "myVariableName");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_pascal_case() {
  let result = LaleParser::parse(Rule::single_identifier, "MyClassName");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_constant_style() {
  let result = LaleParser::parse(Rule::single_identifier, "MAX_VALUE_2024");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_single_letter() {
  let result = LaleParser::parse(Rule::single_identifier, "i");
  assert!(result.is_ok(), "Single letter should be valid");
}

#[test]
fn test_identifier_loop_variable() {
  let result = LaleParser::parse(Rule::single_identifier, "i2");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_type_name_style() {
  let result = LaleParser::parse(Rule::single_identifier, "String");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_function_name_style() {
  let result = LaleParser::parse(Rule::single_identifier, "calculateSum");
  assert!(result.is_ok());
}

#[test]
fn test_identifier_with_numbers_interspersed() {
  let result = LaleParser::parse(Rule::single_identifier, "base64Encode");
  assert!(result.is_ok());
}
