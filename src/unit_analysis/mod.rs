//! Physical Unit Analysis and Validation
//!
//! This module handles computation and validation of physical units (dimensions) in Lale programs.
//! It provides utilities for unit propagation, checking, and runtime unit assertions.
//!
//! # Purpose
//!
//! Lale is a language designed for physical simulations and engineering calculations.
//! It enforces strict dimensional analysis at compile time to catch unit errors early.
//!
//! Examples:
//! - `x as i32 in <m> + y as i32 in <s>` → **Error**: cannot add meters to seconds
//! - `x as i32 in <m> + y as i32 in <m>` → **OK**: same units
//! - `distance as f64 in <m> / time as f64 in <s>` → **OK**: yields <m/s>
//!
//! # Key Types
//!
//! - [`UnitAnalyzer`] - Static unit analysis functions
//! - Uses `ExprUnit` from AST for unit representation
//!
//! # Module Organization
//!
//! This module is focused and minimal:
//! - Extracted from `semantic_analysis/unit_analysis.rs` to reduce coupling
//! - Self-contained; only depends on AST types
//! - Provides reusable utility functions for unit checking
//!
//! # Integration
//!
//! Called by `semantic_analysis::analyzer` during expression type checking:
//!
//! ```text
//! use crate::unit_analysis::UnitAnalyzer;
//!
//! // Check if binary operation operands are unit-compatible
//! if !UnitAnalyzer::check_binary_operand_types_ok(left_type, right_type, &op) {
//!     error("Unit mismatch");
//! }
//! ```
//!
//! # Comparison Operations and Physical Units
//!
//! For comparison operations (`<`, `>`, `<=`, `>=`, `==`, `!=`), operands must have the same unit:
//! - `distance1 < distance2` where both in `<m>` → **OK**
//! - `distance < time` where distance in `<m>` and time in `<s>` → **Error**
//!
//! For arithmetic operations:
//! - **Addition/Subtraction**: Operands must have identical units
//! - **Multiplication**: Units multiply together `<m> * <s> = <m⋅s>`
//! - **Division**: Units divide `<m> / <s> = <m/s>`
//! - **Power**: Exponent must be unitless; result unit is `unit ^ exponent`
//!
//! # Special Case: Power Operations
//!
//! Power operations have special type rules matching LLVM semantics:
//! - **Base**: Any numeric type (i32, f64, etc.)
//! - **Exponent**: Any integer type OR the same floating-point type as base
//! - Example: `5.0 ^ 2` (f64 ^ i32) → **OK**
//! - Example: `5.0 ^ 2.5` (f64 ^ f64) → **OK**
//! - Example: `5 ^ 2.5` (i32 ^ f64) → **Error** (base is not float)

use crate::ast::*;
use std::collections::HashMap;

/// Physical unit analysis and validation.
///
/// Provides utility functions for working with physical units in Lale expressions.
/// All methods are static (stateless) to facilitate reuse across compilation stages.
pub struct UnitAnalyzer;

impl UnitAnalyzer {
  /// Extract an integer value from a compile-time constant expression.
  ///
  /// Used primarily for power operation exponents, which must be known at compile time.
  ///
  /// # Arguments
  ///
  /// * `expr` - Expression that should evaluate to an integer
  ///
  /// # Returns
  ///
  /// - `Some(n)` - If expression is a constant integer or evaluates to one
  /// - `None` - If expression contains variables or non-integer constants
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lale::unit_analysis::UnitAnalyzer;
  /// # use lale::ast::{Expr, IntLiteral, IdentifierExpr, UnaryOp, UnaryExpr, SourceLocation};
  /// let loc = SourceLocation::dummy();
  /// let expr = Expr::IntLiteral(IntLiteral { value: 5, unit: None, location: loc.clone() });
  /// assert_eq!(UnitAnalyzer::try_extract_int_value(&expr), Some(5));
  ///
  /// let expr = Expr::Unary(UnaryExpr {
  ///     operator: UnaryOp::Neg,
  ///     operand: Box::new(Expr::IntLiteral(IntLiteral { value: 5, unit: None, location: loc.clone() })),
  ///     location: loc.clone(),
  /// });
  /// assert_eq!(UnitAnalyzer::try_extract_int_value(&expr), Some(-5));
  ///
  /// let expr = Expr::Identifier(IdentifierExpr { path: vec!["x".into()], location: loc });
  /// assert_eq!(UnitAnalyzer::try_extract_int_value(&expr), None);
  /// ```
  pub fn try_extract_int_value(expr: &Expr) -> Option<i64> {
    match expr {
      Expr::IntLiteral(lit) => Some(lit.value),
      Expr::UintLiteral(lit) => Some(lit.value as i64),
      Expr::FloatLiteral(lit) => {
        if lit.value.fract() == 0.0 {
          Some(lit.value as i64)
        } else {
          None
        }
      }
      Expr::Grouped(inner) => Self::try_extract_int_value(inner),
      Expr::Unary(un) => {
        if matches!(un.operator, UnaryOp::Neg) {
          Self::try_extract_int_value(&un.operand).map(|v| -v)
        } else {
          None
        }
      }
      Expr::HexLiteral(hex) => i64::from_str_radix(
        hex.value.trim_start_matches("0x").trim_start_matches("0X"),
        16,
      )
      .ok(),
      _ => None,
    }
  }

  /// Find the first return expression in a function body.
  ///
  /// Used to infer function return types from the first explicit return statement.
  /// This is a heuristic; proper type checking validates all returns have compatible types.
  ///
  /// # Arguments
  ///
  /// * `body` - Function body statements
  ///
  /// # Returns
  ///
  /// First return expression found, or `None` if no explicit return.
  ///
  /// # Examples
  ///
  /// ```text
  /// let body = vec![
  ///     Stmt::If(...),
  ///     Stmt::Return(Some(Expr::IntLiteral(5))),
  ///     Stmt::Return(Some(Expr::IntLiteral(10))),  // Not found; uses first
  /// ];
  /// assert_eq!(
  ///     UnitAnalyzer::find_return_expr(&body),
  ///     Some(Expr::IntLiteral(5))
  /// );
  /// ```
  pub fn find_return_expr(body: &[Stmt]) -> Option<Expr> {
    for stmt in body {
      if let Stmt::Return(ret) = stmt
        && let Some(value) = &ret.value
      {
        return Some(value.clone());
      }
    }
    None
  }

  /// Collect physical units declared on local variables in a function body.
  ///
  /// Scans function body for variable definitions with explicit unit annotations.
  /// Used for type inference when variables are referenced within the function.
  ///
  /// # Arguments
  ///
  /// * `body` - Function body statements
  ///
  /// # Returns
  ///
  /// Map from variable name to declared unit (or `Unitless` if no unit annotation).
  ///
  /// # Examples
  ///
  /// ```text
  /// let body = vec![
  ///     Stmt::VarDef(VarDefStmt {
  ///         name: "distance".to_string(),
  ///         unit: Some(ExprUnit::from_str("<m>")),
  ///         ...
  ///     }),
  ///     Stmt::VarDef(VarDefStmt {
  ///         name: "count".to_string(),
  ///         unit: None,  // No unit annotation
  ///         ...
  ///     }),
  /// ];
  ///
  /// let units = UnitAnalyzer::collect_local_var_units(&body);
  /// assert_eq!(units.get("distance"), Some(&ExprUnit::from_str("<m>")));
  /// ```
  pub fn collect_local_var_units(body: &[Stmt]) -> HashMap<String, ExprUnit> {
    let mut local_units = HashMap::new();
    for stmt in body {
      if let Stmt::VarDef(def) = stmt {
        let unit = ExprUnit::from_option(&def.unit);
        local_units.insert(def.name.node.clone(), unit);
      }
    }
    local_units
  }

  /// Validate that operand types are compatible for a binary operation.
  ///
  /// Lale enforces strict type checking: implicit type conversions are never allowed.
  /// Both operands must have the exact same type (case-insensitive comparison).
  ///
  /// # Special Case: Power Operations
  ///
  /// Power operations (`^`) have special type rules matching LLVM semantics:
  /// - **Base** can be any numeric type (i8, i16, i32, i64, u8, u16, u32, u64, f16, f32, f64)
  /// - **Exponent** can be:
  ///   - Any integer type (i8, i16, i32, i64, u8, u16, u32, u64), OR
  ///   - The exact same floating-point type as the base (f16 ^ f16, f32 ^ f32, f64 ^ f64)
  ///
  /// # Arguments
  ///
  /// * `left_type` - Type name of left operand
  /// * `right_type` - Type name of right operand
  /// * `operator` - The binary operator
  ///
  /// # Returns
  ///
  /// - `true` - If operands are type-compatible for the operation
  /// - `false` - If there is a type mismatch
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lale::unit_analysis::UnitAnalyzer;
  /// # use lale::ast::BinaryOp;
  ///
  /// // Regular operations: types must match
  /// assert!(UnitAnalyzer::check_binary_operand_types_ok("i32", "i32", &BinaryOp::Add));
  /// assert!(!UnitAnalyzer::check_binary_operand_types_ok("i32", "f64", &BinaryOp::Add));
  ///
  /// // Power operations: allow int exponent with float base
  /// assert!(UnitAnalyzer::check_binary_operand_types_ok("f64", "i32", &BinaryOp::Pow));
  /// assert!(UnitAnalyzer::check_binary_operand_types_ok("f64", "f64", &BinaryOp::Pow));
  /// assert!(!UnitAnalyzer::check_binary_operand_types_ok("i32", "f64", &BinaryOp::Pow));
  /// ```
  pub fn check_binary_operand_types_ok(
    left_type: &str,
    right_type: &str,
    operator: &BinaryOp,
  ) -> bool {
    // Normalize types for comparison (case-insensitive)
    let left_normalized = left_type.to_lowercase();
    let right_normalized = right_type.to_lowercase();

    // Helper: Check if type is a pointer
    let is_pointer = |t: &str| t.to_lowercase() == "pointer";

    // Helper: Check if type is an unsigned integer
    let is_uint = |t: &str| matches!(t.to_lowercase().as_str(), "u8" | "u16" | "u32" | "u64");

    // Pointer arithmetic (special case before general type matching)
    // ptr + u64 → pointer
    // ptr - u64 → pointer
    // ptr - ptr → i64
    if is_pointer(&left_normalized) {
      match operator {
        BinaryOp::Add => {
          // Allow: pointer + u64
          if is_uint(&right_normalized) {
            return true;
          }
          // Reject: pointer + pointer (nonsensical)
          if is_pointer(&right_normalized) {
            return false;
          }
          // Reject: pointer + other types
          return left_type == "unknown" || right_type == "unknown";
        }
        BinaryOp::Sub => {
          // Allow: pointer - u64 (returns pointer)
          if is_uint(&right_normalized) {
            return true;
          }
          // Allow: pointer - pointer (returns i64)
          if is_pointer(&right_normalized) {
            return true;
          }
          // Reject: pointer - other types
          return left_type == "unknown" || right_type == "unknown";
        }
        BinaryOp::Mul
        | BinaryOp::Div
        | BinaryOp::Mod
        | BinaryOp::Pow
        | BinaryOp::Dot
        | BinaryOp::Cross
        | BinaryOp::Append
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::UnsignedLeftShift
        | BinaryOp::UnsignedRightShift
        | BinaryOp::SignedLeftShift
        | BinaryOp::SignedRightShift
        | BinaryOp::And
        | BinaryOp::Or
        | BinaryOp::Xor => {
          // Reject: all other operations with pointers
          return left_type == "unknown" || right_type == "unknown";
        }
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::LtEq
        | BinaryOp::Gt
        | BinaryOp::GtEq => {
          // Allow pointer comparisons (pointers can be compared)
          if is_pointer(&right_normalized) {
            return true;
          }
          // Reject: comparing pointer with non-pointer types
          return left_type == "unknown" || right_type == "unknown";
        }
      }
    }

    // Reject: u64 + pointer (not commutative)
    if is_uint(&left_normalized) && is_pointer(&right_normalized) && *operator == BinaryOp::Add {
      return false;
    }

    // Exception: Shift operations require matching integer types on both sides
    // (Check this BEFORE the general type matching rule below)
    if matches!(
      operator,
      BinaryOp::UnsignedLeftShift
        | BinaryOp::UnsignedRightShift
        | BinaryOp::SignedLeftShift
        | BinaryOp::SignedRightShift
    ) {
      let left_is_int = matches!(
        left_normalized.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
      );
      let right_is_int = matches!(
        right_normalized.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
      );

      if left_is_int && right_is_int && left_normalized == right_normalized {
        return true; // Both are same integer type, shifts are allowed
      }
      return false; // At least one is not an integer, or types don't match
    }

    // Types must match exactly for all binary operations.
    // No implicit conversions, no default type mixing.
    // Types must match exactly for all binary operations.
    // No implicit conversions, no default type mixing.

    // --- Vector operation validation ---
    let is_vector =
      |t: &str| t.starts_with("vec2<") || t.starts_with("vec3<") || t.starts_with("vec4<");
    // Parse vector type string like "vec3<f64>" into ("vec3", "f64")
    fn parse_vec_type(t: &str) -> Option<(&str, &str)> {
      if let Some(inner_start) = t.find('<')
        && t.ends_with('>')
      {
        let dim = &t[..inner_start];
        let inner = &t[inner_start + 1..t.len() - 1];
        return Some((dim, inner));
      }
      None
    }

    match operator {
      BinaryOp::Mul => {
        // Reject: vec * vec (no plain multiplication for vectors)
        if is_vector(&left_normalized) && is_vector(&right_normalized) {
          return false;
        }
      }
      BinaryOp::Dot => {
        let lv = parse_vec_type(&left_normalized);
        let rv = parse_vec_type(&right_normalized);
        match (lv, rv) {
          // Both vectors: same dimension and inner type required
          (Some((ld, li)), Some((rd, ri))) => {
            return ld == rd && li == ri;
          }
          // Both scalars: same type required
          (None, None) => {
            return left_normalized == right_normalized
              || left_type == "unknown"
              || right_type == "unknown";
          }
          // Mixed scalar/vector: reject (use * for scalar-vector)
          _ => return false,
        }
      }
      BinaryOp::Cross => {
        // Cross product: both must be vec3<T> with same inner T
        let lv = parse_vec_type(&left_normalized);
        let rv = parse_vec_type(&right_normalized);
        match (lv, rv) {
          (Some((ld, li)), Some((rd, ri))) => {
            return ld == "vec3" && rd == "vec3" && li == ri;
          }
          _ => return false,
        }
      }
      _ => {}
    }

    if left_normalized == right_normalized || left_type == "unknown" || right_type == "unknown" {
      return true; // Types match exactly or cannot be determined
    }

    // Exception: Power operations allow integer exponents with floating-point base
    // (matches LLVM llvm.powi semantics: float ^ int is allowed)
    if *operator == BinaryOp::Pow {
      let left_is_float = matches!(left_normalized.as_str(), "f16" | "f32" | "f64");
      let right_is_int = matches!(
        right_normalized.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
      );

      if left_is_float && right_is_int {
        return true; // float ^ int is allowed
      }
    }

    // Exception: Append (concatenation) requires matching types for strings/arrays
    // or allows string + any numeric type (implicit string conversion)
    if *operator == BinaryOp::Append {
      let left_is_str = left_normalized.contains("str");
      let right_is_str = right_normalized.contains("str");

      // String + string always works
      if left_is_str && right_is_str {
        return true;
      }

      // String + non-string (implicit conversion) or non-string + string
      if left_is_str || right_is_str {
        return true; // Allow concatenation with type coercion
      }

      // Array + array (same element type)
      let left_is_array = left_normalized.starts_with('[');
      let right_is_array = right_normalized.starts_with('[');
      if left_is_array && right_is_array {
        return left_normalized == right_normalized;
      }
    }

    // Exception: Comparison operators allow mixed int/uint types
    // (with automatic coercion during interpretation/compilation)
    if matches!(
      operator,
      BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::LtEq
        | BinaryOp::Gt
        | BinaryOp::GtEq
    ) {
      let left_is_int = matches!(
        left_normalized.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
      );
      let right_is_int = matches!(
        right_normalized.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
      );

      // Both are integer types (may be different signedness/sizes)
      if left_is_int && right_is_int {
        return true;
      }
    }

    false // Type mismatch
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_type_check_matching_types() {
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "i32",
      &BinaryOp::Add
    ));
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "f64",
      &BinaryOp::Add
    ));
  }

  #[test]
  fn test_type_check_mismatched_types() {
    // i32 + f64 should be invalid (no automatic type conversion)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "f64",
      &BinaryOp::Add
    ));
    // u32 + i32 should be invalid (strict type matching, no mixing)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "u32",
      "i32",
      &BinaryOp::Add
    ));
    // String * integer is invalid
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "str",
      "i32",
      &BinaryOp::Mul
    ));
  }

  #[test]
  fn test_power_allows_int_exponent_with_float_base() {
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "i32",
      &BinaryOp::Pow
    ));
  }

  #[test]
  fn test_power_allows_matching_float_types() {
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "f64",
      &BinaryOp::Pow
    ));
  }

  #[test]
  fn test_power_rejects_int_base_with_float_exponent() {
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "f64",
      &BinaryOp::Pow
    ));
  }

  #[test]
  fn test_shift_operators_require_integers() {
    // Shift operators require both operands to be of the SAME integer type
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "i32",
      &BinaryOp::SignedLeftShift
    ));
    // Different integer types are not allowed for shift operations
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "u64",
      "u8",
      &BinaryOp::UnsignedRightShift
    ));
    // Same type is allowed
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "i64",
      "i64",
      &BinaryOp::SignedRightShift
    ));
  }

  #[test]
  fn test_shift_operators_reject_float_operands() {
    // Shift operators reject float operands
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "i32",
      &BinaryOp::SignedLeftShift
    ));
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "f64",
      &BinaryOp::UnsignedLeftShift
    ));
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "f32",
      "f32",
      &BinaryOp::SignedRightShift
    ));
  }

  #[test]
  fn test_append_string_operations() {
    // String + string
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "str",
      "str",
      &BinaryOp::Append
    ));
    // String + numeric (implicit conversion)
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "str",
      "i32",
      &BinaryOp::Append
    ));
    // Numeric + string (implicit conversion)
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "str",
      &BinaryOp::Append
    ));
  }

  #[test]
  fn test_append_array_operations() {
    // Array + matching array
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "[i32]",
      "[i32]",
      &BinaryOp::Append
    ));
    // Array + different array should fail (without full type info)
    // In practice, this check is limited without semantic analyzer
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "[i32]",
      "[f64]",
      &BinaryOp::Append
    ));
  }

  // ==================== VECTOR DOT PRODUCT TESTS ====================

  #[test]
  fn test_vector_dot_product_same_dimension_and_type() {
    // vec2<f64> · vec2<f64> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec2<f64>",
      &BinaryOp::Dot
    ));
    // vec3<f64> · vec3<f64> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec3<f64>",
      &BinaryOp::Dot
    ));
    // vec4<f64> · vec4<f64> → OK (dot product works for any dimension)
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec4<f64>",
      "vec4<f64>",
      &BinaryOp::Dot
    ));
    // vec2<i32> · vec2<i32> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<i32>",
      "vec2<i32>",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_dot_product_different_dimensions() {
    // vec2<f64> · vec3<f64> → FAIL (different dimensions)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec3<f64>",
      &BinaryOp::Dot
    ));
    // vec3<f64> · vec2<f64> → FAIL (different dimensions)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec2<f64>",
      &BinaryOp::Dot
    ));
    // vec3<f64> · vec4<f64> → FAIL (different dimensions)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec4<f64>",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_dot_product_different_inner_types() {
    // vec3<f64> · vec3<i32> → FAIL (different inner types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec3<i32>",
      &BinaryOp::Dot
    ));
    // vec2<f64> · vec2<f32> → FAIL (different inner types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec2<f32>",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_dot_product_scalar() {
    // f64 · f64 → OK (scalar dot product acts like multiplication)
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "f64",
      &BinaryOp::Dot
    ));
    // i32 · i32 → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "i32",
      "i32",
      &BinaryOp::Dot
    ));
    // f64 · i32 → FAIL (different types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "i32",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_dot_product_mixed_vector_scalar() {
    // vec2<f64> · f64 → FAIL (vector with scalar not allowed in dot)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "f64",
      &BinaryOp::Dot
    ));
    // f64 · vec2<f64> → FAIL (scalar with vector not allowed in dot)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "vec2<f64>",
      &BinaryOp::Dot
    ));
  }

  // ==================== VECTOR CROSS PRODUCT TESTS ====================

  #[test]
  fn test_vector_cross_product_vec3_only() {
    // vec3<f64> × vec3<f64> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec3<f64>",
      &BinaryOp::Cross
    ));
    // vec3<i32> × vec3<i32> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<i32>",
      "vec3<i32>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_cross_product_vec2_rejected() {
    // vec2<f64> × vec2<f64> → FAIL (only vec3)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec2<f64>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_cross_product_vec4_rejected() {
    // vec4<f64> × vec4<f64> → FAIL (only vec3)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec4<f64>",
      "vec4<f64>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_cross_product_different_inner_types_rejected() {
    // vec3<f64> × vec3<i32> → FAIL (different inner types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec3<i32>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_cross_product_scalar_rejected() {
    // f64 × f64 → FAIL (cross product requires vectors)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "f64",
      "f64",
      &BinaryOp::Cross
    ));
  }

  // ==================== VECTOR MUL REJECTION TESTS ====================

  #[test]
  fn test_vector_mul_vec2_rejected() {
    // vec2<f64> * vec2<f64> → FAIL (no plain mul for vectors)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec2<f64>",
      &BinaryOp::Mul
    ));
  }

  #[test]
  fn test_vector_mul_vec3_rejected() {
    // vec3<f64> * vec3<f64> → FAIL (no plain mul for vectors)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<f64>",
      "vec3<f64>",
      &BinaryOp::Mul
    ));
  }

  #[test]
  fn test_vector_mul_vec4_rejected() {
    // vec4<f64> * vec4<f64> → FAIL (no plain mul for vectors)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec4<f64>",
      "vec4<f64>",
      &BinaryOp::Mul
    ));
  }

  #[test]
  fn test_vector_mul_mixed_dimensions_rejected() {
    // vec2<f64> * vec3<f64> → FAIL (no plain mul for vectors)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<f64>",
      "vec3<f64>",
      &BinaryOp::Mul
    ));
  }

  #[test]
  fn test_vector_dot_vec3_i32() {
    // vec3<i32> · vec3<i32> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<i32>",
      "vec3<i32>",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_dot_scalar_u64() {
    // u64 · u64 with Dot → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "u64",
      "u64",
      &BinaryOp::Dot
    ));
  }

  #[test]
  fn test_vector_cross_vec3_u64() {
    // vec3<u64> × vec3<u64> → OK
    assert!(UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<u64>",
      "vec3<u64>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_cross_vec3_mixed_inner_rejected() {
    // vec3<i32> × vec3<u64> → FAIL (different inner types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec3<i32>",
      "vec3<u64>",
      &BinaryOp::Cross
    ));
  }

  #[test]
  fn test_vector_dot_vec2_mixed_inner_rejected() {
    // vec2<i32> · vec2<u64> → FAIL (different inner types)
    assert!(!UnitAnalyzer::check_binary_operand_types_ok(
      "vec2<i32>",
      "vec2<u64>",
      &BinaryOp::Dot
    ));
  }
}
