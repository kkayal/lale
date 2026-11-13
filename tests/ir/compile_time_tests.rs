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
  let result = LaleParser::parse(Rule::ct_if, "#if true move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_simple_false() {
  let result = LaleParser::parse(Rule::ct_if, "#if false move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_comparison() {
  let result = LaleParser::parse(Rule::ct_if, "#if 1 > 0 move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_expression() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_logical_and() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG and TEST move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_logical_or() {
  let result = LaleParser::parse(Rule::ct_if, "#if WINDOWS or MACOS move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_with_negation() {
  let result = LaleParser::parse(Rule::ct_if, "#if not RELEASE move on #end if");
  assert!(result.is_ok());
}

// ==================== CT_IF WITH ELSE ====================

#[test]
fn test_ct_if_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG move on #else move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_else_if() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if DEBUG move on #else if TEST move on #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_else_if_chain() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if A move on #else if B move on #else if C move on #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_full_chain() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if WINDOWS move on #else if MACOS move on #else if LINUX move on #else move on #end if",
  );
  assert!(result.is_ok());
}

// ==================== REALISTIC PATTERNS ====================

#[test]
fn test_ct_if_debug_code() {
  let result = LaleParser::parse(Rule::ct_if, "#if DEBUG move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_platform_selection() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if WINDOWS move on #else if UNIX move on #else move on #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_version_check() {
  let result = LaleParser::parse(Rule::ct_if, "#if VERSION > 1 move on #end if");
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
    let stmt = format!("#if A {} B move on #end if", op);
    let result = LaleParser::parse(Rule::ct_if, &stmt);
    assert!(result.is_ok(), "Should support comparison: {}", op);
  }
}

#[test]
fn test_ct_if_arithmetic() {
  let result = LaleParser::parse(Rule::ct_if, "#if 10 + 5 > 10 move on #end if");
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
  let result1 = LaleParser::parse(Rule::ct_if, "#if true move on #end if");
  let result2 = LaleParser::parse(Rule::ct_if, "#if false move on #end if");
  assert!(result1.is_ok() && result2.is_ok());
}

// ==================== COMPOUND CONDITIONS ====================

#[test]
fn test_ct_if_complex_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if (A and B) or (C and D) move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_triple_and() {
  let result = LaleParser::parse(Rule::ct_if, "#if A and B and C move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_triple_or() {
  let result = LaleParser::parse(Rule::ct_if, "#if A or B or C move on #end if");
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
  let result = LaleParser::parse(Rule::ct_if, "#if   A   move on #end if");
  assert!(result.is_ok());
}

// ==================== MULTIPLE ELSE-IF ====================

#[test]
fn test_ct_if_many_branches() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if A move on #else if B move on #else if C move on #else if D move on #else if E move on #else move on #end if",
  );
  assert!(result.is_ok());
}

// ==================== CONDITION VARIATIONS ====================

#[test]
fn test_ct_if_identifier_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if CONFIG_ENABLED move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_compiler_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #source_file move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_negated_condition() {
  let result = LaleParser::parse(Rule::ct_if, "#if not RELEASE_BUILD move on #end if");
  assert!(result.is_ok());
}

// ==================== OS DETECTION COMPILER CONSTANTS ====================

#[test]
fn test_ct_if_posix_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #posix move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_constant() {
  let result = LaleParser::parse(Rule::ct_if, "#if #windows move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_posix_with_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if #posix move on #else move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_with_else() {
  let result = LaleParser::parse(Rule::ct_if, "#if #windows move on #else move on #end if");
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_os_conditional_branch() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if #posix move on #else if #windows move on #else move on #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_posix_comparison() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if #posix and #source_file == \"test.lale\" move on #end if",
  );
  assert!(result.is_ok());
}

#[test]
fn test_ct_if_windows_comparison() {
  let result = LaleParser::parse(
    Rule::ct_if,
    "#if #windows or #source_file == \"test.lale\" move on #end if",
  );
  assert!(result.is_ok());
}

// ==================== CT_WHEN TESTS ====================

#[test]
fn test_ct_when_simple() {
  let result = LaleParser::parse(Rule::ct_when, "#when true move on #end when");
  assert!(result.is_ok());
}

#[test]
fn test_ct_when_with_body() {
  let result = LaleParser::parse(Rule::ct_when, "#when #debug\nwrite \"x\"\n#end when");
  assert!(result.is_ok());
}

// ==================== CT_MATCH TESTS ====================

#[test]
fn test_ct_match_empty_rejected() {
  let result = LaleParser::parse(Rule::ct_match, "#match #end match");
  assert!(
    result.is_err(),
    "compile-time match without any arm must fail to parse"
  );
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
fn test_ct_switch_empty_rejected() {
  let result = LaleParser::parse(Rule::ct_switch, "#switch 5 #end switch");
  assert!(
    result.is_err(),
    "compile-time switch without any case must fail to parse"
  );
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

// ==================== EMPTY BODY REJECTION ====================
// Compile-time blocks must contain at least one statement, like their runtime
// counterparts. An empty body is rejected by the parser.

#[test]
fn test_ct_if_empty_body_rejected() {
  let result = LaleParser::parse(Rule::ct_if, "#if true #end if");
  assert!(
    result.is_err(),
    "#if without any statement must fail to parse"
  );
}

#[test]
fn test_ct_if_empty_else_rejected() {
  let result = LaleParser::parse(Rule::ct_if, "#if true move on #else #end if");
  assert!(
    result.is_err(),
    "#else without any statement must fail to parse"
  );
}

#[test]
fn test_ct_when_empty_body_rejected() {
  let result = LaleParser::parse(Rule::ct_when, "#when true #end when");
  assert!(
    result.is_err(),
    "#when without any statement must fail to parse"
  );
}

#[test]
fn test_ct_match_arm_empty_body_rejected() {
  let result = LaleParser::parse(Rule::ct_match, "#match\n#when true:\n#end match");
  assert!(
    result.is_err(),
    "#when arm without any statement must fail to parse"
  );
}

#[test]
fn test_ct_match_default_empty_body_rejected() {
  let result = LaleParser::parse(
    Rule::ct_match,
    "#match\n#when true:\nmove on\n#else:\n#end match",
  );
  assert!(
    result.is_err(),
    "#else arm without any statement must fail to parse"
  );
}
