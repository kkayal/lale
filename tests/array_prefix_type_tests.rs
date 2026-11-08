//! Regression test: the bracket-prefix array spelling `[i32]` must be rejected
//! with a clear error, not silently lowered to a scalar `i32`.
//!
//! See `doc/implementation_notes.md` item A.

use lale::ast_builder::build_program;
use lale::{LaleParser, Rule};
use pest::Parser;

#[test]
fn bracket_prefix_array_type_is_rejected() {
  let code = "var x as [i32] = [1, 2, 3]\n";
  let pairs = LaleParser::parse(Rule::program, code).expect("parse");
  let err = build_program(pairs, "test.lale")
    .expect_err("`[i32]` must be rejected, not silently lowered to `i32`");
  assert!(
    err.contains("require an explicit size"),
    "unexpected error: {err}"
  );
}
