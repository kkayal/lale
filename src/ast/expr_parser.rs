//! Expression parser using the Pratt parsing algorithm.
//!
//! This module handles the precedence-climbing algorithm for expression parsing,
//! separating the complexity of operator precedence from statement-level parsing.
//!
//! # Architecture
//!
//! The Pratt parser processes expressions using three closure variants:
//! - **Primary**: Identifiers, literals, parenthesized expressions
//! - **Infix**: Binary operators (`+`, `*`, `==`, etc.)
//! - **Prefix**: Unary operators (`-`, `!`, `typeof`, etc.)
//! - **Postfix**: Member access, type conversion, unsafe cast
//!
//! # Example
//!
//! ```text
//! let pair = ...; // from pest parser
//! let expr = build_expression(pair)?;
//! ```

use std::sync::LazyLock;

use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};

use super::Spanned;
use super::builder::location_from_pair;
use super::definitions::*;
use crate::Rule;

/// Pratt parser for handling operator precedence in expressions.
static PRATT_PARSER: LazyLock<PrattParser<Rule>> = LazyLock::new(|| {
  PrattParser::new()
    .op(Op::infix(Rule::logical_or, Assoc::Left))
    .op(Op::infix(Rule::logical_xor, Assoc::Left))
    .op(Op::infix(Rule::logical_and, Assoc::Left))
    .op(Op::infix(Rule::bitwise_or, Assoc::Left))
    .op(Op::infix(Rule::bitwise_xor, Assoc::Left))
    .op(Op::infix(Rule::bitwise_and, Assoc::Left))
    .op(Op::infix(Rule::equality, Assoc::Left))
    .op(Op::infix(Rule::comparison, Assoc::Left))
    .op(Op::infix(Rule::addition, Assoc::Left))
    .op(Op::infix(Rule::multiplication, Assoc::Left))
    .op(Op::infix(Rule::unsigned_left_shift, Assoc::Left))
    .op(Op::infix(Rule::unsigned_right_shift, Assoc::Left))
    .op(Op::infix(Rule::signed_left_shift, Assoc::Left))
    .op(Op::infix(Rule::signed_right_shift, Assoc::Left))
    .op(Op::infix(Rule::power, Assoc::Right))
    .op(Op::infix(Rule::append, Assoc::Left))
    .op(Op::prefix(Rule::unary))
    .op(Op::postfix(Rule::member_access))
    .op(Op::postfix(Rule::arr_index))
    .op(Op::postfix(Rule::superscript_power))
    .op(Op::postfix(Rule::conversion))
    .op(Op::postfix(Rule::unsafe_cast_wrapper))
    .op(Op::postfix(Rule::has_value_op))
    .op(Op::postfix(Rule::has_no_value_op))
    .op(Op::postfix(Rule::try_propagate_op))
});

/// Build an expression using the Pratt parser (precedence-climbing algorithm).
///
/// # Overview
///
/// | Aspect   | Description                                                         |
/// |----------|---------------------------------------------------------------------|
/// | Input    | `Pairs<Rule>` from `pair.into_inner()`                              |
/// | Output   | `Result<Expr, String>` — a fully-constructed Lale expression AST    |
///
/// Unlike a standard pest parser (which returns raw `Pairs` for manual traversal),
/// the Pratt parser consumes tokens and transforms them into AST nodes on-the-fly
/// via three mapping functions.
///
/// # Mapping Functions
///
/// Each function defines how a token type becomes an AST node:
///
/// ## `map_primary` — Leaf Expressions
///
/// Handles identifiers, literals, and other atomic expressions.
///
/// ```text
/// Pair<Rule> (primary token)  →  build_primary()  →  Result<Expr, String>
/// ```
///
/// ## `map_infix` — Binary Operators
///
/// Combines two operands with an operator (e.g., `a + b`, `x && y`).
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │  lhs: Result<Expr>   ─┐                                         │
/// │  op:  Pair<Rule>      ├──→  pattern match op  ──→  Expr::Binary │
/// │  rhs: Result<Expr>   ─┘                                         │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// Supported operators: `||`, `&&`, `^^`, `==`, `!=`, `<`, `>`, `<=`, `>=`,
/// `+`, `-`, `*`, `/`, `%`, `^`, `++`
///
/// ## `map_prefix` — Unary Operators
///
/// Applies a prefix operator to a single operand (e.g., `!x`, `-y`).
///
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │ op: Pair<Rule>   ─┐                                        │
/// │                   ├──→  build_unary_op()  ──→  Expr::Unary │
/// │ rhs: Result<Expr> ┘                                        │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// # Precedence Climbing
///
/// The `.parse(pair.into_inner())` call:
/// 1. Iterates through the token stream
/// 2. Classifies each token (primary, infix, or prefix)
/// 3. Invokes the appropriate mapping function
/// 4. Respects operator precedence (defined in `PRATT_PARSER` initialization)
/// 5. Returns the final `Result<Expr, String>`
pub fn build_expression(pair: Pair<Rule>) -> Result<Expr, String> {
  PRATT_PARSER
    .map_primary(|primary| build_primary(primary))
    .map_infix(|lhs, op, rhs| {
      let lhs = lhs?;
      let rhs = rhs?;
      let operator = match op.as_rule() {
        Rule::logical_or => BinaryOp::Or,
        Rule::logical_and => BinaryOp::And,
        Rule::logical_xor => BinaryOp::Xor,
        Rule::bitwise_and => BinaryOp::BitAnd,
        Rule::bitwise_or => BinaryOp::BitOr,
        Rule::bitwise_xor => BinaryOp::BitXor,
        Rule::equality => match op.as_str() {
          "==" => BinaryOp::Eq,
          "!=" | "≠" => BinaryOp::NotEq,
          _ => BinaryOp::Eq,
        },
        Rule::comparison => match op.as_str() {
          "<" => BinaryOp::Lt,
          ">" => BinaryOp::Gt,
          "<=" | "≤" => BinaryOp::LtEq,
          ">=" | "≥" => BinaryOp::GtEq,
          _ => BinaryOp::Lt,
        },
        Rule::addition => match op.as_str() {
          "+" => BinaryOp::Add,
          "-" => BinaryOp::Sub,
          _ => BinaryOp::Add,
        },
        Rule::multiplication => match op.as_str() {
          "*" => BinaryOp::Mul,
          "/" | "÷" => BinaryOp::Div,
          "%" => BinaryOp::Mod,
          "⋅" | "dot" => BinaryOp::Dot,
          "⨯" | "cross" => BinaryOp::Cross,
          _ => BinaryOp::Mul,
        },
        Rule::unsigned_left_shift => BinaryOp::UnsignedLeftShift,
        Rule::unsigned_right_shift => BinaryOp::UnsignedRightShift,
        Rule::signed_left_shift => BinaryOp::SignedLeftShift,
        Rule::signed_right_shift => BinaryOp::SignedRightShift,
        Rule::power => BinaryOp::Pow,
        Rule::append => BinaryOp::Append,
        _ => return Err(format!("Unknown binary operator: {:?}", op.as_rule())),
      };

      Ok(Expr::Binary(BinaryExpr {
        left: Box::new(lhs),
        operator,
        right: Box::new(rhs),
        location: location_from_pair(&op),
      }))
    })
    .map_prefix(|op, rhs| {
      let rhs = rhs?;
      let operator = build_unary_op(op.clone())?;

      Ok(Expr::Unary(UnaryExpr {
        operator,
        operand: Box::new(rhs),
        location: location_from_pair(&op),
      }))
    })
    .map_postfix(|lhs, op| {
      let lhs = lhs?;
      match op.as_rule() {
        Rule::conversion => {
          let mut target_type = None;
          for inner in op.clone().into_inner() {
            if inner.as_rule() == Rule::type_name {
              target_type = Some(build_type_name(inner)?);
            }
          }
          Ok(Expr::Conversion(ConversionExpr {
            operand: Box::new(lhs),
            target_type: target_type.ok_or("Missing target type in conversion")?,
            location: location_from_pair(&op),
          }))
        }
        Rule::member_access => {
          let location = location_from_pair(&op);
          let mut member_name = String::new();
          let mut member_location = location.clone();

          for inner in op.clone().into_inner() {
            match inner.as_rule() {
              Rule::single_identifier => {
                member_name = inner.as_str().to_string();
                member_location = location_from_pair(&inner);
              }
              Rule::fn_call => {
                // For function calls like obj.method(), extract the method name
                let fn_call = build_fn_call(inner)?;
                if let Some(last_part) = fn_call.target.node.last() {
                  member_name = last_part.clone();
                  member_location = fn_call.target.span.clone();
                }
              }
              _ => {}
            }
          }

          Ok(Expr::MemberAccess(MemberAccess {
            object: Box::new(lhs),
            member: Spanned {
              node: member_name,
              span: member_location,
            },
            location,
          }))
        }
        Rule::unsafe_cast_wrapper => {
          // `unsafe cast` wraps the previous expression (e.g., `value at p unsafe cast`)
          let location = location_from_pair(&op);
          Ok(Expr::Unary(UnaryExpr {
            operator: UnaryOp::UnsafeCast,
            operand: Box::new(lhs),
            location,
          }))
        }
        Rule::has_value_op => Ok(Expr::HasValue(Box::new(lhs))),
        Rule::has_no_value_op => Ok(Expr::HasNoValue(Box::new(lhs))),
        Rule::try_propagate_op => Ok(Expr::TryPropagate(Box::new(lhs))),
        Rule::arr_index => {
          let location = location_from_pair(&op);
          let mut indices = Vec::new();
          for inner in op.clone().into_inner() {
            if inner.as_rule() == Rule::expression {
              indices.push(build_expression(inner)?);
            }
          }
          Ok(Expr::ArrayIndex(ArrayIndex {
            array: Box::new(lhs),
            indices,
            location,
          }))
        }
        Rule::superscript_power => {
          // Parse superscript exponent: m² → m ^ 2, m⁻³ → m ^ (-3)
          let location = location_from_pair(&op);
          let text = op.as_str();
          let (sign, digits) = if text.starts_with('⁻') {
            (-1, &text["⁻".len()..])
          } else if text.starts_with('⁺') {
            (1, &text["⁺".len()..])
          } else {
            (1, text)
          };
          let mut exponent: i64 = 0;
          for ch in digits.chars() {
            let digit = match ch {
              '⁰' => 0,
              '¹' => 1,
              '²' => 2,
              '³' => 3,
              '⁴' => 4,
              '⁵' => 5,
              '⁶' => 6,
              '⁷' => 7,
              '⁸' => 8,
              '⁹' => 9,
              _ => continue,
            };
            exponent = exponent * 10 + digit;
          }
          let exp_value = sign * exponent;
          Ok(Expr::Binary(BinaryExpr {
            left: Box::new(lhs),
            operator: BinaryOp::Pow,
            right: Box::new(Expr::IntLiteral(IntLiteral {
              value: exp_value,
              unit: None,
              location: location.clone(),
            })),
            location,
          }))
        }
        _ => Err(format!("Unknown postfix operator: {:?}", op.as_rule())),
      }
    })
    .parse(pair.into_inner())
}

/// Build a unary operator.
fn build_unary_op(pair: Pair<Rule>) -> Result<UnaryOp, String> {
  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::not_op => return Ok(UnaryOp::Not),
      Rule::invert_op => return Ok(UnaryOp::Invert),
      Rule::type_op => return Ok(UnaryOp::TypeOf),
      Rule::size_op => return Ok(UnaryOp::SizeOf),
      Rule::unit_op => return Ok(UnaryOp::UnitOf),
      Rule::ptr_op => return Ok(UnaryOp::PointerTo),
      Rule::val_at_op => return Ok(UnaryOp::ValueAt),
      Rule::value_of_op => return Ok(UnaryOp::ValueOf),
      _ => {}
    }
  }
  Ok(UnaryOp::Neg)
}

/// Build a primary expression.
fn build_primary(pair: Pair<Rule>) -> Result<Expr, String> {
  let location = location_from_pair(&pair);

  match pair.as_rule() {
    Rule::nothing_expr => Ok(Expr::NothingExpr),
    Rule::has_errors_expr => Ok(Expr::HasErrors),
    Rule::last_error_expr => Ok(Expr::LastError),
    Rule::qualified_identifier => {
      // Iterate over single_identifier children to build the path
      let path: Vec<String> = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::single_identifier)
        .map(|p| p.as_str().to_string())
        .collect();
      Ok(Expr::Identifier(IdentifierExpr { path, location }))
    }
    Rule::single_identifier => {
      // A bare identifier (no link separators)
      let path = vec![pair.as_str().to_string()];
      Ok(Expr::Identifier(IdentifierExpr { path, location }))
    }
    Rule::n_literal => build_number_literal(pair),
    Rule::h_literal => Ok(Expr::HexLiteral(HexLiteral {
      value: pair.as_str().to_string(),
      location,
    })),
    Rule::c_literal => {
      let s = pair.as_str();
      let ch = if s.len() >= 3 {
        s.chars().nth(1).unwrap_or(' ')
      } else {
        ' '
      };
      Ok(Expr::CharLiteral(CharLiteral {
        value: ch,
        location,
      }))
    }
    Rule::b_literal => Ok(Expr::BoolLiteral(BoolLiteral {
      value: pair.as_str() == "true",
      location,
    })),
    Rule::s_literal => build_string_literal(pair),
    Rule::a_literal => build_array_literal(pair),
    Rule::fn_call => Ok(Expr::FnCallExpr(build_fn_call(pair)?)),
    Rule::comp_const => build_compiler_const(pair),
    Rule::allocate_expr => {
      // allocate(size_expr)
      let mut inner = pair.into_inner();
      // skip kw_allocate, get expression
      let size = build_expression(inner.next().ok_or("Expected size expression")?)?;
      Ok(Expr::Allocate(AllocateExpr {
        size: Box::new(size),
        location,
      }))
    }
    Rule::expression => build_expression(pair),
    _ => Err(format!("Unknown primary expression: {:?}", pair.as_rule())),
  }
}

/// Build a number literal.
fn build_number_literal(pair: Pair<Rule>) -> Result<Expr, String> {
  let location = location_from_pair(&pair);
  let mut unit = None;
  let mut value_str = pair.as_str();

  for inner in pair.clone().into_inner() {
    match inner.as_rule() {
      Rule::float => value_str = inner.as_str(),
      Rule::int => value_str = inner.as_str(),
      Rule::u_int => value_str = inner.as_str(),
      Rule::unit => unit = Some(build_unit(inner)),
      _ => {}
    }
  }

  if value_str.contains('.') || value_str.contains('e') || value_str.contains('E') {
    let cleaned = value_str.trim_start_matches('+');
    Ok(Expr::FloatLiteral(FloatLiteral {
      value: cleaned
        .parse()
        .map_err(|e| format!("Float parse error: {}", e))?,
      unit,
      location,
    }))
  } else if value_str.starts_with('-') {
    // Negative literals are always signed (IntLiteral)
    Ok(Expr::IntLiteral(IntLiteral {
      value: value_str.parse().map_err(|e| {
        format!(
          "{}:{}:{}: error: {}",
          location.source_file, location.line, location.col, e
        )
      })?,
      unit,
      location,
    }))
  } else {
    // For positive integers, try to parse as i64 first (default to signed).
    // If it doesn't fit in i64, fall back to u64 (UintLiteral).
    // This allows `0 - 5` to work (both parsed as IntLiteral/i64),
    // while still supporting large unsigned values like u64::MAX.
    let cleaned = value_str.trim_start_matches('+');
    match cleaned.parse::<i64>() {
      Ok(value) => Ok(Expr::IntLiteral(IntLiteral {
        value,
        unit,
        location,
      })),
      Err(_) => {
        // Doesn't fit in i64, try u64
        let unsigned_value: u64 = cleaned.parse().map_err(|e| {
          format!(
            "{}:{}:{}: error: {}",
            location.source_file, location.line, location.col, e
          )
        })?;
        Ok(Expr::UintLiteral(UintLiteral {
          value: unsigned_value,
          unit,
          location,
        }))
      }
    }
  }
}

/// Build a string literal.
fn build_string_literal(pair: Pair<Rule>) -> Result<Expr, String> {
  let location = location_from_pair(&pair);
  let mut parts = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::text => {
        let text_location = location_from_pair(&inner);
        parts.push(StringPart::Text(Spanned::new(
          expand_escapes(inner.as_str()),
          text_location,
        )));
      }
      Rule::embedded_value => {
        for inner_pair in inner.into_inner() {
          if inner_pair.as_rule() == Rule::expression {
            parts.push(StringPart::EmbeddedValue(Box::new(build_expression(
              inner_pair,
            )?)));
          }
        }
      }
      _ => {}
    }
  }

  Ok(Expr::StringLiteral(StringLiteral { parts, location }))
}

/// Expand escape sequences (\n, \t, \r, \\, \", \') in a string.
pub fn expand_escapes_str(s: &str) -> String {
  expand_escapes(s)
}

/// Expand escape sequences (\n, \t, \r, \\, \", \') in string content.
fn expand_escapes(s: &str) -> String {
  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\\' {
      match chars.next() {
        Some('n') => result.push('\n'),
        Some('t') => result.push('\t'),
        Some('r') => result.push('\r'),
        Some('\\') => result.push('\\'),
        Some('"') => result.push('"'),
        Some('\'') => result.push('\''),
        Some(c) => {
          result.push('\\');
          result.push(c);
        }
        None => result.push('\\'),
      }
    } else {
      result.push(ch);
    }
  }
  result
}

/// Build an array literal.
fn build_array_literal(pair: Pair<Rule>) -> Result<Expr, String> {
  let location = location_from_pair(&pair);
  let raw_text = pair.as_str();
  let mut elements = Vec::new();
  let mut expressions: Vec<Expr> = Vec::new();

  for inner in pair.into_inner() {
    if inner.as_rule() == Rule::expression {
      expressions.push(build_expression(inner)?);
    }
  }

  // Check if this is a fill literal: [fill with value]
  let fill = if expressions.len() == 1 && raw_text.contains("fill") && raw_text.contains("with") {
    Some(Box::new(expressions.remove(0)))
  } else {
    elements = expressions;
    None
  };

  Ok(Expr::ArrayLiteral(ArrayLiteral {
    elements,
    fill,
    location,
  }))
}

/// Build a compiler constant.
fn build_compiler_const(pair: Pair<Rule>) -> Result<Expr, String> {
  let text = pair.as_str();
  let location = location_from_pair(&pair);
  let kind = if text == "#main" {
    CompilerConstKind::Main
  } else if text == "#source_file" {
    CompilerConstKind::SourceFile
  } else if text == "#source_line" {
    CompilerConstKind::SourceLine
  } else if text == "#compile_time" {
    CompilerConstKind::CompileTime
  } else if text == "#compiler_version" {
    CompilerConstKind::CompilerVersion
  } else if text == "#function_name" {
    CompilerConstKind::Function
  } else if text == "#posix" {
    CompilerConstKind::Posix
  } else if text == "#windows" {
    CompilerConstKind::Windows
  } else if text == "#debug" {
    CompilerConstKind::Debug
  } else if text == "#mode" {
    CompilerConstKind::Mode
  } else {
    return Err(format!("Unknown compiler constant: {}", text));
  };

  Ok(Expr::CompilerConst(CompilerConst { kind, location }))
}

/// Build a type name.
pub fn build_type_name(pair: Pair<Rule>) -> Result<TypeName, String> {
  let location = location_from_pair(&pair);
  let type_text = pair.as_str().to_string();
  let mut base_type = BaseType::I32;
  let mut inner_type: Option<Box<BaseType>> = None;
  let mut array_dimensions = Vec::new();
  let mut is_optional = false;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::u8 => base_type = BaseType::U8,
      Rule::i8 => base_type = BaseType::I8,
      Rule::u16 => base_type = BaseType::U16,
      Rule::i16 => base_type = BaseType::I16,
      Rule::u32 => base_type = BaseType::U32,
      Rule::i32 => base_type = BaseType::I32,
      Rule::u64 => base_type = BaseType::U64,
      Rule::i64 => base_type = BaseType::I64,
      Rule::f16 => base_type = BaseType::F16,
      Rule::f32 => base_type = BaseType::F32,
      Rule::f64 => base_type = BaseType::F64,
      Rule::str => base_type = BaseType::Str,
      Rule::bool => base_type = BaseType::Bool,
      Rule::byte => base_type = BaseType::Byte,
      Rule::char => base_type = BaseType::Char,
      Rule::pointer => base_type = BaseType::Pointer,
      Rule::single_identifier => base_type = BaseType::Custom(inner.as_str().to_string()),
      Rule::vector_type => {
        for vinner in inner.into_inner() {
          match vinner.as_rule() {
            Rule::vector_kind => {
              let kind = vinner.as_str();
              base_type = match kind {
                "vec2" => BaseType::Vec2,
                "vec3" => BaseType::Vec3,
                "vec4" => BaseType::Vec4,
                _ => BaseType::Vec3,
              };
            }
            Rule::type_name => {
              let tn = build_type_name(vinner)?;
              inner_type = Some(Box::new(tn.base_type));
            }
            _ => {}
          }
        }
      }
      Rule::arr_index => {
        for arr_inner in inner.into_inner() {
          if arr_inner.as_rule() == Rule::expression {
            array_dimensions.push(build_expression(arr_inner)?);
          }
        }
      }
      _ => {
        // optional_marker is silent — detected via pair text below
      }
    }
  }

  // Detect optional marker from the pair text (silent rule, no inner pair)
  if type_text.ends_with('?') {
    is_optional = true;
  }

  Ok(TypeName {
    base_type,
    inner_type,
    array_dimensions,
    is_optional,
    location,
  })
}

/// Build a unit.
fn build_unit(pair: Pair<Rule>) -> Unit {
  Unit {
    raw: pair.as_str().to_string(),
    location: location_from_pair(&pair),
  }
}

/// Build a function call.
/// Note: This is in expr_parser rather than builder to avoid circular dependencies,
/// since fn_call expressions need to call build_expression(), and expression parsing
/// is delegated to this module.
pub fn build_fn_call(pair: Pair<Rule>) -> Result<FnCall, String> {
  let location = location_from_pair(&pair);
  let mut leading = Vec::new();
  let mut trailing = None;
  let mut inline_comments = Vec::new();
  let mut target: Option<Spanned<Vec<String>>> = None;
  let mut arguments = Vec::new();
  let mut unit: Option<Unit> = None;
  let mut in_leading = true;
  let mut last_was_expr = false;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::doc => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        let doc = AttachedComment::Doc(DocStmt {
          content: content_text,
          location: location_from_pair(&inner),
        });
        if in_leading {
          leading.push(doc);
        } else {
          inline_comments.push(doc);
        }
        last_was_expr = false;
      }
      Rule::comment => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        let comment = AttachedComment::Comment(CommentStmt {
          content: content_text,
          location: location_from_pair(&inner),
        });
        if in_leading {
          leading.push(comment);
        } else {
          inline_comments.push(comment);
        }
        last_was_expr = false;
      }
      Rule::qualified_identifier => {
        in_leading = false;
        for seg in inner.into_inner() {
          if seg.as_rule() == Rule::single_identifier {
            let id_location = location_from_pair(&seg);
            let id_str = seg.as_str().to_string();
            match &mut target {
              None => target = Some(Spanned::new(vec![id_str], id_location)),
              Some(t) => t.node.push(id_str),
            }
          }
        }
        last_was_expr = false;
      }
      Rule::single_identifier => {
        in_leading = false;
        let id_location = location_from_pair(&inner);
        let id_str = inner.as_str().to_string();
        match &mut target {
          None => target = Some(Spanned::new(vec![id_str], id_location)),
          Some(t) => t.node.push(id_str),
        }
        last_was_expr = false;
      }
      Rule::expression => {
        in_leading = false;
        arguments.push(build_expression(inner)?);
        last_was_expr = true;
      }
      Rule::unit => {
        unit = Some(build_unit(inner));
        last_was_expr = false;
      }
      _ => {
        last_was_expr = false;
      }
    }
  }

  // The last inline comment (if any, and if it came after an expression) should be trailing
  if last_was_expr && !inline_comments.is_empty() {
    trailing = inline_comments.pop();
  } else if !inline_comments.is_empty() {
    // Check if the very last item we processed was a comment - make it trailing
    trailing = inline_comments.pop();
  }

  Ok(FnCall {
    target: target.ok_or("Function call requires a target")?,
    arguments,
    unit,
    location,
    comments: AttachedComments { leading, trailing },
    inline_comments,
  })
}
