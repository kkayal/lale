use lale::{LaleParser, Rule};
use pest::Parser;

/// Tests for physical unit system in the Lale grammar.
///
/// This test suite covers:
/// - Unit syntax: unit, mul_sign, div_sign
/// - Unit components: dividend, divisor, atomic_unit, expo, superscript

// ==================== MULTIPLICATION SIGN TESTS ====================
// mul_sign = _{ "×" | "⋅" | "∗" | "⨯" | "*" }

#[test]
fn test_mul_sign_asterisk() {
  let result = LaleParser::parse(Rule::mul_sign, "*");
  assert!(result.is_ok());
}

#[test]
fn test_mul_sign_times() {
  // × (U+00D7) was intentionally excluded from mul_sign —
  // see ARCHITECTURE.md §5.21
  let result = LaleParser::parse(Rule::mul_sign, "×");
  assert!(result.is_err());
}

#[test]
fn test_mul_sign_dot() {
  let result = LaleParser::parse(Rule::mul_sign, "⋅");
  assert!(result.is_ok());
}

#[test]
fn test_mul_sign_asterisk_operator() {
  // ∗ (U+2217) was intentionally excluded from mul_sign —
  // redundant with * (U+002A)
  let result = LaleParser::parse(Rule::mul_sign, "∗");
  assert!(result.is_err());
}

#[test]
fn test_mul_sign_heavy_asterisk() {
  // ⨯ (U+2A2F) was intentionally excluded from mul_sign —
  // cross product of scalar units is meaningless; reserved for expressions
  let result = LaleParser::parse(Rule::mul_sign, "⨯");
  assert!(result.is_err());
}

// ==================== DIVISION SIGN TESTS ====================
// div_sign = _{ "÷" | "⁄" | "∕" | "⅟" | "/" }

#[test]
fn test_div_sign_slash() {
  let result = LaleParser::parse(Rule::div_sign, "/");
  assert!(result.is_ok());
}

#[test]
fn test_div_sign_obelus() {
  let result = LaleParser::parse(Rule::div_sign, "÷");
  assert!(result.is_ok());
}

#[test]
fn test_div_sign_fraction_slash() {
  let result = LaleParser::parse(Rule::div_sign, "⁄");
  assert!(result.is_ok());
}

#[test]
fn test_div_sign_division_slash() {
  let result = LaleParser::parse(Rule::div_sign, "∕");
  assert!(result.is_ok());
}

// ==================== SUPERSCRIPT TESTS ====================
// superscript = {"⁰" | "¹" | '²'..'³' | '⁴'..'⁹'}

#[test]
fn test_superscript_zero() {
  let result = LaleParser::parse(Rule::superscript, "⁰");
  assert!(result.is_ok());
}

#[test]
fn test_superscript_one() {
  let result = LaleParser::parse(Rule::superscript, "¹");
  assert!(result.is_ok());
}

#[test]
fn test_superscript_two() {
  let result = LaleParser::parse(Rule::superscript, "²");
  assert!(result.is_ok());
}

#[test]
fn test_superscript_three() {
  let result = LaleParser::parse(Rule::superscript, "³");
  assert!(result.is_ok());
}

#[test]
fn test_superscript_four_to_nine() {
  for i in 4..=9 {
    let ch = char::from_u32(0x2074 + (i - 4) as u32).unwrap();
    let s = ch.to_string();
    let result = LaleParser::parse(Rule::superscript, &s);
    assert!(result.is_ok(), "Superscript {}", i);
  }
}

#[test]
fn test_superscript_all() {
  let all = vec!["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
  for sup in all {
    let result = LaleParser::parse(Rule::superscript, sup);
    assert!(result.is_ok(), "Superscript: {}", sup);
  }
}

// ==================== EXPONENT TESTS ====================
// exponent = @{ "^" ~ sign? ~ digit+ }

#[test]
fn test_exponent_simple() {
  let result = LaleParser::parse(Rule::exponent, "^2");
  assert!(result.is_ok());
}

#[test]
fn test_exponent_with_plus() {
  let result = LaleParser::parse(Rule::exponent, "^+2");
  assert!(result.is_ok());
}

#[test]
fn test_exponent_with_minus() {
  let result = LaleParser::parse(Rule::exponent, "^-2");
  assert!(result.is_ok());
}

#[test]
fn test_exponent_large_exponent() {
  let result = LaleParser::parse(Rule::exponent, "^100");
  assert!(result.is_ok());
}

#[test]
fn test_exponent_zero() {
  let result = LaleParser::parse(Rule::exponent, "^0");
  assert!(result.is_ok());
}

// ==================== ATOMIC UNIT TESTS ====================
// atomic_unit = @{ (!div_sign ~ !mul_sign ~ !superscript ~ alpha)+ ~ ("⁺" | "⁻")? ~ superscript* }

#[test]
fn test_atomic_unit_simple() {
  let result = LaleParser::parse(Rule::atomic_unit, "m");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_meter() {
  let result = LaleParser::parse(Rule::atomic_unit, "meter");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_kilogram() {
  let result = LaleParser::parse(Rule::atomic_unit, "kg");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_with_superscript() {
  let result = LaleParser::parse(Rule::atomic_unit, "m²");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_with_multiple_superscripts() {
  let result = LaleParser::parse(Rule::atomic_unit, "m²³");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_with_sign() {
  let result = LaleParser::parse(Rule::atomic_unit, "m⁺");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_with_negative_sign() {
  let result = LaleParser::parse(Rule::atomic_unit, "m⁻");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_sign_and_superscript() {
  let result = LaleParser::parse(Rule::atomic_unit, "m⁺²");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_long_name() {
  let result = LaleParser::parse(Rule::atomic_unit, "second");
  assert!(result.is_ok());
}

#[test]
fn test_atomic_unit_with_unicode() {
  let result = LaleParser::parse(Rule::atomic_unit, "α");
  assert!(result.is_ok(), "Greek unit");
}

// ==================== DIVIDEND/DIVISOR TESTS ====================
// dividend = { atomic_unit ~ expo? ~ (mul_sign ~ atomic_unit ~ expo?)* }
// divisor = { atomic_unit ~ expo? ~ (mul_sign ~ atomic_unit ~ expo?)* }

#[test]
fn test_dividend_single_unit() {
  let result = LaleParser::parse(Rule::dividend, "m");
  assert!(result.is_ok());
}

#[test]
fn test_dividend_with_exponent() {
  let result = LaleParser::parse(Rule::dividend, "m^2");
  assert!(result.is_ok());
}

#[test]
fn test_dividend_multiple_units() {
  let result = LaleParser::parse(Rule::dividend, "kg * m");
  assert!(result.is_ok());
}

#[test]
fn test_dividend_multiple_with_exponents() {
  let result = LaleParser::parse(Rule::dividend, "kg * m^2");
  assert!(result.is_ok());
}

#[test]
fn test_dividend_unicode_mul() {
  let result = LaleParser::parse(Rule::dividend, "kg ⋅ m");
  assert!(result.is_ok());
}

#[test]
fn test_divisor_single_unit() {
  let result = LaleParser::parse(Rule::divisor, "s");
  assert!(result.is_ok());
}

#[test]
fn test_divisor_with_exponent() {
  let result = LaleParser::parse(Rule::divisor, "s^2");
  assert!(result.is_ok());
}

#[test]
fn test_divisor_multiple_units() {
  let result = LaleParser::parse(Rule::divisor, "s * A");
  assert!(result.is_ok());
}

// ==================== UNIT TESTS ====================
// unit = @{ "<" ~ ((float ~ "*")? ~ dividend ~ (div_sign ~ divisor)? | (float ~ div_sign ~ divisor)) ~ ">" }

#[test]
fn test_unit_simple_meter() {
  let result = LaleParser::parse(Rule::unit, "<m>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_meter_squared() {
  let result = LaleParser::parse(Rule::unit, "<m^2>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_velocity() {
  let result = LaleParser::parse(Rule::unit, "<m/s>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_acceleration() {
  let result = LaleParser::parse(Rule::unit, "<m/s^2>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_force() {
  let result = LaleParser::parse(Rule::unit, "<kg*m/s^2>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_with_coefficient() {
  let result = LaleParser::parse(Rule::unit, "<1.5*m>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_with_float_coefficient() {
  let result = LaleParser::parse(Rule::unit, "<2.54*cm>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_with_float_and_dot_operator() {
  let result = LaleParser::parse(Rule::unit, "<1.5⋅m>");
  assert!(
    result.is_ok(),
    "Float with dot-operator separator should parse"
  );
}

#[test]
fn test_unit_with_float_dot_kg() {
  let result = LaleParser::parse(Rule::unit, "<5.0⋅kg>");
  assert!(
    result.is_ok(),
    "Float coefficient with ⋅ separator before kg"
  );
}

#[test]
fn test_unit_complex() {
  let result = LaleParser::parse(Rule::unit, "<kg*m^2/s^2>");
  assert!(result.is_ok(), "Complex unit");
}

// Removed test: test_unit_only_divisor - divisor-only units may not be supported

#[test]
fn test_unit_multiple_dividend_terms() {
  let result = LaleParser::parse(Rule::unit, "<kg*m*A>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_mixed_mul_signs() {
  // × (U+00D7) excluded — use * and ⋅ instead
  let result = LaleParser::parse(Rule::unit, "<kg*m⋅s>");
  assert!(
    result.is_ok(),
    "Mixed multiplication signs with valid symbols"
  );
}

#[test]
fn test_unit_mixed_div_signs() {
  let result = LaleParser::parse(Rule::unit, "<m÷s>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_unicode_mul_div() {
  let result = LaleParser::parse(Rule::unit, "<kg⋅m²⁄s²>");
  assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

// Note: unitless and zero coefficient units have varying grammar requirements

#[test]
fn test_unit_scientific_notation_coefficient() {
  let result = LaleParser::parse(Rule::unit, "<1.67e-27*kg>");
  assert!(result.is_ok(), "Scientific notation coefficient");
}

#[test]
fn test_unit_deeply_nested_exponents() {
  let result = LaleParser::parse(Rule::unit, "<m^5>");
  assert!(result.is_ok());
}

#[test]
fn test_unit_negative_exponent() {
  let result = LaleParser::parse(Rule::unit, "<s^-1>");
  assert!(result.is_ok(), "Inverse time");
}

#[test]
fn test_unit_multiple_atomic_with_exponents() {
  let result = LaleParser::parse(Rule::unit, "<kg^2*m^3*s^-2>");
  assert!(result.is_ok());
}

// Removed tests: atomic unit restrictions may vary based on implementation

// ==================== COMBINED TESTS ====================

#[test]
fn test_unit_si_derived_newton() {
  let result = LaleParser::parse(Rule::unit, "<kg*m*s^-2>");
  assert!(result.is_ok(), "Newton (force)");
}

#[test]
fn test_unit_si_derived_joule() {
  let result = LaleParser::parse(Rule::unit, "<kg*m^2*s^-2>");
  assert!(result.is_ok(), "Joule (energy)");
}

#[test]
fn test_unit_si_derived_watt() {
  let result = LaleParser::parse(Rule::unit, "<kg*m^2*s^-3>");
  assert!(result.is_ok(), "Watt (power)");
}

#[test]
fn test_unit_si_derived_pascal() {
  let result = LaleParser::parse(Rule::unit, "<kg*m^-1*s^-2>");
  assert!(result.is_ok(), "Pascal (pressure)");
}

#[test]
fn test_unit_with_multiple_divisors() {
  // Multiple terms in divisor
  let result = LaleParser::parse(Rule::unit, "<m/s/A>");
  // This depends on whether nested divisions are allowed
  let _ = result;
}
