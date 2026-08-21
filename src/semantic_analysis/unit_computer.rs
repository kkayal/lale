//! Unit computation for expressions in semantic analysis.
//!
//! This module handles the computation of physical units for expressions during semantic analysis.
//! It extends the generic `UnitAnalyzer` utilities in `crate::unit_analysis` with context-aware
//! unit computation that integrates with symbol table lookups and error reporting.
//!
//! # Architecture
//!
//! UnitComputer integrates with symbol table lookups to determine the physical
//! unit of each expression during semantic analysis. It extends the stateless
//! utility functions in `crate::unit_analysis` with context-aware computation.
//!
//! Unit computation follows these rules:
//! - **Addition/Subtraction**: Operands must have identical units
//! - **Multiplication**: Units multiply together `<m> * <s> = <m⋅s>`
//! - **Division**: Units divide `<m> / <s> = <m/s>`
//! - **Power**: Exponent must be unitless; result unit is `unit ^ exponent`
//! - **Comparisons**: Operands must have the same unit
//!
//! # Integration
//!
//! Used by the `SemanticAnalyzer` during expression type checking to validate
//! dimensional consistency and compute result units.

use super::error_types::SemanticError;
use super::sqlite_symbol_management::SqliteSymbolManager;
use crate::ast::*;
use crate::unit_analysis::UnitAnalyzer as UtilityAnalyzer;
use std::collections::HashMap;

/// Unit computation for semantic analysis.
///
/// Provides methods for computing expression units with access to symbol tables
/// and error reporting capabilities.
pub struct UnitComputer<'a> {
  /// Reference to symbol manager for variable unit lookup
  symbols: &'a SqliteSymbolManager,
  /// Error callback for unit validation failures
  errors: &'a mut Vec<SemanticError>,
}

impl<'a> UnitComputer<'a> {
  /// Create a new unit computer with symbol and error references.
  pub fn new(symbols: &'a SqliteSymbolManager, errors: &'a mut Vec<SemanticError>) -> Self {
    Self { symbols, errors }
  }

  /// Add an error for unit violations.
  fn add_error(&mut self, message: impl Into<String>, location: &SourceLocation) {
    self
      .errors
      .push(SemanticError::new(message.into(), location.clone()));
  }

  /// Compute the resulting unit of an expression.
  pub fn expr_unit(&mut self, expr: &Expr) -> ExprUnit {
    match expr {
      Expr::Binary(bin) => {
        let lu = self.expr_unit(&bin.left);
        let ru = self.expr_unit(&bin.right);
        if bin.operator == BinaryOp::Pow {
          return self.unit_for_power(&lu, &bin.right, &bin.location);
        }
        self.unit_for_binary(&bin.operator, &lu, &ru, &bin.location)
      }
      Expr::Unary(un) => {
        let ou = self.expr_unit(&un.operand);
        self.unit_for_unary(&un.operator, &ou, &un.location)
      }
      Expr::Identifier(id) => self.symbols.lookup_var_unit(id.name()),
      Expr::IntLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::UintLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::FloatLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::HexLiteral(_) => ExprUnit::Unitless,
      Expr::CharLiteral(_) => ExprUnit::Unitless,
      Expr::BoolLiteral(_) => ExprUnit::Unitless,
      Expr::StringLiteral(_) => ExprUnit::Unitless,
      Expr::ArrayLiteral(arr) => {
        if let Some(first) = arr.elements.first() {
          self.expr_unit(first)
        } else {
          ExprUnit::Unitless
        }
      }
      Expr::FnCallExpr(fn_call) => {
        // If the call has a direct unit annotation (e.g., vec3(1,2,3)<m>), use it
        if let Some(ref unit) = fn_call.unit {
          ExprUnit::from_string(&unit.raw)
        } else {
          self.compute_fn_call_unit(fn_call)
        }
      }
      Expr::MemberAccess(acc) => {
        // Look up the field's unit from the type definition
        if let Expr::Identifier(obj) = &*acc.object
          && let Ok(Some(symbol)) = self.symbols.lookup_var_symbol(obj.name())
          && let Some(type_info) = self.symbols.lookup_type(&symbol.data_type)
        {
          for (field_name, _, field_unit, _) in &type_info.fields {
            if field_name == &acc.member.node {
              return ExprUnit::from_string(field_unit.as_deref().unwrap_or(""));
            }
          }
          ExprUnit::Unknown
        } else {
          ExprUnit::Unknown
        }
      }
      Expr::ArrayIndex(idx) => self.expr_unit(&idx.array),
      Expr::CompilerConst(_) => ExprUnit::Unitless,
      Expr::Grouped(inner) => self.expr_unit(inner),
      Expr::Conversion(conv) => self.expr_unit(&conv.operand),
      Expr::HasValue(_) => ExprUnit::Unitless,
      Expr::HasNoValue(_) => ExprUnit::Unitless,
      Expr::NothingExpr => ExprUnit::Unitless,
      Expr::HasErrors => ExprUnit::Unitless,
      Expr::LastError => ExprUnit::Unitless,
      Expr::TryPropagate(_) => ExprUnit::Unitless,
      Expr::Allocate(_) => ExprUnit::Unitless,
    }
  }

  /// Compute the unit for a binary operation.
  fn unit_for_binary(
    &mut self,
    op: &BinaryOp,
    lu: &ExprUnit,
    ru: &ExprUnit,
    loc: &SourceLocation,
  ) -> ExprUnit {
    match op {
      BinaryOp::Or | BinaryOp::And | BinaryOp::Xor => {
        if !lu.is_unitless() && !lu.is_unknown() {
          self.add_error(
            "Logical operator cannot be applied to quantity with unit",
            loc,
          );
        }
        if !ru.is_unitless() && !ru.is_unknown() {
          self.add_error(
            "Logical operator cannot be applied to quantity with unit",
            loc,
          );
        }
        ExprUnit::Unitless
      }

      BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
        if !lu.is_unitless() && !lu.is_unknown() {
          self.add_error("Bitwise operation requires operand without units", loc);
        }
        if !ru.is_unitless() && !ru.is_unknown() {
          self.add_error("Bitwise operation requires operand without units", loc);
        }
        ExprUnit::Unitless
      }

      BinaryOp::Eq
      | BinaryOp::NotEq
      | BinaryOp::Lt
      | BinaryOp::Gt
      | BinaryOp::LtEq
      | BinaryOp::GtEq => {
        // For comparisons, if one operand is unitless (e.g. a bare literal like 0),
        // infer its unit from the other operand. If both have different units, error.
        if lu.is_unitless() && !ru.is_unitless() {
          // Left is unitless, infer from right — both now have same unit
        } else if !lu.is_unitless() && ru.is_unitless() {
          // Right is unitless, infer from left — both now have same unit
        } else if !ExprUnit::same_unit(lu, ru) {
          self.add_error(
            format!(
              "Cannot compare quantities with different units: {} and {}",
              lu.display(),
              ru.display()
            ),
            loc,
          );
        }
        ExprUnit::Unitless
      }

      BinaryOp::Add | BinaryOp::Sub => {
        if lu.is_unknown() || ru.is_unknown() {
          return ExprUnit::Unknown;
        }
        if !ExprUnit::same_unit(lu, ru) {
          self.add_error(
            format!(
              "Cannot add/subtract quantities with different units: {} and {}",
              lu.display(),
              ru.display()
            ),
            loc,
          );
          return ExprUnit::Unknown;
        }
        lu.clone()
      }

      BinaryOp::Mul => ExprUnit::combine_mul(lu, ru),

      BinaryOp::Div => ExprUnit::combine_div(lu, ru),

      BinaryOp::Mod => {
        if lu.is_unknown() || ru.is_unknown() {
          return ExprUnit::Unknown;
        }
        if !ExprUnit::same_unit(lu, ru) {
          self.add_error(
            format!(
              "Modulo requires matching units; got {} and {}",
              lu.display(),
              ru.display()
            ),
            loc,
          );
          return ExprUnit::Unknown;
        }
        lu.clone()
      }

      BinaryOp::Pow => {
        if !ru.is_unitless() && !ru.is_unknown() {
          self.add_error("Exponent must be unitless", loc);
        }
        if lu.is_unknown() {
          return ExprUnit::Unknown;
        }
        if lu.is_unitless() {
          return ExprUnit::Unitless;
        }
        // For power with non-constant exponent and complex unit: cannot compute result unit
        // This is correct behavior - unit tracking requires constant exponent to know result unit
        self.add_error(
          "Cannot compute unit for power operation with non-constant exponent and quantity with units. \
           Unit computation requires the exponent to be a compile-time constant.",
          loc,
        );
        ExprUnit::Unknown
      }

      BinaryOp::Append => {
        if !lu.is_unitless() && !lu.is_unknown() {
          self.add_error("Cannot append value with physical units", loc);
        }
        if !ru.is_unitless() && !ru.is_unknown() {
          self.add_error("Cannot append value with physical units", loc);
        }
        ExprUnit::Unitless
      }

      BinaryOp::Dot | BinaryOp::Cross => {
        // Vector dot/cross products: units combine via multiplication.
        // dot(u, v) -> unit_u * unit_v, cross(u, v) -> unit_u * unit_v
        ExprUnit::combine_mul(lu, ru)
      }

      BinaryOp::UnsignedLeftShift
      | BinaryOp::UnsignedRightShift
      | BinaryOp::SignedLeftShift
      | BinaryOp::SignedRightShift => {
        if !lu.is_unitless() && !lu.is_unknown() {
          self.add_error("Bitwise shift requires operand without units", loc);
        }
        if !ru.is_unitless() && !ru.is_unknown() {
          self.add_error("Shift amount must not have units", loc);
        }
        lu.clone()
      }
    }
  }

  /// Compute the unit for a power operation, extracting the actual exponent value if possible.
  fn unit_for_power(
    &mut self,
    base_unit: &ExprUnit,
    exponent_expr: &Expr,
    loc: &SourceLocation,
  ) -> ExprUnit {
    let exp_unit = self.expr_unit(exponent_expr);
    if !exp_unit.is_unitless() && !exp_unit.is_unknown() {
      self.add_error("Exponent must be unitless", loc);
    }

    if base_unit.is_unknown() {
      return ExprUnit::Unknown;
    }
    if base_unit.is_unitless() {
      return ExprUnit::Unitless;
    }

    let exp_value = UtilityAnalyzer::try_extract_int_value(exponent_expr);

    match (base_unit, exp_value) {
      (ExprUnit::Unit(nu), Some(exp)) => ExprUnit::power(&ExprUnit::Unit(nu.clone()), exp),
      (ExprUnit::Unit(_), None) => ExprUnit::Unknown,
      _ => base_unit.clone(),
    }
  }

  /// Compute the unit for a unary operation.
  fn unit_for_unary(&mut self, op: &UnaryOp, u: &ExprUnit, loc: &SourceLocation) -> ExprUnit {
    match op {
      UnaryOp::Neg => u.clone(),

      UnaryOp::Not => {
        if !u.is_unitless() && !u.is_unknown() {
          self.add_error("Logical not cannot be applied to quantity with units", loc);
        }
        ExprUnit::Unitless
      }

      UnaryOp::Invert => {
        if !u.is_unitless() && !u.is_unknown() {
          self.add_error(
            "Bitwise invert cannot be applied to quantity with units",
            loc,
          );
        }
        ExprUnit::Unitless
      }

      UnaryOp::TypeOf | UnaryOp::SizeOf | UnaryOp::UnitOf => ExprUnit::Unitless,

      UnaryOp::PointerTo => ExprUnit::Unitless,

      UnaryOp::ValueAt => ExprUnit::Unknown,

      UnaryOp::UnsafeCast => {
        // `unsafe cast` only operates on unitless values (enforced by semantic analysis)
        // Result is always unitless since bit reinterpretation is meaningless for dimensional values
        ExprUnit::Unitless
      }

      UnaryOp::ValueOf => u.clone(),
    }
  }

  /// Compute the unit for a function call by substituting argument units
  /// for parameter names and evaluating the return expression's unit.
  fn compute_fn_call_unit(&mut self, fn_call: &FnCall) -> ExprUnit {
    let fn_name = fn_call.target.node.last().map(|s| s.as_str()).unwrap_or("");

    let fn_info = match self.symbols.lookup_function(fn_name) {
      Some(info) => info.clone(),
      None => return ExprUnit::Unknown,
    };

    if fn_call.arguments.len() != fn_info.parameters.len() {
      return ExprUnit::Unknown;
    }

    let arg_units: Vec<ExprUnit> = fn_call
      .arguments
      .iter()
      .map(|arg| self.expr_unit(arg))
      .collect();

    let mut unit_map: HashMap<String, ExprUnit> = HashMap::new();
    for (param, arg_unit) in fn_info.parameters.iter().zip(arg_units.iter()) {
      if param.unit.is_some() {
        unit_map.insert(param.name.node.clone(), ExprUnit::from_option(&param.unit));
      } else {
        unit_map.insert(param.name.node.clone(), arg_unit.clone());
      }
    }

    let local_units = UtilityAnalyzer::collect_local_var_units(&fn_info.body);
    unit_map.extend(local_units);

    let return_expr = UtilityAnalyzer::find_return_expr(&fn_info.body);
    match return_expr {
      Some(expr) => self.expr_unit_with_substitution(&expr, &unit_map),
      None => ExprUnit::Unitless,
    }
  }

  /// Compute expression unit with parameter substitution.
  fn expr_unit_with_substitution(
    &mut self,
    expr: &Expr,
    param_map: &HashMap<String, ExprUnit>,
  ) -> ExprUnit {
    match expr {
      Expr::Binary(bin) => {
        let lu = self.expr_unit_with_substitution(&bin.left, param_map);
        if bin.operator == BinaryOp::Pow {
          return self.unit_for_power(&lu, &bin.right, &bin.location);
        }
        let ru = self.expr_unit_with_substitution(&bin.right, param_map);
        self.unit_for_binary(&bin.operator, &lu, &ru, &bin.location)
      }
      Expr::Unary(un) => {
        let ou = self.expr_unit_with_substitution(&un.operand, param_map);
        self.unit_for_unary(&un.operator, &ou, &un.location)
      }
      Expr::Identifier(id) => {
        let name = id.name();
        if let Some(unit) = param_map.get(name) {
          unit.clone()
        } else {
          self.symbols.lookup_var_unit(name)
        }
      }
      Expr::IntLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::UintLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::FloatLiteral(lit) => ExprUnit::from_option(&lit.unit),
      Expr::HexLiteral(_) => ExprUnit::Unitless,
      Expr::CharLiteral(_) => ExprUnit::Unitless,
      Expr::BoolLiteral(_) => ExprUnit::Unitless,
      Expr::StringLiteral(_) => ExprUnit::Unitless,
      Expr::ArrayLiteral(arr) => {
        if let Some(first) = arr.elements.first() {
          self.expr_unit_with_substitution(first, param_map)
        } else {
          ExprUnit::Unitless
        }
      }
      Expr::FnCallExpr(_) => ExprUnit::Unknown,
      Expr::MemberAccess(acc) => {
        if let Expr::Identifier(obj) = &*acc.object
          && let Ok(Some(symbol)) = self.symbols.lookup_var_symbol(obj.name())
          && let Some(type_info) = self.symbols.lookup_type(&symbol.data_type)
        {
          for (field_name, _, field_unit, _) in &type_info.fields {
            if field_name == &acc.member.node {
              if let Some(unit_str) = field_unit {
                return ExprUnit::from_string(unit_str);
              }
              return ExprUnit::Unitless;
            }
          }
        }
        self.expr_unit_with_substitution(&acc.object, param_map)
      }
      Expr::ArrayIndex(idx) => self.expr_unit_with_substitution(&idx.array, param_map),
      Expr::CompilerConst(_) => ExprUnit::Unitless,
      Expr::Grouped(inner) => self.expr_unit_with_substitution(inner, param_map),
      Expr::Conversion(conv) => self.expr_unit_with_substitution(&conv.operand, param_map),
      Expr::HasValue(_) => ExprUnit::Unitless,
      Expr::HasNoValue(_) => ExprUnit::Unitless,
      Expr::NothingExpr => ExprUnit::Unitless,
      Expr::HasErrors => ExprUnit::Unitless,
      Expr::LastError => ExprUnit::Unitless,
      Expr::TryPropagate(_) => ExprUnit::Unitless,
      Expr::Allocate(_) => ExprUnit::Unitless,
    }
  }
}
