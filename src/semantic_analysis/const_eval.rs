//! Typed binary folding for compile-time constant evaluation.
//!
//! This module is the shared arithmetic core for constant-propagation consumers
//! (dead-branch elimination, compound-assignment folding, and — later — a more
//! precise division-by-zero / unsigned-underflow proof). Unlike
//! [`super::const_value::ConstValue`], which only stores a value, this module
//! knows the *declared numeric type* of an operation and can therefore fold an
//! expression to the exact value the interpreter would produce — including
//! trapping on integer overflow and rejecting `NaN`.
//!
//! # Semantics (must match the interpreter in `src/interpreter.rs`)
//!
//! * **Integers** are folded in `i128`/`u128` to avoid intermediate overflow,
//!   then reduced to the declared width:
//!   * `+`, `-`, `*`, unary `-` trap on overflow/underflow when
//!     `checked_overflow` is true (the default), and wrap two's-complement
//!     otherwise.
//!   * `/` and `%` **always** wrap (`MIN / -1 == MIN`, `MIN % -1 == 0`) and only
//!     trap on a zero divisor. Integer `/` truncates toward zero; `%` uses the
//!     Euclidean remainder (non-negative result, matching `i64::rem_euclid`).
//! * **Floats** are folded in `f64` (the interpreter represents all float
//!   widths — including `f16` — as `f64`, so no per-width rounding is applied).
//!   Division/remainder by zero traps; a result of `NaN` traps per Lale's
//!   "no silent errors" policy (see §5.25 in `doc/ARCHITECTURE.md`).
//! * **Comparisons** fold to a `Bool` constant. A comparison involving `NaN`
//!   traps (it is not a clean boolean).
//!
//! # Three-valued result
//!
//! [`EvalResult`] distinguishes a folded [`EvalResult::Value`] from
//! [`EvalResult::Unknown`] (an operand was not a known constant) and
//! [`EvalResult::Trap`] (the operation would trap at runtime). Consumers decide
//! how to interpret each: for dead-branch elimination both `Unknown` and `Trap`
//! mean "emit both branches" (conservative); for a compile-time proof, `Trap`
//! can be reported as an error.

use super::const_value::ConstValue;
use super::type_conversion::NumericTypeInfo;
use crate::ast::BinaryOp;

/// The result of folding a typed numeric operation at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalResult {
  /// The operation produced a concrete constant.
  Value(ConstValue),
  /// Not enough information: an operand is not a known compile-time constant.
  Unknown,
  /// The operation traps at runtime (overflow, underflow, division by zero, or
  /// a floating-point `NaN` result).
  Trap,
}

impl EvalResult {
  /// True if this is a folded constant value.
  pub fn is_value(&self) -> bool {
    matches!(self, EvalResult::Value(_))
  }

  /// True if this could not be folded because an operand was unknown.
  pub fn is_unknown(&self) -> bool {
    matches!(self, EvalResult::Unknown)
  }

  /// True if this operation traps at runtime.
  pub fn is_trap(&self) -> bool {
    matches!(self, EvalResult::Trap)
  }

  /// Extract the folded value, if any.
  pub fn into_value(self) -> Option<ConstValue> {
    match self {
      EvalResult::Value(v) => Some(v),
      EvalResult::Unknown | EvalResult::Trap => None,
    }
  }
}

/// A binary arithmetic operator supported by the fold core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
  Add,
  Sub,
  Mul,
  Div,
  Rem,
}

/// A comparison operator supported by the fold core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
  Eq,
  Ne,
  Lt,
  Le,
  Gt,
  Ge,
}

/// The numeric fold requested by a binary operator, when it is supported by the
/// typed constant evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericFoldOp {
  Arith(ArithOp),
  Cmp(CmpOp),
}

/// Map a binary operator to a numeric fold, or `None` if the operator is not
/// foldable (logical/bitwise/shift/pow/vector operations, string append, …).
///
/// `Dot` maps to multiplication: a scalar dot product is multiplication, while a
/// vector dot product has vector operands (which are never tracked as constants,
/// so it folds to `Unknown`).
pub fn numeric_fold_op(op: BinaryOp) -> Option<NumericFoldOp> {
  match op {
    BinaryOp::Add => Some(NumericFoldOp::Arith(ArithOp::Add)),
    BinaryOp::Sub => Some(NumericFoldOp::Arith(ArithOp::Sub)),
    BinaryOp::Mul => Some(NumericFoldOp::Arith(ArithOp::Mul)),
    BinaryOp::Div => Some(NumericFoldOp::Arith(ArithOp::Div)),
    BinaryOp::Mod => Some(NumericFoldOp::Arith(ArithOp::Rem)),
    BinaryOp::Dot => Some(NumericFoldOp::Arith(ArithOp::Mul)),
    BinaryOp::Eq => Some(NumericFoldOp::Cmp(CmpOp::Eq)),
    BinaryOp::NotEq => Some(NumericFoldOp::Cmp(CmpOp::Ne)),
    BinaryOp::Lt => Some(NumericFoldOp::Cmp(CmpOp::Lt)),
    BinaryOp::LtEq => Some(NumericFoldOp::Cmp(CmpOp::Le)),
    BinaryOp::Gt => Some(NumericFoldOp::Cmp(CmpOp::Gt)),
    BinaryOp::GtEq => Some(NumericFoldOp::Cmp(CmpOp::Ge)),
    _ => None,
  }
}

/// Fold a binary arithmetic operation of declared type `info`.
///
/// `checked_overflow` mirrors the compiler's `--unchecked_overflow` flag: when
/// true (the default) integer `+`/`-`/`*` trap on overflow, otherwise they wrap.
/// It does not affect `/`/`%`, which always wrap.
pub fn fold_binary(
  op: ArithOp,
  info: &NumericTypeInfo,
  checked_overflow: bool,
  lhs: &ConstValue,
  rhs: &ConstValue,
) -> EvalResult {
  if info.is_float {
    fold_float_binary(op, lhs, rhs)
  } else if info.is_signed {
    fold_signed_binary(op, info, checked_overflow, lhs, rhs)
  } else {
    fold_unsigned_binary(op, info, checked_overflow, lhs, rhs)
  }
}

/// Fold a unary negation of declared type `info`.
///
/// Negation of an unsigned integer is not a valid Lale operation (the
/// interpreter rejects it), so it folds to [`EvalResult::Trap`].
pub fn fold_neg(info: &NumericTypeInfo, checked_overflow: bool, src: &ConstValue) -> EvalResult {
  if info.is_float {
    let f = to_f64(src);
    match f {
      None => EvalResult::Unknown,
      Some(f) => {
        let result = -f;
        if result.is_nan() {
          EvalResult::Trap
        } else {
          EvalResult::Value(ConstValue::float(result))
        }
      }
    }
  } else if info.is_signed {
    let i = match to_i128(src) {
      Some(i) => i,
      None => return EvalResult::Unknown,
    };
    let raw = i.wrapping_neg();
    if checked_overflow {
      let (min, max) = signed_range(info);
      if raw < min || raw > max {
        EvalResult::Trap
      } else {
        EvalResult::Value(ConstValue::Int(raw as i64))
      }
    } else {
      EvalResult::Value(ConstValue::Int(wrap_signed(raw, info.bits) as i64))
    }
  } else {
    // Unsigned negation is not a valid Lale operation.
    EvalResult::Trap
  }
}

/// Fold a comparison of declared type `info` to a `Bool` constant.
///
/// A comparison involving a `NaN` float operand traps rather than folding to a
/// (possibly misleading) boolean.
pub fn fold_cmp(
  op: CmpOp,
  info: &NumericTypeInfo,
  lhs: &ConstValue,
  rhs: &ConstValue,
) -> EvalResult {
  if info.is_float {
    let l = match to_f64(lhs) {
      Some(l) => l,
      None => return EvalResult::Unknown,
    };
    let r = match to_f64(rhs) {
      Some(r) => r,
      None => return EvalResult::Unknown,
    };
    if l.is_nan() || r.is_nan() {
      return EvalResult::Trap;
    }
    EvalResult::Value(ConstValue::Bool(cmp_f64(op, l, r)))
  } else if info.is_signed {
    let l = match to_i128(lhs) {
      Some(l) => l,
      None => return EvalResult::Unknown,
    };
    let r = match to_i128(rhs) {
      Some(r) => r,
      None => return EvalResult::Unknown,
    };
    EvalResult::Value(ConstValue::Bool(cmp_i128(op, l, r)))
  } else {
    let l = match to_u128(lhs) {
      Some(l) => l,
      None => return EvalResult::Unknown,
    };
    let r = match to_u128(rhs) {
      Some(r) => r,
      None => return EvalResult::Unknown,
    };
    EvalResult::Value(ConstValue::Bool(cmp_u128(op, l, r)))
  }
}

// ---------------------------------------------------------------------------
// Float folding (f64 — the interpreter stores every float width as f64).
// ---------------------------------------------------------------------------

fn fold_float_binary(op: ArithOp, lhs: &ConstValue, rhs: &ConstValue) -> EvalResult {
  let l = match to_f64(lhs) {
    Some(l) => l,
    None => return EvalResult::Unknown,
  };
  let r = match to_f64(rhs) {
    Some(r) => r,
    None => return EvalResult::Unknown,
  };

  // Matches the interpreter's ZeroCheck, which traps on an exactly-zero
  // (or negative-zero) divisor.
  if matches!(op, ArithOp::Div | ArithOp::Rem) && r == 0.0 {
    return EvalResult::Trap;
  }

  let result = match op {
    ArithOp::Add => l + r,
    ArithOp::Sub => l - r,
    ArithOp::Mul => l * r,
    ArithOp::Div => l / r,
    ArithOp::Rem => l % r,
  };

  if result.is_nan() {
    EvalResult::Trap
  } else {
    EvalResult::Value(ConstValue::float(result))
  }
}

// ---------------------------------------------------------------------------
// Signed integer folding.
// ---------------------------------------------------------------------------

fn fold_signed_binary(
  op: ArithOp,
  info: &NumericTypeInfo,
  checked_overflow: bool,
  lhs: &ConstValue,
  rhs: &ConstValue,
) -> EvalResult {
  let l = match to_i128(lhs) {
    Some(l) => l,
    None => return EvalResult::Unknown,
  };
  let r = match to_i128(rhs) {
    Some(r) => r,
    None => return EvalResult::Unknown,
  };

  // `/` and `%` trap only on a zero divisor; the result then wraps (no overflow
  // trap), matching the interpreter's plain `Div`/`Rem` instructions.
  let raw = match op {
    ArithOp::Add => l.wrapping_add(r),
    ArithOp::Sub => l.wrapping_sub(r),
    ArithOp::Mul => l.wrapping_mul(r),
    ArithOp::Div => {
      if r == 0 {
        return EvalResult::Trap;
      }
      l.wrapping_div(r)
    }
    ArithOp::Rem => {
      if r == 0 {
        return EvalResult::Trap;
      }
      // Euclidean remainder: non-negative for any divisor sign.
      l.rem_euclid(r)
    }
  };

  let always_wraps = matches!(op, ArithOp::Div | ArithOp::Rem);
  if checked_overflow && !always_wraps {
    let (min, max) = signed_range(info);
    if raw < min || raw > max {
      return EvalResult::Trap;
    }
    EvalResult::Value(ConstValue::Int(raw as i64))
  } else {
    EvalResult::Value(ConstValue::Int(wrap_signed(raw, info.bits) as i64))
  }
}

// ---------------------------------------------------------------------------
// Unsigned integer folding.
// ---------------------------------------------------------------------------

fn fold_unsigned_binary(
  op: ArithOp,
  info: &NumericTypeInfo,
  checked_overflow: bool,
  lhs: &ConstValue,
  rhs: &ConstValue,
) -> EvalResult {
  let l = match to_u128(lhs) {
    Some(l) => l,
    None => return EvalResult::Unknown,
  };
  let r = match to_u128(rhs) {
    Some(r) => r,
    None => return EvalResult::Unknown,
  };

  // `/` and `%` trap only on a zero divisor and cannot otherwise overflow.
  if matches!(op, ArithOp::Div | ArithOp::Rem) {
    if r == 0 {
      return EvalResult::Trap;
    }
    let result = match op {
      ArithOp::Div => l / r,
      ArithOp::Rem => l % r,
      _ => unreachable!("only Div/Rem reach this arm"),
    };
    return EvalResult::Value(ConstValue::Uint(result as u64));
  }

  let max = unsigned_max(info);
  if checked_overflow {
    let result = match op {
      ArithOp::Add => l.checked_add(r),
      ArithOp::Sub => l.checked_sub(r),
      ArithOp::Mul => l.checked_mul(r),
      ArithOp::Div | ArithOp::Rem => unreachable!("handled above"),
    };
    match result {
      None => EvalResult::Trap,
      Some(v) if v > max => EvalResult::Trap,
      Some(v) => EvalResult::Value(ConstValue::Uint(v as u64)),
    }
  } else {
    let result = match op {
      ArithOp::Add => l.wrapping_add(r),
      ArithOp::Sub => l.wrapping_sub(r),
      ArithOp::Mul => l.wrapping_mul(r),
      ArithOp::Div | ArithOp::Rem => unreachable!("handled above"),
    };
    EvalResult::Value(ConstValue::Uint(wrap_unsigned(result, info.bits) as u64))
  }
}

// ---------------------------------------------------------------------------
// Operand coercion — interpret a stored constant in the declared signedness.
//
// The `variables` table stores a literal's *source* tag (`Int` for an unsuffixed
// literal even when it was later inferred to an unsigned/float type), so these
// helpers coerce by declared type rather than requiring an exact tag match.
// ---------------------------------------------------------------------------

fn to_i128(v: &ConstValue) -> Option<i128> {
  match v {
    ConstValue::Int(i) => Some(*i as i128),
    ConstValue::Uint(u) if *u <= i64::MAX as u64 => Some(*u as i128),
    _ => None,
  }
}

fn to_u128(v: &ConstValue) -> Option<u128> {
  match v {
    ConstValue::Uint(u) => Some(*u as u128),
    ConstValue::Int(i) if *i >= 0 => Some(*i as u128),
    _ => None,
  }
}

fn to_f64(v: &ConstValue) -> Option<f64> {
  v.to_f64()
}

// ---------------------------------------------------------------------------
// Width reduction and range helpers.
// ---------------------------------------------------------------------------

/// The inclusive `(min, max)` range of a signed integer of `info.bits` width.
fn signed_range(info: &NumericTypeInfo) -> (i128, i128) {
  let min = -(1i128 << (info.bits - 1));
  let max = (1i128 << (info.bits - 1)) - 1;
  (min, max)
}

/// The inclusive maximum of an unsigned integer of `info.bits` width.
fn unsigned_max(info: &NumericTypeInfo) -> u128 {
  (1u128 << info.bits) - 1
}

/// Reduce an arbitrary-precision signed result to `bits`-bit two's complement.
fn wrap_signed(value: i128, bits: usize) -> i128 {
  let modulus = 1i128 << bits;
  let half = modulus >> 1;
  let mut v = value % modulus;
  if v >= half {
    v -= modulus;
  } else if v < -half {
    v += modulus;
  }
  v
}

/// Reduce an arbitrary-precision unsigned result to `bits` bits.
fn wrap_unsigned(value: u128, bits: usize) -> u128 {
  value & ((1u128 << bits) - 1)
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------

fn cmp_i128(op: CmpOp, l: i128, r: i128) -> bool {
  match op {
    CmpOp::Eq => l == r,
    CmpOp::Ne => l != r,
    CmpOp::Lt => l < r,
    CmpOp::Le => l <= r,
    CmpOp::Gt => l > r,
    CmpOp::Ge => l >= r,
  }
}

fn cmp_u128(op: CmpOp, l: u128, r: u128) -> bool {
  match op {
    CmpOp::Eq => l == r,
    CmpOp::Ne => l != r,
    CmpOp::Lt => l < r,
    CmpOp::Le => l <= r,
    CmpOp::Gt => l > r,
    CmpOp::Ge => l >= r,
  }
}

fn cmp_f64(op: CmpOp, l: f64, r: f64) -> bool {
  match op {
    CmpOp::Eq => l == r,
    CmpOp::Ne => l != r,
    CmpOp::Lt => l < r,
    CmpOp::Le => l <= r,
    CmpOp::Gt => l > r,
    CmpOp::Ge => l >= r,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn i8_info() -> NumericTypeInfo {
    NumericTypeInfo {
      bits: 8,
      is_signed: true,
      is_float: false,
    }
  }
  fn u8_info() -> NumericTypeInfo {
    NumericTypeInfo {
      bits: 8,
      is_signed: false,
      is_float: false,
    }
  }
  fn u64_info() -> NumericTypeInfo {
    NumericTypeInfo {
      bits: 64,
      is_signed: false,
      is_float: false,
    }
  }
  fn f64_info() -> NumericTypeInfo {
    NumericTypeInfo {
      bits: 64,
      is_signed: true,
      is_float: true,
    }
  }

  fn int(v: i64) -> ConstValue {
    ConstValue::Int(v)
  }
  fn uint(v: u64) -> ConstValue {
    ConstValue::Uint(v)
  }

  #[test]
  fn signed_add_checked_overflow_traps() {
    // 127 + 1 overflows i8 in checked mode.
    assert_eq!(
      fold_binary(ArithOp::Add, &i8_info(), true, &int(127), &int(1)),
      EvalResult::Trap
    );
    // In-range addition folds normally.
    assert_eq!(
      fold_binary(ArithOp::Add, &i8_info(), true, &int(100), &int(27)),
      EvalResult::Value(int(127))
    );
  }

  #[test]
  fn signed_add_unchecked_wraps() {
    assert_eq!(
      fold_binary(ArithOp::Add, &i8_info(), false, &int(127), &int(1)),
      EvalResult::Value(int(-128))
    );
    assert_eq!(
      fold_binary(ArithOp::Add, &i8_info(), false, &int(100), &int(100)),
      EvalResult::Value(int(-56))
    );
  }

  #[test]
  fn signed_sub_checked_underflow_traps() {
    assert_eq!(
      fold_binary(ArithOp::Sub, &i8_info(), true, &int(-128), &int(1)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(ArithOp::Sub, &i8_info(), true, &int(10), &int(5)),
      EvalResult::Value(int(5))
    );
  }

  #[test]
  fn signed_mul_checked_overflow_traps() {
    assert_eq!(
      fold_binary(ArithOp::Mul, &i8_info(), true, &int(64), &int(2)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(ArithOp::Mul, &i8_info(), true, &int(7), &int(9)),
      EvalResult::Value(int(63))
    );
  }

  #[test]
  fn signed_neg_checked_overflow_traps() {
    assert_eq!(fold_neg(&i8_info(), true, &int(-128)), EvalResult::Trap);
    assert_eq!(
      fold_neg(&i8_info(), true, &int(-5)),
      EvalResult::Value(int(5))
    );
  }

  #[test]
  fn signed_neg_unchecked_wraps() {
    assert_eq!(
      fold_neg(&i8_info(), false, &int(-128)),
      EvalResult::Value(int(-128))
    );
  }

  #[test]
  fn signed_div_wraps_min_div_neg_one() {
    // MIN / -1 wraps to MIN (always wrapping, even in checked mode).
    assert_eq!(
      fold_binary(ArithOp::Div, &i8_info(), true, &int(-128), &int(-1)),
      EvalResult::Value(int(-128))
    );
    // Truncation toward zero.
    assert_eq!(
      fold_binary(ArithOp::Div, &i8_info(), true, &int(-7), &int(2)),
      EvalResult::Value(int(-3))
    );
  }

  #[test]
  fn signed_rem_is_euclidean() {
    // Euclidean remainder is always non-negative, regardless of operand signs.
    assert_eq!(
      fold_binary(ArithOp::Rem, &i8_info(), true, &int(-5), &int(3)),
      EvalResult::Value(int(1))
    );
    assert_eq!(
      fold_binary(ArithOp::Rem, &i8_info(), true, &int(5), &int(-3)),
      EvalResult::Value(int(2))
    );
    assert_eq!(
      fold_binary(ArithOp::Rem, &i8_info(), true, &int(5), &int(3)),
      EvalResult::Value(int(2))
    );
    assert_eq!(
      fold_binary(ArithOp::Rem, &i8_info(), true, &int(-5), &int(-3)),
      EvalResult::Value(int(1))
    );
    // MIN % -1 == 0 (no overflow).
    assert_eq!(
      fold_binary(ArithOp::Rem, &i8_info(), true, &int(-128), &int(-1)),
      EvalResult::Value(int(0))
    );
  }

  #[test]
  fn division_by_zero_traps_for_ints() {
    assert_eq!(
      fold_binary(ArithOp::Div, &i8_info(), true, &int(5), &int(0)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(ArithOp::Rem, &u8_info(), true, &uint(5), &uint(0)),
      EvalResult::Trap
    );
  }

  #[test]
  fn unsigned_add_checked_overflow_traps() {
    assert_eq!(
      fold_binary(ArithOp::Add, &u8_info(), true, &uint(255), &uint(1)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(ArithOp::Add, &u8_info(), true, &uint(200), &uint(55)),
      EvalResult::Value(uint(255))
    );
  }

  #[test]
  fn unsigned_add_unchecked_wraps() {
    assert_eq!(
      fold_binary(ArithOp::Add, &u8_info(), false, &uint(255), &uint(1)),
      EvalResult::Value(uint(0))
    );
    assert_eq!(
      fold_binary(ArithOp::Add, &u8_info(), false, &uint(200), &uint(100)),
      EvalResult::Value(uint(44))
    );
  }

  #[test]
  fn unsigned_sub_underflow_traps_when_checked() {
    assert_eq!(
      fold_binary(ArithOp::Sub, &u8_info(), true, &uint(5), &uint(10)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(ArithOp::Sub, &u8_info(), true, &uint(10), &uint(5)),
      EvalResult::Value(uint(5))
    );
  }

  #[test]
  fn unsigned_sub_unchecked_wraps() {
    assert_eq!(
      fold_binary(ArithOp::Sub, &u8_info(), false, &uint(5), &uint(10)),
      EvalResult::Value(uint(251))
    );
  }

  #[test]
  fn unsigned_mul_checked_overflow_traps() {
    assert_eq!(
      fold_binary(ArithOp::Mul, &u8_info(), true, &uint(16), &uint(16)),
      EvalResult::Trap
    );
    // u64 * u64 that overflows u128 is definitely out of range.
    assert_eq!(
      fold_binary(
        ArithOp::Mul,
        &u64_info(),
        true,
        &uint(u64::MAX),
        &uint(u64::MAX)
      ),
      EvalResult::Trap
    );
  }

  #[test]
  fn unsigned_mul_unchecked_wraps_to_width() {
    // (2^64 - 1)^2 has low 64 bits == 1.
    assert_eq!(
      fold_binary(
        ArithOp::Mul,
        &u64_info(),
        false,
        &uint(u64::MAX),
        &uint(u64::MAX)
      ),
      EvalResult::Value(uint(1))
    );
  }

  #[test]
  fn unsigned_div_and_rem() {
    assert_eq!(
      fold_binary(ArithOp::Div, &u8_info(), true, &uint(200), &uint(3)),
      EvalResult::Value(uint(66))
    );
    assert_eq!(
      fold_binary(ArithOp::Rem, &u8_info(), true, &uint(200), &uint(3)),
      EvalResult::Value(uint(2))
    );
  }

  #[test]
  fn unsigned_neg_is_invalid() {
    assert_eq!(fold_neg(&u8_info(), true, &uint(5)), EvalResult::Trap);
  }

  #[test]
  fn float_arithmetic_folds() {
    assert_eq!(
      fold_binary(
        ArithOp::Add,
        &f64_info(),
        true,
        &ConstValue::float(1.5),
        &ConstValue::float(2.25)
      ),
      EvalResult::Value(ConstValue::float(3.75))
    );
    assert_eq!(
      fold_binary(
        ArithOp::Div,
        &f64_info(),
        true,
        &ConstValue::float(10.0),
        &ConstValue::float(4.0)
      ),
      EvalResult::Value(ConstValue::float(2.5))
    );
  }

  #[test]
  fn float_division_by_zero_traps() {
    assert_eq!(
      fold_binary(
        ArithOp::Div,
        &f64_info(),
        true,
        &ConstValue::float(1.0),
        &ConstValue::float(0.0)
      ),
      EvalResult::Trap
    );
    assert_eq!(
      fold_binary(
        ArithOp::Div,
        &f64_info(),
        true,
        &ConstValue::float(1.0),
        &ConstValue::float(-0.0)
      ),
      EvalResult::Trap
    );
  }

  #[test]
  fn float_nan_result_traps() {
    let nan = ConstValue::float(f64::NAN);
    assert_eq!(
      fold_binary(
        ArithOp::Add,
        &f64_info(),
        true,
        &nan,
        &ConstValue::float(1.0)
      ),
      EvalResult::Trap
    );
    assert_eq!(fold_neg(&f64_info(), true, &nan), EvalResult::Trap);
  }

  #[test]
  fn int_comparisons_fold() {
    assert_eq!(
      fold_cmp(CmpOp::Lt, &i8_info(), &int(-2), &int(3)),
      EvalResult::Value(ConstValue::Bool(true))
    );
    assert_eq!(
      fold_cmp(CmpOp::Ge, &u8_info(), &uint(200), &uint(100)),
      EvalResult::Value(ConstValue::Bool(true))
    );
    assert_eq!(
      fold_cmp(CmpOp::Eq, &u8_info(), &uint(1), &uint(2)),
      EvalResult::Value(ConstValue::Bool(false))
    );
  }

  #[test]
  fn float_comparison_folds() {
    assert_eq!(
      fold_cmp(
        CmpOp::Lt,
        &f64_info(),
        &ConstValue::float(1.0),
        &ConstValue::float(2.0)
      ),
      EvalResult::Value(ConstValue::Bool(true))
    );
  }

  #[test]
  fn float_nan_comparison_traps() {
    let nan = ConstValue::float(f64::NAN);
    assert_eq!(
      fold_cmp(CmpOp::Lt, &f64_info(), &nan, &ConstValue::float(1.0)),
      EvalResult::Trap
    );
    assert_eq!(
      fold_cmp(CmpOp::Eq, &f64_info(), &ConstValue::float(1.0), &nan),
      EvalResult::Trap
    );
  }

  #[test]
  fn non_numeric_operand_is_unknown() {
    assert_eq!(
      fold_binary(
        ArithOp::Add,
        &i8_info(),
        true,
        &ConstValue::Text("x".into()),
        &int(1)
      ),
      EvalResult::Unknown
    );
  }

  #[test]
  fn coerces_literal_tags_by_declared_type() {
    // An unsuffixed `5` stored as Int(5) in a float context folds as 5.0 + 0.5.
    assert_eq!(
      fold_binary(
        ArithOp::Add,
        &f64_info(),
        true,
        &int(5),
        &ConstValue::float(0.5)
      ),
      EvalResult::Value(ConstValue::float(5.5))
    );
    // Int(5) in an unsigned context folds as 5.
    assert_eq!(
      fold_binary(ArithOp::Add, &u8_info(), true, &int(5), &uint(1)),
      EvalResult::Value(uint(6))
    );
  }

  #[test]
  fn wrap_signed_matches_twos_complement() {
    assert_eq!(wrap_signed(128, 8), -128);
    assert_eq!(wrap_signed(200, 8), -56);
    assert_eq!(wrap_signed(-200, 8), 56);
    assert_eq!(wrap_signed(1i128 << 63, 64), i64::MIN as i128);
  }

  #[test]
  fn wrap_unsigned_matches_modulo() {
    assert_eq!(wrap_unsigned(256, 8), 0);
    assert_eq!(wrap_unsigned(300, 8), 44);
    assert_eq!(wrap_unsigned(u128::MAX, 64), u64::MAX as u128);
  }
}
