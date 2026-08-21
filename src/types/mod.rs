//! Normalized physical units for semantic unit comparison.
//!
//! This module provides a normalized representation of physical units that allows
//! correct comparison of semantically equivalent units like `kg*m^2/s^2` and `<kg>*<m/s>^2`.
//!
//! Units are represented as a map of base unit names to their exponents:
//! - `kg*m^2/s^2` → {kg: 1, m: 2, s: -2}
//! - `m/s` → {m: 1, s: -1}

use std::collections::BTreeMap;

/// A normalized physical unit represented as base units with their exponents.
///
/// Examples:
/// - `kg` → {kg: 1}
/// - `m/s` → {m: 1, s: -1}
/// - `kg*m^2/s^2` → {kg: 1, m: 2, s: -2}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedUnit {
  /// Map from base unit name to its exponent.
  /// A value without unit is represented by an empty map.
  components: BTreeMap<String, i64>,
}

impl NormalizedUnit {
  /// Create a unit without components (unitless).
  pub fn unitless() -> Self {
    NormalizedUnit {
      components: BTreeMap::new(),
    }
  }

  /// Create a unit from a single base unit with exponent 1.
  pub fn base(name: &str) -> Self {
    let mut components = BTreeMap::new();
    components.insert(name.to_string(), 1);
    NormalizedUnit { components }
  }

  /// Check if this unit is without unit.
  pub fn is_unitless(&self) -> bool {
    self.components.is_empty()
  }

  /// Parse a unit string into a normalized unit.
  ///
  /// Handles formats like:
  /// - `kg` → single base unit
  /// - `m/s` → division
  /// - `kg*m^2/s^2` → compound unit
  /// - `<kg>*<m/s>^2` → nested units from expression evaluation
  pub fn parse(s: &str) -> Self {
    let s = s.trim();
    if s.is_empty() {
      return Self::unitless();
    }

    let mut result = Self::unitless();
    let mut current_pos = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut pending_op: Option<char> = None;

    while current_pos < chars.len() {
      let ch = chars[current_pos];

      if ch == '*' || ch == '⋅' {
        pending_op = Some('*');
        current_pos += 1;
        continue;
      }

      if ch == '/' || ch == '÷' || ch == '⁄' || ch == '∕' {
        pending_op = Some('/');
        current_pos += 1;
        continue;
      }

      if ch.is_whitespace() {
        current_pos += 1;
        continue;
      }

      let (term, new_pos) = Self::parse_term(&chars, current_pos);
      current_pos = new_pos;

      match pending_op {
        None | Some('*') => result = result.multiply(&term),
        Some('/') => result = result.divide(&term),
        _ => {}
      }
      pending_op = None;
    }

    result.cleanup();
    result
  }

  /// Parse a single term (base unit with optional exponent).
  fn parse_term(chars: &[char], start: usize) -> (Self, usize) {
    let mut pos = start;

    if pos < chars.len() && chars[pos] == '<' {
      return Self::parse_bracketed(chars, pos);
    }

    if pos < chars.len() && chars[pos] == '(' {
      return Self::parse_parenthesized(chars, pos);
    }

    let base_start = pos;
    while pos < chars.len() {
      let ch = chars[pos];
      if ch == '*' || ch == '/' || ch == '^' || ch == '÷' || ch == '⋅' || ch == '⁄' || ch == '∕' {
        break;
      }
      if ch == '<' || ch == '(' {
        break;
      }
      pos += 1;
    }

    let base: String = chars[base_start..pos].iter().collect();
    let base = base.trim();

    // Handle superscript exponents directly attached to unit name (e.g., m² → m^2)
    let (base, sup_exp) = Self::strip_superscript_exponent(base);

    if base.is_empty() || base == "1" {
      return (Self::unitless(), pos);
    }

    let (caret_exp, new_pos) = Self::parse_exponent(chars, pos);
    let exp = sup_exp * caret_exp;

    let mut result = Self::base(&base);
    if exp != 1 {
      result = result.power(exp);
    }

    (result, new_pos)
  }

  /// Strip trailing superscript exponent from a unit name.
  /// Returns (base_name, exponent). E.g., "m²" → ("m", 2), "m⁻²" → ("m", -2).
  fn strip_superscript_exponent(base: &str) -> (String, i64) {
    let chars: Vec<char> = base.chars().collect();
    let len = chars.len();
    if len == 0 {
      return (base.to_string(), 1);
    }

    // Check if the last character is a superscript digit
    let superscript_digit = |ch: char| -> Option<i64> {
      match ch {
        '⁰' => Some(0),
        '¹' => Some(1),
        '²' => Some(2),
        '³' => Some(3),
        '⁴' => Some(4),
        '⁵' => Some(5),
        '⁶' => Some(6),
        '⁷' => Some(7),
        '⁸' => Some(8),
        '⁹' => Some(9),
        _ => None,
      }
    };

    // Look for superscript sign followed by digits: e.g., "⁻²"
    let superscript_sign = |ch: char| -> Option<i64> {
      match ch {
        '⁺' => Some(1),
        '⁻' => Some(-1),
        _ => None,
      }
    };

    let mut idx = len;
    let mut exponent: i64 = 0;
    let mut digit_count = 0;
    let mut multiplier: i64 = 1;

    // Parse superscript digits from right to left
    while idx > 0 {
      let ch = chars[idx - 1];
      if let Some(d) = superscript_digit(ch) {
        exponent += d * 10_i64.pow(digit_count);
        digit_count += 1;
        idx -= 1;
      } else {
        break;
      }
    }

    // Check for superscript sign before the digits
    if idx > 0
      && let Some(s) = superscript_sign(chars[idx - 1])
    {
      multiplier = s;
      idx -= 1;
    }

    if digit_count == 0 {
      return (base.to_string(), 1);
    }

    let name = chars[..idx].iter().collect::<String>();
    (name, multiplier * exponent)
  }

  /// Parse a bracketed unit like `<m/s>`.
  fn parse_bracketed(chars: &[char], start: usize) -> (Self, usize) {
    let mut pos = start + 1;
    let mut depth = 1;
    let content_start = pos;

    while pos < chars.len() && depth > 0 {
      if chars[pos] == '<' {
        depth += 1;
      } else if chars[pos] == '>' {
        depth -= 1;
      }
      if depth > 0 {
        pos += 1;
      }
    }

    let content: String = chars[content_start..pos].iter().collect();
    pos += 1;

    let inner = Self::parse(&content);

    let (exp, new_pos) = Self::parse_exponent(chars, pos);

    let result = if exp != 1 { inner.power(exp) } else { inner };

    (result, new_pos)
  }

  /// Parse a parenthesized unit like `(m/s)`.
  fn parse_parenthesized(chars: &[char], start: usize) -> (Self, usize) {
    let mut pos = start + 1;
    let mut depth = 1;
    let content_start = pos;

    while pos < chars.len() && depth > 0 {
      if chars[pos] == '(' {
        depth += 1;
      } else if chars[pos] == ')' {
        depth -= 1;
      }
      if depth > 0 {
        pos += 1;
      }
    }

    let content: String = chars[content_start..pos].iter().collect();
    pos += 1;

    let inner = Self::parse(&content);

    let (exp, new_pos) = Self::parse_exponent(chars, pos);

    let result = if exp != 1 { inner.power(exp) } else { inner };

    (result, new_pos)
  }

  /// Parse an exponent like `^2` or `^-1`.
  fn parse_exponent(chars: &[char], start: usize) -> (i64, usize) {
    let mut pos = start;

    if pos >= chars.len() || chars[pos] != '^' {
      return (1, pos);
    }
    pos += 1;

    let mut negative = false;
    if pos < chars.len() && chars[pos] == '-' {
      negative = true;
      pos += 1;
    } else if pos < chars.len() && chars[pos] == '+' {
      pos += 1;
    }

    let mut exp: i64 = 0;
    while pos < chars.len() && chars[pos].is_ascii_digit() {
      exp = exp * 10 + (chars[pos] as i64 - '0' as i64);
      pos += 1;
    }

    if exp == 0 {
      exp = 1;
    }

    if negative {
      exp = -exp;
    }

    (exp, pos)
  }

  /// Multiply two units.
  pub fn multiply(&self, other: &Self) -> Self {
    let mut result = self.components.clone();
    for (base, exp) in &other.components {
      *result.entry(base.clone()).or_insert(0) += exp;
    }
    let mut nu = NormalizedUnit { components: result };
    nu.cleanup();
    nu
  }

  /// Divide this unit by another.
  pub fn divide(&self, other: &Self) -> Self {
    let mut result = self.components.clone();
    for (base, exp) in &other.components {
      *result.entry(base.clone()).or_insert(0) -= exp;
    }
    let mut nu = NormalizedUnit { components: result };
    nu.cleanup();
    nu
  }

  /// Raise this unit to a power.
  pub fn power(&self, exp: i64) -> Self {
    if exp == 0 {
      return Self::unitless();
    }
    let mut result = BTreeMap::new();
    for (base, e) in &self.components {
      result.insert(base.clone(), e * exp);
    }
    let mut nu = NormalizedUnit { components: result };
    nu.cleanup();
    nu
  }

  /// Remove zero-exponent components.
  fn cleanup(&mut self) {
    self.components.retain(|_, exp| *exp != 0);
  }

  /// Convert to a canonical string representation for display.
  fn format_unit(&self) -> String {
    if self.components.is_empty() {
      return "1".to_string();
    }

    let mut positive: Vec<(&String, &i64)> =
      self.components.iter().filter(|(_, e)| **e > 0).collect();
    let mut negative: Vec<(&String, &i64)> =
      self.components.iter().filter(|(_, e)| **e < 0).collect();

    positive.sort_by(|a, b| a.0.cmp(b.0));
    negative.sort_by(|a, b| a.0.cmp(b.0));

    /// Convert an integer exponent to superscript notation (e.g., 2 → "²").
    /// Negative exponents are shown in the denominator, so only positive digits appear.
    fn format_exponent(abs_exp: u64) -> String {
      if abs_exp <= 1 {
        return String::new();
      }
      let superscript_digit = |d: u32| -> char {
        match d {
          0 => '⁰',
          1 => '¹',
          2 => '²',
          3 => '³',
          4 => '⁴',
          5 => '⁵',
          6 => '⁶',
          7 => '⁷',
          8 => '⁸',
          9 => '⁹',
          _ => '?',
        }
      };
      abs_exp
        .to_string()
        .chars()
        .filter_map(|c| c.to_digit(10).map(superscript_digit))
        .collect()
    }

    let numerator: Vec<String> = positive
      .iter()
      .map(|(base, exp)| format!("{}{}", base, format_exponent(**exp as u64)))
      .collect();

    let denominator: Vec<String> = negative
      .iter()
      .map(|(base, exp)| format!("{}{}", base, format_exponent((-**exp) as u64)))
      .collect();

    let sep = "⋅";
    if denominator.is_empty() {
      numerator.join(sep)
    } else if numerator.is_empty() {
      format!("1/{}", denominator.join(sep))
    } else {
      format!("{}/{}", numerator.join(sep), denominator.join(sep))
    }
  }
}

impl std::fmt::Display for NormalizedUnit {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.format_unit())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_simple() {
    let u = NormalizedUnit::parse("kg");
    assert_eq!(u.components.get("kg"), Some(&1));
  }

  #[test]
  fn test_parse_division() {
    let u = NormalizedUnit::parse("m/s");
    assert_eq!(u.components.get("m"), Some(&1));
    assert_eq!(u.components.get("s"), Some(&-1));
  }

  #[test]
  fn test_parse_compound() {
    let u = NormalizedUnit::parse("kg*m^2/s^2");
    assert_eq!(u.components.get("kg"), Some(&1));
    assert_eq!(u.components.get("m"), Some(&2));
    assert_eq!(u.components.get("s"), Some(&-2));
  }

  #[test]
  fn test_parse_dot_operator() {
    // ⋅ (U+22C5 DOT OPERATOR) should be treated the same as *
    let u = NormalizedUnit::parse("N⋅m");
    assert_eq!(u.components.get("N"), Some(&1));
    assert_eq!(u.components.get("m"), Some(&1));
  }

  #[test]
  fn test_parse_mixed_mul_signs() {
    // * and ⋅ should normalize to the same components in units
    let u1 = NormalizedUnit::parse("kg*m/s");
    let u2 = NormalizedUnit::parse("kg⋅m/s");
    assert_eq!(u1.components, u2.components);
  }

  #[test]
  fn test_parse_bracketed() {
    let u = NormalizedUnit::parse("<m/s>^2");
    assert_eq!(u.components.get("m"), Some(&2));
    assert_eq!(u.components.get("s"), Some(&-2));
  }

  #[test]
  fn test_parse_complex_expression() {
    let u = NormalizedUnit::parse("<kg>*<m/s>^2");
    assert_eq!(u.components.get("kg"), Some(&1));
    assert_eq!(u.components.get("m"), Some(&2));
    assert_eq!(u.components.get("s"), Some(&-2));
  }

  #[test]
  fn test_equivalent_units() {
    let u1 = NormalizedUnit::parse("kg*m^2/s^2");
    let u2 = NormalizedUnit::parse("<kg>*<m/s>^2");
    assert_eq!(u1, u2);
  }

  #[test]
  fn test_multiply() {
    let u1 = NormalizedUnit::parse("kg");
    let u2 = NormalizedUnit::parse("m/s");
    let result = u1.multiply(&u2);
    assert_eq!(result.components.get("kg"), Some(&1));
    assert_eq!(result.components.get("m"), Some(&1));
    assert_eq!(result.components.get("s"), Some(&-1));
  }

  #[test]
  fn test_power() {
    let u = NormalizedUnit::parse("m/s");
    let result = u.power(2);
    assert_eq!(result.components.get("m"), Some(&2));
    assert_eq!(result.components.get("s"), Some(&-2));
  }

  #[test]
  fn test_unitless() {
    let u = NormalizedUnit::unitless();
    assert!(u.is_unitless());
  }

  #[test]
  fn test_parse_superscript_single_digit() {
    // m² should parse as m^2
    let u = NormalizedUnit::parse("m²");
    assert_eq!(u.components.get("m"), Some(&2));
  }

  #[test]
  fn test_parse_superscript_negative() {
    // m⁻² should parse as m^-2
    let u = NormalizedUnit::parse("m⁻²");
    assert_eq!(u.components.get("m"), Some(&-2));
  }

  #[test]
  fn test_parse_superscript_multi_digit() {
    // m²³ should parse as m^23
    let u = NormalizedUnit::parse("m²³");
    assert_eq!(u.components.get("m"), Some(&23));
  }

  #[test]
  fn test_parse_superscript_positive_sign() {
    // m⁺² should parse as m^2 (explicit positive)
    let u = NormalizedUnit::parse("m⁺²");
    assert_eq!(u.components.get("m"), Some(&2));
  }

  #[test]
  fn test_parse_superscript_in_compound_unit() {
    // kg⋅m²/s³ should parse correctly
    let u = NormalizedUnit::parse("kg⋅m²/s³");
    assert_eq!(u.components.get("kg"), Some(&1));
    assert_eq!(u.components.get("m"), Some(&2));
    assert_eq!(u.components.get("s"), Some(&-3));
  }

  #[test]
  fn test_parse_superscript_in_bracketed() {
    // <m²>^3 should parse as m^6
    let u = NormalizedUnit::parse("<m²>^3");
    assert_eq!(u.components.get("m"), Some(&6));
  }

  #[test]
  fn test_parse_superscript_negative_in_compound() {
    // m⁻²⋅kg should parse as kg/m²
    let u = NormalizedUnit::parse("m⁻²⋅kg");
    assert_eq!(u.components.get("m"), Some(&-2));
    assert_eq!(u.components.get("kg"), Some(&1));
  }

  #[test]
  fn test_parse_superscript_zero_exponent() {
    // m⁰ should be unitless (exponent 0)
    let u = NormalizedUnit::parse("m⁰");
    assert!(u.components.is_empty());
  }

  #[test]
  fn test_no_superscript_unaffected() {
    // Regular unit without superscript should work as before
    let u = NormalizedUnit::parse("kg");
    assert_eq!(u.components.get("kg"), Some(&1));
  }

  #[test]
  fn test_parse_fraction_slash() {
    // U+2044 FRACTION SLASH should be treated as division
    let u = NormalizedUnit::parse("m⁄s");
    assert_eq!(u.components.get("m"), Some(&1));
    assert_eq!(u.components.get("s"), Some(&-1));
  }

  #[test]
  fn test_parse_division_slash() {
    // U+2215 DIVISION SLASH should be treated as division
    let u = NormalizedUnit::parse("m∕s");
    assert_eq!(u.components.get("m"), Some(&1));
    assert_eq!(u.components.get("s"), Some(&-1));
  }

  #[test]
  fn test_parse_fraction_slash_compound() {
    // kg⋅m⁄s² should parse as kg*m/s^2
    let u = NormalizedUnit::parse("kg⋅m⁄s²");
    assert_eq!(u.components.get("kg"), Some(&1));
    assert_eq!(u.components.get("m"), Some(&1));
    assert_eq!(u.components.get("s"), Some(&-2));
  }

  #[test]
  fn test_all_division_symbols_equivalent() {
    let u1 = NormalizedUnit::parse("m/s");
    let u2 = NormalizedUnit::parse("m÷s");
    let u3 = NormalizedUnit::parse("m⁄s");
    let u4 = NormalizedUnit::parse("m∕s");
    assert_eq!(u1, u2, "m/s should equal m÷s");
    assert_eq!(u1, u3, "m/s should equal m⁄s");
    assert_eq!(u1, u4, "m/s should equal m∕s");
  }

  #[test]
  fn test_superscript_and_caret_combine() {
    // m²^3 should parse as m^6 (superscript * caret)
    let u = NormalizedUnit::parse("m²^3");
    assert_eq!(u.components.get("m"), Some(&6));
  }
}
