//! Regression tests for the whole-input parse helper.
//!
//! These guard against the prefix-match hazard documented in
//! `doc/implementation_notes.md`: `LaleParser::parse` succeeds on a *prefix*
//! match, so a bare `assert!(parse(...).is_err())` can pass for the wrong
//! reason. `common::parse_fully` requires the rule to consume the entire input.

#[path = "../common/mod.rs"]
mod common;

use common::parse_fully;
use lale::Rule;

#[test]
fn type_name_rejects_a_partial_match() {
  // `type_name` matches the leading `i32` and stops, so a whole-input parse
  // must fail rather than silently pass. The input deliberately avoids
  // `array of ...`: grammars that accept the nested array spelling in the
  // grammar make that a full match, not a prefix match.
  let result = parse_fully(Rule::type_name, "i32 trailing");
  assert!(result.is_err(), "expected a prefix match to be rejected");
  let err = result.unwrap_err();
  assert!(
    err.contains("matched only a prefix"),
    "unexpected error: {err}"
  );
}

#[test]
fn type_name_accepts_a_full_match() {
  assert!(parse_fully(Rule::type_name, "i32").is_ok());
  assert!(parse_fully(Rule::type_name, "i32[5]").is_ok());
  assert!(parse_fully(Rule::type_name, "vec3 of f64").is_ok());
}
