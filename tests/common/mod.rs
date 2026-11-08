//! Shared helpers for integration tests.

use lale::{LaleParser, Rule};
use pest::Parser;
use pest::iterators::Pairs;

/// Parse `input` with `rule` and require that the match consumes the entire
/// input.
///
/// Pest's `Parser::parse` otherwise succeeds on a *prefix* match: a rule such
/// as `Rule::type_name` will happily match the leading identifier of
/// `"array of array of i32"` and ignore the rest. A test that only checks
/// `.is_ok()` / `.is_err()` can therefore pass for the wrong reason. Callers
/// that need a whole-input parse should use this helper and then inspect the
/// returned pairs.
pub fn parse_fully(rule: Rule, input: &str) -> Result<Pairs<'_, Rule>, String> {
  let pairs = LaleParser::parse(rule, input).map_err(|e| format!("parse error: {e}"))?;
  let consumed_end = pairs
    .clone()
    .map(|pair| pair.as_span().end())
    .max()
    .unwrap_or(0);
  if consumed_end != input.len() {
    return Err(format!(
      "rule {rule:?} matched only a prefix ({consumed_end} of {} bytes) of {input:?}",
      input.len()
    ));
  }
  Ok(pairs)
}
