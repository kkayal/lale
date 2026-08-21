//! Type System and Type Checking
//!
//! This module handles type inference, type compatibility checking,
//! and type conversion for expressions.

use crate::ast::*;

/// Type inference for expressions
pub struct TypeInference;

impl TypeInference {
  // Convert a BaseType to its string representation.
  pub fn base_type_to_string(base_type: &BaseType) -> String {
    match base_type {
      BaseType::U8 => "u8".to_string(),
      BaseType::I8 => "i8".to_string(),
      BaseType::U16 => "u16".to_string(),
      BaseType::I16 => "i16".to_string(),
      BaseType::U32 => "u32".to_string(),
      BaseType::I32 => "i32".to_string(),
      BaseType::U64 => "u64".to_string(),
      BaseType::I64 => "i64".to_string(),
      BaseType::F16 => "f16".to_string(),
      BaseType::F32 => "f32".to_string(),
      BaseType::F64 => "f64".to_string(),
      BaseType::Str => "str".to_string(),
      BaseType::Bool => "bool".to_string(),
      BaseType::Byte => "byte".to_string(),
      BaseType::Char => "char".to_string(),
      BaseType::Pointer => "pointer".to_string(),
      BaseType::Vec2 => "vec2".to_string(),
      BaseType::Vec3 => "vec3".to_string(),
      BaseType::Vec4 => "vec4".to_string(),
      BaseType::Custom(name) => name.clone(),
    }
  }

  /// Convert a TypeName to its string representation.
  /// Produces format: base_type (no dims) or base_type[dim1][dim2]... (with dims)
  pub fn type_name_to_string(type_name: &TypeName) -> String {
    let _base = match &type_name.base_type {
      BaseType::U8 => "u8".to_string(),
      BaseType::I8 => "i8".to_string(),
      BaseType::U16 => "u16".to_string(),
      BaseType::I16 => "i16".to_string(),
      BaseType::U32 => "u32".to_string(),
      BaseType::I32 => "i32".to_string(),
      BaseType::U64 => "u64".to_string(),
      BaseType::I64 => "i64".to_string(),
      BaseType::F16 => "f16".to_string(),
      BaseType::F32 => "f32".to_string(),
      BaseType::F64 => "f64".to_string(),
      BaseType::Str => "str".to_string(),
      BaseType::Bool => "bool".to_string(),
      BaseType::Byte => "byte".to_string(),
      BaseType::Char => "char".to_string(),
      BaseType::Pointer => "pointer".to_string(),
      BaseType::Vec2 => Self::format_vec("vec2", type_name),
      BaseType::Vec3 => Self::format_vec("vec3", type_name),
      BaseType::Vec4 => Self::format_vec("vec4", type_name),
      BaseType::Custom(name) => name.clone(),
    };

    Self::type_name_to_string_with_dimensions(type_name)
  }

  /// Format a vector type string including its inner type.
  fn format_vec(dim: &str, type_name: &TypeName) -> String {
    match &type_name.inner_type {
      Some(inner) => format!("{}<{}>", dim, Self::base_type_to_string(inner)),
      None => dim.to_string(),
    }
  }

  /// Convert a TypeName to its string representation with dimension information.
  /// For example: `i32[5]` or `f64[3][3]` instead of `[i32]`.
  /// This is used for storing array dimension metadata for compile-time bounds checking.
  pub fn type_name_to_string_with_dimensions(type_name: &TypeName) -> String {
    let base = match &type_name.base_type {
      BaseType::U8 => "u8".to_string(),
      BaseType::I8 => "i8".to_string(),
      BaseType::U16 => "u16".to_string(),
      BaseType::I16 => "i16".to_string(),
      BaseType::U32 => "u32".to_string(),
      BaseType::I32 => "i32".to_string(),
      BaseType::U64 => "u64".to_string(),
      BaseType::I64 => "i64".to_string(),
      BaseType::F16 => "f16".to_string(),
      BaseType::F32 => "f32".to_string(),
      BaseType::F64 => "f64".to_string(),
      BaseType::Str => "str".to_string(),
      BaseType::Bool => "bool".to_string(),
      BaseType::Byte => "byte".to_string(),
      BaseType::Char => "char".to_string(),
      BaseType::Pointer => "pointer".to_string(),
      BaseType::Vec2 => Self::format_vec("vec2", type_name),
      BaseType::Vec3 => Self::format_vec("vec3", type_name),
      BaseType::Vec4 => Self::format_vec("vec4", type_name),
      BaseType::Custom(name) => name.clone(),
    };

    // Extract dimension sizes if present and append as [N][M]... format
    let mut result = base;
    for dim_expr in &type_name.array_dimensions {
      // Try to extract literal integer from dimension expression
      if let Expr::IntLiteral(int_lit) = dim_expr {
        result.push('[');
        result.push_str(&int_lit.value.to_string());
        result.push(']');
      } else if let Expr::UintLiteral(uint_lit) = dim_expr {
        result.push('[');
        result.push_str(&uint_lit.value.to_string());
        result.push(']');
      } else {
        // Non-literal dimension: just append empty brackets as placeholder
        result.push_str("[]");
      }
    }
    if type_name.is_optional {
      result.push('?');
    }
    result
  }

  /// Check if a type string represents an optional type (ends with `?`).
  pub fn is_optional_type(type_str: &str) -> bool {
    type_str.ends_with('?')
  }

  /// Get the inner type of an optional type by stripping the trailing `?`.
  /// Returns the original string if the type is not optional.
  pub fn optional_inner_type(type_str: &str) -> &str {
    type_str.strip_suffix('?').unwrap_or(type_str)
  }

  /// Check if two types are compatible (for assignment without explicit conversion).
  /// Types must match exactly; implicit conversions are not allowed.
  /// Exception: numeric literals are flexible and infer their type from context.
  /// Exception: optional types accept their inner concrete type (T → T? coercion).
  /// Comparison is case-insensitive to handle type name variations.
  pub fn types_compatible(lhs_type: &str, rhs_type: &str) -> bool {
    // Exact match always compatible (case-insensitive)
    if lhs_type.to_lowercase() == rhs_type.to_lowercase() {
      return true;
    }
    // Optional type coercion: T? and T are compatible
    if Self::is_optional_type(lhs_type) {
      let lhs_inner = Self::optional_inner_type(lhs_type);
      if lhs_inner.to_lowercase() == rhs_type.to_lowercase() {
        return true;
      }
    }
    if Self::is_optional_type(rhs_type) {
      let rhs_inner = Self::optional_inner_type(rhs_type);
      if lhs_type.to_lowercase() == rhs_inner.to_lowercase() {
        return true;
      }
    }
    // Empty array type compatible with any array type (for array literal inference)
    if (lhs_type.starts_with('[') && rhs_type == "[]")
      || (lhs_type == "[]" && rhs_type.starts_with('['))
    {
      return true;
    }

    // Handle array type format: type[dim1][dim2]... vs type[dim1][dim2]...
    if lhs_type.contains('[') && rhs_type.contains('[') {
      // Extract base types and dimensions
      if let (Some(lhs_base), Some(rhs_base)) = (
        Self::extract_array_base(lhs_type),
        Self::extract_array_base(rhs_type),
      ) {
        // Base types must be compatible (either exact match or both numeric)
        let base_compatible = lhs_base == rhs_base
          || (Self::is_numeric_base_type(&lhs_base) && Self::is_numeric_base_type(&rhs_base));

        if !base_compatible {
          return false;
        }

        // For dimensions: must be identical
        // No partial initialization allowed - arrays must be completely initialized
        let lhs_dims = Self::extract_array_dimensions(lhs_type);
        let rhs_dims = Self::extract_array_dimensions(rhs_type);

        // Dimensions must match exactly
        return lhs_dims == rhs_dims;
      }
    }
    false
  }

  /// Extract the base type from an array type string (e.g., "i32" from "i32[5][3]")
  fn extract_array_base(type_str: &str) -> Option<String> {
    type_str
      .find('[')
      .map(|bracket_pos| type_str[..bracket_pos].to_string())
  }

  /// Extract the dimension part from an array type string (e.g., "[5][3]" from "i32[5][3]")
  pub fn extract_array_dimensions(type_str: &str) -> String {
    if let Some(bracket_pos) = type_str.find('[') {
      type_str[bracket_pos..].to_string()
    } else {
      String::new()
    }
  }

  /// Check if a type is a numeric base type (not including arrays)
  fn is_numeric_base_type(base_type: &str) -> bool {
    matches!(
      base_type.to_lowercase().as_str(),
      "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
    )
  }

  /// Check if a type string is a numeric type (case-insensitive, supports array notation).
  pub fn is_numeric_type(type_str: &str) -> bool {
    let normalized = type_str.to_lowercase();
    // Remove array brackets if present
    let base = normalized.trim_matches(|c: char| c == '[' || c == ']');
    matches!(
      base,
      "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
    )
  }
}

/// Type checking operations for expressions
pub struct TypeChecker;

impl TypeChecker {
  /// Check if expression is a numeric value (literal or arithmetic expression) that can infer its type from context.
  /// This includes:
  /// - Direct numeric literals
  /// - Grouped/parenthesized numeric expressions
  /// - Binary arithmetic operations (which produce numeric results)
  /// - Unary operations on numeric values (except logical NOT)
  /// - Introspection operations that return numeric types (#size of)
  pub fn is_numeric_valued(expr: &Expr) -> bool {
    match expr {
      // Direct numeric literals
      Expr::IntLiteral(_) | Expr::UintLiteral(_) | Expr::FloatLiteral(_) | Expr::HexLiteral(_) => {
        true
      }
      Expr::Grouped(inner) => Self::is_numeric_valued(inner),
      // Arithmetic expressions
      Expr::Binary(bin) => matches!(
        bin.operator,
        BinaryOp::Add
          | BinaryOp::Sub
          | BinaryOp::Mul
          | BinaryOp::Div
          | BinaryOp::Mod
          | BinaryOp::Pow
      ),
      // Unary operations
      Expr::Unary(un) => match un.operator {
        UnaryOp::Not => false,
        UnaryOp::TypeOf | UnaryOp::UnitOf => false, // String results
        UnaryOp::SizeOf => true,                    // Returns numeric u32
        UnaryOp::Neg | UnaryOp::Invert => Self::is_numeric_valued(&un.operand),
        _ => false,
      },
      _ => false,
    }
  }

  /// Get human-readable operator name for error messages.
  pub fn operator_name(op: &BinaryOp) -> &'static str {
    match op {
      BinaryOp::Add => "addition",
      BinaryOp::Sub => "subtraction",
      BinaryOp::Mul => "multiplication",
      BinaryOp::Div => "division",
      BinaryOp::Mod => "modulo",
      BinaryOp::Pow => "power",
      BinaryOp::Eq => "equality",
      BinaryOp::NotEq => "inequality",
      BinaryOp::Lt => "less-than",
      BinaryOp::LtEq => "less-than-or-equal",
      BinaryOp::Gt => "greater-than",
      BinaryOp::GtEq => "greater-than-or-equal",
      BinaryOp::And => "logical AND",
      BinaryOp::Or => "logical OR",
      BinaryOp::Xor => "logical XOR",
      BinaryOp::BitAnd => "bitwise AND",
      BinaryOp::BitOr => "bitwise OR",
      BinaryOp::BitXor => "bitwise XOR",
      BinaryOp::UnsignedLeftShift => "unsigned left shift",
      BinaryOp::UnsignedRightShift => "unsigned right shift",
      BinaryOp::SignedLeftShift => "signed left shift",
      BinaryOp::SignedRightShift => "signed right shift",
      BinaryOp::Append => "append",
      BinaryOp::Dot => "dot product",
      BinaryOp::Cross => "cross product",
    }
  }
}
