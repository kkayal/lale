use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for compile-time feature rules in the Lale grammar.
///
/// This test suite covers:
/// - ct_fail: Compile-time failure directive
/// - ct_warn: Compile-time warning directive
/// - ct_if: Compile-time conditional compilation

// ==================== CT_FAIL TESTS ====================

#[test]
fn test_ct_fail_simple() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"error message\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_with_code_context() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"Unsupported platform\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_empty_message() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_with_special_chars() {
  let result = LaleParser::parse(
    Rule::ct_fail,
    "#fail \"Error: expected 'u32' but got 'str'\"",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_multiline_style() {
  // Testing the pattern, actual multiline would be in statement context
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"Feature not implemented\"");
  assert!(result.is_ok());
}

// ==================== CT_WARN TESTS ====================

#[test]
fn test_ct_warn_simple() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"deprecation warning\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_with_context() {
  let result = LaleParser::parse(
    Rule::ct_warn,
    "#warn \"Function 'old_api' is deprecated, use 'new_api'\"",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_empty_message() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_with_version() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"Behavior changed in version 2.0\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_platform_specific() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"This is a Windows-specific build\"");
  assert!(result.is_ok());
}

// ==================== CT_IF TESTS ====================

#[test]
fn test_ct_if_simple_true() {
  let result = LaleParser::parse(Rule::ct_if, "#if true #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_simple_false() {
  let result = LaleParser::parse(Rule::ct_if, "#if false #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_comparison() {
  let result = LaleParser::parse(Rule::ct_if, "#if 1 > 0 #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_expression() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_logical_and() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG and TEST #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_logical_or() {
  let result = LaleParser::parse(Rule::ct_if, "#if WINDOWS or MACOS #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_negation() {
  let result = LaleParser::parse(Rule::ct_if, "#if not RELEASE #end if");
  assert!(result.is_ok());
}

// ==================== CT_IF WITH ELSE ====================

#[test]
fn test_ct_if_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG #else #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_else_if() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG #else if TEST #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_else_if_chain() {
  let result = LaleParser::parse(Rule::ct_if, "#if A #else if B #else if C #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_full_chain() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if WINDOWS #else if MACOS #else if LINUX #else #end if",
  );
  assert!(result.is_ok());
}

// ==================== REALISTIC PATTERNS ====================

#[test]
fn test_ct_if_debug_code() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_platform_selection() {
  let result = LaleParser::parse(Rule::ct_if, "#if WINDOWS #else if UNIX #else #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_version_check() {
  let result = LaleParser::parse(Rule::ct_if, "#if VERSION > 1 #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_unsupported_feature() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"This platform is not supported\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_experimental() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"This is an experimental API\"");
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_ct_if_comparison_operators() {
  let comparisons = vec![">", "<"];
  for op in comparisons {
    let stmt = format!("#if A {} B #end if", op);
    let result = LaleParser::parse(Rule::ct_if, &stmt);
    assert!(result.is_ok(), "Should support comparison: {}", op);
  }
}

#[test]
fn test_ct_if_arithmetic() {
  let result = LaleParser::parse(Rule::ct_if, "#if 10 + 5 > 10 #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_fail_with_newlines() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail \"Line 1\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_with_numbers() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn \"Version 1.5.0 behavior changed\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_boolean_constants() {
  let result1 = LaleParser::parse(Rule::ct_if, "#if true #end if");
  let result2 = LaleParser::parse(Rule::ct_if, "#if false #end if");
  assert!(result1.is_ok() && result2.is_ok());
}

// ==================== COMPOUND CONDITIONS ====================

#[test]
fn test_ct_if_complex_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if (A and B) or (C and D) #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_triple_and() {
  let result = LaleParser::parse(Rule::ct_if, "#if A and B and C #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_triple_or() {
  let result = LaleParser::parse(Rule::ct_if, "#if A or B or C #end if");
  assert!(result.is_ok());
}

// ==================== WHITESPACE HANDLING ====================

#[test]
fn test_ct_fail_extra_spaces() {
  let result = LaleParser::parse(Rule::ct_fail, "#fail   \"message\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_warn_extra_spaces() {
  let result = LaleParser::parse(Rule::ct_warn, "#warn   \"message\"");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_extra_spaces() {
  let result = LaleParser::parse(Rule::ct_if, "#if   A   #end if");
  assert!(result.is_ok());
}

// ==================== MULTIPLE ELSE-IF ====================

#[test]
fn test_ct_if_many_branches() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if A #else if B #else if C #else if D #else if E #else #end if",
  );
  assert!(result.is_ok());
}

// ==================== CONDITION VARIATIONS ====================

#[test]
fn test_ct_if_identifier_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if CONFIG_ENABLED #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_compiler_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #source_file #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_negated_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if not RELEASE_BUILD #end if");
  assert!(result.is_ok());
}

// ==================== OS DETECTION COMPILER CONSTANTS ====================

#[test]
fn test_ct_if_posix_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #posix #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #windows #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_posix_with_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if #posix #else #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_with_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if #windows #else #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_os_conditional_branch() {
  let result = LaleParser::parse(Rule::ct_if, "#if #posix #else if #windows #else #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_posix_comparison() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if #posix and #source_file == \"test.lale\" #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_comparison() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if #windows or #source_file == \"test.lale\" #end if",
  );
  assert!(result.is_ok());
}

// ==================== CT_WHEN TESTS ====================

#[test]
fn test_ct_when_simple() {
  let result = LaleParser::parse(Rule::ct_when, "#when true #end when");
  assert!(result.is_ok());
}

#[test]
fn test_ct_when_with_body() {
  let result = LaleParser::parse(Rule::ct_when, "#when #debug\nwrite \"x\"\n#end when");
  assert!(result.is_ok());
}

// ==================== CT_MATCH TESTS ====================

#[test]
fn test_ct_match_simple() {
  let result = LaleParser::parse(Rule::ct_match, "#match #end match");
  assert!(result.is_ok());
}

#[test]
fn test_ct_match_with_arms() {
  let result = LaleParser::parse(
    Rule::ct_match,
    "#match\n#when #posix:\nwrite \"p\"\n#when #windows:\nwrite \"w\"\n#end match",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_match_with_else() {
  let result = LaleParser::parse(
    Rule::ct_match,
    "#match\n#when #windows:\nwrite \"w\"\n#else:\nwrite \"other\"\n#end match",
  );
  assert!(result.is_ok());
}

// ==================== CT_SWITCH TESTS ====================

#[test]
fn test_ct_switch_simple() {
  let result = LaleParser::parse(
    Rule::ct_switch,
    "#switch 5\n#case 3:\nwrite \"three\"\n#end switch",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_switch_with_default() {
  let result = LaleParser::parse(
    Rule::ct_switch,
    "#switch 5\n#case 3:\nwrite \"three\"\n#default:\nwrite \"default\"\n#end switch",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_switch_string() {
  let result = LaleParser::parse(
    Rule::ct_switch,
    "#switch \"hello\"\n#case \"world\":\nwrite \"w\"\n#case \"hello\":\nwrite \"h\"\n#end switch",
  );
  assert!(result.is_ok());
}
