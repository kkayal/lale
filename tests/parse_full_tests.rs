//! Regression tests for the whole-input parse helper.
//!
//! These guard against the prefix-match hazard documented in
//! `doc/implementation_notes.md`: `LaleParser::parse` succeeds on a *prefix*
//! match, so a bare `assert!(parse(...).is_err())` can pass for the wrong
//! reason. `common::parse_fully` requires the rule to consume the entire input.

mod common;

use common::parse_fully;
use lale::Rule;

#[test]
fn type_name_rejects_a_partial_match() {
  // `type_name` matches the leading identifier `array` and stops, so a
  // whole-input parse must fail rather than silently pass.
  let result = parse_fully(Rule::type_name, "array of array of i32");
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
