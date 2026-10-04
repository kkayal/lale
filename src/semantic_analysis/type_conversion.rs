//! Type validation and conversion checking for semantic analysis.
//!
//! This module handles type conversion validation, type categorization, and numeric
//! range checking for type compatibility. It provides a structured approach to
//! validating type conversions during semantic analysis.
//!
//! # Type Categories
//!
//! Types are categorized for conversion validation:
//! - **Numeric**: Integer and floating-point types with bit-width and signedness info
//! - **Bool**: Boolean type
//! - **String**: String type
//! - **Char**: Character type
//! - **Pointer**: Pointer types
//! - **Array**: Array types
//! - **Other**: Custom or unknown types
//!
//! # Conversion Rules
//!
//! - **Widening conversions**: Allowed (i32→i64, f32→f64, i32→f64)
//! - **Narrowing conversions**: Rejected (f64→f32, i64→i32)
//! - **Same type**: Allowed (no-op)
//! - **Incompatible categories**: Rejected (str↔numeric, bool↔numeric, array↔scalar)
//! - **Pointer conversions**: Only unsigned integers to pointers, or pointer to pointer
//! - **Numeric literals**: Can convert to any numeric type if value fits in range

use super::error_types::SemanticError;
use crate::ast::SourceLocation;

/// Information about a numeric type for conversion checking.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericTypeInfo {
  pub bits: usize,
  pub is_signed: bool,
  pub is_float: bool,
}

/// Category of a type for conversion validation.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeCategory {
  Numeric(NumericTypeInfo),
  Bool,
  String,
  Char,
  Pointer,
  Array,
  Other,
}

/// Type validator for conversion and compatibility checking.
///
/// Provides methods for validating type conversions, categorizing types,
/// and checking numeric ranges for literal value conversions.
pub struct TypeValidator;

impl TypeValidator {
  /// Validate that a type conversion is allowed.
  ///
  /// Rules:
  /// - Widening numeric conversions: allowed (i32→i64, f32→f64, i32→f64)
  /// - Narrowing numeric conversions: rejected (f64→f32, i64→i32)
  /// - Same type: allowed (no-op)
  /// - Incompatible types: rejected (str↔numeric, bool↔numeric, array↔scalar)
  pub fn validate_type_conversion(
    source_type: &str,
    target_type: &str,
    location: &SourceLocation,
    errors: &mut Vec<SemanticError>,
  ) {
    // Same type is always allowed (no-op)
    if source_type.to_lowercase() == target_type.to_lowercase() {
      return;
    }

    // Get type categories
    let source_cat = Self::get_type_category(source_type);
    let target_cat = Self::get_type_category(target_type);

    // Check for incompatible categories
    match (&source_cat, &target_cat) {
      (TypeCategory::String, TypeCategory::Numeric(_))
      | (TypeCategory::Numeric(_), TypeCategory::String) => {
        errors.push(SemanticError::new(
          format!(
            "Invalid type conversion: cannot convert '{}' to '{}'. String and numeric types are incompatible.",
            source_type, target_type
          ),
          location.clone(),
        ));
      }
      (TypeCategory::String, TypeCategory::Pointer) => {
        // Allow str to pointer conversion (extracts the pointer from the fat string)
        // This is allowed because Lale strings are fat pointers (ptr, len)
      }
      (TypeCategory::Bool, TypeCategory::Numeric(_))
      | (TypeCategory::Numeric(_), TypeCategory::Bool) => {
        errors.push(SemanticError::new(
          format!(
            "Invalid type conversion: cannot convert '{}' to '{}'. Bool and numeric types are incompatible.",
            source_type, target_type
          ),
          location.clone(),
        ));
      }
      (TypeCategory::Array, _) | (_, TypeCategory::Array) => {
        if source_cat != target_cat {
          errors.push(SemanticError::new(
            format!(
              "Invalid type conversion: cannot convert '{}' to '{}'. Array conversions are not supported.",
              source_type, target_type
            ),
            location.clone(),
          ));
        }
      }
      (TypeCategory::Pointer, TypeCategory::Pointer) => {
        // Pointer to pointer is allowed (identity)
      }
      (TypeCategory::Numeric(info), TypeCategory::Pointer) if !info.is_signed => {
        // Unsigned integer to pointer is allowed (for null pointers and FFI)
      }
      (TypeCategory::Pointer, _) | (_, TypeCategory::Pointer) => {
        errors.push(SemanticError::new(
          format!(
            "Invalid type conversion: cannot convert '{}' to '{}'. Only unsigned integers can be converted to pointers.",
            source_type, target_type
          ),
          location.clone(),
        ));
      }
      (TypeCategory::Numeric(source_info), TypeCategory::Numeric(target_info)) => {
        // Check for narrowing conversions
        if !Self::is_widening_conversion(source_info, target_info) {
          let is_signed_unsigned_ints = source_info.is_signed != target_info.is_signed
            && !source_info.is_float
            && !target_info.is_float;
          let msg = if is_signed_unsigned_ints {
            format!(
              "Lossy conversion not allowed: cannot convert '{}' to '{}'. Signed↔unsigned \
               conversion can change the value (e.g. a negative value becomes large). \
               Use `unsafe bitcast` for a bit reinterpretation, or widen first.",
              source_type, target_type
            )
          } else {
            format!(
              "Narrowing conversion not allowed: cannot convert '{}' ({}-bit) to '{}' ({}-bit). \
               Consider widening the narrow data type (e.g., '{}') instead of trying to narrow the wide data type (e.g., '{}').",
              source_type,
              source_info.bits,
              target_type,
              target_info.bits,
              target_type,
              source_type
            )
          };
          errors.push(SemanticError::new(msg, location.clone()));
        }
      }
      (TypeCategory::Numeric(_), TypeCategory::Char) => {
        // Semantic conversion: Numeric → Char
        // This is allowed because it's a semantic category change, not data loss.
        // A numeric value in the valid Unicode range (0x00000000 to 0x10FFFF) can be
        // reinterpreted as a Unicode character without losing information.
        // The conversion is type-safe because we check Unicode validity.
        // Note: Runtime/compile-time validation of the Unicode range is deferred to IR generation
      }
      (TypeCategory::Other, TypeCategory::Char) => {
        // Allow unknown types to convert to char (e.g., during IR generation when types haven't been resolved yet)
      }
      (TypeCategory::Numeric(_), TypeCategory::Other) => {
        // Allow numeric to unknown type conversion (type inference will resolve it)
      }
      (TypeCategory::Other, TypeCategory::Numeric(_)) => {
        // Allow unknown to numeric conversion (type inference will resolve it)
      }
      (TypeCategory::Other, TypeCategory::Other) => {
        // Allow unknown to unknown (will be resolved later)
      }
      _ => {
        // Other category mismatches
        if source_cat != target_cat {
          errors.push(SemanticError::new(
            format!(
              "Invalid type conversion: cannot convert '{}' to '{}'.",
              source_type, target_type
            ),
            location.clone(),
          ));
        }
      }
    }
  }

  /// Categorize a type for conversion checking.
  pub fn get_type_category(type_name: &str) -> TypeCategory {
    match type_name.to_lowercase().as_str() {
      "i8" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 8,
        is_signed: true,
        is_float: false,
      }),
      "i16" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 16,
        is_signed: true,
        is_float: false,
      }),
      "i32" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 32,
        is_signed: true,
        is_float: false,
      }),
      "i64" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 64,
        is_signed: true,
        is_float: false,
      }),
      "u8" | "byte" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 8,
        is_signed: false,
        is_float: false,
      }),
      "u16" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 16,
        is_signed: false,
        is_float: false,
      }),
      "u32" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 32,
        is_signed: false,
        is_float: false,
      }),
      "u64" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 64,
        is_signed: false,
        is_float: false,
      }),
      "f16" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 16,
        is_signed: true,
        is_float: true,
      }),
      "f32" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 32,
        is_signed: true,
        is_float: true,
      }),
      "f64" => TypeCategory::Numeric(NumericTypeInfo {
        bits: 64,
        is_signed: true,
        is_float: true,
      }),
      "bool" => TypeCategory::Bool,
      "text" => TypeCategory::String,
      "char" => TypeCategory::Char,
      "pointer" => TypeCategory::Pointer,
      s if s.starts_with('[') || s.contains('[') => TypeCategory::Array,
      _ => TypeCategory::Other,
    }
  }

  /// Check if a numeric conversion is widening (allowed) or narrowing (rejected).
  ///
  /// Widening rules:
  /// - Same or larger bit width for same category (int→int, float→float)
  /// - Integer to float is always allowed (even if lossy for large values)
  /// - Float to integer is always narrowing (loses fractional part)
  /// - Signed ↔ unsigned is only value-preserving when the target is strictly
  ///   wider than an unsigned source; signed → unsigned is never value-preserving
  ///   (negative values do not fit).
  pub fn is_widening_conversion(source: &NumericTypeInfo, target: &NumericTypeInfo) -> bool {
    // Float to integer is always narrowing (loses fractional part)
    if source.is_float && !target.is_float {
      return false;
    }

    // Integer to float is always allowed (semantically widening)
    // Note: large integers may lose precision in smaller floats, but this is
    // commonly allowed in languages and useful for numeric computation
    if !source.is_float && target.is_float {
      return true;
    }

    // Float to float: widening if target has at least as many bits
    if source.is_float && target.is_float {
      return target.bits >= source.bits;
    }

    // Integer to integer: value-preserving iff every source value fits in target.
    match (source.is_signed, target.is_signed) {
      // signed → signed: target must be at least as wide
      (true, true) => target.bits >= source.bits,
      // unsigned → unsigned: target must be at least as wide
      (false, false) => target.bits >= source.bits,
      // signed → unsigned: negative values never fit
      (true, false) => false,
      // unsigned → signed: target must be strictly wider
      (false, true) => target.bits > source.bits,
    }
  }

  /// Get the storage bit width of a type, for `unsafe bitcast` bit-width checking.
  /// Returns `None` for composite or unknown types.
  pub fn get_bit_width(type_name: &str) -> Option<usize> {
    match type_name.to_lowercase().as_str() {
      "i8" | "u8" | "byte" => Some(8),
      "i16" | "u16" => Some(16),
      "i32" | "u32" | "f32" | "char" => Some(32),
      "i64" | "u64" | "f64" | "pointer" => Some(64),
      "f16" => Some(16),
      "bool" => Some(1),
      _ => None, // Composite types, strings, arrays, etc.
    }
  }

  /// Get the integer value range for a numeric (non-float) type.
  /// Returns (min, max) as i128, or None for floats.
  pub fn get_numeric_int_range(info: &NumericTypeInfo) -> Option<(i128, i128)> {
    if info.is_float {
      return None;
    }

    let bits = info.bits as u32;
    if info.is_signed {
      // Signed: [-2^(bits-1), 2^(bits-1)-1]
      let min = -(1i128 << (bits - 1));
      let max = (1i128 << (bits - 1)) - 1;
      Some((min, max))
    } else {
      // Unsigned: [0, 2^bits - 1]
      let min = 0i128;
      let max = (1i128 << bits) - 1;
      Some((min, max))
    }
  }
}
