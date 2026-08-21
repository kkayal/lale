use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for the `alpha` rule in the Lale grammar.
///
/// The `alpha` rule defines the valid characters that can start an identifier
/// in the Lale language. It accepts ASCII letters, underscore, and comprehensive
/// Unicode letter ranges (letter_ranges) including Latin Extended, Greek/Coptic,
/// Cyrillic, CJK, Indic scripts, and more. Does NOT include subscript digits or
/// combining marks (which are only allowed in identifier_continue).

#[test]
fn test_alpha_rule_ascii_letters() {
  // Test that standard ASCII letters (a-z, A-Z) are accepted
  let result = LaleParser::parse(Rule::alpha, "a");
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "Z");
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "m");
  assert!(result.is_ok());
}

#[test]
fn test_alpha_rule_underscore() {
  // Test that underscore (_) is accepted as a valid identifier start character
  let result = LaleParser::parse(Rule::alpha, "_");
  assert!(result.is_ok());
}

#[test]
fn test_alpha_rule_hiragana() {
  // Test Japanese hiragana characters (U+3041 to U+309F) are accepted
  // This allows for internationalized identifiers in Japanese
  let result = LaleParser::parse(Rule::alpha, "あ"); // U+3042 (hiragana 'a')
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "か"); // U+304B (hiragana 'ka')
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "ん"); // U+3093 (hiragana 'n')
  assert!(result.is_ok());
}

#[test]
fn test_alpha_rule_greek_coptic() {
  // Test Greek and Coptic characters (U+0370 to U+03FF) are accepted
  // This allows for mathematical symbols and Greek letters in identifiers
  let result = LaleParser::parse(Rule::alpha, "α"); // U+03B1 (Greek alpha)
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "β"); // U+03B2 (Greek beta)
  assert!(result.is_ok());

  let result = LaleParser::parse(Rule::alpha, "Ω"); // U+03A9 (Greek Omega)
  assert!(result.is_ok());
}

#[test]
fn test_alpha_rule_subscript_digits() {
  // Test that subscript digit characters (U+2080 to U+2089) are NOT accepted as starters
  // but ARE accepted within identifiers (after an alpha character)
  // This enforces the rule that subscript digits cannot start identifiers

  let result = LaleParser::parse(Rule::alpha, "₀"); // U+2080 (subscript 0)
  assert!(
    result.is_err(),
    "Subscript 0 should not start an identifier"
  );

  let result = LaleParser::parse(Rule::alpha, "₅"); // U+2085 (subscript 5)
  assert!(
    result.is_err(),
    "Subscript 5 should not start an identifier"
  );

  let result = LaleParser::parse(Rule::alpha, "₉"); // U+2089 (subscript 9)
  assert!(
    result.is_err(),
    "Subscript 9 should not start an identifier"
  );

  // But subscript digits ARE valid within complete identifiers via the identifier rule
  let result = LaleParser::parse(Rule::single_identifier, "x₀");
  assert!(result.is_ok(), "x₀ should be a valid identifier");

  let result = LaleParser::parse(Rule::single_identifier, "α₅");
  assert!(result.is_ok(), "α₅ should be a valid identifier");

  let result = LaleParser::parse(Rule::single_identifier, "_₉");
  assert!(result.is_ok(), "_₉ should be a valid identifier");
}

#[test]
fn test_alpha_rule_latin1_safe() {
  // Test Latin-1 extended characters are accepted (new limited range)
  // latin1_safe: '\u{00C0}'..'\u{00D6}' | '\u{00D8}'..'\u{00F6}' | '\u{00F8}'..'\u{00FF}'
  let result = LaleParser::parse(Rule::alpha, "À"); // U+00C0 (Latin A with grave)
  assert!(result.is_ok(), "U+00C0 should be valid");

  let result = LaleParser::parse(Rule::alpha, "é"); // U+00E9 (Latin e with acute)
  assert!(result.is_ok(), "U+00E9 should be valid");

  let result = LaleParser::parse(Rule::alpha, "ÿ"); // U+00FF (Latin y with diaeresis)
  assert!(result.is_ok(), "U+00FF should be valid");

  // U+00A1 (inverted exclamation) is NOT in the new latin1_safe range
  let result = LaleParser::parse(Rule::alpha, "¡");
  assert!(
    result.is_err(),
    "U+00A1 should not be valid in new latin1_safe"
  );
}

#[test]
fn test_alpha_rule_invalid_characters() {
  // Test that numeric digits are rejected (they cannot start identifiers)
  let result = LaleParser::parse(Rule::alpha, "0");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "9");
  assert!(result.is_err());

  // Test that subscript digit characters are rejected (they cannot start identifiers)
  // Subscript digits are valid within identifiers (via identifier_continue rule) but not as starters
  let result = LaleParser::parse(Rule::alpha, "₀"); // U+2080 (subscript 0)
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "₅"); // U+2085 (subscript 5)
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "₉"); // U+2089 (subscript 9)
  assert!(result.is_err());

  // Test that common symbols are rejected
  let result = LaleParser::parse(Rule::alpha, "!");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "@");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "#");
  assert!(result.is_err());
}

#[test]
fn test_alpha_rule_empty_string() {
  // Test that empty input is rejected (identifiers must have at least one character)
  let result = LaleParser::parse(Rule::alpha, "");
  assert!(result.is_err());
}

#[test]
fn test_alpha_rule_multiple_characters() {
  // Test that the alpha rule only matches the first character of multi-character input
  // The rule should succeed but only consume the first valid character
  let result = LaleParser::parse(Rule::alpha, "ab");
  assert!(result.is_ok());

  // Verify that only the first character ('a') was consumed
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "a");
}

#[test]
fn test_alpha_rule_negative_cases() {
  // Test sequences starting with invalid characters - all should fail
  // These test cases ensure that invalid leading characters are properly rejected

  // Digit followed by letter - should fail because digits cannot start identifiers
  let result = LaleParser::parse(Rule::alpha, "5f");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "0a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "9_");
  assert!(result.is_err());

  // Symbols followed by letter - should fail because symbols cannot start identifiers
  let result = LaleParser::parse(Rule::alpha, "!a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "@b");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "#c");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "$d");
  assert!(result.is_err());

  // Whitespace followed by letter - should fail because whitespace cannot start identifiers
  let result = LaleParser::parse(Rule::alpha, " a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "\ta");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "\na");
  assert!(result.is_err());

  // Punctuation that might be confused with valid characters
  let result = LaleParser::parse(Rule::alpha, ".lale");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "-a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "+a");
  assert!(result.is_err());

  // Brackets and parentheses - should fail as they cannot start identifiers
  let result = LaleParser::parse(Rule::alpha, "(a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "[a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::alpha, "{a");
  assert!(result.is_err());
}

// ==================== UNICODE BOUNDARY TESTS ====================

#[test]
fn test_alpha_rule_hiragana_boundaries() {
  // Test lower boundary of combined hiragana+katakana range (U+3040)
  // The new grammar uses '\u{3040}'..'\u{30FF}' which includes both Hiragana and Katakana
  let result = LaleParser::parse(Rule::alpha, "\u{3040}");
  assert!(
    result.is_ok(),
    "U+3040 (combined hiragana/katakana lower boundary) should be valid"
  );

  // Test actual hiragana start (U+3041)
  let result = LaleParser::parse(Rule::alpha, "\u{3041}");
  assert!(result.is_ok(), "U+3041 (hiragana) should be valid");

  // Test upper boundary of hiragana+katakana range (U+30FF)
  let result = LaleParser::parse(Rule::alpha, "\u{30FF}");
  assert!(
    result.is_ok(),
    "U+30FF (hiragana/katakana upper boundary) should be valid"
  );

  // Test just below the range (U+303F) - should fail
  let result = LaleParser::parse(Rule::alpha, "\u{303F}");
  assert!(
    result.is_err(),
    "U+303F (below hiragana/katakana range) should be invalid"
  );

  // Test just above the range (U+3100) - should fail
  let result = LaleParser::parse(Rule::alpha, "\u{3100}");
  assert!(
    result.is_err(),
    "U+3100 (above hiragana/katakana range) should be invalid"
  );
}

#[test]
fn test_alpha_rule_greek_boundaries() {
  // Test lower boundary of Greek & Coptic range (U+0370)
  let result = LaleParser::parse(Rule::alpha, "\u{0370}");
  assert!(
    result.is_ok(),
    "U+0370 (Greek lower boundary) should be valid"
  );

  // Test upper boundary of Greek & Coptic range (U+03FF)
  let result = LaleParser::parse(Rule::alpha, "\u{03FF}");
  assert!(
    result.is_ok(),
    "U+03FF (Greek upper boundary) should be valid"
  );

  // Test just below Greek range (U+036F) - should fail
  let result = LaleParser::parse(Rule::alpha, "\u{036F}");
  assert!(
    result.is_err(),
    "U+036F (below Greek range) should be invalid"
  );

  // Test Cyrillic (U+0400 is start of Cyrillic, which is now also valid)
  let result = LaleParser::parse(Rule::alpha, "\u{0400}");
  assert!(
    result.is_ok(),
    "U+0400 (Cyrillic - now in expanded letter_ranges) should be valid"
  );
}

#[test]
fn test_alpha_rule_subscript_digit_boundaries() {
  // Test lower boundary of subscript digits (U+2080)
  let result = LaleParser::parse(Rule::alpha, "\u{2080}");
  assert!(
    result.is_err(),
    "U+2080 (subscript 0) should not start identifier"
  );

  // Test upper boundary of subscript digits (U+2089)
  let result = LaleParser::parse(Rule::alpha, "\u{2089}");
  assert!(
    result.is_err(),
    "U+2089 (subscript 9) should not start identifier"
  );

  // Test just below subscript range (U+207F) - should fail
  let result = LaleParser::parse(Rule::alpha, "\u{207F}");
  assert!(
    result.is_err(),
    "U+207F (below subscript range) should be invalid"
  );

  // Test just above subscript range (U+208A) - should fail
  let result = LaleParser::parse(Rule::alpha, "\u{208A}");
  assert!(
    result.is_err(),
    "U+208A (above subscript range) should be invalid"
  );
}

#[test]
fn test_alpha_rule_latin1_safe_boundaries() {
  // Updated ranges for latin1_safe:
  // '\u{00C0}'..'\u{00D6}' | '\u{00D8}'..'\u{00F6}' | '\u{00F8}'..'\u{00FF}'

  // Range 1: U+00C0..U+00D6 (Latin A-grave through Latin O-diaeresis)
  let result = LaleParser::parse(Rule::alpha, "\u{00C0}");
  assert!(
    result.is_ok(),
    "U+00C0 (Latin1 range 1 lower) should be valid"
  );

  let result = LaleParser::parse(Rule::alpha, "\u{00D6}");
  assert!(
    result.is_ok(),
    "U+00D6 (Latin1 range 1 upper) should be valid"
  );

  let result = LaleParser::parse(Rule::alpha, "\u{00BF}");
  assert!(result.is_err(), "U+00BF (below range 1) should be invalid");

  let result = LaleParser::parse(Rule::alpha, "\u{00D7}");
  assert!(
    result.is_err(),
    "U+00D7 (between ranges, multiplication sign, excluded) should be invalid"
  );

  // Range 2: U+00D8..U+00F6 (Latin O-stroke through latin o-diaeresis)
  let result = LaleParser::parse(Rule::alpha, "\u{00D8}");
  assert!(
    result.is_ok(),
    "U+00D8 (Latin1 range 2 lower) should be valid"
  );

  let result = LaleParser::parse(Rule::alpha, "\u{00F6}");
  assert!(
    result.is_ok(),
    "U+00F6 (Latin1 range 2 upper) should be valid"
  );

  let result = LaleParser::parse(Rule::alpha, "\u{00F7}");
  assert!(
    result.is_err(),
    "U+00F7 (between ranges, division sign, excluded) should be invalid"
  );

  // Range 3: U+00F8..U+00FF (Latin o-stroke through Latin y-diaeresis)
  let result = LaleParser::parse(Rule::alpha, "\u{00F8}");
  assert!(
    result.is_ok(),
    "U+00F8 (Latin1 range 3 lower) should be valid"
  );

  let result = LaleParser::parse(Rule::alpha, "\u{00FF}");
  assert!(
    result.is_ok(),
    "U+00FF (Latin1 range 3 upper) should be valid"
  );
}

#[test]
fn test_alpha_rule_valid_identifier_sequences() {
  // Test various valid multi-character identifier starts (should consume only first char)
  let result = LaleParser::parse(Rule::alpha, "αβγ");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(
    pair.as_str(),
    "α",
    "Should only consume first Greek character"
  );

  // Test underscore followed by digits
  let result = LaleParser::parse(Rule::alpha, "_123");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(pair.as_str(), "_", "Should only consume underscore");

  // Test hiragana followed by ASCII
  let result = LaleParser::parse(Rule::alpha, "あabc");
  assert!(result.is_ok());
  let pairs = result.unwrap();
  let pair = pairs.into_iter().next().unwrap();
  assert_eq!(
    pair.as_str(),
    "あ",
    "Should only consume first hiragana character"
  );
}
