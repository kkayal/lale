use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for whitespace handling rules in the Lale grammar.
///
/// The whitespace rules define how the parser handles spaces, tabs, newlines,
/// and line continuations. These rules are critical for:
/// - Optional whitespace handling (os, os_ln)
/// - Mandatory whitespace handling (ms_ln)
/// - Statement delimiters (sd)
/// - Line continuation with backslash-newline

// ==================== s rule tests ====================
// s = _{ " " | "\t" | "\r" | ("\\\n") }

#[test]
fn test_s_rule_accepts_space() {
  // Test that space character is accepted as whitespace
  let result = LaleParser::parse(Rule::s, " ");
  assert!(result.is_ok());
}

#[test]
fn test_s_rule_accepts_tab() {
  // Test that tab character is accepted as whitespace
  let result = LaleParser::parse(Rule::s, "\t");
  assert!(result.is_ok());
}

#[test]
fn test_s_rule_accepts_carriage_return() {
  // Test that carriage return is accepted as whitespace
  let result = LaleParser::parse(Rule::s, "\r");
  assert!(result.is_ok());
}

#[test]
fn test_s_rule_accepts_line_continuation() {
  // Test that backslash followed by newline is accepted as line continuation
  // This allows statements to span multiple lines
  let result = LaleParser::parse(Rule::s, "\\\n");
  assert!(
    result.is_ok(),
    "Line continuation should be valid whitespace"
  );
}

#[test]
fn test_s_rule_rejects_newline() {
  // Test that newline alone is rejected by s rule
  // (newlines are handled separately by other rules like os_ln and ms_ln)
  let result = LaleParser::parse(Rule::s, "\n");
  assert!(result.is_err(), "Newline should not match s rule");
}

#[test]
fn test_s_rule_rejects_non_whitespace() {
  // Test that non-whitespace characters are rejected
  let result = LaleParser::parse(Rule::s, "a");
  assert!(result.is_err());

  let result = LaleParser::parse(Rule::s, "0");
  assert!(result.is_err());
}

// ==================== ms rule tests ====================
// ms = _{ s+ }
// Mandatory whitespace (one or more horizontal whitespace characters, excluding newlines)

#[test]
fn test_ms_rule_rejects_empty() {
  // Test that ms (mandatory whitespace) rejects empty input
  let result = LaleParser::parse(Rule::ms, "");
  assert!(
    result.is_err(),
    "ms should require at least one whitespace character"
  );
}

#[test]
fn test_ms_rule_accepts_single_space() {
  // Test that ms accepts a single space
  let result = LaleParser::parse(Rule::ms, " ");
  assert!(result.is_ok(), "ms should accept a single space");
}

#[test]
fn test_ms_rule_accepts_single_tab() {
  // Test that ms accepts a single tab
  let result = LaleParser::parse(Rule::ms, "\t");
  assert!(result.is_ok(), "ms should accept a single tab");
}

#[test]
fn test_ms_rule_accepts_multiple_spaces() {
  // Test that ms accepts multiple spaces
  let result = LaleParser::parse(Rule::ms, "     ");
  assert!(result.is_ok(), "ms should accept multiple spaces");
}

#[test]
fn test_ms_rule_accepts_mixed_whitespace() {
  // Test that ms accepts mixed tabs and spaces
  let result = LaleParser::parse(Rule::ms, " \t  \t \r ");
  assert!(
    result.is_ok(),
    "ms should accept mixed horizontal whitespace"
  );
}

#[test]
fn test_ms_rule_accepts_line_continuation() {
  // Test that ms accepts line continuation within whitespace
  let result = LaleParser::parse(Rule::ms, " \\\n ");
  assert!(result.is_ok(), "ms should accept line continuation");
}

#[test]
fn test_ms_rule_accepts_single_line_continuation() {
  // Test that ms accepts a single line continuation as valid mandatory whitespace
  let result = LaleParser::parse(Rule::ms, "\\\n");
  assert!(
    result.is_ok(),
    "ms should accept a single line continuation"
  );
}

#[test]
fn test_ms_rule_rejects_newline_only() {
  // Test that ms rejects a newline alone (newlines are handled by ms_ln)
  let result = LaleParser::parse(Rule::ms, "\n");
  assert!(result.is_err(), "ms should not accept newline alone");
}

#[test]
fn test_ms_rule_accepts_carriage_return() {
  // Test that ms accepts carriage return as valid whitespace
  let result = LaleParser::parse(Rule::ms, "\r");
  assert!(result.is_ok(), "ms should accept carriage return");
}

#[test]
fn test_ms_rule_accepts_multiple_tabs() {
  // Test that ms accepts multiple tabs
  let result = LaleParser::parse(Rule::ms, "\t\t\t");
  assert!(result.is_ok(), "ms should accept multiple tabs");
}

#[test]
fn test_ms_rule_rejects_non_whitespace() {
  // Test that ms rejects non-whitespace characters
  let result = LaleParser::parse(Rule::ms, "a");
  assert!(result.is_err(), "ms should reject letters");

  let result = LaleParser::parse(Rule::ms, "0");
  assert!(result.is_err(), "ms should reject digits");
}

// ==================== os rule tests ====================
// os = _{ s* }
// Optional whitespace (zero or more horizontal whitespace characters)

#[test]
fn test_os_rule_accepts_empty() {
  // Test that os (optional whitespace) accepts empty input (zero whitespace)
  let result = LaleParser::parse(Rule::os, "");
  assert!(result.is_ok(), "os should accept empty input");
}

#[test]
fn test_os_rule_accepts_single_space() {
  // Test that os accepts a single space
  let result = LaleParser::parse(Rule::os, " ");
  assert!(result.is_ok());
}

#[test]
fn test_os_rule_accepts_multiple_spaces() {
  // Test that os accepts multiple spaces (zero or more means * is greedy)
  let result = LaleParser::parse(Rule::os, "     ");
  assert!(result.is_ok());
}

#[test]
fn test_os_rule_accepts_mixed_whitespace() {
  // Test that os accepts mixed tabs and spaces
  let result = LaleParser::parse(Rule::os, " \t  \t \r ");
  assert!(result.is_ok());
}

#[test]
fn test_os_rule_accepts_line_continuation() {
  // Test that os accepts line continuation within whitespace
  let result = LaleParser::parse(Rule::os, " \\\n ");
  assert!(result.is_ok());
}

// ==================== os_ln rule tests ====================
// os_ln = _{ info? ~ (s | NEWLINE)* }
// Optional whitespace including newlines (zero or more whitespace or newlines)

#[test]
fn test_os_ln_rule_accepts_empty() {
  // Test that os_ln (optional whitespace including newlines) accepts empty input
  let result = LaleParser::parse(Rule::os_ln, "");
  assert!(result.is_ok());
}

#[test]
fn test_os_ln_rule_accepts_spaces() {
  // Test that os_ln accepts spaces
  let result = LaleParser::parse(Rule::os_ln, "   ");
  assert!(result.is_ok());
}

#[test]
fn test_os_ln_rule_accepts_newline() {
  // Test that os_ln accepts a newline
  let result = LaleParser::parse(Rule::os_ln, "\n");
  assert!(result.is_ok());
}

#[test]
fn test_os_ln_rule_accepts_multiple_newlines() {
  // Test that os_ln accepts multiple newlines
  let result = LaleParser::parse(Rule::os_ln, "\n\n\n");
  assert!(result.is_ok());
}

#[test]
fn test_os_ln_rule_accepts_mixed_whitespace_and_newlines() {
  // Test that os_ln accepts mixed spaces, tabs, and newlines
  let result = LaleParser::parse(Rule::os_ln, "  \n\t  \n  ");
  assert!(result.is_ok());
}

#[test]
fn test_os_ln_rule_accepts_complex_pattern() {
  // Test complex os_ln patterns
  let result = LaleParser::parse(Rule::os_ln, "\n\n   \n\t\n");
  assert!(result.is_ok());
}

// ==================== ms_ln rule tests ====================
// ms_ln = _{ info? ~ (s | NEWLINE)+ }
// Mandatory whitespace including newlines (one or more whitespace or newlines)

#[test]
fn test_ms_ln_rule_rejects_empty() {
  // Test that ms_ln (mandatory whitespace) rejects empty input
  let result = LaleParser::parse(Rule::ms_ln, "");
  assert!(
    result.is_err(),
    "ms_ln should require at least one whitespace"
  );
}

#[test]
fn test_ms_ln_rule_accepts_single_space() {
  // Test that ms_ln accepts a single space
  let result = LaleParser::parse(Rule::ms_ln, " ");
  assert!(result.is_ok());
}

#[test]
fn test_ms_ln_rule_accepts_single_newline() {
  // Test that ms_ln accepts a single newline
  let result = LaleParser::parse(Rule::ms_ln, "\n");
  assert!(result.is_ok());
}

#[test]
fn test_ms_ln_rule_accepts_spaces_and_newlines() {
  // Test that ms_ln accepts spaces followed by newlines
  let result = LaleParser::parse(Rule::ms_ln, "  \n  \n ");
  assert!(result.is_ok());
}

#[test]
fn test_ms_ln_rule_accepts_tab_and_newline() {
  // Test that ms_ln accepts tab followed by newline
  let result = LaleParser::parse(Rule::ms_ln, "\t\n");
  assert!(result.is_ok());
}

#[test]
fn test_ms_ln_rule_handles_windows_line_ending() {
  // Test that ms_ln handles Windows-style line endings (CR+LF)
  let result = LaleParser::parse(Rule::ms_ln, "\r\n");
  assert!(result.is_ok());
}

// ==================== sd rule tests ====================
// sd = _{ (os ~ (NEWLINE | ";") ) + ~ os }
// Statement delimiter: one or more (optional whitespace + newline or semicolon) + optional whitespace

#[test]
fn test_sd_rule_accepts_single_newline() {
  // Test that sd (statement delimiter) accepts a single newline
  let result = LaleParser::parse(Rule::sd, "\n");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_single_semicolon() {
  // Test that sd accepts a single semicolon
  let result = LaleParser::parse(Rule::sd, ";");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_multiple_newlines() {
  // Test that sd accepts multiple newlines
  let result = LaleParser::parse(Rule::sd, "\n\n\n");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_multiple_semicolons() {
  // Test that sd accepts multiple semicolons
  let result = LaleParser::parse(Rule::sd, ";;;");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_mixed_delimiters() {
  // Test that sd accepts mixed newlines and semicolons
  let result = LaleParser::parse(Rule::sd, ";\n;;\n;");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_delimiters_with_surrounding_whitespace() {
  // Test that sd accepts delimiters with surrounding horizontal whitespace
  let result = LaleParser::parse(Rule::sd, "  \n  ");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_accepts_complex_delimiter_pattern() {
  // Test sd with optional whitespace before and after delimiters
  let result = LaleParser::parse(Rule::sd, "  ;  \n  ;  ");
  assert!(result.is_ok());
}

#[test]
fn test_sd_rule_rejects_empty() {
  // Test that sd rejects empty input (requires at least one delimiter)
  let result = LaleParser::parse(Rule::sd, "");
  assert!(result.is_err(), "sd should require at least one delimiter");
}

#[test]
fn test_sd_rule_rejects_spaces_only() {
  // Test that sd rejects spaces without delimiters
  let result = LaleParser::parse(Rule::sd, "   ");
  assert!(
    result.is_err(),
    "sd should not accept spaces without delimiters"
  );
}

#[test]
fn test_sd_rule_rejects_non_delimiter_characters() {
  // Test that sd rejects non-delimiter characters
  let result = LaleParser::parse(Rule::sd, "abc");
  assert!(result.is_err());
}

// ==================== INFO COMPONENT TESTS ====================
// These tests verify that os_ln and ms_ln correctly handle the optional info component
// (comments and documentation that can precede whitespace)

#[test]
fn test_os_ln_with_comment_before_newline() {
  // Test os_ln with a comment followed by newline
  let result = LaleParser::parse(Rule::os_ln, "// comment\n");
  assert!(
    result.is_ok(),
    "os_ln should accept comment followed by newline"
  );
}

#[test]
fn test_os_ln_with_doc_before_newline() {
  // Test os_ln with documentation comment followed by newline
  let result = LaleParser::parse(Rule::os_ln, "/// doc\n");
  assert!(
    result.is_ok(),
    "os_ln should accept doc comment followed by newline"
  );
}

#[test]
fn test_ms_ln_with_comment_before_whitespace() {
  // Test ms_ln with a comment followed by mandatory whitespace
  let result = LaleParser::parse(Rule::ms_ln, "// comment\n ");
  assert!(
    result.is_ok(),
    "ms_ln should accept comment followed by newline and spaces"
  );
}

#[test]
fn test_ms_ln_with_doc_before_multiple_newlines() {
  // Test ms_ln with doc comment followed by multiple newlines
  let result = LaleParser::parse(Rule::ms_ln, "/// documentation\n\n\n");
  assert!(
    result.is_ok(),
    "ms_ln should accept doc followed by multiple newlines"
  );
}

#[test]
fn test_os_ln_with_comment_and_multiple_newlines() {
  // Test os_ln combining comment with multiple newlines
  let result = LaleParser::parse(Rule::os_ln, "// single-line comment\n\n\n");
  assert!(
    result.is_ok(),
    "os_ln should handle comment with multiple subsequent newlines"
  );
}

// ==================== LINE CONTINUATION EDGE CASES ====================

#[test]
fn test_os_ln_with_multiple_line_continuations() {
  // Test os_ln with multiple backslash-newline continuations
  let result = LaleParser::parse(Rule::os_ln, " \\\n \\\n ");
  assert!(
    result.is_ok(),
    "os_ln should handle multiple line continuations"
  );
}

#[test]
fn test_ms_ln_with_line_continuation_only() {
  // Test ms_ln with line continuation as the mandatory whitespace
  let result = LaleParser::parse(Rule::ms_ln, "\\\n");
  assert!(
    result.is_ok(),
    "ms_ln should accept line continuation as mandatory whitespace"
  );
}

#[test]
fn test_os_ln_line_continuation_followed_by_newline() {
  // Test os_ln with line continuation followed by regular newline
  let result = LaleParser::parse(Rule::os_ln, " \\\n \n ");
  assert!(
    result.is_ok(),
    "os_ln should handle line continuation followed by regular newline"
  );
}

#[test]
fn test_os_line_with_multiple_continuations() {
  // Test os (no newlines) with multiple line continuations
  let result = LaleParser::parse(Rule::os, "\\\n\\\n");
  assert!(
    result.is_ok(),
    "os should accept multiple sequential line continuations"
  );
}

// ==================== MIXED LINE ENDING TESTS ====================

#[test]
fn test_os_ln_with_windows_and_unix_mixed() {
  // Test os_ln with mixed Windows (CR+LF) and Unix (LF) line endings
  let result = LaleParser::parse(Rule::os_ln, "\r\n\n\r\n");
  assert!(
    result.is_ok(),
    "os_ln should handle mixed Windows/Unix line endings"
  );
}

#[test]
fn test_ms_ln_with_windows_line_ending_and_spaces() {
  // Test ms_ln with Windows line ending and spaces
  let result = LaleParser::parse(Rule::ms_ln, "  \r\n  ");
  assert!(
    result.is_ok(),
    "ms_ln should handle Windows line endings with spaces"
  );
}

#[test]
fn test_os_ln_carriage_return_before_newline() {
  // Test os_ln with CR followed by LF (Windows style)
  let result = LaleParser::parse(Rule::os_ln, "\r\n");
  assert!(result.is_ok(), "os_ln should handle CR followed by LF");
}

#[test]
fn test_sd_with_windows_line_ending() {
  // Test sd with Windows line ending
  let result = LaleParser::parse(Rule::sd, "\r\n");
  assert!(
    result.is_ok(),
    "sd should accept Windows line ending (CR+LF)"
  );
}

#[test]
fn test_sd_mixed_windows_and_unix_delimiters() {
  // Test sd with mixed Windows and Unix line endings
  let result = LaleParser::parse(Rule::sd, "\r\n\n;");
  assert!(
    result.is_ok(),
    "sd should handle mixed Windows/Unix line endings with semicolons"
  );
}

// ==================== COMPLEX MULTI-COMPONENT TESTS ====================

#[test]
fn test_os_ln_comment_with_line_continuation() {
  // Test os_ln with comment containing line continuation-like text
  // (the backslash is part of the comment, not a line continuation)
  let result = LaleParser::parse(Rule::os_ln, "// comment\n ");
  assert!(
    result.is_ok(),
    "os_ln should handle comment followed by newline and space"
  );
}

#[test]
fn test_ms_ln_tabs_newlines_spaces_mix() {
  // Test ms_ln with complex mix of tabs, newlines, and spaces
  let result = LaleParser::parse(Rule::ms_ln, "\t\n  \t  \n\t");
  assert!(
    result.is_ok(),
    "ms_ln should handle complex whitespace patterns"
  );
}

#[test]
fn test_os_rule_tabs_and_line_continuations() {
  // Test os with tabs and line continuations mixed
  let result = LaleParser::parse(Rule::os, "\t \\\n \t");
  assert!(
    result.is_ok(),
    "os should handle tabs with line continuations"
  );
}

#[test]
fn test_sd_multiple_semicolons_with_spaces() {
  // Test sd with multiple semicolons separated by spaces
  let result = LaleParser::parse(Rule::sd, " ; ; ; ");
  assert!(
    result.is_ok(),
    "sd should handle multiple semicolons with surrounding spaces"
  );
}

#[test]
fn test_s_rule_only_carriage_return() {
  // Additional test for carriage return alone (should pass s rule)
  let result = LaleParser::parse(Rule::s, "\r");
  assert!(result.is_ok(), "s rule should accept carriage return");
}

#[test]
fn test_os_rule_carriage_return_handling() {
  // Test os with carriage returns
  let result = LaleParser::parse(Rule::os, "\r\r\r");
  assert!(result.is_ok(), "os should accept multiple carriage returns");
}

#[test]
fn test_os_ln_newline_only() {
  // Verify os_ln accepts single newline (should be fine)
  let result = LaleParser::parse(Rule::os_ln, "\n");
  assert!(result.is_ok(), "os_ln should accept newline");
}

#[test]
fn test_sd_rule_semicolon_with_whitespace_before_and_after() {
  // Test sd with specific spacing pattern
  let result = LaleParser::parse(Rule::sd, "  ;  \n  ;  ");
  assert!(
    result.is_ok(),
    "sd should match the pattern documented in comments"
  );
}
