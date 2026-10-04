//! Compile-time constant value.
//!
//! A single value type used across semantic analysis for two purposes:
//!
//! 1. `#switch` case matching (duplicate-case detection) — kept in memory only.
//! 2. Tracking whether a variable's value is a known compile-time constant. This
//!    is persisted in the SQLite `variables` table (`const_value` column) and is
//!    the single source of truth for constant-propagation-style analyses such as
//!    division-by-zero detection.
//!
//! Floats are stored as their IEEE-754 bit pattern so the type remains `Eq` +
//! `Hash` (required for `#switch` duplicate detection) and round-trips losslessly.
//!
//! All scalar kinds (`Bool`, `Int`, `Uint`, `Float`, `Text`) are serializable;
//! aggregate values (structs, enums, arrays, optionals, vectors) are not — see
//! the `variables` table comment in `sqlite_symbol_management.rs`.

use std::fmt;

/// A compile-time constant value.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ConstValue {
  Bool(bool),
  Int(i64),
  Uint(u64),
  /// IEEE-754 bit pattern of an `f64`.
  Float(u64),
  Text(String),
}

impl ConstValue {
  /// Construct a float constant from an `f64`, storing its bit pattern.
  pub fn float(v: f64) -> Self {
    ConstValue::Float(v.to_bits())
  }

  /// The `f64` value of a float constant, if this is a float.
  pub fn as_f64(&self) -> Option<f64> {
    match self {
      ConstValue::Float(bits) => Some(f64::from_bits(*bits)),
      _ => None,
    }
  }

  /// The unsigned integer magnitude of this value, if it is a non-negative
  /// integer. Used for unsigned-arithmetic safety checks (e.g. subtraction
  /// underflow) where only the magnitude matters.
  pub fn as_unsigned(&self) -> Option<u64> {
    match self {
      ConstValue::Uint(u) => Some(*u),
      ConstValue::Int(i) if *i >= 0 => Some(*i as u64),
      _ => None,
    }
  }

  /// The signed integer value of this value, if it is an integer that fits in
  /// an `i64`. Used to coerce a constant into a signed integer IR type.
  pub fn as_signed(&self) -> Option<i64> {
    match self {
      ConstValue::Int(i) => Some(*i),
      ConstValue::Uint(u) if *u <= i64::MAX as u64 => Some(*u as i64),
      _ => None,
    }
  }

  /// The floating-point value of this value, coercing integer kinds to `f64`.
  /// Used to coerce a constant into a floating-point IR type.
  pub fn to_f64(&self) -> Option<f64> {
    match self {
      ConstValue::Int(i) => Some(*i as f64),
      ConstValue::Uint(u) => Some(*u as f64),
      ConstValue::Float(bits) => Some(f64::from_bits(*bits)),
      _ => None,
    }
  }

  /// Whether this value is provably non-zero.
  ///
  /// Only numeric values have a meaningful "zero": a `Bool` or `Text` constant is
  /// conservatively reported as *not* provably non-zero, so a numeric divisor
  /// that somehow resolves to one is still warned about.
  pub fn is_non_zero(&self) -> bool {
    match self {
      ConstValue::Int(i) => *i != 0,
      ConstValue::Uint(u) => *u != 0,
      ConstValue::Float(_) => self.as_f64() != Some(0.0),
      ConstValue::Bool(_) | ConstValue::Text(_) => false,
    }
  }

  /// Serialize to the SQLite storage format (`tag:payload`). The payload is
  /// everything after the first colon, so it may itself contain colons (strings).
  pub fn to_db(&self) -> Option<String> {
    match self {
      ConstValue::Bool(b) => Some(format!("b:{}", if *b { "true" } else { "false" })),
      ConstValue::Int(i) => Some(format!("i:{i}")),
      ConstValue::Uint(u) => Some(format!("u:{u}")),
      ConstValue::Float(bits) => Some(format!("f:{bits}")),
      ConstValue::Text(s) => Some(format!("s:{s}")),
    }
  }

  /// Deserialize from the SQLite storage format. Returns `None` on any malformed
  /// or unknown input (the constant is then treated as absent).
  pub fn from_db(s: &str) -> Option<ConstValue> {
    let (tag, rest) = s.split_once(':')?;
    match tag {
      "b" => match rest {
        "true" => Some(ConstValue::Bool(true)),
        "false" => Some(ConstValue::Bool(false)),
        _ => None,
      },
      "i" => match rest.parse::<i64>() {
        Ok(v) => Some(ConstValue::Int(v)),
        Err(_) => None,
      },
      "u" => match rest.parse::<u64>() {
        Ok(v) => Some(ConstValue::Uint(v)),
        Err(_) => None,
      },
      "f" => match rest.parse::<u64>() {
        Ok(bits) => Some(ConstValue::Float(bits)),
        Err(_) => None,
      },
      "s" => Some(ConstValue::Text(rest.to_string())),
      _ => None,
    }
  }
}

impl fmt::Debug for ConstValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ConstValue::Bool(b) => write!(f, "Bool({b})"),
      ConstValue::Int(i) => write!(f, "Int({i})"),
      ConstValue::Uint(u) => write!(f, "Uint({u})"),
      ConstValue::Float(bits) => write!(f, "Float({})", f64::from_bits(*bits)),
      ConstValue::Text(s) => write!(f, "Text({s:?})"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_float_round_trips_bits() {
    for v in [
      0.0,
      -0.0,
      std::f64::consts::PI,
      1e300,
      -1e-300,
      f64::INFINITY,
    ] {
      let c = ConstValue::float(v);
      assert_eq!(c.as_f64().unwrap().to_bits(), v.to_bits());
    }
  }

  #[test]
  fn test_is_non_zero() {
    assert!(!ConstValue::Int(0).is_non_zero());
    assert!(ConstValue::Int(5).is_non_zero());
    assert!(!ConstValue::Uint(0).is_non_zero());
    assert!(ConstValue::Uint(7).is_non_zero());
    assert!(!ConstValue::float(0.0).is_non_zero());
    assert!(!ConstValue::float(-0.0).is_non_zero());
    assert!(ConstValue::float(0.5).is_non_zero());
    // Bool/Text are conservatively not provably non-zero.
    assert!(!ConstValue::Bool(true).is_non_zero());
    assert!(!ConstValue::Text("x".to_string()).is_non_zero());
  }

  #[test]
  fn test_numeric_coercions() {
    assert_eq!(ConstValue::Int(-5).as_signed(), Some(-5));
    assert_eq!(ConstValue::Int(5).as_signed(), Some(5));
    assert_eq!(ConstValue::Uint(5).as_signed(), Some(5));
    assert_eq!(ConstValue::Uint(u64::MAX).as_signed(), None);
    assert_eq!(ConstValue::float(1.5).as_signed(), None);

    assert_eq!(ConstValue::Int(5).as_unsigned(), Some(5));
    assert_eq!(ConstValue::Int(-5).as_unsigned(), None);
    assert_eq!(ConstValue::Uint(7).as_unsigned(), Some(7));

    assert_eq!(ConstValue::Int(5).to_f64(), Some(5.0));
    assert_eq!(ConstValue::Uint(7).to_f64(), Some(7.0));
    assert_eq!(ConstValue::float(2.5).to_f64(), Some(2.5));
    assert_eq!(ConstValue::Bool(true).to_f64(), None);
  }

  #[test]
  fn test_db_round_trip() {
    let values = [
      ConstValue::Bool(true),
      ConstValue::Bool(false),
      ConstValue::Int(0),
      ConstValue::Int(-42),
      ConstValue::Uint(0),
      ConstValue::Uint(u64::MAX),
      ConstValue::float(std::f64::consts::PI),
      ConstValue::float(-0.0),
      ConstValue::Text(String::new()),
      ConstValue::Text("hello".to_string()),
      ConstValue::Text("hello:world".to_string()),
    ];
    for v in values {
      let s = v.to_db().expect("scalar values must serialize");
      assert_eq!(
        ConstValue::from_db(&s),
        Some(v),
        "round-trip failed for {s:?}"
      );
    }
    assert!(ConstValue::from_db("garbage").is_none());
    assert!(ConstValue::from_db("i:notanint").is_none());
  }
}
