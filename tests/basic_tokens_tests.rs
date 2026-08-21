use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for basic token rules in the Lale grammar.
///
/// This test suite covers fundamental building blocks:
/// - Character classes: alpha, digit, subscript_digits, combining_marks
/// - Identifier continuation rules
/// - Unicode character ranges via letter_ranges (Latin Extended, Greek, Cyrillic, CJK, Indic, etc.)

// ==================== ALPHA RULE TESTS ====================
// alpha = { (ASCII_ALPHA | "_" | letter_ranges) }

#[test]
fn test_alpha_ascii_lowercase() {
  for ch in 'a'..='z' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::alpha, &s);
    assert!(result.is_ok(), "Should accept lowercase ASCII: {}", ch);
  }
}

#[test]
fn test_alpha_ascii_uppercase() {
  for ch in 'A'..='Z' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::alpha, &s);
    assert!(result.is_ok(), "Should accept uppercase ASCII: {}", ch);
  }
}

#[test]
fn test_alpha_underscore() {
  let result = LaleParser::parse(Rule::alpha, "_");
  assert!(result.is_ok(), "Underscore should be valid alpha");
}

#[test]
fn test_alpha_rejects_digits() {
  for ch in '0'..='9' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::alpha, &s);
    assert!(result.is_err(), "Should reject digit: {}", ch);
  }
}

#[test]
fn test_alpha_rejects_special_chars() {
  let invalid = vec!["!", "@", "#", "$", "%", "&", "*", "(", ")", "+", "="];
  for ch in invalid {
    let result = LaleParser::parse(Rule::alpha, ch);
    assert!(result.is_err(), "Should reject special char: {}", ch);
  }
}

// ==================== DIGIT RULE TESTS ====================
// digit = { '0'..'9' }

#[test]
fn test_digit_all_valid() {
  for ch in '0'..='9' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::digit, &s);
    assert!(result.is_ok(), "Should accept digit: {}", ch);
  }
}

#[test]
fn test_digit_rejects_letters() {
  for ch in 'a'..='z' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::digit, &s);
    assert!(result.is_err(), "Should reject letter: {}", ch);
  }
}

#[test]
fn test_digit_rejects_special() {
  let invalid = vec!["+", "-", ".", "e", "E"];
  for ch in invalid {
    let result = LaleParser::parse(Rule::digit, ch);
    assert!(result.is_err(), "Should reject: {}", ch);
  }
}

// ==================== HEX RULE TESTS ====================
// hex = { '0'..'9' | 'A'..'F' | 'a'..'f' }

#[test]
fn test_hex_decimal_digits() {
  for ch in '0'..='9' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::hex, &s);
    assert!(result.is_ok(), "Should accept hex digit: {}", ch);
  }
}

#[test]
fn test_hex_uppercase() {
  for ch in 'A'..='F' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::hex, &s);
    assert!(result.is_ok(), "Should accept uppercase hex: {}", ch);
  }
}

#[test]
fn test_hex_lowercase() {
  for ch in 'a'..='f' {
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::hex, &s);
    assert!(result.is_ok(), "Should accept lowercase hex: {}", ch);
  }
}

#[test]
fn test_hex_rejects_beyond_f() {
  let invalid = vec!["G", "Z", "g", "z"];
  for ch in invalid {
    let result = LaleParser::parse(Rule::hex, ch);
    assert!(result.is_err(), "Should reject: {}", ch);
  }
}

// ==================== IDENTIFIER_CONTINUE TESTS ====================
// identifier_continue = { alpha | digit | subscript_digits | combining_marks }

#[test]
fn test_identifier_continue_alpha() {
  let result = LaleParser::parse(Rule::identifier_continue, "a");
  assert!(result.is_ok(), "Should accept alpha");
}

#[test]
fn test_identifier_continue_digit() {
  let result = LaleParser::parse(Rule::identifier_continue, "5");
  assert!(result.is_ok(), "Should accept digit");
}

#[test]
fn test_identifier_continue_subscript() {
  // Subscript 5 (U+2085)
  let result = LaleParser::parse(Rule::identifier_continue, "\u{2085}");
  assert!(result.is_ok(), "Should accept subscript digit");
}

#[test]
fn test_identifier_continue_rejects_special() {
  let result = LaleParser::parse(Rule::identifier_continue, "!");
  assert!(result.is_err(), "Should reject special chars");
}

// ==================== UNICODE LETTER RANGES (via alpha) ====================
// These ranges are now part of letter_ranges and tested through the alpha rule
// Includes: Latin Extended, Greek/Coptic, Cyrillic, CJK, Hiragana+Katakana,
// Indic scripts, Arabic, Hebrew, Thai, Lao, Tibetan, Myanmar, Georgian, Armenian, etc.

#[test]
fn test_letter_ranges_hiragana() {
  // Hiragana (part of U+3040-U+30FF in letter_ranges)
  let result = LaleParser::parse(Rule::alpha, "\u{3042}"); // あ
  assert!(result.is_ok(), "Hiragana should be valid");
  let result = LaleParser::parse(Rule::alpha, "\u{304B}"); // か
  assert!(result.is_ok(), "Hiragana should be valid");
}

#[test]
fn test_letter_ranges_katakana() {
  // Katakana (part of U+3040-U+30FF in letter_ranges)
  let result = LaleParser::parse(Rule::alpha, "\u{30A2}"); // ア
  assert!(result.is_ok(), "Katakana should be valid");
  let result = LaleParser::parse(Rule::alpha, "\u{30B3}"); // コ
  assert!(result.is_ok(), "Katakana should be valid");
}

#[test]
fn test_letter_ranges_cyrillic() {
  // Cyrillic (U+0400-U+04FF)
  let result = LaleParser::parse(Rule::alpha, "\u{0414}"); // Д (Cyrillic De)
  assert!(result.is_ok(), "Cyrillic should be valid");
  let result = LaleParser::parse(Rule::alpha, "\u{043C}"); // м (Cyrillic Em)
  assert!(result.is_ok(), "Cyrillic should be valid");
}

#[test]
fn test_letter_ranges_cjk() {
  // CJK Unified Ideographs (U+4E00-U+9FFF)
  let result = LaleParser::parse(Rule::alpha, "\u{4E00}"); // 一 (CJK one)
  assert!(result.is_ok(), "CJK should be valid");
  let result = LaleParser::parse(Rule::alpha, "\u{8FBB}"); // 辻 (CJK)
  assert!(result.is_ok(), "CJK should be valid");
}

#[test]
fn test_letter_ranges_hangul() {
  // Hangul Syllables (U+AC00-U+D7AF)
  let result = LaleParser::parse(Rule::alpha, "\u{AC00}"); // 가 (Korean ga)
  assert!(result.is_ok(), "Hangul syllable should be valid");
  let result = LaleParser::parse(Rule::alpha, "\u{D55C}"); // 한 (Korean han)
  assert!(result.is_ok(), "Hangul syllable should be valid");
}

#[test]
fn test_letter_ranges_indic() {
  // Devanagari (U+0900-U+097F)
  let result = LaleParser::parse(Rule::alpha, "\u{0915}"); // क (Devanagari Ka)
  assert!(result.is_ok(), "Devanagari should be valid");

  // Bengali (U+0980-U+09FF)
  let result = LaleParser::parse(Rule::alpha, "\u{09AC}"); // ব (Bengali Ba)
  assert!(result.is_ok(), "Bengali should be valid");
}

#[test]
fn test_letter_ranges_arabic() {
  // Arabic (U+0600-U+06FF)
  let result = LaleParser::parse(Rule::alpha, "\u{0639}"); // ع (Arabic Ain)
  assert!(result.is_ok(), "Arabic should be valid");
}

#[test]
fn test_letter_ranges_hebrew() {
  // Hebrew (U+0590-U+05FF)
  let result = LaleParser::parse(Rule::alpha, "\u{05E9}"); // ש (Hebrew Shin)
  assert!(result.is_ok(), "Hebrew should be valid");
}

// ==================== SUBSCRIPT_DIGITS TESTS ====================
// subscript_digits = { '\u{2080}'..'\u{2089}' }

#[test]
fn test_subscript_lower_boundary() {
  let result = LaleParser::parse(Rule::subscript_digits, "\u{2080}");
  assert!(result.is_ok(), "U+2080 (subscript 0) should be valid");
}

#[test]
fn test_subscript_upper_boundary() {
  let result = LaleParser::parse(Rule::subscript_digits, "\u{2089}");
  assert!(result.is_ok(), "U+2089 (subscript 9) should be valid");
}

#[test]
fn test_subscript_all_digits() {
  for i in 0..=9 {
    let ch = char::from_u32(0x2080 + i).unwrap();
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::subscript_digits, &s);
    assert!(result.is_ok(), "Should accept subscript digit {}", i);
  }
}

#[test]
fn test_subscript_below_range() {
  let result = LaleParser::parse(Rule::subscript_digits, "\u{207F}");
  assert!(result.is_err(), "U+207F should be invalid");
}

#[test]
fn test_subscript_above_range() {
  let result = LaleParser::parse(Rule::subscript_digits, "\u{208A}");
  assert!(result.is_err(), "U+208A should be invalid");
}

// ==================== COMBINING MARKS TESTS ====================
// combining_marks = { '\u{0300}'..'\u{036F}' | '\u{1AB0}'..'\u{1AFF}' | '\u{1DC0}'..'\u{1DFF}' }
// Note: combining_marks are NOT allowed as identifier starters (alpha)
// They are only allowed in identifier_continue

#[test]
fn test_combining_marks_not_allowed_as_starter() {
  // Combining Grave Accent (U+0300) - should NOT be accepted as alpha
  let result = LaleParser::parse(Rule::alpha, "\u{0300}");
  assert!(result.is_err(), "Combining marks cannot start identifiers");

  // Combining Acute Accent (U+0301)
  let result = LaleParser::parse(Rule::alpha, "\u{0301}");
  assert!(result.is_err(), "Combining marks cannot start identifiers");
}

#[test]
fn test_combining_marks_allowed_in_identifier_continue() {
  // Combining Grave Accent (U+0300)
  let result = LaleParser::parse(Rule::identifier_continue, "\u{0300}");
  assert!(
    result.is_ok(),
    "Combining marks allowed in identifier_continue"
  );

  // Combining Acute Accent (U+0301)
  let result = LaleParser::parse(Rule::identifier_continue, "\u{0301}");
  assert!(
    result.is_ok(),
    "Combining marks allowed in identifier_continue"
  );
}

#[test]
fn test_combining_marks_in_full_identifier() {
  // identifier allows combining marks via identifier_continue
  // e.g., "café" using combining acute: "cafe" + combining acute on 'e'
  let result = LaleParser::parse(Rule::single_identifier, "e\u{0301}");
  assert!(result.is_ok(), "Identifier can use combining marks");
}

#[test]
fn test_combining_marks_range1_boundaries() {
  // Range 1: U+0300..U+036F (Latin combining diacriticals)
  // identifier_continue = { alpha | digit | subscript_digits | combining_marks }
  // So U+0300 matches as a combining_mark
  let result = LaleParser::parse(Rule::identifier_continue, "\u{0300}");
  assert!(result.is_ok(), "U+0300 (combining grave) should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{036F}");
  assert!(result.is_ok(), "U+036F should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{02FF}");
  assert!(result.is_err(), "U+02FF (below range 1) should be invalid");

  // U+0370 is Greek/Coptic. It will match via alpha part of identifier_continue,
  // so it's valid here even though it's not a combining mark.
  let result = LaleParser::parse(Rule::identifier_continue, "\u{0370}");
  assert!(
    result.is_ok(),
    "U+0370 (Greek) will match via alpha in identifier_continue"
  );
}

#[test]
fn test_combining_marks_range2_boundaries() {
  // Range 2: U+1AB0..U+1AFF (Latin extended combining)
  let result = LaleParser::parse(Rule::identifier_continue, "\u{1AB0}");
  assert!(result.is_ok(), "U+1AB0 should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1AFF}");
  assert!(result.is_ok(), "U+1AFF should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1AAF}");
  assert!(result.is_err(), "U+1AAF (below range 2) should be invalid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1B00}");
  assert!(result.is_err(), "U+1B00 (above range 2) should be invalid");
}

#[test]
fn test_combining_marks_range3_boundaries() {
  // Range 3: U+1DC0..U+1DFF (Latin extended combining)
  let result = LaleParser::parse(Rule::identifier_continue, "\u{1DC0}");
  assert!(result.is_ok(), "U+1DC0 should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1DFF}");
  assert!(result.is_ok(), "U+1DFF should be valid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1DBF}");
  assert!(result.is_err(), "U+1DBF (below range 3) should be invalid");

  let result = LaleParser::parse(Rule::identifier_continue, "\u{1E00}");
  assert!(result.is_err(), "U+1E00 (above range 3) should be invalid");
}
