//! Memory safety validation for pointer and cast operations.
//!
//! This module provides checks for unsafe pointer operations to prevent
//! memory safety issues, particularly:
//! - Escaping pointers to local variables in return statements
//! - Assigning pointers to locals to global variables
//! - Unsafe cast compatibility with pointer bit widths and units

use super::error_types::SemanticError;
use super::expression_analysis::ExpressionAnalyzer;
use super::sqlite_symbol_management::{SqliteSymbolManager, VarScope};
use crate::ast::{Expr, SourceLocation, UnaryOp};

/// Validates memory safety of pointer and cast operations.
pub struct MemorySafetyChecker<'a> {
  pub symbols: &'a SqliteSymbolManager,
  pub errors: &'a mut Vec<SemanticError>,
  pub warnings: &'a mut Vec<SemanticError>,
}

impl<'a> MemorySafetyChecker<'a> {
  /// Creates a new MemorySafetyChecker.
  pub fn new(
    symbols: &'a SqliteSymbolManager,
    errors: &'a mut Vec<SemanticError>,
    warnings: &'a mut Vec<SemanticError>,
  ) -> Self {
    MemorySafetyChecker {
      symbols,
      errors,
      warnings,
    }
  }

  /// Check if returning an expression would escape a pointer to a local variable.
  pub fn check_escaping_pointer_in_return(&mut self, expr: &Expr, location: &SourceLocation) {
    let is_local = |name: &str| self.symbols.is_local_variable(name);
    if let Some(var_name) = ExpressionAnalyzer::find_pointer_to_local(expr, &is_local) {
      self.add_error(
        format!(
          "Cannot return pointer to local variable '{}': reference would escape function scope",
          var_name
        ),
        location,
      );
    }
  }

  /// Check if assigning to a global variable with a pointer to local.
  pub fn check_escaping_pointer_to_global(
    &mut self,
    expr: &Expr,
    target_name: &str,
    location: &SourceLocation,
  ) {
    if *self.symbols.current_scope() == VarScope::Global {
      return;
    }
    if let Some(global_table) = self
      .symbols
      .get_all_symbol_tables()
      .unwrap_or_default()
      .get(&VarScope::Global)
      && global_table.contains_key(target_name)
    {
      let is_local = |name: &str| self.symbols.is_local_variable(name);
      if let Some(var_name) = ExpressionAnalyzer::find_pointer_to_local(expr, &is_local) {
        self.add_error(
            format!(
              "Cannot assign pointer to local variable '{}' to global '{}': reference would escape function scope",
              var_name, target_name
            ),
            location,
          );
      }
    }
  }

  /// Validate `value at ptr unsafe cast` for bit-width compatibility and unitless source.
  ///
  /// Due to Pratt parser precedence, the AST structure is:
  ///   ValueAt -> UnsafeCast -> Identifier
  /// (i.e., `value at` is outer, `unsafe cast` is inner, applied to the pointer identifier)
  ///
  /// Checks that:
  /// 1. The pointer source has no physical unit (bit reinterpretation is meaningless for dimensional values)
  /// 2. The pointer source type (if known) has the same bit width as the target type
  pub fn validate_unsafe_cast_compatibility(
    &mut self,
    target_type: &str,
    value_at_expr: &Expr,
    location: &SourceLocation,
  ) {
    // AST structure: ValueAt(UnsafeCast(Identifier))
    // We receive the ValueAt expression
    if let Expr::Unary(value_at) = value_at_expr
      && matches!(value_at.operator, UnaryOp::ValueAt)
      && let Expr::Unary(unsafe_cast) = &*value_at.operand
      && matches!(unsafe_cast.operator, UnaryOp::UnsafeCast)
    {
      // The operand of `unsafe cast` is the pointer variable
      if let Expr::Identifier(ptr_ident) = &*unsafe_cast.operand {
        // Look up the pointer's source type in the symbol table
        if let Some(ptr_symbol) = self
          .symbols
          .lookup_var_symbol(ptr_ident.name())
          .unwrap_or(None)
        {
          // pointer_to_type contains the NAME of the variable the pointer points to
          if let Some(source_var_name) = &ptr_symbol.pointer_to_type {
            // Look up the original variable to get its type and unit
            if let Some(source_symbol) = self
              .symbols
              .lookup_var_symbol(source_var_name)
              .unwrap_or(None)
            {
              // Check 1: Source must be unitless
              if let Some(unit) = &source_symbol.physical_unit {
                self.add_error(
                  format!(
                    "Unsafe cast requires unitless source: variable '{}' has unit '{}'. \
                            Bit reinterpretation is meaningless for dimensional values. \
                            Assign to a unitless variable first.",
                    source_var_name, unit
                  ),
                  location,
                );
              }

              // Check 2: Bit-width compatibility (still check even if unit error)
              let source_bits = self.get_type_bits(&source_symbol.data_type);
              let target_bits = self.get_type_bits(target_type);

              if let (Some(source_bits), Some(target_bits)) = (source_bits, target_bits)
                && source_bits != target_bits
              {
                self.add_error(
                           format!(
                             "Unsafe cast bit-width mismatch: pointer points to '{}' ({}-bit) but target type '{}' is {}-bit",
                             source_symbol.data_type, source_bits, target_type, target_bits
                           ),
                           location,
                         );
              }
            }
          }
          // Note: Pointers without tracked source type (e.g., from FFI or pointer
          // arithmetic) are silently allowed. The `unsafe` keyword already signals
          // that the programmer accepts responsibility for correctness.
        }
      }
    }
  }

  /// Get the bit width of a type. Returns None for composite or unknown types.
  fn get_type_bits(&self, type_name: &str) -> Option<usize> {
    match type_name.to_lowercase().as_str() {
      "i8" | "u8" | "byte" | "char" => Some(8),
      "i16" | "u16" => Some(16),
      "i32" | "u32" | "f32" => Some(32),
      "i64" | "u64" | "f64" | "pointer" => Some(64),
      "f16" => Some(16),
      "bool" => Some(1),
      _ => None, // Composite types, strings, arrays, etc.
    }
  }

  /// Add an error to the errors collection.
  fn add_error(&mut self, message: String, location: &SourceLocation) {
    self
      .errors
      .push(SemanticError::new(message, location.clone()));
  }

  /// Add a warning to the warnings collection.
  #[allow(dead_code)]
  fn add_warning(&mut self, message: String, location: &SourceLocation) {
    self
      .warnings
      .push(SemanticError::new(message, location.clone()));
  }
}
