//! Expression Analysis and Type Checking
//!
//! This module handles semantic analysis of expressions, including type inference,
//! unit computation, and pointer escape detection.

use super::type_compatibility::TypeInference;
use crate::ast::*;

/// Expression utilities and checking
pub struct ExpressionAnalyzer;

impl ExpressionAnalyzer {
  /// Convert an expression to a human-readable string representation (for error messages).
  pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
      Expr::Identifier(id) => id.full_path(),
      Expr::IntLiteral(lit) => lit.value.to_string(),
      Expr::UintLiteral(lit) => lit.value.to_string(),
      Expr::FloatLiteral(lit) => lit.value.to_string(),
      Expr::HexLiteral(lit) => lit.value.clone(),
      Expr::CharLiteral(lit) => format!("'{}'", lit.value),
      Expr::BoolLiteral(lit) => lit.value.to_string(),
      Expr::StringLiteral(_) => "\"string\"".to_string(),
      Expr::Grouped(inner) => Self::expr_to_string(inner),
      Expr::HasValue(inner) => format!("({} has value)", Self::expr_to_string(inner)),
      Expr::HasNoValue(inner) => format!("({} has no value)", Self::expr_to_string(inner)),
      Expr::NothingExpr => "nothing".to_string(),
      Expr::Binary(bin) => format!(
        "({} {} {})",
        Self::expr_to_string(&bin.left),
        match bin.operator {
          BinaryOp::Add => "+",
          BinaryOp::Sub => "-",
          BinaryOp::Mul => "*",
          BinaryOp::Div => "/",
          BinaryOp::Mod => "%",
          BinaryOp::Pow => "^",
          BinaryOp::Eq => "==",
          BinaryOp::NotEq => "!=",
          BinaryOp::Lt => "<",
          BinaryOp::LtEq => "<=",
          BinaryOp::Gt => ">",
          BinaryOp::GtEq => ">=",
          BinaryOp::And => "and",
          BinaryOp::Or => "or",
          BinaryOp::Xor => "xor",
          BinaryOp::BitAnd => "bitwise and",
          BinaryOp::BitOr => "bitwise or",
          BinaryOp::BitXor => "bitwise xor",
          BinaryOp::UnsignedLeftShift => "unsigned left shift",
          BinaryOp::UnsignedRightShift => "unsigned right shift",
          BinaryOp::SignedLeftShift => "signed left shift",
          BinaryOp::SignedRightShift => "signed right shift",
          BinaryOp::Append => "~",
          BinaryOp::Dot => "dot",
          BinaryOp::Cross => "cross",
        },
        Self::expr_to_string(&bin.right)
      ),
      Expr::Conversion(conv) => {
        format!(
          "{} as {}",
          Self::expr_to_string(&conv.operand),
          TypeInference::type_name_to_string(&conv.target_type)
        )
      }
      Expr::FnCallExpr(fc) => {
        let args: Vec<String> = fc.arguments.iter().map(Self::expr_to_string).collect();
        let name = fc.target.node.last().map(|s| s.as_str()).unwrap_or("");
        format!("{}({})", name, args.join(", "))
      }
      Expr::MemberAccess(ma) => {
        format!("{}.{}", Self::expr_to_string(&ma.object), ma.member.node)
      }
      Expr::ArrayIndex(ai) => {
        let indices: Vec<String> = ai.indices.iter().map(Self::expr_to_string).collect();
        format!(
          "{}[{}]",
          Self::expr_to_string(&ai.array),
          indices.join(", ")
        )
      }
      Expr::Unary(un) => {
        let op_str = match un.operator {
          UnaryOp::Neg => "-",
          UnaryOp::Not => "not ",
          _ => "",
        };
        format!("{}{}", op_str, Self::expr_to_string(&un.operand))
      }
      _ => "expression".to_string(),
    }
  }

  /// Check if an expression contains a pointer to a local variable.
  /// Returns Some(variable_name) if it does, None otherwise.
  pub fn find_pointer_to_local<F: Fn(&str) -> bool>(
    expr: &Expr,
    is_local_var: &F,
  ) -> Option<String> {
    match expr {
      Expr::Unary(un) => {
        if matches!(un.operator, UnaryOp::PointerTo)
          && let Expr::Identifier(id) = un.operand.as_ref()
          && is_local_var(id.name())
        {
          return Some(id.name().to_string());
        }
        Self::find_pointer_to_local(&un.operand, is_local_var)
      }
      Expr::Binary(bin) => Self::find_pointer_to_local(&bin.left, is_local_var)
        .or_else(|| Self::find_pointer_to_local(&bin.right, is_local_var)),
      Expr::Grouped(inner) => Self::find_pointer_to_local(inner, is_local_var),
      Expr::ArrayLiteral(arr) => {
        for elem in &arr.elements {
          if let Some(name) = Self::find_pointer_to_local(elem, is_local_var) {
            return Some(name);
          }
        }
        None
      }
      Expr::FnCallExpr(_) => None,
      _ => None,
    }
  }
}
