use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for comments and documentation in the Lale grammar.
///
/// This test suite covers:
/// - doc: Documentation comments (///)
/// - comment: Code comments (//)
/// - info: Optional documentation/comment blocks

// ==================== DOC COMMENT TESTS ====================
// doc = @{"///" ~ (!NEWLINE ~ ANY)* }

#[test]
fn test_doc_basic() {
  let result = LaleParser::parse(Rule::doc, "/// This is documentation");
  assert!(result.is_ok(), "Should parse basic doc comment");
}

#[test]
fn test_doc_with_spaces() {
  let result = LaleParser::parse(Rule::doc, "///   Multiple spaces and content");
  assert!(result.is_ok(), "Should parse doc with extra spaces");
}

#[test]
fn test_doc_empty() {
  let result = LaleParser::parse(Rule::doc, "///");
  assert!(result.is_ok(), "Should parse empty doc comment");
}

#[test]
fn test_doc_with_special_chars() {
  let result = LaleParser::parse(Rule::doc, "/// @param x: The input value!");
  assert!(result.is_ok(), "Should handle special characters in doc");
}

#[test]
fn test_doc_with_code_snippet() {
  let result = LaleParser::parse(Rule::doc, "/// Example: x = 5 * 2");
  assert!(result.is_ok(), "Should handle code-like content in doc");
}

#[test]
fn test_doc_with_punctuation() {
  let result = LaleParser::parse(Rule::doc, "/// Returns: (x * 2) + 3");
  assert!(result.is_ok(), "Should handle mathematical notation");
}

#[test]
fn test_doc_rejects_newline_in_content() {
  let result = LaleParser::parse(Rule::doc, "/// Line 1\nLine 2");
  // The doc rule uses (!NEWLINE ~ ANY)*, so it stops at newline
  // The result will parse "/// Line 1" and leave "Line 2" unparsed
  // With @, it should consume only up to newline
  assert!(result.is_ok(), "Should parse up to newline");
}

#[test]
fn test_doc_unicode_content() {
  let result = LaleParser::parse(Rule::doc, "/// Fonction pour calculer α et β");
  assert!(result.is_ok(), "Should handle Unicode in doc");
}

#[test]
fn test_doc_with_numbers() {
  let result = LaleParser::parse(Rule::doc, "/// Returns value in range [0, 100]");
  assert!(result.is_ok(), "Should handle numbers and brackets");
}

// ==================== COMMENT TESTS ====================
// comment = @{"//" ~ (!NEWLINE ~ ANY)* }

#[test]
fn test_comment_basic() {
  let result = LaleParser::parse(Rule::comment, "// This is a comment");
  assert!(result.is_ok(), "Should parse basic comment");
}

#[test]
fn test_comment_empty() {
  let result = LaleParser::parse(Rule::comment, "//");
  assert!(result.is_ok(), "Should parse empty comment");
}

#[test]
fn test_comment_with_code() {
  let result = LaleParser::parse(Rule::comment, "// refactor: improve this on line 42");
  assert!(result.is_ok(), "Should handle code-like comments");
}

#[test]
fn test_comment_with_operators() {
  let result = LaleParser::parse(Rule::comment, "// a + b * c = result");
  assert!(result.is_ok(), "Should handle operators in comments");
}

#[test]
fn test_comment_with_quotes() {
  let result = LaleParser::parse(Rule::comment, "// Don't forget the 'important' part");
  assert!(result.is_ok(), "Should handle quotes in comments");
}

#[test]
fn test_comment_with_slashes() {
  let result = LaleParser::parse(Rule::comment, "// path/to/file.txt");
  assert!(result.is_ok(), "Should handle slashes in comments");
}

#[test]
fn test_comment_multiple_spaces() {
  let result = LaleParser::parse(Rule::comment, "//     Extra spacing");
  assert!(result.is_ok(), "Should handle multiple spaces");
}

#[test]
fn test_comment_unicode() {
  let result = LaleParser::parse(Rule::comment, "// Commentaire en français");
  assert!(result.is_ok(), "Should handle Unicode in comments");
}

#[test]
fn test_comment_mathematical_symbols() {
  let result = LaleParser::parse(Rule::comment, "// Calculate: α + β = γ");
  assert!(result.is_ok(), "Should handle mathematical symbols");
}

#[test]
fn test_comment_not_doc() {
  // Verify that single slash comments don't parse as doc
  let result = LaleParser::parse(Rule::doc, "// Not a doc comment");
  assert!(result.is_err(), "Single-slash should not match doc rule");
}

// ==================== INFO TESTS ====================
// info = _{ os ~ (doc | comment) }
// Note: info requires preceding optional whitespace and either doc or comment

#[test]
fn test_info_comment_no_preceding_space() {
  let result = LaleParser::parse(Rule::info, "// comment");
  assert!(result.is_ok(), "Should parse comment as info");
}

#[test]
fn test_info_doc_no_preceding_space() {
  let result = LaleParser::parse(Rule::info, "/// doc");
  assert!(result.is_ok(), "Should parse doc as info");
}

#[test]
fn test_info_with_leading_space() {
  let result = LaleParser::parse(Rule::info, " // comment");
  assert!(result.is_ok(), "Should parse comment with leading space");
}

#[test]
fn test_info_with_leading_tab() {
  let result = LaleParser::parse(Rule::info, "\t// comment");
  assert!(result.is_ok(), "Should parse comment with leading tab");
}

#[test]
fn test_info_with_multiple_leading_spaces() {
  let result = LaleParser::parse(Rule::info, "   // comment with many spaces");
  assert!(result.is_ok(), "Should parse with multiple leading spaces");
}

#[test]
fn test_info_rejects_no_comment() {
  let result = LaleParser::parse(Rule::info, "   ");
  assert!(result.is_err(), "Should reject spaces without comment");
}

#[test]
fn test_info_rejects_empty() {
  let result = LaleParser::parse(Rule::info, "");
  assert!(result.is_err(), "Should reject empty input");
}

#[test]
fn test_info_comment_with_content() {
  let result = LaleParser::parse(Rule::info, "  // Important implementation note");
  assert!(
    result.is_ok(),
    "Should parse comment with spaces and content"
  );
}

#[test]
fn test_info_doc_with_content() {
  let result = LaleParser::parse(Rule::info, "  /// Returns the sum of two numbers");
  assert!(result.is_ok(), "Should parse doc with spaces and content");
}

// ==================== COMMENT CONTENT EDGE CASES ====================

#[test]
fn test_comment_very_long() {
  let long_comment = "// ".to_string() + &"a".repeat(1000);
  let result = LaleParser::parse(Rule::comment, &long_comment);
  assert!(result.is_ok(), "Should handle very long comments");
}

#[test]
fn test_doc_with_html_like_syntax() {
  let result = LaleParser::parse(Rule::doc, "/// <summary>Function description</summary>");
  assert!(result.is_ok(), "Should handle HTML-like syntax in doc");
}

#[test]
fn test_comment_with_emojis() {
  let result = LaleParser::parse(Rule::comment, "// ✅ This is working");
  assert!(result.is_ok(), "Should handle emoji in comments");
}

#[test]
fn test_comment_with_tabs_and_spaces_mixed() {
  let result = LaleParser::parse(Rule::comment, "// \t indented \t with tabs");
  assert!(result.is_ok(), "Should handle mixed tabs and spaces");
}

#[test]
fn test_doc_multiple_parameters() {
  let result = LaleParser::parse(
    Rule::doc,
    "/// @param x First number @param y Second number",
  );
  assert!(result.is_ok(), "Should handle multiple parameters");
}

#[test]
fn test_comment_with_url() {
  let result = LaleParser::parse(Rule::comment, "// See https://example.com/api");
  assert!(result.is_ok(), "Should handle URLs in comments");
}

#[test]
fn test_comment_with_email() {
  let result = LaleParser::parse(Rule::comment, "// Contact: author@example.com");
  assert!(result.is_ok(), "Should handle email addresses");
}

#[test]
fn test_comment_with_multiple_slashes() {
  let result = LaleParser::parse(Rule::comment, "// note // reminder // important");
  assert!(result.is_ok(), "Should handle multiple slash sequences");
}

// ==================== COMMENT/DOC DIFFERENTIATION ====================

#[test]
fn test_doc_vs_comment_distinction() {
  let doc = LaleParser::parse(Rule::doc, "/// Doc");
  let comment = LaleParser::parse(Rule::comment, "/// Doc");
  let comment2 = LaleParser::parse(Rule::comment, "// Comment");

  assert!(doc.is_ok(), "/// should be doc");
  assert!(comment.is_ok(), "/// should also parse as comment");
  assert!(comment2.is_ok(), "// should be comment");
}

#[test]
fn test_single_slash_not_doc() {
  let result = LaleParser::parse(Rule::doc, "/ not a comment");
  assert!(result.is_err(), "Single slash should not be doc");
}

#[test]
fn test_comment_with_forward_slashes() {
  let result = LaleParser::parse(Rule::comment, "// path/to/some/file/name.txt");
  assert!(result.is_ok(), "Should handle forward slashes in comment");
}

// ==================== BOUNDARY TESTS ====================

#[test]
fn test_comment_exactly_at_newline_boundary() {
  // Comments should stop at newline (due to !NEWLINE)
  let result = LaleParser::parse(Rule::comment, "// comment");
  assert!(result.is_ok(), "Should parse comment without newline");
}

#[test]
fn test_doc_with_carriage_return() {
  // Carriage return is not NEWLINE (in Pest, NEWLINE is \n)
  let result = LaleParser::parse(Rule::doc, "/// comment\r");
  assert!(result.is_ok(), "Should handle CR in doc");
}

#[test]
fn test_info_with_mixed_whitespace_and_comment() {
  let result = LaleParser::parse(Rule::info, " \t \r // comment");
  assert!(
    result.is_ok(),
    "Should handle mixed whitespace before comment"
  );
}
