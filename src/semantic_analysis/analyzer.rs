//! Main Semantic Analyzer Implementation
//!
//! This module implements the core semantic analysis logic by combining
//! symbol management, type checking, and unit analysis.
//!
//! # Scope Rules
//!
//! Lale enforces strict scope rules to maintain language clarity:
//! - **Global Scope**: Top-level declarations only
//!   - Variables (var statements)
//!   - Functions (fn definitions) - must be at global scope only
//!   - Types (type definitions) - must be at global scope only
//! - **Function Scope**: Inside function bodies
//!   - Variables (var statements) - local to the function
//!   - Function parameters - local to the function
//!   - NO nested function definitions allowed
//!   - NO nested type definitions allowed
//!
//! # Architecture
//!
//! The semantic analyzer performs several validation passes:
//! 1. **Scope Validation**: Ensures functions and types are only defined globally
//! 2. **Symbol Table Construction**: Builds symbol tables for all scopes
//! 3. **Type Checking**: Validates type compatibility in assignments and operations
//! 4. **Unit Analysis**: Checks physical unit consistency
//! 5. **Memory Safety**: Detects escaping pointers to local variables
//! 6. **Unused Symbol Detection**: Warns about declared but unused variables and functions
//!
//! # Unused Symbol Warnings
//!
//! The analyzer automatically detects and warns about:
//! - **Unused Variables**: Variables defined but never referenced in expressions
//! - **Unused Functions**: Functions defined but never called
//! - **Unused Parameters**: Function parameters that are never referenced in the function body
//! - **Unused Loop Variables**: Loop variables that are never referenced in the loop body
//!
//! Symbols are excluded from unused detection if they are:
//! - **Exported**: Declared with the `export` modifier (may be used externally)
//! - **Imported**: Declared with the `import` modifier (provided externally)
//!
//! Warnings are emitted after all validation passes and reported along with error location.
//!
//! Quick navigation:
//!   line 66   — detect_os (platform detection helper)
//!   line 88   — struct SemanticAnalyzer
//!   line 135  — impl SemanticAnalyzer: constructor & setup (with_mut_manager, set_resolver, set_current_module_path)
//!   line 275  — Error/warning reporting (add_error, add_warning)
//!   line 340  — Symbol tracking (mark_symbol_defined, mark_symbol_used)
//!   line 366  — Unused symbol detection (emit_scope_unused_warnings, emit_unused_warnings)
//!   line 455  — infer_value_at_type (value-at type inference)
//!   line 488  — validate_signed_literal_to_unsigned_type
//!   line 548  — Type resolution (expr_type, expr_type_with_context)
//!   line 844  — Unit computation (expr_unit)
//!   line 856  — Parameter unit checking (check_fn_call_param_units)
//!   line 964  — Memory safety: escaping pointer checks
//!   line 982  — Variable registration (register_variable_decl)
//!   line 1050 — Type utilities (type_string_to_type_name, extract_pointer_source_type)
//!   line 1114 — Unsafe bitcast validation (validate_unsafe_bitcast_compatibility)
//!   line 1130 — Array initialization validation (validate_array_initialization)
//!   line 1239 — Output helpers (visit_output_expr, check_expr_type_is_concrete)
//!   line 1262 — impl SemanticAnalyzer: module imports (import_symbol)
//!   line 1290 — impl AnalyzerResults for SemanticAnalyzer
//!   line 1347 — impl AstVisitor<()> for SemanticAnalyzer (VISITOR ENTRY POINTS)
//!   line 1349 —   visit_program (top-level dispatch)
//!   line 1393 —   visit_use (module imports)
//!   line 1543 —   visit_type_def
//!   line 1581 —   visit_var_def / visit_unsafe_decl (L1778) / visit_assign (L1796)
//!   line 1920 —   visit_compound_assign / visit_value_at_assign (L1986)
//!   line 2001 —   visit_fn_def / visit_fn_signature (L2135) / visit_fn_call_stmt (L2175)
//!   line 2201 —   visit_if / visit_loop (L2223)
//!   line 2243 —   visit_return (escaping pointer + type/unit mismatch checks)
//!   line 2325 —   visit_stdout / visit_stderr (L2330) / visit_debug (L2334)
//!   line 2341 —   visit_stdin
//!   line 2364 —   visit_ct_if (compile-time conditional)
//!   line 2431 —   visit_ct_fail / visit_ct_warn (L2439)
//!   line 2453 —   visit_has_value / visit_has_no_value (optional type checking)
//!   line 2465 —   visit_try_propagate (? operator)
//!   line 2496 —   visit_binary / visit_unary (L2504)
//!   line 2587 —   visit_conversion (type conversion validation: widening/narrowing/literal range)
//!   line 2670 —   visit_identifier (symbol resolution + ambiguity detection)
//!   line 2747 —   visit_string_literal / visit_array_literal (L2756) / visit_fn_call_expr (L2763)
//!   line 2833 —   visit_member_access (private field read check)
//!   line 2865 —   visit_array_index (1-based bounds checking)
//!   line 2957 —   visit_type_name / visit_parameter (L2960) / visit_condition (L2963) / visit_range (L2985)
//!   line 3013 — check_binary_operand_types_impl (free function)
//!   line 3097 — check_binary_type_compatibility (free function)
//!   line 3168 — impl SemanticAnalyzer: binary ops & underflow
//!   line 3176 —   check_unsigned_underflow
//!   line 3235 —   extract_literal_value / extract_dimension_from_type (L3258) / extract_element_type (L3294)
//!   line 3307 — Compile-time evaluation (eval_const_condition, eval_const_expr, eval_compiler_const)
//!   line 3532 — Stdlib loading (load_stdlib_signatures, load_stdlib_signatures_recursive)
//!   line 3642 — is_non_boolean_condition / is_non_boolean_expr (free functions)
//!   line 3668 — pub trait AnalyzerResults
//!   line 3692 — pub struct OwnedAnalyzer / impl OwnedAnalyzer (L3705)
//!   line 3949 — impl AnalyzerResults for OwnedAnalyzer
//!   line 4002 — is_bare_numeric_literal (free function)
//!   line 4010 — analyze_ast / analyze_ast_with_options (L4015) / analyze_ast_with_stdlib (L4141)
//!   line 4070 — process_ct_directives

use crate::ast::AstVisitor;
use crate::ast::*;
use crate::config::{Backend, CompilerOptions};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use super::const_eval::{
  ArithOp, EvalResult, NumericFoldOp, fold_binary, fold_cmp, fold_neg, numeric_fold_op,
};
use super::const_value::ConstValue;
use super::error_types::SemanticError;
use super::expression_analysis::ExpressionAnalyzer;
use super::memory_safety::MemorySafetyChecker;
use super::module_resolver::{ModuleId, ModuleResolver};
use super::sqlite_symbol_management::{
  FnInfo, FnScopeKey, SqliteSymbolManager, TypeFieldInfo, VarScope,
};
use super::type_compatibility::{TypeChecker, TypeInference};
use super::type_conversion::{TypeCategory, TypeValidator};
use super::unit_computer::UnitComputer;
use crate::unit_analysis::UnitAnalyzer;

// Re-export symbol types
pub use crate::ast::{Linkage, StorageClass, Symbol, SymbolKind, SymbolTable, Visibility};

/// Detect the current operating system.
/// Returns a tuple (is_posix, is_windows).
fn detect_os() -> (bool, bool) {
  #[cfg(target_os = "windows")]
  {
    (false, true)
  }
  #[cfg(not(target_os = "windows"))]
  {
    (true, false)
  }
}

/// Return the numeric value of a numeric literal (`Int`, `Uint`, `Float`, or
/// `Hex`). Used to treat different literal spellings of the same number
/// (e.g. `4`, `4.0`, `0x4`) as equal when matching division-by-zero guards.
fn numeric_literal_value(expr: &Expr) -> Option<f64> {
  match expr {
    Expr::IntLiteral(l) => Some(l.value as f64),
    Expr::UintLiteral(u) => Some(u.value as f64),
    Expr::FloatLiteral(f) => Some(f.value),
    Expr::HexLiteral(h) => {
      match i64::from_str_radix(
        h.value.trim_start_matches("0x").trim_start_matches("0X"),
        16,
      ) {
        Ok(v) => Some(v as f64),
        Err(_) => None,
      }
    }
    _ => None,
  }
}

/// Return the name of the root variable of an lvalue expression, if any.
///
/// `pointer to arr[i]` / `pointer to p.field` make the whole array/struct
/// variable mutable, so those forms are unwrapped to the root identifier.
fn lvalue_root_name(expr: &Expr) -> Option<String> {
  match expr {
    Expr::Identifier(id) => Some(id.name().to_string()),
    Expr::Grouped(inner) => lvalue_root_name(inner),
    Expr::Conversion(conv) => lvalue_root_name(&conv.operand),
    Expr::ArrayIndex(ai) => lvalue_root_name(&ai.array),
    Expr::MemberAccess(ma) => lvalue_root_name(&ma.object),
    _ => None,
  }
}

/// Check whether two expressions are structurally identical (ignoring source
/// locations). Used to match divisor expressions against guard conditions.
fn structurally_equal(a: &Expr, b: &Expr) -> bool {
  // Numeric literals of different kinds (e.g. `4` and `4.0`) denote the same
  // value; treat them as equal. This is what lets a guard `4 ⋅ π != 0` cover a
  // divisor written `4.0 ⋅ π`.
  if let (Some(av), Some(bv)) = (numeric_literal_value(a), numeric_literal_value(b)) {
    return av == bv;
  }
  match (a, b) {
    (Expr::Binary(a), Expr::Binary(b)) => {
      a.operator == b.operator
        && structurally_equal(&a.left, &b.left)
        && structurally_equal(&a.right, &b.right)
    }
    (Expr::Unary(a), Expr::Unary(b)) => {
      a.operator == b.operator && structurally_equal(&a.operand, &b.operand)
    }
    (Expr::Conversion(a), Expr::Conversion(b)) => {
      type_name_equal(&a.target_type, &b.target_type) && structurally_equal(&a.operand, &b.operand)
    }
    (Expr::Identifier(a), Expr::Identifier(b)) => a.path == b.path,
    (Expr::IntLiteral(a), Expr::IntLiteral(b)) => a.value == b.value,
    (Expr::UintLiteral(a), Expr::UintLiteral(b)) => a.value == b.value,
    (Expr::FloatLiteral(a), Expr::FloatLiteral(b)) => a.value == b.value,
    (Expr::HexLiteral(a), Expr::HexLiteral(b)) => a.value == b.value,
    (Expr::CharLiteral(a), Expr::CharLiteral(b)) => a.value == b.value,
    (Expr::BoolLiteral(a), Expr::BoolLiteral(b)) => a.value == b.value,
    (Expr::StringLiteral(a), Expr::StringLiteral(b)) => {
      if a.parts.len() != b.parts.len() {
        return false;
      }
      a.parts.iter().zip(&b.parts).all(|(ap, bp)| match (ap, bp) {
        (StringPart::Text(at), StringPart::Text(bt)) => at.node == bt.node,
        (StringPart::EmbeddedValue(ae), StringPart::EmbeddedValue(be)) => {
          structurally_equal(ae, be)
        }
        _ => false,
      })
    }
    (Expr::ArrayLiteral(a), Expr::ArrayLiteral(b)) => {
      if a.elements.len() != b.elements.len() {
        return false;
      }
      if a.fill.is_some() != b.fill.is_some() {
        return false;
      }
      let elements_eq = a
        .elements
        .iter()
        .zip(&b.elements)
        .all(|(ae, be)| structurally_equal(ae, be));
      let fill_eq = match (&a.fill, &b.fill) {
        (Some(av), Some(bv)) => structurally_equal(av, bv),
        _ => true,
      };
      elements_eq && fill_eq
    }
    (Expr::FnCallExpr(a), Expr::FnCallExpr(b)) => {
      a.target.node == b.target.node
        && a.arguments.len() == b.arguments.len()
        && a
          .arguments
          .iter()
          .zip(&b.arguments)
          .all(|(aa, ba)| structurally_equal(aa, ba))
    }
    (Expr::MemberAccess(a), Expr::MemberAccess(b)) => {
      structurally_equal(&a.object, &b.object) && a.member.node == b.member.node
    }
    (Expr::ArrayIndex(a), Expr::ArrayIndex(b)) => {
      structurally_equal(&a.array, &b.array)
        && a.indices.len() == b.indices.len()
        && a
          .indices
          .iter()
          .zip(&b.indices)
          .all(|(ai, bi)| structurally_equal(ai, bi))
    }
    (Expr::CompilerConst(a), Expr::CompilerConst(b)) => a.kind == b.kind,
    (Expr::Grouped(a), Expr::Grouped(b)) => structurally_equal(a, b),
    (Expr::HasValue(a), Expr::HasValue(b)) => structurally_equal(a, b),
    (Expr::HasNoValue(a), Expr::HasNoValue(b)) => structurally_equal(a, b),
    (Expr::NothingExpr, Expr::NothingExpr) => true,
    (Expr::HasErrors, Expr::HasErrors) => true,
    (Expr::LastError, Expr::LastError) => true,
    (Expr::TryPropagate(a), Expr::TryPropagate(b)) => structurally_equal(a, b),
    _ => false,
  }
}

/// Structurally compare two conditions for equality (ignoring locations).
/// Used to detect duplicate match arms.
fn conditions_structurally_equal(a: &Condition, b: &Condition) -> bool {
  a.negated == b.negated && structurally_equal(&a.expr, &b.expr)
}

/// Whether an expression is a valid `switch` literal pattern.
///
/// Accepts the scalar literals Lale can compare by value, plus grouped literals.
/// Strings with embedded values (`"x{v}"`) are rejected because they are not
/// compile-time constants.
fn is_switch_literal(expr: &Expr) -> bool {
  match expr {
    Expr::IntLiteral(_)
    | Expr::UintLiteral(_)
    | Expr::FloatLiteral(_)
    | Expr::HexLiteral(_)
    | Expr::CharLiteral(_)
    | Expr::BoolLiteral(_) => true,
    Expr::StringLiteral(sl) => sl.parts.iter().all(|p| matches!(p, StringPart::Text(_))),
    Expr::Grouped(inner) => is_switch_literal(inner),
    _ => false,
  }
}

/// Whether a type string names a floating-point scalar type.
fn is_float_scalar_type(ty: &str) -> bool {
  matches!(ty, "f16" | "f32" | "f64")
}

/// Produce a canonical key for a `switch` literal pattern, used for duplicate
/// detection. Numeric spellings are normalized by the scrutinee type so that
/// `0x1A` and `26` (or `3` and `3.0`) compare equal.
fn switch_case_key(expr: &Expr, scrutinee_type: &str) -> Option<String> {
  match expr {
    Expr::Grouped(inner) => switch_case_key(inner, scrutinee_type),
    Expr::BoolLiteral(l) => Some(format!("bool:{}", l.value)),
    Expr::CharLiteral(l) => Some(format!("char:{}", l.value as u32)),
    Expr::StringLiteral(l) => {
      let mut text = String::new();
      for part in &l.parts {
        match part {
          StringPart::Text(t) => text.push_str(&t.node),
          StringPart::EmbeddedValue(_) => return None,
        }
      }
      Some(format!("str:{}", text))
    }
    Expr::FloatLiteral(l) => Some(format!("float:{:016x}", l.value.to_bits())),
    Expr::IntLiteral(l) => {
      if is_float_scalar_type(scrutinee_type) {
        Some(format!("float:{:016x}", (l.value as f64).to_bits()))
      } else {
        Some(format!("int:{}", l.value))
      }
    }
    Expr::UintLiteral(l) => {
      if is_float_scalar_type(scrutinee_type) {
        Some(format!("float:{:016x}", (l.value as f64).to_bits()))
      } else {
        Some(format!("int:{}", l.value as i128))
      }
    }
    Expr::HexLiteral(l) => {
      let digits = l.value.trim_start_matches("0x").trim_start_matches("0X");
      match u64::from_str_radix(digits, 16) {
        Ok(parsed) => {
          if is_float_scalar_type(scrutinee_type) {
            Some(format!("float:{:016x}", (parsed as f64).to_bits()))
          } else {
            Some(format!("int:{}", parsed as i128))
          }
        }
        // Overflow means the literal is out of range for any Lale integer type.
        // Fall back to the raw spelling so identical oversized literals still
        // compare equal for duplicate detection.
        Err(_) => Some(format!("hex:{}", l.value)),
      }
    }
    _ => None,
  }
}

/// Structurally compare two TypeName values for equality (ignoring locations).
fn type_name_equal(a: &TypeName, b: &TypeName) -> bool {
  if a.base_type != b.base_type || a.is_optional != b.is_optional {
    return false;
  }
  match (&a.inner_type, &b.inner_type) {
    (None, None) => {}
    (Some(ia), Some(ib)) => {
      if ia != ib {
        return false;
      }
    }
    _ => return false,
  }
  if a.array_dimensions.len() != b.array_dimensions.len() {
    return false;
  }
  a.array_dimensions
    .iter()
    .zip(&b.array_dimensions)
    .all(|(ad, bd)| structurally_equal(ad, bd))
}

/// Collect all structurally-equivalent forms of an expression for guard matching.
///
/// Conversion and Grouped wrappers are transparent for zero-checks:
/// `x as f64` is equivalent to `x`, and `(x)` is equivalent to `x`.
/// Returns a vec containing the expression itself plus all unwrapped forms.
fn divisor_equivalent_forms(expr: &Expr) -> Vec<&Expr> {
  let mut forms = vec![expr];
  let mut current = expr;
  loop {
    match current {
      Expr::Conversion(conv) => {
        current = &conv.operand;
        forms.push(current);
      }
      Expr::Grouped(inner) => {
        current = inner;
        forms.push(current);
      }
      _ => break,
    }
  }
  forms
}

/// Extract the integer value from a literal expression, if it is one.
fn try_extract_constant_int(expr: &Expr) -> Option<i64> {
  match expr {
    Expr::IntLiteral(l) => Some(l.value),
    Expr::UintLiteral(u) => {
      if u.value <= i64::MAX as u64 {
        Some(u.value as i64)
      } else {
        None
      }
    }
    Expr::HexLiteral(h) => i64::from_str_radix(
      h.value.trim_start_matches("0x").trim_start_matches("0X"),
      16,
    )
    .ok(),
    Expr::Grouped(inner) => try_extract_constant_int(inner),
    Expr::Conversion(conv) => try_extract_constant_int(&conv.operand),
    _ => None,
  }
}

/// Determine whether the comparison `expr OP constant` excludes zero from
/// the allowed range of `expr`, given that the comparison is known to be `polarity`.
///
/// Returns `Some(expr)` if zero is provably excluded (i.e., `expr ≠ 0`),
/// `None` otherwise (zero could occur).
fn compare_against_constant(
  expr: &Expr,
  constant: i64,
  op: &BinaryOp,
  polarity: bool,
) -> Option<Expr> {
  match (op, polarity) {
    // x > C (true): x ∈ [C+1, +∞]. Zero excluded if C+1 > 0, i.e. C >= 0.
    (BinaryOp::Gt, true) => (constant >= 0).then(|| expr.clone()),
    // x > C (false): x ∈ [-∞, C]. Zero excluded if C < 0, i.e. C <= -1.
    (BinaryOp::Gt, false) => (constant < 0).then(|| expr.clone()),
    // x >= C (true): x ∈ [C, +∞]. Zero excluded if C > 0.
    (BinaryOp::GtEq, true) => (constant > 0).then(|| expr.clone()),
    // x >= C (false): x ∈ [-∞, C-1]. Zero excluded if C-1 < 0, i.e. C <= 0.
    (BinaryOp::GtEq, false) => (constant <= 0).then(|| expr.clone()),
    // x < C (true): x ∈ [-∞, C-1]. Zero excluded if C <= 0.
    (BinaryOp::Lt, true) => (constant <= 0).then(|| expr.clone()),
    // x < C (false): x ∈ [C, +∞]. Zero excluded if C > 0.
    (BinaryOp::Lt, false) => (constant > 0).then(|| expr.clone()),
    // x <= C (true): x ∈ [-∞, C]. Zero excluded if C < 0.
    (BinaryOp::LtEq, true) => (constant < 0).then(|| expr.clone()),
    // x <= C (false): x ∈ [C+1, +∞]. Zero excluded if C >= 0.
    (BinaryOp::LtEq, false) => (constant >= 0).then(|| expr.clone()),
    // x == C (true): exactly one value. x ≠ 0 iff C ≠ 0.
    (BinaryOp::Eq, true) => (constant != 0).then(|| expr.clone()),
    // x == C (false): x ≠ C. x ≠ 0 is guaranteed iff C == 0.
    (BinaryOp::Eq, false) => (constant == 0).then(|| expr.clone()),
    // x != C (true): x ≠ C. x ≠ 0 is guaranteed iff C == 0.
    (BinaryOp::NotEq, true) => (constant == 0).then(|| expr.clone()),
    // x != C (false): x == C. x ≠ 0 iff C ≠ 0.
    (BinaryOp::NotEq, false) => (constant != 0).then(|| expr.clone()),
    _ => None,
  }
}

/// Extract a set of expressions proven non-zero from a condition expression.
///
/// `polarity` indicates whether the condition is known to be true or false.
fn extract_facts(expr: &Expr, polarity: bool) -> Vec<Expr> {
  match expr {
    Expr::Binary(bin) => extract_facts_from_binary(bin, polarity),
    Expr::Unary(un) if matches!(un.operator, UnaryOp::Not) => extract_facts(&un.operand, !polarity),
    Expr::Grouped(inner) => extract_facts(inner, polarity),
    _ => Vec::new(),
  }
}

/// Extract non-zero facts from a binary boolean expression, given its truth polarity.
fn extract_facts_from_binary(bin: &BinaryExpr, polarity: bool) -> Vec<Expr> {
  match bin.operator {
    BinaryOp::And => {
      if polarity {
        let mut facts = extract_facts(&bin.left, true);
        facts.extend(extract_facts(&bin.right, true));
        facts
      } else {
        Vec::new()
      }
    }
    BinaryOp::Or => {
      if !polarity {
        let mut facts = extract_facts(&bin.left, false);
        facts.extend(extract_facts(&bin.right, false));
        facts
      } else {
        Vec::new()
      }
    }
    BinaryOp::Eq
    | BinaryOp::NotEq
    | BinaryOp::Gt
    | BinaryOp::GtEq
    | BinaryOp::Lt
    | BinaryOp::LtEq => {
      // expr OP constant
      if let Some(c) = try_extract_constant_int(&bin.right) {
        compare_against_constant(&bin.left, c, &bin.operator, polarity)
          .into_iter()
          .collect()
      }
      // constant OP expr (swap)
      else if let Some(c) = try_extract_constant_int(&bin.left) {
        let swapped_op = match bin.operator {
          BinaryOp::Gt => BinaryOp::Lt,
          BinaryOp::GtEq => BinaryOp::LtEq,
          BinaryOp::Lt => BinaryOp::Gt,
          BinaryOp::LtEq => BinaryOp::GtEq,
          other => other,
        };
        compare_against_constant(&bin.right, c, &swapped_op, polarity)
          .into_iter()
          .collect()
      } else {
        Vec::new()
      }
    }
    _ => Vec::new(),
  }
}

/// A `var` definition captured while visiting one branch of a control-flow
/// construct. Used to compute definite assignment across branches.
#[derive(Debug, Clone)]
struct BranchVarDef {
  ty: String,
  unit: Option<String>,
  location: SourceLocation,
}

/// The set of `var` definitions made in a single branch of a control-flow
/// construct.
#[derive(Debug, Default)]
struct BranchDefs {
  vars: std::collections::HashMap<String, BranchVarDef>,
}

/// Semantic analyzer that builds symbol tables and detects undefined variables.
///
/// Lale has two scope levels: global and function-local. Variables defined
/// at the top level are in the global scope. Function parameters and local
/// variables are in the function scope, which also has access to globals.
///
/// # Scope Restrictions
///
/// - **Function Definitions**: Can only be defined at global scope
/// - **Type Definitions**: Can only be defined at global scope
/// - **Variables**: Can be defined in both global and function scopes
pub struct SemanticAnalyzer<'a> {
  symbols: &'a mut SqliteSymbolManager,
  errors: Vec<SemanticError>,
  warnings: Vec<SemanticError>,
  /// Expected return unit for the current function (for return statement checking)
  current_return_unit: Option<ExprUnit>,
  /// Expected return type for the current function (for return statement checking)
  current_return_type: Option<String>,
  /// Compiler options (backend selection, etc.)
  options: CompilerOptions,
  /// Track used variables/functions to detect unused declarations
  /// Maps symbol name -> true if used anywhere (name-based tracking)
  used_symbols: std::collections::HashSet<String>,
  /// Track used symbol locations for more accurate checking
  /// Maps "line:col" (definition location) -> true if this specific symbol was used
  used_symbol_locations: std::collections::HashSet<String>, // "line:col" format
  /// Track unused symbol locations (for display purposes)
  /// Maps "line:col" (definition location) -> true if this specific symbol was determined to be unused
  unused_symbol_locations: std::collections::HashSet<String>, // "line:col" format
  /// Track defined variables/functions for unused detection (current scope only)
  defined_symbols: std::collections::HashMap<String, (SourceLocation, bool)>, // (location, is_function)
  /// Track which symbols were defined in the current scope
  scope_symbols: Vec<String>,
  /// Track all defined symbols including those from exited scopes (for display only)
  /// Key: "location_line:location_col" for uniqueness across scopes
  all_defined_symbols: std::collections::HashMap<String, (String, SourceLocation, bool)>, // (name, location, is_function)
  /// Module resolver for cross-module symbol resolution.
  module_resolver: Option<Rc<RefCell<ModuleResolver>>>,
  /// Path of the current module being analyzed.
  current_module_path: Option<PathBuf>,
  /// Path of the main/root file being analyzed (for relative path calculation).
  root_file_path: Option<PathBuf>,
  /// True if running on POSIX systems (Unix-like)
  is_posix: bool,
  /// True if running on Windows
  is_windows: bool,
  /// Current type context for assignment checking (used for literal type inference)
  current_assignment_type: Option<String>,
  /// T? variables defined in the current function scope (name → definition location).
  /// Used to detect unchecked optional values — variables of type T? that are never
  /// tested with `has value` / `has no value` before leaving scope.
  optional_vars_in_scope: std::collections::HashMap<String, SourceLocation>,
  /// T? variables that have been checked with `has value` / `has no value`.
  /// Subset of optional_vars_in_scope.
  checked_optional_vars: std::collections::HashSet<String>,
  /// Stack of fact frames for division-by-zero guard analysis.
  /// Each frame is a set of expressions proven non-zero by surrounding guard conditions.
  /// Pushed when entering a guarded branch, popped when leaving.
  /// The union of all frames represents current known non-zero expressions.
  div_zero_guard_stack: Vec<Vec<Expr>>,
  /// Variables bound in branch arms that are NOT accessible after end branch.
  /// Maps variable name to the first arm location where it was bound.
  /// Checked in visit_identifier — if a variable is in this set, it's only
  /// accessible inside a branch arm, not after end branch.
  branch_only_vars: std::collections::HashMap<String, SourceLocation>,
  /// Stack of open branch frames (innermost last). `visit_var_def` records a
  /// `var` definition into the top frame; each control-flow construct pops its
  /// branches and computes the definite-assignment join.
  branch_def_stack: Vec<BranchDefs>,
  /// Stack of "sibling-defined" name sets, one per open construct. When a branch
  /// is visited, a same-named `var` definition is treated as a sibling join
  /// (allowed) rather than a duplicate. After each branch, the branch's defined
  /// names are added to the top set so later sibling branches see them.
  sibling_defined_stack: Vec<std::collections::HashSet<String>>,
  /// Variables allocated via `allocate` in the current function scope.
  /// Used to detect missing `release` / `on exit release`.
  allocated_vars_in_scope: std::collections::HashMap<String, SourceLocation>,
  /// Subset of allocated_vars_in_scope — variables that have been explicitly
  /// released via `release var` or `on exit release var`.
  released_vars: std::collections::HashSet<String>,
  /// Subset of allocated_vars_in_scope — variables covered by `on exit release`.
  on_exit_released_vars: std::collections::HashSet<String>,
  /// Pointer variables in the current function scope known to point at a local
  /// variable (pointer name → pointed-to local name). Extends the escape rule to
  /// `var p = pointer to x; return p` / `global = p`.
  local_pointer_vars: std::collections::HashMap<String, String>,
  /// Variables explicitly released via `release var` in the current function.
  /// Used to warn about double-free and use-after-free. Kept separate from
  /// `released_vars` (which also includes `on exit release`) so that an
  /// `on exit release` does not falsely flag a dereference before scope end.
  explicitly_released_vars: std::collections::HashSet<String>,
  /// True while visiting the body of an `on exit` statement. Prevents
  /// `visit_release` from treating an `on exit release` as an immediate release.
  in_on_exit_body: bool,
  /// True while visiting the body of a `test case`. Used to reject nested
  /// `test suite` / `test case` blocks.
  in_test_case: bool,
  /// Names of test suites already seen in this program (for duplicate detection).
  seen_test_suite_names: std::collections::HashSet<String>,
  /// Name of the suite whose cases are currently being visited (for scope keys).
  current_suite_name: Option<String>,
}

impl<'a> SemanticAnalyzer<'a> {
  /// Initialize analyzer with shared symbol manager and specific compiler options.
  pub fn with_mut_manager(symbols: &'a mut SqliteSymbolManager, options: CompilerOptions) -> Self {
    // Detect the current OS
    let (is_posix, is_windows) = detect_os();

    SemanticAnalyzer {
      symbols,
      errors: Vec::new(),
      warnings: Vec::new(),
      current_return_unit: None,
      current_return_type: None,
      options,
      used_symbols: std::collections::HashSet::new(),
      used_symbol_locations: std::collections::HashSet::new(),
      unused_symbol_locations: std::collections::HashSet::new(),
      defined_symbols: std::collections::HashMap::new(),
      scope_symbols: Vec::new(),
      all_defined_symbols: std::collections::HashMap::new(),
      module_resolver: None,
      current_module_path: None,
      root_file_path: None,
      is_posix,
      is_windows,
      current_assignment_type: None,
      optional_vars_in_scope: std::collections::HashMap::new(),
      checked_optional_vars: std::collections::HashSet::new(),
      div_zero_guard_stack: Vec::new(),
      branch_only_vars: std::collections::HashMap::new(),
      branch_def_stack: Vec::new(),
      sibling_defined_stack: Vec::new(),
      allocated_vars_in_scope: std::collections::HashMap::new(),
      released_vars: std::collections::HashSet::new(),
      on_exit_released_vars: std::collections::HashSet::new(),
      local_pointer_vars: std::collections::HashMap::new(),
      explicitly_released_vars: std::collections::HashSet::new(),
      in_on_exit_body: false,
      in_test_case: false,
      seen_test_suite_names: std::collections::HashSet::new(),
      current_suite_name: None,
    }
  }

  /// Set the module resolver for cross-module symbol resolution.
  pub fn set_resolver(&mut self, resolver: Rc<RefCell<ModuleResolver>>) {
    self.module_resolver = Some(resolver);
  }

  /// Set the root/main file path for relative path calculation.
  pub fn set_root_file_path(&mut self, path: PathBuf) {
    self.root_file_path = Some(path.clone());
    self.symbols.set_root_file_path(path);
  }

  /// Set the current module path for resolution.
  pub fn set_current_module_path(&mut self, path: PathBuf) {
    // Use just the filename for module display, not the full path
    let path_str = path
      .file_name()
      .map(|f| f.to_string_lossy().to_string())
      .unwrap_or_else(|| path.to_string_lossy().to_string());
    self.symbols.set_module_path(path_str.clone());

    // Also register the module in the database (required for FOREIGN KEY constraints)
    // Use empty string for base_dir since we're using relative paths
    self.symbols.register_module(&path_str, "");

    self.current_module_path = Some(path);
  }

  /// Returns the selected backend.
  pub fn backend(&self) -> Backend {
    self.options.backend
  }

  /// Returns all collected semantic errors.
  pub fn get_errors(&self) -> &[SemanticError] {
    &self.errors
  }

  /// Returns all collected semantic warnings.
  pub fn get_warnings(&self) -> &[SemanticError] {
    &self.warnings
  }

  /// Returns true if debug mode is active (default).
  pub fn is_debug(&self) -> bool {
    self.options.is_debug
  }

  /// Returns true if integer overflow should trap (default). Mirrors the
  /// `--unchecked_overflow` compiler flag, and is used by typed constant
  /// folding so the folded value matches the interpreter's runtime semantics.
  pub fn checked_overflow(&self) -> bool {
    self.options.checked_overflow
  }

  /// Returns true if no errors were detected.
  pub fn is_valid(&self) -> bool {
    self.errors.is_empty()
  }

  /// Returns all symbol tables (global + function scopes).
  pub fn get_all_symbol_tables(&self) -> HashMap<VarScope, SymbolTable> {
    self.symbols.get_all_symbol_tables().unwrap_or_default()
  }

  /// Returns the symbol table for a specific scope.
  pub fn get_symbol_table(&self, scope: VarScope) -> SymbolTable {
    self.symbols.get_symbol_table(scope)
  }

  /// Returns all defined symbols including from exited scopes (for unused detection display).
  pub fn get_defined_symbols(
    &self,
  ) -> &std::collections::HashMap<String, (String, SourceLocation, bool)> {
    &self.all_defined_symbols
  }

  /// Returns all used symbols (for unused detection display).
  pub fn get_used_symbols(&self) -> &std::collections::HashSet<String> {
    &self.used_symbols
  }

  /// Returns all used symbol locations (for more accurate display).
  pub fn get_used_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.used_symbol_locations
  }

  /// Returns all unused symbol locations (for more accurate display).
  pub fn get_unused_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.unused_symbol_locations
  }

  /// Look up the type of a variable from the symbol table.
  /// Returns the type string (e.g., "i32", "i32[5]", "f64[3][3]").
  pub fn lookup_var_type(&self, name: &str) -> Option<String> {
    self.symbols.lookup_var_type(name)
  }

  /// Look up a variable symbol from the symbol table.
  /// Returns the complete Symbol including pointer_to_type information.
  pub fn lookup_var_symbol(&self, name: &str) -> Option<crate::ast::Symbol> {
    self.symbols.lookup_var_symbol(name).unwrap_or(None)
  }

  pub fn lookup_function(&self, name: &str) -> Option<FnInfo> {
    self.symbols.lookup_function(name)
  }

  /// Resolve a function call name to its suite-qualified form when a suite
  /// function with that name exists in the enclosing test suite. Suite functions
  /// shadow same-named global functions inside their suite; globals are used
  /// when no suite function matches.
  fn resolve_fn_name(&self, name: &str) -> String {
    if let Some(suite) = &self.current_suite_name {
      let qualified = format!("{}::{}", suite, name);
      if self.symbols.lookup_function(&qualified).is_some() {
        return qualified;
      }
    }
    name.to_string()
  }

  /// Analyze a suite-level function definition.
  ///
  /// Suite functions behave like regular functions but live in the suite's
  /// namespace: their stored name is `suite::fn` so they don't collide with
  /// global functions, and their bodies fall back to the suite scope (not the
  /// global scope) for variable lookups via `current_suite`.
  fn visit_suite_fn_def(&mut self, fn_def: &FnDefStmt) {
    let suite = match self.current_suite_name.as_deref() {
      Some(s) => s.to_string(),
      None => ice!("visit_suite_fn_def called outside a test suite — this is a compiler bug"),
    };
    let mut mangled = fn_def.clone();
    mangled.name.node = format!("{}::{}", suite, fn_def.name.node);
    self.visit_fn_def(&mangled);
  }

  /// Look up a function's return type by name.
  pub fn lookup_function_return_type(&self, name: &str) -> Option<String> {
    self
      .get_all_symbol_tables()
      .get(&VarScope::Global)
      .and_then(|t| t.get(name))
      .map(|sym| sym.data_type.clone())
  }

  /// If `object` names an enum type and `member` is one of its zero-argument
  /// variants, return the enum type name. Otherwise `None`.
  ///
  /// A zero-argument variant is registered as a function under the variant's
  /// simple name that returns the enum type (see `visit_enum_def`), so this
  /// check reuses that registration rather than a separate enum table.
  fn enum_variant_access_type(&self, object: &Expr, member: &str) -> Option<String> {
    let Expr::Identifier(id) = object else {
      return None;
    };
    let enum_name = id.name().to_string();
    // The object must name a type (enum or struct).
    self.symbols.lookup_type(&enum_name)?;
    // A zero-argument variant constructor returns the enum type.
    let resolved = self.resolve_fn_name(member);
    let fn_info = self.lookup_function(&resolved)?;
    if !fn_info.parameters.is_empty() {
      return None;
    }
    let ret = self.lookup_function_return_type(&resolved)?;
    if ret == enum_name {
      Some(enum_name)
    } else {
      None
    }
  }

  /// Resolve module-qualified member access (`math.e`). Returns the exported
  /// symbol's type when `object` names a single-segment module and `member` is
  /// one of its exported symbols, or `None` otherwise.
  fn module_member_access_type(&self, object: &Expr, member: &str) -> Option<String> {
    let Expr::Identifier(id) = object else {
      return None;
    };
    self.module_export_type(id.name(), member)
  }

  /// Resolve a module-qualified symbol by module name and member name, returning
  /// the exported symbol's type, or `None` when the name is not a module or the
  /// member is not exported.
  fn module_export_type(&self, module_name: &str, member: &str) -> Option<String> {
    // The module name must be neither a type (enum-variant access) nor a value
    // (field access). A module name is neither.
    if self.symbols.lookup_type(module_name).is_some() {
      return None;
    }
    if self.symbols.is_variable_defined(module_name) {
      return None;
    }
    // A single-segment module name maps to its source file name in the database.
    let module_filename = format!("{}.lale", module_name);
    let exports = match self.symbols.get_exports(&module_filename) {
      Ok(e) => e,
      Err(_) => return None,
    };
    exports.get(member).map(|s| s.data_type.clone())
  }

  /// Record a semantic error at the given location.
  fn add_error(&mut self, message: impl Into<String>, location: &SourceLocation) {
    self.add_error_internal(message.into(), location.clone(), None);
  }

  /// Record a semantic warning at the given location.
  fn add_warning(&mut self, message: impl Into<String>, location: &SourceLocation) {
    self.warnings.push(SemanticError {
      message: message.into(),
      location: location.clone(),
      hint: None,
    });
  }

  /// Collect all tracked std sub-module imports for the current module as
  /// `(submodule, imported-symbols)` pairs, from the metadata table.
  fn get_std_submodules_from_db(&self) -> Vec<(String, String)> {
    let prefix = format!("std_submod:{}:", self.symbols.get_module_path());
    // Scan the metadata table for keys with this module's std_submod prefix.
    if let Ok(mut stmt) = self
      .symbols
      .conn()
      .prepare("SELECT key, value FROM metadata WHERE key LIKE ?1")
    {
      let pattern = format!("{}%", prefix);
      let rows = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
      });
      if let Ok(rows) = rows {
        let mut result = Vec::new();
        for r in rows {
          if let Ok((key, value)) = r
            && let Some(submod) = key.strip_prefix(&prefix)
          {
            result.push((submod.to_string(), value));
          }
        }
        return result;
      }
    }
    Vec::new()
  }

  /// Extract a source location from any expression.
  fn expr_location(&self, expr: &Expr) -> SourceLocation {
    match expr {
      Expr::Binary(b) => b.location.clone(),
      Expr::Unary(u) => u.location.clone(),
      Expr::Conversion(c) => c.location.clone(),
      Expr::Identifier(i) => i.location.clone(),
      Expr::IntLiteral(l) => l.location.clone(),
      Expr::UintLiteral(l) => l.location.clone(),
      Expr::FloatLiteral(l) => l.location.clone(),
      Expr::HexLiteral(l) => l.location.clone(),
      Expr::CharLiteral(l) => l.location.clone(),
      Expr::BoolLiteral(l) => l.location.clone(),
      Expr::StringLiteral(l) => l.location.clone(),
      Expr::ArrayLiteral(l) => l.location.clone(),
      Expr::FnCallExpr(f) => f.location.clone(),
      Expr::MemberAccess(m) => m.location.clone(),
      Expr::ArrayIndex(a) => a.location.clone(),
      Expr::CompilerConst(c) => c.location.clone(),
      _ => SourceLocation::dummy(),
    }
  }

  /// Record that a T? variable has been checked via `has value` / `has no value`.
  /// If the expression is a simple identifier, mark it in the checked set.
  fn record_optional_check(&mut self, expr: &Expr) {
    if let Expr::Identifier(id) = expr {
      self.checked_optional_vars.insert(id.name().to_string());
    }
  }

  /// Warn if `value of` is applied to a T? variable that hasn't been guarded
  /// by a `has value` / `has no value` check in the current scope.
  fn check_unguarded_value_of(&mut self, expr: &Expr, location: &SourceLocation) {
    if let Expr::Identifier(id) = expr {
      let name = id.name();
      if self.optional_vars_in_scope.contains_key(name)
        && !self.checked_optional_vars.contains(name)
      {
        self.add_warning(
          format!(
            "`value of {}` used without a visible `has value` / `has no value` check. \
             If '{}' is absent, this will abort the program at runtime.",
            name, name
          ),
          location,
        );
      }
    }
  }

  // ==================== Division-by-zero guard analysis ====================

  /// Push a new frame of guard facts onto the stack.
  fn push_guard_frame(&mut self, facts: Vec<Expr>) {
    if !facts.is_empty() {
      self.div_zero_guard_stack.push(facts);
    }
  }

  /// Pop the most recently pushed guard frame.
  fn pop_guard_frame(&mut self) {
    self.div_zero_guard_stack.pop();
  }

  /// Check whether `divisor` is provably non-zero given current guard facts.
  /// Returns true if the divisor is a compile-time non-zero constant (or a
  /// product/quotient/negation of such), or if it structurally matches an
  /// expression in the guard fact set.
  ///
  /// Conversion and Grouped wrappers are transparent: `x as f64` is considered
  /// guarded if `x` is guarded, and vice versa. Type conversions don't change
  /// whether a value is zero.
  fn is_guarded_nonzero(&mut self, divisor: &Expr) -> bool {
    // A compile-time constant that is provably non-zero is always safe,
    // regardless of surrounding guards.
    if self.expr_is_constant_nonzero(divisor) {
      return true;
    }

    // Collect all structurally-equivalent forms of the divisor:
    // the divisor itself, plus versions with Conversion/Grouped wrappers stripped.
    let divisor_forms = divisor_equivalent_forms(divisor);

    // Check against guard stack facts.
    // Check both the raw fact expression and its unwrapped equivalents.
    self
      .div_zero_guard_stack
      .iter()
      .flatten()
      .any(|guarded_expr| {
        let fact_forms = divisor_equivalent_forms(guarded_expr);
        divisor_forms
          .iter()
          .any(|d| fact_forms.iter().any(|f| structurally_equal(f, d)))
      })
  }

  /// Whether `expr` is a compile-time constant that is provably non-zero.
  ///
  /// This is the division-by-zero consumer of the SQLite constant-value store. It
  /// folds the full expression with the declared numeric type (via
  /// `expr_const_value_typed` + `const_eval`), so `a + b` is folded exactly —
  /// `5 + -5` is recognised as zero while `5 + 5` is recognised as non-zero,
  /// where the old proof had to stay conservative about `+`/`-`.
  ///
  /// When no declared type is available (bare-literal combinations such as
  /// `5 ⋅ 5` or `-5`), it falls back to the structural product/quotient/negation
  /// recursion, which the typed fold subsumes whenever a type is present.
  fn expr_is_constant_nonzero(&mut self, expr: &Expr) -> bool {
    match self.expr_const_value_typed(expr, None) {
      EvalResult::Value(v) => v.is_non_zero(),
      EvalResult::Trap => false,
      EvalResult::Unknown => match expr {
        Expr::Unary(un) if matches!(un.operator, UnaryOp::Neg) => {
          self.expr_is_constant_nonzero(&un.operand)
        }
        Expr::Binary(bin)
          if matches!(bin.operator, BinaryOp::Mul | BinaryOp::Dot | BinaryOp::Div) =>
        {
          self.expr_is_constant_nonzero(&bin.left) && self.expr_is_constant_nonzero(&bin.right)
        }
        _ => false,
      },
    }
  }

  /// Evaluate a constant expression to its value, resolving identifiers through
  /// the SQLite constant-value store.
  ///
  /// This is the general constant-value evaluator for analyses that need the
  /// actual value (rather than just "non-zero"). It folds literal leaves,
  /// identifier lookups, transparent wrappers (`Grouped`, `Conversion`), and
  /// unary negation. Binary arithmetic folding is intentionally absent for now:
  /// folding `+`/`-`/`*`/`/` requires declared-type-aware arithmetic to be sound
  /// (integer overflow / truncation), which is deferred to the
  /// constant-propagation consumer.
  fn expr_const_value(&self, expr: &Expr) -> Option<ConstValue> {
    match expr {
      Expr::BoolLiteral(l) => Some(ConstValue::Bool(l.value)),
      Expr::IntLiteral(l) => Some(ConstValue::Int(l.value)),
      Expr::UintLiteral(u) => Some(ConstValue::Uint(u.value)),
      Expr::FloatLiteral(f) => Some(ConstValue::float(f.value)),
      Expr::HexLiteral(h) => match i64::from_str_radix(
        h.value.trim_start_matches("0x").trim_start_matches("0X"),
        16,
      ) {
        Ok(v) => Some(ConstValue::Int(v)),
        Err(_) => None,
      },
      Expr::StringLiteral(lit) => {
        let mut result = String::new();
        for part in &lit.parts {
          match part {
            StringPart::Text(text) => result.push_str(&text.node),
            StringPart::EmbeddedValue(_) => return None,
          }
        }
        Some(ConstValue::Text(result))
      }
      Expr::Identifier(id) => {
        if id.is_pi() {
          Some(ConstValue::float(PI_CONSTANT_VALUE))
        } else {
          self.symbols.lookup_variable_const_value(id.name())
        }
      }
      Expr::Grouped(inner) => self.expr_const_value(inner),
      Expr::Conversion(conv) => self.expr_const_value(&conv.operand),
      Expr::Unary(un) if matches!(un.operator, UnaryOp::Neg) => {
        self.expr_const_value(&un.operand).and_then(|v| match v {
          ConstValue::Int(i) => i.checked_neg().map(ConstValue::Int),
          ConstValue::Float(bits) => Some(ConstValue::float(-f64::from_bits(bits))),
          _ => None,
        })
      }
      _ => None,
    }
  }

  /// Evaluate an expression to a *typed* compile-time constant, folding binary
  /// arithmetic and comparisons according to the declared numeric type.
  ///
  /// This extends [`Self::expr_const_value`] (which only folds leaves,
  /// transparent wrappers, and unary negation) with declared-type-aware binary
  /// folding via `super::const_eval`. The result is three-valued:
  ///
  /// * `EvalResult::Value` — the expression folded to a concrete constant.
  /// * `EvalResult::Unknown` — some operand is not a known constant, or the
  ///   operation is not a numeric fold (logical/bitwise/shift/vector/string).
  /// * `EvalResult::Trap` — the operation would trap at runtime (integer
  ///   overflow/underflow, division by zero, or a floating-point `NaN`).
  ///
  /// `ty_hint` supplies the declared-type context for bare literals (the same
  /// role it plays in [`Self::expr_type_with_context`]); pass `None` when no such
  /// context exists. The operand type is always resolved through
  /// [`Self::expr_type_with_context`], so a comparison folds with the *operand*
  /// type rather than its `bool` result type.
  ///
  /// # Soundness
  ///
  /// The folded value matches what the interpreter would produce because it
  /// reduces in `i128`/`u128` and then checks/truncates to the declared width,
  /// honouring `checked_overflow` exactly like `emit_int_add`/`emit_int_sub`/
  /// `emit_int_mul`/`emit_int_neg` in `src/ir_gen.rs`.
  pub fn expr_const_value_typed(&mut self, expr: &Expr, ty_hint: Option<&str>) -> EvalResult {
    match expr {
      Expr::Binary(bin) => {
        // The fold key is the *operand* type: for arithmetic it equals the
        // result type, and for comparisons it is the numeric operand type
        // (the expression's result type would be `bool`).
        let operand_type = self.expr_type_with_context(&bin.left, ty_hint);
        let info = match TypeValidator::get_type_category(&operand_type) {
          TypeCategory::Numeric(info) => info,
          _ => return EvalResult::Unknown,
        };

        let fold_op = match numeric_fold_op(bin.operator) {
          Some(op) => op,
          None => return EvalResult::Unknown,
        };

        let lhs = self.expr_const_value_typed(&bin.left, Some(&operand_type));
        let rhs = self.expr_const_value_typed(&bin.right, Some(&operand_type));
        let (l, r) = match (lhs, rhs) {
          (EvalResult::Value(l), EvalResult::Value(r)) => (l, r),
          (EvalResult::Trap, _) | (_, EvalResult::Trap) => return EvalResult::Trap,
          _ => return EvalResult::Unknown,
        };

        match fold_op {
          NumericFoldOp::Arith(op) => fold_binary(op, &info, self.checked_overflow(), &l, &r),
          NumericFoldOp::Cmp(op) => fold_cmp(op, &info, &l, &r),
        }
      }
      Expr::Unary(un) if matches!(un.operator, UnaryOp::Neg) => {
        let operand_type = self.expr_type_with_context(&un.operand, ty_hint);
        let info = match TypeValidator::get_type_category(&operand_type) {
          TypeCategory::Numeric(info) => info,
          _ => return EvalResult::Unknown,
        };
        match self.expr_const_value_typed(&un.operand, Some(&operand_type)) {
          EvalResult::Value(v) => fold_neg(&info, self.checked_overflow(), &v),
          EvalResult::Trap => EvalResult::Trap,
          EvalResult::Unknown => EvalResult::Unknown,
        }
      }
      // `Grouped` and `Conversion` are transparent: recurse with the typed
      // evaluator so a wrapped binary expression (e.g. `100 / (a + b)`) still
      // folds instead of being treated as an unknown leaf.
      Expr::Grouped(inner) => self.expr_const_value_typed(inner, ty_hint),
      Expr::Conversion(conv) => self.expr_const_value_typed(&conv.operand, ty_hint),
      // Other leaves (literals, identifiers) are handled by the untyped evaluator.
      _ => match self.expr_const_value(expr) {
        Some(v) => EvalResult::Value(v),
        None => EvalResult::Unknown,
      },
    }
  }

  /// Fold the result of a compound assignment (`x += rhs`, `x /= rhs`, …) to a
  /// new constant value, when both the variable's current value and the RHS are
  /// known constants.
  ///
  /// Returns `None` when the result cannot be proven (an operand is unknown,
  /// the type is not numeric, or the operation would trap), which clears the
  /// variable's flow-sensitive `const_value`. This keeps the constant store
  /// precise for the division-by-zero and unsigned-underflow checks that read it.
  fn fold_compound_assign_const(
    &mut self,
    target_name: &str,
    compound: &CompoundAssignStmt,
  ) -> Option<ConstValue> {
    let op = match compound.operator.node {
      CompoundOp::AddAssign => ArithOp::Add,
      CompoundOp::SubAssign => ArithOp::Sub,
      CompoundOp::MulAssign => ArithOp::Mul,
      CompoundOp::DivAssign => ArithOp::Div,
      CompoundOp::ModAssign => ArithOp::Rem,
    };

    let var_type = self.symbols.lookup_var_type(target_name)?;
    let info = match TypeValidator::get_type_category(&var_type) {
      TypeCategory::Numeric(info) => info,
      _ => return None,
    };

    let current = self.symbols.lookup_variable_const_value(target_name)?;
    let rhs = match self.expr_const_value_typed(&compound.value, Some(var_type.as_str())) {
      EvalResult::Value(v) => v,
      EvalResult::Unknown | EvalResult::Trap => return None,
    };

    match fold_binary(op, &info, self.checked_overflow(), &current, &rhs) {
      EvalResult::Value(v) => Some(v),
      EvalResult::Unknown | EvalResult::Trap => None,
    }
  }

  /// Record a `var` definition into the current (innermost) branch frame, if
  /// one is open. Called by `visit_var_def` so each branch captures the names it
  /// defines for the definite-assignment join.
  fn record_branch_def(
    &mut self,
    name: &str,
    ty: &str,
    unit: Option<String>,
    location: &SourceLocation,
  ) {
    if let Some(top) = self.branch_def_stack.last_mut() {
      top.vars.insert(
        name.to_string(),
        BranchVarDef {
          ty: ty.to_string(),
          unit,
          location: location.clone(),
        },
      );
    }
  }

  /// Push a fresh branch frame (before visiting a branch body).
  fn begin_branch(&mut self) {
    self.branch_def_stack.push(BranchDefs::default());
  }

  /// Pop and return the definitions captured while visiting the current branch,
  /// and mark those names as sibling-defined so a later branch of the same
  /// construct treats a same-named definition as a join rather than a duplicate.
  fn end_branch(&mut self) -> BranchDefs {
    let defs = match self.branch_def_stack.pop() {
      Some(d) => d,
      None => {
        eprintln!("WARNING: end_branch called without a matching begin_branch");
        BranchDefs::default()
      }
    };
    if let Some(sibling) = self.sibling_defined_stack.last_mut() {
      for name in defs.vars.keys() {
        sibling.insert(name.clone());
      }
    }
    defs
  }

  /// Enter a control-flow construct: open a sibling-defined set for its branches.
  fn enter_construct(&mut self) {
    self
      .sibling_defined_stack
      .push(std::collections::HashSet::new());
  }

  /// Exit a control-flow construct: drop its sibling-defined set.
  fn exit_construct(&mut self) {
    self.sibling_defined_stack.pop();
  }

  /// True if `name` was defined in a previously-visited sibling branch of the
  /// current construct (i.e. this is a join, not a duplicate).
  fn is_sibling_join(&self, name: &str) -> bool {
    self
      .sibling_defined_stack
      .last()
      .is_some_and(|s| s.contains(name))
  }

  /// Compute the definite-assignment join across a construct's branches. A name
  /// defined in *every* branch with a consistent type and unit is definitely
  /// assigned (allowed afterwards); any other name defined in a branch becomes
  /// branch-only and is rejected if used after the construct.
  fn join_branch_defs(&mut self, branches: Vec<BranchDefs>) {
    let all_names: std::collections::HashSet<String> = branches
      .iter()
      .flat_map(|b| b.vars.keys().cloned())
      .collect();
    for name in all_names {
      let mut first: Option<&BranchVarDef> = None;
      let mut consistent = true;
      for branch in &branches {
        match branch.vars.get(&name) {
          Some(def) => {
            if let Some(prev) = first {
              if prev.ty != def.ty || prev.unit != def.unit {
                consistent = false;
              }
            } else {
              first = Some(def);
            }
          }
          None => {
            consistent = false;
          }
        }
      }
      if !consistent && let Some(first_def) = first {
        self
          .branch_only_vars
          .insert(name, first_def.location.clone());
      }
    }
  }

  /// Track a symbol definition for unused detection.
  fn mark_symbol_defined(&mut self, name: &str, location: SourceLocation, is_function: bool) {
    self
      .defined_symbols
      .insert(name.to_string(), (location.clone(), is_function));
    // Use location as unique key to handle symbols with same name in different scopes
    let unique_key = format!("{}:{}", location.line, location.col);
    self
      .all_defined_symbols
      .insert(unique_key, (name.to_string(), location, is_function));
    self.scope_symbols.push(name.to_string());
  }

  /// Mark a symbol as used.
  fn mark_symbol_used(&mut self, name: &str) {
    self.used_symbols.insert(name.to_string());
  }

  /// Report an error and return true when `name` is the reserved constant `π`.
  /// `π` is part of the language, so any user definition of it is rejected.
  fn reject_reserved_pi(&mut self, name: &str, location: &SourceLocation) -> bool {
    if name == PI_CONSTANT_NAME {
      self.add_error(
        "'π' is a reserved constant of the language and cannot be defined",
        location,
      );
      true
    } else {
      false
    }
  }

  /// Mark a symbol definition location as used.
  /// This is more precise than mark_symbol_used and handles scope collisions.
  fn mark_symbol_location_used(&mut self, location: &SourceLocation) {
    let key = format!("{}:{}", location.line, location.col);
    self.used_symbol_locations.insert(key);
  }

  /// Emit warnings for unused symbols in the current scope, then clear them.
  /// Called when exiting a function scope to check local variables.
  fn emit_scope_unused_warnings(&mut self, scope_symbols: Vec<String>) {
    for name in scope_symbols {
      // Skip variables named `_` — intentionally discarded values
      if name == "_" {
        continue;
      }
      if let Some((location, is_function)) = self.defined_symbols.remove(&name)
        && !self.used_symbols.contains(&name)
      {
        let kind = if is_function { "function" } else { "variable" };
        self.add_warning(format!("Unused {} '{}'", kind, name), &location);
        let location_key = format!("{}:{}", location.line, location.col);
        self.unused_symbol_locations.insert(location_key);
      }
    }
  }

  /// Warn about T? variables in the current scope that were never checked
  /// with `has value` / `has no value`.
  fn emit_unchecked_optional_warnings(&mut self) {
    // Collect warnings first to avoid borrowing conflicts
    let warnings: Vec<(String, SourceLocation)> = self
      .optional_vars_in_scope
      .iter()
      .filter(|(name, _)| !self.checked_optional_vars.contains(*name))
      .map(|(name, loc)| (name.clone(), loc.clone()))
      .collect();

    for (name, location) in warnings {
      self.add_warning(
        format!(
          "Optional variable '{}' is never checked with `has value` / `has no value`. \
           Did you forget to handle the absent case?",
          name
        ),
        &location,
      );
    }
  }

  /// Warn about allocate() calls without a matching release or on exit release
  /// in the current function scope.
  fn emit_missing_release_warnings(&mut self) {
    let warnings: Vec<(String, SourceLocation)> = self
      .allocated_vars_in_scope
      .iter()
      .filter(|(name, _)| {
        !self.released_vars.contains(*name) && !self.on_exit_released_vars.contains(*name)
      })
      .map(|(name, loc)| (name.clone(), loc.clone()))
      .collect();

    for (name, location) in warnings {
      self.add_warning(
        format!(
          "'{}' is allocated via allocate() but never released. \
           Add `release {}` or `on exit release {}` to free the memory.",
          name, name, name
        ),
        &location,
      );
    }
  }

  /// Emit warnings for all unused symbols (called at end of analysis for global scope).
  fn emit_unused_warnings(&mut self) {
    let unused: Vec<_> = self
      .defined_symbols
      .iter()
      .filter(|(name, _)| !self.used_symbols.contains(*name) && *name != "_")
      .map(|(name, (location, is_function))| (name.clone(), location.clone(), *is_function))
      .collect();

    for (name, location, is_function) in unused {
      let kind = if is_function { "function" } else { "variable" };
      self.add_warning(format!("Unused {} '{}'", kind, name), &location);
    }
  }

  /// Internal helper: record an error with optional hint.
  fn add_error_internal(
    &mut self,
    message: String,
    location: SourceLocation,
    hint: Option<String>,
  ) {
    let error = if let Some(h) = hint {
      SemanticError {
        message,
        location,
        hint: Some(h),
      }
    } else {
      SemanticError {
        message,
        location,
        hint: None,
      }
    };
    self.errors.push(error);
  }

  /// Infer the type of a `value at ptr` expression by looking up the pointer's source type.
  ///
  /// When we dereference a pointer with `value at`, the resulting type depends on what
  /// type the pointer points to. This is tracked in the symbol table's `pointer_to_type` field.
  ///
  /// The algorithm:
  /// 1. Check if the operand is an identifier (the pointer variable)
  /// 2. Look up the pointer's symbol in the symbol table
  /// 3. Check the `pointer_to_type` field - this contains the name of the variable the pointer points to
  /// 4. Look up that variable's type
  /// 5. Return the variable's type as the type of the dereferenced pointer
  fn infer_value_at_type(&self, operand: &Expr) -> String {
    // Handle `value at (unsafe bitcast ptr)` pattern: AST is ValueAt -> UnsafeBitcast -> Identifier
    let ptr_expr = if let Expr::Unary(un) = operand {
      if matches!(un.operator, UnaryOp::UnsafeBitcast) {
        &*un.operand
      } else {
        operand
      }
    } else {
      operand
    };

    // Extract the pointer variable name
    if let Expr::Identifier(ptr_ident) = ptr_expr {
      // Look up the pointer's symbol
      if let Some(ptr_symbol) = self
        .symbols
        .lookup_var_symbol(ptr_ident.name())
        .unwrap_or(None)
      {
        // Check if we know what type this pointer points to
        if let Some(source_var_name) = &ptr_symbol.pointer_to_type {
          // Look up the source variable to get its type
          if let Some(source_symbol) = self
            .symbols
            .lookup_var_symbol(source_var_name)
            .unwrap_or(None)
          {
            return source_symbol.data_type.clone();
          }
        }
      }
    }

    "unknown".to_string()
  }

  /// Validate that negative integer literals cannot be assigned to unsigned types.
  /// Examples:
  ///   `var a as u64 = -1` → ERROR: Cannot assign negative literal to unsigned type
  ///   `var a as i64 = -1` → OK: Negative literals allowed for signed types
  /// Validate that literal values fit in the declared type's range.
  /// Covers both unsigned (`u8`, `u16`, etc.) and signed (`i8`, `i16`, etc.) types.
  fn validate_literal_range(
    &mut self,
    declared_type: &str,
    expr: &Expr,
    location: &SourceLocation,
  ) {
    // Extract the numeric literal value.
    // For UintLiteral, we can't fit in i64 — store in u64 and handle separately.
    match expr {
      Expr::UintLiteral(lit) => {
        match declared_type {
          "u64" => (), // UintLiteral always fits u64
          "u8" | "u16" | "u32" => {
            let max: u64 = match declared_type {
              "u8" => u8::MAX as u64,
              "u16" => u16::MAX as u64,
              "u32" => u32::MAX as u64,
              _ => unreachable!(),
            };
            if lit.value > max {
              self.add_error(
                format!(
                  "Literal value {} is out of range for type '{}' (valid range: 0..{})",
                  lit.value, declared_type, max
                ),
                location,
              );
            }
          }
          _ => (), // UintLiteral in signed context — let conversion handle it
        }
      }
      Expr::IntLiteral(lit) => {
        self.validate_int_literal_range(lit.value, declared_type, location);
      }
      Expr::HexLiteral(lit) => {
        let value = match u64::from_str_radix(
          lit.value.trim_start_matches("0x").trim_start_matches("0X"),
          16,
        ) {
          Ok(v) => v,
          Err(_) => {
            ice!(
              "Failed to parse hex literal '{}' in range validation — parser should have validated this",
              lit.value
            );
          }
        };
        // Hex literals are unsigned; validate only the 8-bit `byte`/`u8` range
        // here (wider unsigned types are handled by the general conversion checks).
        match declared_type {
          "byte" | "u8" if value > u8::MAX as u64 => {
            self.add_error(
              format!(
                "Literal value 0x{:X} is out of range for type '{}' (valid range: 0x00..0xFF)",
                value, declared_type
              ),
              location,
            );
          }
          _ => (), // Wider unsigned or signed contexts: let conversion handle it.
        }
      }
      // Negated literals: `-5` now parses as `Unary(Neg, IntLiteral(5))` since `-`
      // is a prefix operator. Validate the negated value against the declared type.
      Expr::Unary(un) if matches!(un.operator, UnaryOp::Neg) => {
        self.validate_negated_literal_range(declared_type, &un.operand, location);
      }
      _ => (),
    }
  }

  /// Validate a negated numeric literal (`-<literal>`) against a declared type.
  fn validate_negated_literal_range(
    &mut self,
    declared_type: &str,
    operand: &Expr,
    location: &SourceLocation,
  ) {
    match operand {
      Expr::IntLiteral(lit) => match lit.value.checked_neg() {
        Some(negated) => self.validate_int_literal_range(negated, declared_type, location),
        None => self.add_error(
          format!(
            "Negated literal -{} is out of range for type '{}'",
            lit.value, declared_type
          ),
          location,
        ),
      },
      Expr::UintLiteral(lit) => {
        // `-<u64 literal>` is negative. It fits in i64 only when the magnitude is
        // at most 2^63 (yielding i64::MIN); larger magnitudes overflow every type.
        let max_negatable = (i64::MAX as u64) + 1; // 2^63
        if lit.value <= max_negatable {
          let negated = -(lit.value as i128);
          self.validate_int_literal_range(negated as i64, declared_type, location);
        } else {
          self.add_error(
            format!(
              "Negated literal -{} is out of range for type '{}'",
              lit.value, declared_type
            ),
            location,
          );
        }
      }
      // Negated float literals have no integer range concern: float→float never
      // overflows the representable range, and float→int is rejected by the type
      // checker (a float literal may only narrow to another float type).
      Expr::FloatLiteral(_) => {}
      _ => {}
    }
  }

  fn validate_int_literal_range(
    &mut self,
    value: i64,
    declared_type: &str,
    location: &SourceLocation,
  ) {
    match declared_type {
      "u8" => {
        if value < 0 {
          self.add_error(
            format!("Cannot assign negative integer {} to unsigned type 'u8'. Use a signed type like i8 instead.", value),
            location,
          );
        } else if value > u8::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'u8' (valid range: 0..{})",
              value,
              u8::MAX
            ),
            location,
          );
        }
      }
      "u16" => {
        if value < 0 {
          self.add_error(
            format!("Cannot assign negative integer {} to unsigned type 'u16'. Use a signed type like i16 instead.", value),
            location,
          );
        } else if value > u16::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'u16' (valid range: 0..{})",
              value,
              u16::MAX
            ),
            location,
          );
        }
      }
      "u32" => {
        if value < 0 {
          self.add_error(
            format!("Cannot assign negative integer {} to unsigned type 'u32'. Use a signed type like i32 instead.", value),
            location,
          );
        } else if value > u32::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'u32' (valid range: 0..{})",
              value,
              u32::MAX
            ),
            location,
          );
        }
      }
      "u64" => {
        if value < 0 {
          self.add_error(
            format!("Cannot assign negative integer {} to unsigned type 'u64'. Use a signed type like i64 instead.", value),
            location,
          );
        }
        // Positive IntLiteral always fits in u64 (i64::MAX < u64::MAX).
        // UintLiteral always fits in u64 by definition.
      }
      "i8" => {
        if value < i8::MIN as i64 || value > i8::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'i8' (valid range: {}..{})",
              value,
              i8::MIN,
              i8::MAX
            ),
            location,
          );
        }
      }
      "i16" => {
        if value < i16::MIN as i64 || value > i16::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'i16' (valid range: {}..{})",
              value,
              i16::MIN,
              i16::MAX
            ),
            location,
          );
        }
      }
      "i32" => {
        if value < i32::MIN as i64 || value > i32::MAX as i64 {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'i32' (valid range: {}..{})",
              value,
              i32::MIN,
              i32::MAX
            ),
            location,
          );
        }
      }
      "i64" => {
        // i64 is the default literal type — always fits
      }
      _ => {}
    }
  }

  /// Validate that every expression in the entire AST has a concrete type.
  /// This is a pipeline safety net: any "unknown" type means the analyzer failed
  /// to infer a type, and the compiler must abort before IR generation can propagate
  /// the "unknown" into debug output or code generation.
  pub fn validate_all_expression_types(&mut self, program: &Program) {
    for stmt in &program.statements {
      self.validate_expr_types_in_stmt(stmt);
    }
  }

  /// Recursively check all expressions within a statement.
  fn validate_expr_types_in_stmt(&mut self, stmt: &Stmt) {
    match stmt {
      Stmt::VarDef(def) => {
        self.validate_expr_type(&def.value, &def.location);
      }
      Stmt::Assign(assign) => {
        self.validate_expr_type(&assign.value, &assign.location);
      }
      Stmt::CompoundAssign(ca) => {
        self.validate_expr_type(&ca.value, &ca.location);
      }
      Stmt::If(if_stmt) => {
        self.validate_expr_type(&if_stmt.condition.expr, &if_stmt.location);
        for s in &if_stmt.then_branch {
          self.validate_expr_types_in_stmt(s);
        }
        for (cond, body) in &if_stmt.else_if_branches {
          self.validate_expr_type(&cond.expr, &cond.location);
          for s in body {
            self.validate_expr_types_in_stmt(s);
          }
        }
        if let Some(else_body) = &if_stmt.else_branch {
          for s in else_body {
            self.validate_expr_types_in_stmt(s);
          }
        }
      }
      Stmt::When(when_stmt) => {
        self.validate_expr_type(&when_stmt.condition.expr, &when_stmt.location);
        for s in &when_stmt.body {
          self.validate_expr_types_in_stmt(s);
        }
        if let Some(else_body) = &when_stmt.else_branch {
          for s in else_body {
            self.validate_expr_types_in_stmt(s);
          }
        }
      }
      Stmt::MoveOn(_) | Stmt::MissingCode(_) => {
        // No-op statements: nothing to validate
      }
      Stmt::Loop(loop_stmt) => {
        if let Some(cond) = &loop_stmt.pre_condition {
          self.validate_expr_type(&cond.expr, &loop_stmt.location);
        }
        if let Some(cond) = &loop_stmt.post_condition {
          self.validate_expr_type(&cond.expr, &loop_stmt.location);
        }
        for s in &loop_stmt.body {
          self.validate_expr_types_in_stmt(s);
        }
      }
      Stmt::Return(ret) => {
        if let Some(val) = &ret.value {
          self.validate_expr_type(val, &ret.location);
        }
      }
      Stmt::FnCall(fc) => {
        for arg in &fc.arguments {
          self.validate_expr_type(arg, &fc.location);
        }
      }
      Stmt::Stdout(out) => {
        self.validate_expr_type(&out.value, &out.location);
      }
      Stmt::Stderr(err) => {
        self.validate_expr_type(&err.value, &err.location);
      }
      Stmt::Log(log) => {
        self.validate_expr_type(&log.value, &log.location);
      }
      Stmt::Debug(debug) => {
        self.validate_expr_type(&debug.value, &debug.location);
      }
      Stmt::Assert(assert) => {
        self.validate_expr_type(&assert.condition, &assert.location);
      }
      Stmt::Alert(alert) => {
        self.validate_expr_type(&alert.value, &alert.location);
      }
      Stmt::Release(release) => {
        self.validate_expr_type(&release.pointer, &release.location);
      }
      Stmt::OnExit(on_exit) => {
        self.validate_expr_types_in_stmt(&on_exit.body);
      }
      Stmt::ValueAtAssign(va) => {
        self.validate_expr_type(&va.value, &va.location);
      }
      Stmt::FnDef(_fn_def) => {
        // Function bodies are already fully validated by the main analyzer.
        // Skip recursive walk to avoid false positives for function parameters
        // (whose types require the fn_info_cache, not just the symbol table).
      }
      Stmt::AddError(ae) => {
        self.validate_expr_type(&ae.value, &ae.location);
      }
      Stmt::Switch(switch_stmt) => {
        self.validate_expr_type(&switch_stmt.value, &switch_stmt.location);
        for case in &switch_stmt.cases {
          for s in &case.body {
            self.validate_expr_types_in_stmt(s);
          }
        }
        if let Some(default_body) = &switch_stmt.default_case {
          for s in default_body {
            self.validate_expr_types_in_stmt(s);
          }
        }
      }
      // Stmts with no expressions to validate:
      Stmt::Use(_)
      | Stmt::TypeDef(_)
      | Stmt::EnumDef(_)
      | Stmt::UnsafeDecl(_)
      | Stmt::FnSignature(_)
      | Stmt::ExitLoop(_)
      | Stmt::ExitProgram(_)
      | Stmt::Rewind(_)
      | Stmt::Stdin(_)
      | Stmt::CtIf(_)
      | Stmt::CtFail(_)
      | Stmt::CtWarn(_)
      | Stmt::CtWhen(_)
      | Stmt::CtMatch(_)
      | Stmt::CtSwitch(_)
      | Stmt::Doc(_)
      | Stmt::Comment(_)
      | Stmt::AlertErrors(_)
      | Stmt::Match(_)
      | Stmt::TestSuite(_) => {}
    }
  }

  /// Check a single expression and its sub-expressions for "unknown" type.
  /// Only validates identifiers and function calls — expression nodes like Binary/Unary
  /// derive their type from operands and may legitimately return "unknown" without context.
  fn validate_expr_type(&mut self, expr: &Expr, location: &SourceLocation) {
    // Recursively check sub-expressions first
    match expr {
      Expr::Binary(bin) => {
        self.validate_expr_type(&bin.left, location);
        self.validate_expr_type(&bin.right, location);
      }
      Expr::Unary(un) => {
        self.validate_expr_type(&un.operand, location);
      }
      Expr::FnCallExpr(fc) => {
        for arg in &fc.arguments {
          self.validate_expr_type(arg, location);
        }
      }
      Expr::Identifier(id) => {
        // Identifiers must resolve to a known type
        let id_type = self.expr_type(expr);
        if id_type == "unknown" {
          self.add_error(
            format!(
              "Type inference failed: '{}' is not defined or its type cannot be determined. \
               Check that the variable or function is declared.",
              id.name()
            ),
            location,
          );
        }
      }
      Expr::ArrayLiteral(arr) => {
        for elem in &arr.elements {
          self.validate_expr_type(elem, location);
        }
        if let Some(val_expr) = &arr.fill {
          self.validate_expr_type(val_expr, location);
        }
      }
      Expr::ArrayIndex(ai) => {
        for idx in &ai.indices {
          self.validate_expr_type(idx, location);
        }
      }
      Expr::MemberAccess(ma) => {
        // Enum-variant access (`Shape.Point`): the object is a type name, not a
        // value, so skip validating it as a variable.
        if self
          .enum_variant_access_type(&ma.object, &ma.member.node)
          .is_none()
        {
          self.validate_expr_type(&ma.object, location);
        }
      }
      Expr::Grouped(inner) => {
        self.validate_expr_type(inner, location);
      }
      Expr::Conversion(conv) => {
        self.validate_expr_type(&conv.operand, location);
      }
      Expr::StringLiteral(sl) => {
        for part in &sl.parts {
          if let StringPart::EmbeddedValue(expr_inner) = part {
            self.validate_expr_type(expr_inner, location);
          }
        }
      }
      // Bare literals and compiler constants are always valid
      _ => {}
    }
  }

  /// Infer the type of an expression.
  pub fn expr_type(&mut self, expr: &Expr) -> String {
    self.expr_type_with_context(expr, None)
  }

  /// Type inference with optional context (declared type).
  /// When context is provided, standalone literals will infer their type from context.
  /// When context is None, literals return "unknown" to force explicit casting in expressions.
  pub fn expr_type_with_context(&mut self, expr: &Expr, context: Option<&str>) -> String {
    match expr {
      Expr::Binary(bin) => {
        let left_type = self.expr_type_with_context(&bin.left, context);
        match bin.operator {
          BinaryOp::Eq
          | BinaryOp::NotEq
          | BinaryOp::Lt
          | BinaryOp::LtEq
          | BinaryOp::Gt
          | BinaryOp::GtEq
          | BinaryOp::And
          | BinaryOp::Or
          | BinaryOp::Xor => "bool".to_string(),
          BinaryOp::Add | BinaryOp::Sub => {
            // Pointer arithmetic special cases
            if left_type.to_lowercase() == "pointer" {
              match bin.operator {
                BinaryOp::Add => {
                  // ptr + u64 → pointer
                  "pointer".to_string()
                }
                BinaryOp::Sub => {
                  // ptr - u64 → pointer, ptr - ptr → i64
                  let right_type = self.expr_type_with_context(&bin.right, None);
                  if right_type.to_lowercase() == "pointer" {
                    "i64".to_string() // ptr - ptr returns signed i64
                  } else {
                    "pointer".to_string() // ptr - u64 returns pointer
                  }
                }
                _ => unreachable!(),
              }
            } else {
              left_type
            }
          }
          _ => {
            // Dot product: vecN<T> · vecN<T> → T, scalar · scalar → scalar
            if bin.operator == BinaryOp::Dot {
              // Return the inner type (scalar result)
              if let Some(inner) = left_type.find('<')
                && left_type.ends_with('>')
              {
                return left_type[inner + 1..left_type.len() - 1].to_string();
              }
              // Scalar dot: return left_type (scalar × scalar = scalar)
              return left_type;
            }
            // Cross product: vec3<T> ⨯ vec3<T> → vec3<T>
            if bin.operator == BinaryOp::Cross {
              return left_type; // result is same vec3 type
            }
            left_type
          }
        }
      }
      Expr::Unary(un) => match un.operator {
        UnaryOp::Not => "bool".to_string(),
        UnaryOp::TypeOf | UnaryOp::UnitOf => "text".to_string(),
        UnaryOp::SizeOf | UnaryOp::CountOf => "u64".to_string(),
        UnaryOp::PointerTo => "pointer".to_string(),
        UnaryOp::ValueAt => self.infer_value_at_type(&un.operand),
        UnaryOp::ValueOf => {
          // `value of expr` strips the optional wrapper:
          //   value of (T?) → T
          //   value of (T) → T (error reported in visit_unary)
          let inner_type = self.expr_type_with_context(&un.operand, context);
          if TypeInference::is_optional_type(&inner_type) {
            TypeInference::optional_inner_type(&inner_type).to_string()
          } else {
            // Non-optional value_of is an error, but we'll report it in visit_unary
            // which has mutable self access. Return inner type for now.
            inner_type
          }
        }
        _ => self.expr_type_with_context(&un.operand, context),
      },
      Expr::Identifier(id) if id.is_pi() => {
        // `π` is a reserved built-in constant, folded to a context-typed float
        // literal: `f16`/`f32`/`f64` from context, otherwise `f64`.
        match context {
          Some(ctx) if matches!(ctx, "f16" | "f32" | "f64") => ctx.to_string(),
          _ => "f64".to_string(),
        }
      }
      Expr::Identifier(id) => {
        let name = id.name();
        self
          .symbols
          .lookup_var_type(name)
          .or_else(|| {
            // Check if this is a zero-argument function (e.g., enum variant, or a
            // suite function referenced as an identifier).
            let resolved = self.resolve_fn_name(name);
            self.symbols.lookup_function(&resolved).and_then(|f| {
              if f.parameters.is_empty() {
                // Return type is stored in the database, not in FnInfo
                self
                  .symbols
                  .lookup_function_return_type(&resolved)
                  .unwrap_or(None)
              } else {
                None
              }
            })
          })
          .unwrap_or_else(|| "unknown".to_string())
      }
      Expr::IntLiteral(_) => {
        // Signed literals infer type from context ONLY if context is numeric, otherwise unknown
        if let Some(ctx) = context {
          // Only accept numeric contexts for numeric literals
          if matches!(
            ctx,
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
          ) {
            ctx.to_string()
          } else {
            "unknown".to_string()
          }
        } else {
          "unknown".to_string()
        }
      }
      Expr::FloatLiteral(_) => {
        // Float literals default to f64 when no context is provided.
        // With context, they only infer to other float types (f16/f32/f64).
        if let Some(ctx) = context {
          if matches!(ctx, "f16" | "f32" | "f64") {
            ctx.to_string()
          } else {
            "f64".to_string()
          }
        } else {
          "f64".to_string()
        }
      }
      Expr::UintLiteral(_) => {
        // Unsigned literals infer type from context ONLY if context is numeric or pointer, otherwise unknown
        if let Some(ctx) = context {
          // Only accept numeric or pointer contexts for unsigned literals
          if matches!(
            ctx,
            "i8"
              | "i16"
              | "i32"
              | "i64"
              | "u8"
              | "u16"
              | "u32"
              | "u64"
              | "f16"
              | "f32"
              | "f64"
              | "pointer"
          ) {
            ctx.to_string()
          } else {
            "unknown".to_string()
          }
        } else {
          "unknown".to_string()
        }
      }
      Expr::HexLiteral(_) => {
        // Hex literals are unsigned, infer from context ONLY if context is numeric or byte, otherwise unknown
        if let Some(ctx) = context {
          // Only accept numeric or byte contexts for numeric literals
          if matches!(
            ctx,
            "i8"
              | "i16"
              | "i32"
              | "i64"
              | "u8"
              | "u16"
              | "u32"
              | "u64"
              | "f16"
              | "f32"
              | "f64"
              | "byte"
          ) {
            ctx.to_string()
          } else {
            "unknown".to_string()
          }
        } else {
          "unknown".to_string()
        }
      }
      Expr::CharLiteral(_) => "char".to_string(),
      Expr::BoolLiteral(_) => "bool".to_string(),
      Expr::StringLiteral(_) => "text".to_string(),
      Expr::Allocate(_) => "pointer".to_string(),
      Expr::ArrayLiteral(arr) => {
        // Handle fill syntax: [fill with value]
        // For the new syntax, the array type must come from the declaration
        // The literal itself doesn't determine the dimensions
        if arr.fill.is_some() {
          // Can't infer type from [fill with value] alone
          // Type must come from the annotation
          "[]".to_string()
        } else if let Some(first) = arr.elements.first() {
          // Extract base type from context if provided (remove any array dimensions)
          let base_context = context.map(|ctx| {
            // If context has array dimensions (e.g., "u32[5]"), extract just the base ("u32")
            if let Some(bracket_pos) = ctx.find('[') {
              &ctx[..bracket_pos]
            } else {
              ctx
            }
          });

          let element_type = self.expr_type_with_context(first, base_context);
          let count = arr.elements.len();
          // For nested arrays, dimensions should be outermost first
          // E.g., [[a,b,c,d], [...], [...]] with 3 elements of type f64[4]
          // should be f64[3][4], not f64[4][3]
          if element_type.contains('[') {
            // Element type already has dimensions (nested array)
            // Insert the count as the outermost dimension
            let base = element_type.split('[').next().unwrap_or("");
            let remaining_dims = &element_type[base.len()..];
            format!("{}[{}]{}", base, count, remaining_dims)
          } else {
            // Simple type, just append the dimension
            format!("{}[{}]", element_type, count)
          }
        } else {
          "[]".to_string()
        }
      }
      Expr::FnCallExpr(fn_call) => {
        let fn_name = fn_call.target.node.last().map(|s| s.as_str()).unwrap_or("");
        // First check if it's the built-in text constructor
        if fn_name == "text" {
          return "text".to_string();
        }
        // Check if it's the built-in binary constructor
        if fn_name == "binary" {
          return "binary".to_string();
        }
        // Check if it's a built-in vector constructor — infer inner type from first argument
        if fn_name == "vec2" || fn_name == "vec3" || fn_name == "vec4" {
          let inner = fn_call
            .arguments
            .first()
            .map(|arg| self.expr_type_with_context(arg, context))
            .unwrap_or_else(|| {
              // Should not reach here: visit_fn_call_expr already rejects wrong argument counts
              "f64".to_string()
            });
          return format!("{}<{}>", fn_name, inner);
        }
        // Check if it's a type constructor
        if let Some(type_def) = self.symbols.lookup_type(fn_name) {
          return type_def.name.clone();
        }
        // Otherwise look up as a function (resolving a suite function name first).
        let resolved_fn_name = self.resolve_fn_name(fn_name);
        self
          .symbols
          .get_all_symbol_tables()
          .unwrap_or_default()
          .get(&VarScope::Global)
          .and_then(|t| t.get(&resolved_fn_name))
          .map(|sym| sym.data_type.clone())
          .unwrap_or_else(|| "unknown".to_string())
      }
      Expr::MemberAccess(acc) => {
        // Enum-variant access (`Shape.Point`): return the enum type directly.
        if let Some(enum_type) = self.enum_variant_access_type(&acc.object, &acc.member.node) {
          return enum_type;
        }
        // Module-qualified access (`math.e`): resolve the exported symbol's type.
        if let Some(member_type) = self.module_member_access_type(&acc.object, &acc.member.node) {
          return member_type;
        }
        let object_type = self.expr_type_with_context(&acc.object, context);
        // Special case for `text`: its `ptr`/`bytes`/`chars` fields are defined
        // in builtins.lale and may not be resolvable through the type registry
        // in every analysis path, so resolve them directly.
        if object_type == "text" {
          match acc.member.node.as_str() {
            "ptr" => "pointer".to_string(),
            "bytes" => "u64".to_string(),
            "chars" => "u64".to_string(),
            _ => "unknown".to_string(),
          }
        } else if object_type == "binary" {
          match acc.member.node.as_str() {
            "ptr" => "pointer".to_string(),
            "bytes" => "u64".to_string(),
            _ => "unknown".to_string(),
          }
        } else if let Some(type_def) = self.symbols.lookup_type(&object_type) {
          // Look up the field type in the type definition.
          type_def
            .fields
            .iter()
            .find(|(name, _, _, _)| name == &acc.member.node)
            .map(|(_, ty, _, _)| ty.clone())
            .unwrap_or_else(|| {
              self.add_error(
                format!(
                  "Field '{}' not found in type '{}'",
                  acc.member.node, object_type
                ),
                &acc.location,
              );
              "unknown".to_string()
            })
        } else {
          "unknown".to_string()
        }
      }
      Expr::ArrayIndex(idx) => {
        let mut arr_type = self.expr_type_with_context(&idx.array, context);
        // Remove N dimensions where N = number of indices
        // E.g., f64[3][4] with 1 index → f64[4], with 2 indices → f64
        for _ in 0..idx.indices.len() {
          if let Some(last_bracket) = arr_type.rfind('[') {
            arr_type = arr_type[..last_bracket].to_string();
          } else {
            break;
          }
        }
        arr_type
      }
      Expr::CompilerConst(_) => "text".to_string(),
      Expr::Grouped(inner) => self.expr_type_with_context(inner, context),
      Expr::Conversion(conv) => TypeInference::type_name_to_string(&conv.target_type),
      Expr::HasValue(_) => "bool".to_string(),
      Expr::HasNoValue(_) => "bool".to_string(),
      Expr::NothingExpr => {
        // `nothing` has type T? where T is determined by context.
        if let Some(ctx) = context
          && TypeInference::is_optional_type(ctx)
        {
          ctx.to_string()
        } else if let Some(ctx) = context
          && ctx == "nothing"
        {
          "nothing".to_string()
        } else {
          "unknown".to_string()
        }
      }
      Expr::HasErrors => "bool".to_string(),
      Expr::LastError => "text".to_string(),
      Expr::TryPropagate(inner) => {
        let inner_type = self.expr_type_with_context(inner, context);
        TypeInference::optional_inner_type(&inner_type).to_string()
      }
    }
  }

  /// Compute the resulting unit of an expression.
  pub fn expr_unit(&mut self, expr: &Expr) -> ExprUnit {
    let mut unit_errors = Vec::new();
    let mut computer = UnitComputer::new(self.symbols, &mut unit_errors);
    let unit = computer.expr_unit(expr);

    // Add collected unit errors to the analyzer's error list
    self.errors.extend(unit_errors);

    unit
  }

  /// Check that argument units match declared parameter units.
  fn check_fn_call_param_units(&mut self, fn_call: &FnCall) {
    let fn_name = fn_call.target.node.last().map(|s| s.as_str()).unwrap_or("");

    // Check if it's the built-in text constructor (no units to check)
    if fn_name == "text" || fn_name == "binary" {
      return; // Built-in text/binary constructors don't have parameter units
    }

    // Check if it's a built-in vector constructor (vec2/vec3/vec4)
    if fn_name == "vec2" || fn_name == "vec3" || fn_name == "vec4" {
      return; // Built-in vector constructors handled by IR gen
    }

    // Check if it's a type constructor — validate field units against argument units
    if let Some(type_info) = self.symbols.lookup_type(fn_name) {
      if fn_call.arguments.len() != type_info.fields.len() {
        self.add_error(
          format!(
            "Type '{}' expects {} argument(s) but got {}",
            fn_name,
            type_info.fields.len(),
            fn_call.arguments.len()
          ),
          &fn_call.location,
        );
        return;
      }

      for (i, ((field_name, _field_type, field_unit, _is_private), arg)) in type_info
        .fields
        .iter()
        .zip(fn_call.arguments.iter())
        .enumerate()
      {
        if let Some(declared_unit) = field_unit {
          let declared = ExprUnit::from_string(declared_unit);
          let arg_unit = self.expr_unit(arg);

          if !ExprUnit::same_unit(&declared, &arg_unit) {
            self.add_error(
              format!(
                "Argument {} unit mismatch for field '{}': expected {} but got {}",
                i + 1,
                field_name,
                declared.display(),
                arg_unit.display(),
              ),
              &fn_call.location,
            );
          }
        }
      }
      return;
    }

    let fn_info = match self.symbols.lookup_function(&self.resolve_fn_name(fn_name)) {
      Some(info) => info.clone(),
      None => {
        // Report error for undefined function (not a type constructor or built-in str)
        self.add_error(
          format!("Function '{}' is not defined", fn_name),
          &fn_call.location,
        );
        return;
      }
    };

    if fn_call.arguments.len() != fn_info.parameters.len() {
      self.add_error(
        format!(
          "Function '{}' expects {} argument(s) but got {}",
          fn_name,
          fn_info.parameters.len(),
          fn_call.arguments.len()
        ),
        &fn_call.location,
      );
      return;
    }

    for (i, (param, arg)) in fn_info
      .parameters
      .iter()
      .zip(fn_call.arguments.iter())
      .enumerate()
    {
      if let Some(ref declared_unit) = param.unit {
        let declared = ExprUnit::from_string(&declared_unit.raw);
        let arg_unit = self.expr_unit(arg);

        if !ExprUnit::same_unit(&declared, &arg_unit) {
          self.add_error(
            format!(
              "Argument {} unit mismatch for parameter '{}': expected {} but got {}",
              i + 1,
              param.name.node,
              declared.display(),
              arg_unit.display(),
            ),
            &fn_call.location,
          );
        }
      }
    }
  }

  /// Invalidate any variables passed by `ref` to a function call.
  ///
  /// A `ref` parameter is a mutable view of the caller's variable: the callee
  /// may write through it, so after the call the caller's variable can no longer
  /// be assumed to hold its previous value. For div-zero analysis we therefore
  /// mark the root name of each `ref` argument as shared. `lvalue_root_name`
  /// also unwraps `arr[i]` / `p.field` so that a `ref` parameter taking an
  /// element/field invalidates the whole aggregate.
  fn invalidate_ref_passed_vars(&mut self, fn_call: &FnCall) {
    let Some(fn_name) = fn_call.target.node.last() else {
      return;
    };
    let Some(fn_info) = self.symbols.lookup_function(&self.resolve_fn_name(fn_name)) else {
      return;
    };
    for (param, arg) in fn_info.parameters.iter().zip(fn_call.arguments.iter()) {
      if param.pass_mode.is_ref()
        && let Some(name) = lvalue_root_name(arg)
      {
        self.symbols.mark_variable_shared(&name);
      }
    }
  }

  /// Check if expression is escaping a pointer to a local variable
  /// and report an error if so. Used for return statements.
  fn check_escaping_pointer_in_return(&mut self, expr: &Expr, location: &SourceLocation) {
    {
      let mut checker =
        MemorySafetyChecker::new(self.symbols, &mut self.errors, &mut self.warnings);
      checker.check_escaping_pointer_in_return(expr, location);
    }

    // Extend the syntactic check to pointer variables that were assigned a
    // literal `pointer to <local>` earlier in this function.
    if let Expr::Identifier(id) = expr
      && let Some(source) = self.local_pointer_vars.get(id.name()).cloned()
    {
      self.add_error(
        format!(
          "Cannot return pointer to local variable '{}' (via pointer '{}'): reference would escape function scope",
          source,
          id.name()
        ),
        location,
      );
    }
  }

  /// Check if assigning to a global variable with a pointer to local.
  fn check_escaping_pointer_to_global(
    &mut self,
    expr: &Expr,
    target_name: &str,
    location: &SourceLocation,
  ) {
    {
      let mut checker =
        MemorySafetyChecker::new(self.symbols, &mut self.errors, &mut self.warnings);
      checker.check_escaping_pointer_to_global(expr, target_name, location);
    }

    // Extend to pointer variables that hold a literal `pointer to <local>`.
    if let Expr::Identifier(id) = expr
      && let Some(source) = self.local_pointer_vars.get(id.name()).cloned()
      && self
        .symbols
        .get_symbol_table(VarScope::Global)
        .contains_key(target_name)
    {
      self.add_error(
        format!(
          "Cannot assign pointer to local variable '{}' (via pointer '{}') to global '{}': reference would escape function scope",
          source,
          id.name(),
          target_name
        ),
        location,
      );
    }
  }

  /// Helper: Register variable declaration (safe or unsafe).
  #[allow(clippy::too_many_arguments)]
  fn register_variable_decl(
    &mut self,
    name: &str,
    location: &SourceLocation,
    type_annotation: Option<&TypeName>,
    unit: Option<String>,
    is_export: bool,
    is_import: bool,
    is_initialized: bool,
  ) {
    self.register_variable_decl_with_pointer(
      name,
      location,
      type_annotation,
      unit,
      is_export,
      is_import,
      is_initialized,
      None,
    );
  }

  #[allow(clippy::too_many_arguments)]
  fn register_variable_decl_with_pointer(
    &mut self,
    name: &str,
    location: &SourceLocation,
    type_annotation: Option<&TypeName>,
    unit: Option<String>,
    is_export: bool,
    is_import: bool,
    is_initialized: bool,
    pointer_to_type: Option<String>,
  ) {
    // Reject definitions of the reserved constant `π` (variables, loop
    // variables, and switch-arm bindings all register through this path).
    if self.reject_reserved_pi(name, location) {
      return;
    }

    // Use the version with dimension info to preserve array bounds for compile-time checking
    let type_str = type_annotation
      .map(TypeInference::type_name_to_string_with_dimensions)
      .unwrap_or_default();

    let linkage = if is_export {
      Linkage::Export
    } else if is_import {
      Linkage::Import
    } else {
      Linkage::Internal
    };

    // Track defined variables for unused detection (only if not exported/imported)
    if !is_export && !is_import {
      self.mark_symbol_defined(name, location.clone(), false);
    }

    if let Err(err_msg) = self.symbols.define_variable_with_pointer_type(
      self.symbols.current_scope().clone(),
      name,
      location,
      &type_str,
      unit,
      linkage,
      is_initialized,
      pointer_to_type,
    ) {
      self.add_error(err_msg, location);
    }
  }

  /// Convert a type string (e.g., "pointer", "i64", "u32") to a TypeName AST node.
  /// Used when inferring types for variables without explicit type annotations.
  fn type_string_to_type_name(type_str: &str) -> TypeName {
    let base_type = match type_str {
      "pointer" => BaseType::Pointer,
      "i8" => BaseType::I8,
      "i16" => BaseType::I16,
      "i32" => BaseType::I32,
      "i64" => BaseType::I64,
      "u8" => BaseType::U8,
      "u16" => BaseType::U16,
      "u32" => BaseType::U32,
      "u64" => BaseType::U64,
      "f16" => BaseType::F16,
      "f32" => BaseType::F32,
      "f64" => BaseType::F64,
      "text" => BaseType::Text,
      "char" => BaseType::Char,
      "bool" => BaseType::Bool,
      _ => BaseType::Custom(type_str.to_string()), // Custom type names (structs, enums, etc.)
    };

    TypeName {
      base_type,
      inner_type: None,
      array_dimensions: vec![], // Inferred types don't track array dimensions yet
      is_optional: false,
      location: SourceLocation {
        line: 0,
        col: 0,
        start_pos: 0,
        end_pos: 0,
        source_file: String::new(),
      },
    }
  }

  /// Extract the source type from a `pointer to` expression.
  /// Returns Some(type_name) if the expression is a `pointer to` operation, None otherwise.
  fn extract_pointer_source_type(expr: &Expr) -> Option<String> {
    match expr {
      Expr::Unary(unary) => {
        match unary.operator {
          UnaryOp::PointerTo => {
            // The operand tells us what type the pointer points to
            match &*unary.operand {
              Expr::Identifier(ident) => Some(ident.name().to_string()),
              _ => None,
            }
          }
          _ => None,
        }
      }
      _ => None,
    }
  }

  /// Returns Some(local_var_name) if `expr` is a literal `pointer to <local>`
  /// and the pointed-to variable is a function-local. Returns None otherwise.
  fn pointer_to_local_source(&self, expr: &Expr) -> Option<String> {
    let name = Self::extract_pointer_source_type(expr)?;
    if self.symbols.is_local_variable(&name) {
      Some(name)
    } else {
      None
    }
  }

  /// Extract the bare pointer variable name from a `value at` operand, handling
  /// the `value at (unsafe bitcast p)` AST shape. Returns None for non-identifier
  /// operands (e.g. pointer arithmetic), which are out of scope for this check.
  fn pointer_identifier_name(expr: &Expr) -> Option<&str> {
    match expr {
      Expr::Identifier(id) => Some(id.name()),
      Expr::Unary(inner) if matches!(inner.operator, UnaryOp::UnsafeBitcast) => {
        match &*inner.operand {
          Expr::Identifier(id) => Some(id.name()),
          _ => None,
        }
      }
      _ => None,
    }
  }

  /// Validate `value at ptr unsafe bitcast` for bit-width compatibility and unitless source.
  ///
  /// Due to Pratt parser precedence, the AST structure is:
  ///   ValueAt -> UnsafeBitcast -> Identifier
  /// (i.e., `value at` is outer, `unsafe bitcast` is inner, applied to the pointer identifier)
  ///
  /// Checks that:
  /// 1. The pointer source has no physical unit (bit reinterpretation is meaningless for dimensional values)
  /// 2. The pointer source type (if known) has the same bit width as the target type
  fn validate_unsafe_bitcast_compatibility(
    &mut self,
    target_type: &str,
    value_at_expr: &Expr,
    location: &SourceLocation,
  ) {
    let mut checker = MemorySafetyChecker::new(self.symbols, &mut self.errors, &mut self.warnings);
    checker.validate_unsafe_bitcast_compatibility(target_type, value_at_expr, location);
  }

  /// Analyze `unsafe bitcast expr as T` — an explicit numeric bit reinterpretation.
  ///
  /// This validates bit-width compatibility and a unitless source, then visits
  /// the inner operand directly (not through `visit_conversion`, which would
  /// reject a lossy signed↔unsigned conversion that `unsafe bitcast` makes explicit).
  fn visit_unsafe_bitcast_conversion(&mut self, conv: &ConversionExpr, location: &SourceLocation) {
    self.visit_type_name(&conv.target_type);
    self.visit_expr(&conv.operand);

    let source_type = self.expr_type(&conv.operand);
    let target_type = TypeInference::type_name_to_string(&conv.target_type);

    // Bit reinterpretation is meaningless for dimensional values.
    let source_unit = self.expr_unit(&conv.operand);
    if !source_unit.is_unitless() && !source_unit.is_unknown() {
      self.add_error(
        format!(
          "Unsafe bitcast requires unitless source: expression has unit '{}'. \
           Bit reinterpretation is meaningless for dimensional values. \
           Assign to a unitless variable first.",
          source_unit.display()
        ),
        location,
      );
    }

    // Bit-width compatibility (still checked even if the unit error fired).
    let source_bits = TypeValidator::get_bit_width(&source_type);
    let target_bits = TypeValidator::get_bit_width(&target_type);
    if let (Some(s), Some(t)) = (source_bits, target_bits)
      && s != t
    {
      self.add_error(
        format!(
          "Unsafe bitcast bit-width mismatch: cannot reinterpret '{}' ({}-bit) as '{}' ({}-bit)",
          source_type, s, target_type, t
        ),
        location,
      );
    }
  }

  /// Validate array initialization.
  ///
  /// Rules:
  /// 1. Partial initialization is not allowed: [1, 2, 3] for i32[10] is an error
  /// 2. Fill syntax requires exact dimension match: [fill with 5] for i32[10]
  /// 3. Element type of fill value must be compatible with array element type
  fn validate_array_initialization(
    &mut self,
    declared_type: &TypeName,
    value: &Expr,
    location: &SourceLocation,
  ) {
    // Only validate if this is an array type
    if declared_type.array_dimensions.is_empty() {
      return;
    }

    // Check if value is an array literal
    if let Expr::ArrayLiteral(arr) = value {
      let declared_type_str = TypeInference::type_name_to_string(declared_type);
      let declared_base = TypeInference::base_type_to_string(&declared_type.base_type);

      // Use context-aware type inference with the declared base type
      let value_type_str = self.expr_type_with_context(value, Some(&declared_base));

      // Handle fill syntax
      if let Some(value_expr) = &arr.fill {
        // For the new [fill with value] syntax, the dimension comes from the type annotation

        // Validate that element type matches with base type context
        let element_type = self.expr_type_with_context(value_expr, Some(&declared_base));

        if !TypeInference::types_compatible(&declared_base, &element_type) {
          // Check if numeric flexibility applies
          let is_element_numeric = matches!(
            element_type.as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
          );
          let is_base_numeric = matches!(
            declared_base.as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
          );

          if !(is_element_numeric && is_base_numeric && TypeChecker::is_numeric_valued(value_expr))
          {
            self.add_error(
              format!(
                "Array element type mismatch in fill: expected element type '{}' but got '{}'",
                declared_base, element_type
              ),
              location,
            );
          }
        }
      } else if !arr.elements.is_empty() {
        // Regular array literal - dimensions must match exactly
        // But elements can have numeric flexibility (e.g., u32 literals in i32[10])
        let declared_dims = TypeInference::extract_array_dimensions(&declared_type_str);
        let value_dims = TypeInference::extract_array_dimensions(&value_type_str);

        // First check dimensions match
        if declared_dims != value_dims {
          self.add_error(
            format!(
              "Array initialization dimensions mismatch: declared type is '{}' but initializer has type '{}'",
              declared_type_str, value_type_str
            ),
            location,
          );
        } else {
          // Dimensions match, check element types with numeric flexibility
          let inferred_full_type = if let Some(first) = arr.elements.first() {
            self.expr_type_with_context(first, Some(&declared_base))
          } else {
            declared_base.clone() // use declared base type if empty
          };

          // Extract base type from inferred type (strip array dimensions)
          let inferred_base = if let Some(bracket_pos) = inferred_full_type.find('[') {
            inferred_full_type[..bracket_pos].to_string()
          } else {
            inferred_full_type.clone()
          };

          if !TypeInference::types_compatible(&declared_base, &inferred_base) {
            // Allow numeric flexibility for literals
            let is_base_numeric = matches!(
              declared_base.as_str(),
              "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
            );
            let is_inferred_numeric = matches!(
              inferred_base.as_str(),
              "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
            );

            if let Some(first) = arr.elements.first() {
              let is_numeric_valued = TypeChecker::is_numeric_valued(first);
              if !(is_base_numeric && is_inferred_numeric && is_numeric_valued) {
                self.add_error(
                  format!(
                    "Array element type mismatch: expected element type '{}' but got '{}'",
                    declared_base, inferred_base
                  ),
                  location,
                );
              }
            }
          }
        }
      }
    }
  }

  /// Helper: Analyze output expression (stdout/stderr/log).
  ///
  /// Output statements require a concrete, self-typing expression. A bare
  /// numeric literal has no type without an explicit cast, so it is rejected
  /// here (mirroring `visit_var_def`) rather than reaching IR generation, where
  /// it would otherwise produce an internal compiler error.
  fn visit_output_expr(&mut self, expr: &Expr, location: &SourceLocation, context: &str) {
    self.visit_expr(expr);
    if is_bare_numeric_literal(expr) {
      self.add_error(
        format!(
          "{} statement cannot infer a type for the literal '{}'. \
           Literals have no type without context; add an explicit conversion such as \
           '{} as <type>'.",
          context,
          ExpressionAnalyzer::expr_to_string(expr),
          ExpressionAnalyzer::expr_to_string(expr)
        ),
        location,
      );
    }
  }

  /// Verify an expression has a concrete type (not unknown/ambiguous).
  /// Reports a semantic error if the type can't be determined.
  fn check_expr_type_is_concrete(&mut self, expr: &Expr, location: &SourceLocation, context: &str) {
    let ty = self.expr_type_with_context(expr, None);
    if ty == "unknown" || ty == "unknown?" || ty == "?" {
      self.add_error(
        format!(
          "{} statement requires a concrete type. The expression '{}' has ambiguous type '{}'. \
           Add an explicit type conversion (e.g., `value as i32`) or type annotation.",
          context,
          crate::semantic_analysis::expression_analysis::ExpressionAnalyzer::expr_to_string(expr),
          ty
        ),
        location,
      );
    }
  }
}

impl<'a> SemanticAnalyzer<'a> {
  /// Import a symbol from another module into the current scope.
  fn import_symbol(&mut self, name: &str, symbol: &Symbol, location: &SourceLocation) {
    // Preserve the original module path from the imported symbol
    let original_module = self.symbols.get_module_path().to_string();
    self.symbols.set_module_path(symbol.module_path.clone());

    if let Err(err_msg) = self.symbols.define_symbol(
      self.symbols.current_scope().clone(),
      name,
      location,
      &symbol.data_type,
      symbol.physical_unit.clone(),
      symbol.kind.clone(),
      Linkage::Import,
      symbol.visibility.clone(),
      symbol.is_definition,
      symbol.is_initialized,
      symbol.pointer_to_type.clone(),
    ) {
      self.add_error(err_msg, location);
    }

    // Restore the original module path
    self.symbols.set_module_path(original_module);
  }

  /// Import one exported symbol, detecting ambiguity when the same name has
  /// already been imported from a *different* module. Re-importing from the same
  /// module is idempotent (a no-op).
  fn import_one(&mut self, name: &str, symbol: &Symbol, location: &SourceLocation) {
    match self.symbols.imported_var_source(name) {
      Ok(Some(src)) if src == symbol.module_path => {
        // Redundant re-import from the same source module — no-op.
      }
      Ok(Some(src)) => {
        self.add_error(
          format!(
            "Symbol '{}' imported multiple times (from '{}' and '{}'). Use a qualified path.",
            name, src, symbol.module_path
          ),
          location,
        );
      }
      Ok(None) => {
        self.import_symbol(name, symbol, location);
      }
      Err(e) => {
        self.add_error(
          format!("Failed to check import of '{}': {}", name, e),
          location,
        );
      }
    }
  }
}

impl<'a> AnalyzerResults for SemanticAnalyzer<'a> {
  fn get_errors(&self) -> &[SemanticError] {
    &self.errors
  }

  fn get_warnings(&self) -> &[SemanticError] {
    &self.warnings
  }

  fn get_symbol_table(&self, scope: VarScope) -> SymbolTable {
    self.symbols.get_symbol_table(scope)
  }

  fn get_all_symbol_tables(&self) -> HashMap<VarScope, SymbolTable> {
    self.symbols.get_all_symbol_tables().unwrap_or_default()
  }

  fn get_defined_symbols(
    &self,
  ) -> &std::collections::HashMap<String, (String, SourceLocation, bool)> {
    &self.all_defined_symbols
  }

  fn get_used_symbols(&self) -> &std::collections::HashSet<String> {
    &self.used_symbols
  }

  fn get_used_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.used_symbol_locations
  }

  fn get_unused_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.unused_symbol_locations
  }

  fn lookup_var_type(&self, name: &str) -> Option<String> {
    self.symbols.lookup_var_type(name)
  }

  fn lookup_function_return_type(&self, name: &str) -> Option<String> {
    self
      .get_all_symbol_tables()
      .get(&VarScope::Global)
      .and_then(|t| t.get(name))
      .map(|sym| sym.data_type.clone())
  }

  fn lookup_function(&self, name: &str) -> Option<FnInfo> {
    SemanticAnalyzer::lookup_function(self, name)
  }

  fn expr_type(&mut self, expr: &Expr) -> String {
    // Delegate to the actual expr_type method
    SemanticAnalyzer::expr_type(self, expr)
  }
}

impl<'a> SemanticAnalyzer<'a> {
  /// Analyze a switch over an enum scrutinee (variant patterns + exhaustiveness).
  fn visit_enum_switch(
    &mut self,
    switch_stmt: &SwitchStmt,
    enum_type: &str,
    all_variants: Vec<(String, String)>,
  ) {
    let all_variant_names: Vec<&str> = all_variants.iter().map(|(_, name)| name.as_str()).collect();

    // Track which variants are covered for exhaustiveness checking
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track which variables are bound in which cases (for post-switch access check)
    let mut var_case_counts: std::collections::HashMap<String, usize> =
      std::collections::HashMap::new();
    // Track the expected type of each variable (None if types differ across cases)
    let mut var_types: std::collections::HashMap<String, Option<String>> =
      std::collections::HashMap::new();
    // Branch definitions for definite-assignment across case/default bodies.
    let mut switch_branch_defs: Vec<BranchDefs> = Vec::new();
    self.enter_construct();

    for case in &switch_stmt.cases {
      let SwitchPattern::Enum {
        variant_name,
        fields,
        ..
      } = &case.pattern
      else {
        self.add_error(
          format!(
            "Cannot use a literal pattern when switching on enum '{}'; use a variant pattern",
            enum_type
          ),
          case.pattern.location(),
        );
        for stmt in &case.body {
          self.visit_stmt(stmt);
        }
        continue;
      };

      let variant_type_name = format!("{}__{}", enum_type, variant_name.node);

      if let Some(variant_info) = self.symbols.lookup_type(&variant_type_name) {
        if !covered.insert(variant_name.node.clone()) {
          self.add_error(
            format!("Duplicate case for variant '{}'", variant_name.node),
            &case.location,
          );
        }

        let variant_fields = variant_info.fields;
        if fields.len() != variant_fields.len() {
          self.add_error(
            format!(
              "Pattern for variant '{}' has {} field(s), but variant has {} field(s)",
              variant_name.node,
              fields.len(),
              variant_fields.len()
            ),
            case.pattern.location(),
          );
        } else {
          // Register pattern field bindings as variables with correct types
          for (idx, field) in fields.iter().enumerate() {
            if let SwitchPatternField::Bind(name) = field {
              let field_type_str = &variant_fields[idx].1;
              let type_annotation = Self::type_string_to_type_name(field_type_str);

              // Same name across arms shares one variable/alloca (§5.24). Only
              // the first arm actually defines it; later arms just record the
              // additional binding for cross-case access/type tracking.
              if !var_case_counts.contains_key(name) {
                self.register_variable_decl_with_pointer(
                  name,
                  case.pattern.location(),
                  Some(&type_annotation),
                  None,
                  false,
                  false,
                  true,
                  None,
                );
              }

              // Track that this variable is bound in this case
              *var_case_counts.entry(name.clone()).or_insert(0) += 1;
              // Track the type for cross-case consistency check
              let existing = var_types
                .entry(name.clone())
                .or_insert(Some(field_type_str.clone()));
              if *existing != Some(field_type_str.clone()) {
                *existing = None; // type differs → mark as inconsistent
              }
            }
          }
        }
      } else {
        self.add_error(
          format!(
            "Unknown variant '{}' for enum '{}'",
            variant_name.node, enum_type
          ),
          &variant_name.span,
        );
      }

      // Visit case body
      self.begin_branch();
      for stmt in &case.body {
        self.visit_stmt(stmt);
      }
      switch_branch_defs.push(self.end_branch());
    }

    // Mark variables not bound in all cases as switch-only (inaccessible after end switch)
    let num_cases = switch_stmt.cases.len();
    for (name, count) in &var_case_counts {
      if *count < num_cases {
        let first_loc = switch_stmt.cases[0].pattern.location().clone();
        self.branch_only_vars.insert(name.clone(), first_loc);
      } else if let Some(None) = var_types.get(name) {
        // Variable is bound in all cases but types differ — also switch-only
        let first_loc = switch_stmt.cases[0].pattern.location().clone();
        self.branch_only_vars.insert(name.clone(), first_loc);
      }
    }

    // Exhaustiveness check
    if switch_stmt.default_case.is_some() {
      // Check for unreachable default: all variants covered + default present
      let all_covered = all_variant_names.iter().all(|v| covered.contains(*v));
      if all_covered {
        let default_loc = switch_stmt
          .default_location
          .as_ref()
          .unwrap_or(&switch_stmt.location);
        self.add_error(
          format!(
            "Unreachable 'default' case: all variants of enum '{}' are already covered",
            enum_type
          ),
          default_loc,
        );
      }
    } else {
      let missing: Vec<&str> = all_variant_names
        .iter()
        .filter(|v| !covered.contains(**v))
        .copied()
        .collect();
      if !missing.is_empty() {
        self.add_error(
          format!(
            "Switch is not exhaustive: missing variant(s) '{}' of enum '{}'. Add the missing case(s) or a default case.",
            missing.join("', '"),
            enum_type
          ),
          &switch_stmt.location,
        );
      }
    }

    // Visit default case body
    self.begin_branch();
    if let Some(default_body) = &switch_stmt.default_case {
      for stmt in default_body {
        self.visit_stmt(stmt);
      }
    }
    switch_branch_defs.push(self.end_branch());

    self.join_branch_defs(switch_branch_defs);
    self.exit_construct();
  }

  /// Analyze a switch over a non-enum scrutinee using literal patterns.
  ///
  /// Open value spaces (integers, floats, strings, chars) require a `default`
  /// arm. `bool` is closed — covering both `true` and `false` is exhaustive.
  fn visit_value_switch(&mut self, switch_stmt: &SwitchStmt, scrutinee_type: &str) {
    let scrutinee_unit = self.expr_unit(&switch_stmt.value);
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut switch_branch_defs: Vec<BranchDefs> = Vec::new();
    self.enter_construct();

    // For bool, the value space is {true, false}; track coverage for exhaustiveness.
    let is_bool = scrutinee_type == "bool";
    let mut bool_seen_true = false;
    let mut bool_seen_false = false;

    for case in &switch_stmt.cases {
      let SwitchPattern::Literal { value, location } = &case.pattern else {
        self.add_error(
          format!(
            "Cannot use an enum variant pattern when switching on non-enum type '{}'; use a literal",
            scrutinee_type
          ),
          case.pattern.location(),
        );
        for stmt in &case.body {
          self.visit_stmt(stmt);
        }
        continue;
      };

      if !is_switch_literal(value) {
        self.add_error("Switch case pattern must be a literal value", location);
        for stmt in &case.body {
          self.visit_stmt(stmt);
        }
        continue;
      }

      // Infer the case literal's type from the scrutinee type (numeric literals
      // take the scrutinee's bit width, mirroring assignment semantics).
      let case_type = self.expr_type_with_context(value, Some(scrutinee_type));
      if case_type != scrutinee_type {
        self.add_error(
          format!(
            "Switch case type '{}' does not match scrutinee type '{}'",
            case_type, scrutinee_type
          ),
          location,
        );
      }

      let case_unit = self.expr_unit(value);
      if !ExprUnit::same_unit(&scrutinee_unit, &case_unit) {
        self.add_error(
          format!(
            "Switch case unit {} does not match scrutinee unit {}",
            case_unit.display(),
            scrutinee_unit.display()
          ),
          location,
        );
      }

      // Duplicate detection uses a canonical key so equivalent spellings
      // (`0x1A` vs `26`, `3` vs `3.0`) are treated as the same case.
      if let Some(key) = switch_case_key(value, scrutinee_type)
        && !seen_keys.insert(key)
      {
        self.add_error("Duplicate case value in switch", location);
      }

      if is_bool && let Expr::BoolLiteral(lit) = &**value {
        if lit.value {
          bool_seen_true = true;
        } else {
          bool_seen_false = true;
        }
      }

      // Visit case body
      self.begin_branch();
      for stmt in &case.body {
        self.visit_stmt(stmt);
      }
      switch_branch_defs.push(self.end_branch());
    }

    if is_bool {
      let all_covered = bool_seen_true && bool_seen_false;
      if switch_stmt.default_case.is_some() {
        if all_covered {
          let default_loc = match switch_stmt.default_location.as_ref() {
            Some(loc) => loc,
            None => &switch_stmt.location,
          };
          self.add_error(
            "Unreachable 'default' case: all bool values are already covered",
            default_loc,
          );
        }
      } else if !all_covered {
        let mut missing = Vec::new();
        if !bool_seen_true {
          missing.push("true");
        }
        if !bool_seen_false {
          missing.push("false");
        }
        self.add_error(
          format!(
            "Switch is not exhaustive: missing case(s) '{}' of bool. Add the missing case(s) or a default case.",
            missing.join("', '")
          ),
          &switch_stmt.location,
        );
      }
    } else if switch_stmt.default_case.is_none() {
      self.add_error(
        format!(
          "Switch on non-enum type '{}' requires a default case (the value space is open)",
          scrutinee_type
        ),
        &switch_stmt.location,
      );
    }

    // Visit default case body
    self.begin_branch();
    if let Some(default_body) = &switch_stmt.default_case {
      for stmt in default_body {
        self.visit_stmt(stmt);
      }
    }
    switch_branch_defs.push(self.end_branch());

    self.join_branch_defs(switch_branch_defs);
    self.exit_construct();
  }
}

impl<'a> AstVisitor<()> for SemanticAnalyzer<'a> {
  /// Analyze all top-level statements.
  fn visit_program(&mut self, program: &Program) {
    // Track whether we've seen non-use statements to warn about misplaced use statements
    let mut seen_non_use_stmt = false;
    // Track the first test suite's location to enforce "test suite last".
    let mut seen_test_suite_location: Option<SourceLocation> = None;

    for stmt in &program.statements {
      // Check for use statement ordering: use statements should be at the beginning
      match stmt {
        Stmt::Use(use_stmt) => {
          if seen_non_use_stmt {
            self.add_warning(
              "use statements should be at the beginning of the file",
              &use_stmt.location,
            );
          }
          if let Some(loc) = &seen_test_suite_location {
            self.add_error("test suite must be the last top-level statement", loc);
          }
        }
        // Comments and docs don't count as "real" statements for ordering purposes
        Stmt::Comment(_) | Stmt::Doc(_) => {}
        Stmt::TestSuite(suite) => {
          seen_test_suite_location = Some(suite.location.clone());
          seen_non_use_stmt = true;
        }
        _ => {
          if let Some(loc) = &seen_test_suite_location {
            self.add_error("test suite must be the last top-level statement", loc);
          }
          seen_non_use_stmt = true;
        }
      }
      self.visit_stmt(stmt);
    }

    // Emit warnings for unused symbols
    self.emit_unused_warnings();

    // Link the global symbol table to the Program AST node
    if let Some(global_table) = self
      .symbols
      .get_all_symbol_tables()
      .unwrap_or_default()
      .get(&VarScope::Global)
    {
      *program.global_symbol_table.borrow_mut() = Some(global_table.clone());
    }
  }

  /// Validate a test suite: module scope, no nesting, at least one case, and
  /// resolve symbols in every case body normally.
  fn visit_test_suite(&mut self, test_suite: &TestSuiteStmt) {
    if self.in_test_case {
      self.add_error(
        "test suites cannot be nested inside a test case",
        &test_suite.location,
      );
      return;
    }
    if self.symbols.current_scope() != &VarScope::Global {
      self.add_error(
        "test suite is only allowed at module scope",
        &test_suite.location,
      );
      return;
    }
    if test_suite.cases().next().is_none() {
      self.add_error(
        "test suite requires at least one test case",
        &test_suite.location,
      );
    }
    // Duplicate test-suite names are an error (suite/case scope keys are suite-qualified).
    if !self.seen_test_suite_names.insert(test_suite.name.clone()) {
      self.add_error(
        format!("duplicate test suite name '{}'", test_suite.name),
        &test_suite.location,
      );
    }

    // Collect suite-level declarations (before the first case) and cases. A
    // `var`/`fn` after the first test case is a structural error: declarations
    // are hoisted to the top of the suite, and allowing them interleaved with
    // cases would make scope ordering ambiguous.
    let mut declarations: Vec<&Stmt> = Vec::new();
    let mut cases: Vec<&TestCaseStmt> = Vec::new();
    let mut seen_case = false;
    for item in &test_suite.items {
      match item {
        TestSuiteItem::Declaration(stmt) => {
          if seen_case {
            let (name, location) = match stmt.as_ref() {
              Stmt::VarDef(v) => (v.name.node.clone(), &v.location),
              Stmt::FnDef(f) => (f.name.node.clone(), &f.location),
              _ => ("<declaration>".to_string(), &test_suite.location),
            };
            self.add_error(
              format!(
                "suite-level declaration '{}' must appear before the first test case",
                name
              ),
              location,
            );
          } else {
            declarations.push(stmt.as_ref());
          }
        }
        TestSuiteItem::Case(case) => {
          seen_case = true;
          cases.push(case);
        }
        TestSuiteItem::Comment(_) | TestSuiteItem::Doc(_) => {}
      }
    }

    // Duplicate test-case names within a suite are an error.
    let mut seen_case_names = std::collections::HashSet::new();
    for case in &cases {
      if !seen_case_names.insert(case.name.clone()) {
        self.add_error(
          format!(
            "duplicate test case name '{}' in test suite '{}'",
            case.name, test_suite.name
          ),
          &case.location,
        );
      }
    }

    // Enter the suite scope (module-level globals become invisible), analyze
    // suite-level declarations, then analyze cases (which fall back to the suite
    // scope rather than the global scope).
    self.symbols.enter_suite_scope(&test_suite.name);
    self.current_suite_name = Some(test_suite.name.clone());

    // The `variables.function_qualified_name` column stores the suite name as the
    // suite scope key and has a foreign key to `functions.qualified_name`.
    // Register a pseudo-function row for the suite name so suite-level variables
    // satisfy that foreign key (mirroring how test cases register a pseudo-
    // function for their case scope).
    if let Err(e) = self.symbols.define_function(
      &test_suite.name,
      &test_suite.location,
      "nothing",
      None,
      FnInfo {
        parameters: Vec::new(),
        return_unit: None,
        body: Vec::new(),
      },
      Linkage::Internal,
    ) {
      self.add_error(
        format!("Failed to register test suite scope: {}", e),
        &test_suite.location,
      );
    }

    for stmt in declarations {
      match stmt {
        // Suite functions get a suite-qualified name so they don't collide with
        // global functions; suite vars register into the suite scope directly.
        Stmt::FnDef(fn_def) => self.visit_suite_fn_def(fn_def),
        _ => self.visit_stmt(stmt),
      }
    }
    for case in cases {
      self.visit_test_case(case);
    }

    self.current_suite_name = None;
    self.symbols.exit_suite_scope();
  }

  /// Visit a single test case: reject nesting and resolve its body normally.
  ///
  /// Test cases behave like functions: variables defined in a case are
  /// case-local (invisible to other cases, other suites, and non-test code),
  /// and lookups fall through to the enclosing suite scope (module globals are
  /// invisible).
  fn visit_test_case(&mut self, test_case: &TestCaseStmt) {
    if self.in_test_case {
      self.add_error("test cases cannot be nested", &test_case.location);
      return;
    }
    self.in_test_case = true;

    // Give the case a distinct function-like scope so its variables don't leak
    // into other cases, other suites, or non-test code. The scope key is the
    // suite-qualified case name (duplicate names are rejected in
    // `visit_test_suite`).
    let suite_name = match self.current_suite_name.as_deref() {
      Some(s) => s,
      None => ice!("visit_test_case called outside a test suite — this is a compiler bug"),
    };
    let scope_key = FnScopeKey {
      name: format!("{}::{}", suite_name, test_case.name),
      param_types: Vec::new(),
      return_type: "nothing".to_string(),
    };
    // Register a pseudo-function so the scope is reconstructable from the
    // database (needed by IR-gen symbol lookups and symbol-table display).
    if let Err(e) = self.symbols.define_function(
      &scope_key.name,
      &test_case.location,
      "nothing",
      None,
      FnInfo {
        parameters: Vec::new(),
        return_unit: None,
        body: test_case.body.clone(),
      },
      Linkage::Internal,
    ) {
      self.add_error(
        format!("Failed to register test case scope: {}", e),
        &test_case.location,
      );
      self.in_test_case = false;
      return;
    }
    self.symbols.enter_test_case_scope(scope_key);

    // Save scope symbols to check for unused case-local variables.
    let scope_start = self.scope_symbols.len();
    for stmt in &test_case.body {
      self.visit_stmt(stmt);
    }
    let local_symbols: Vec<String> = self.scope_symbols.drain(scope_start..).collect();
    self.emit_scope_unused_warnings(local_symbols);

    self.symbols.exit_function();
    self.in_test_case = false;
  }

  /// Process a module use statement.
  ///
  /// Use statements must be at global scope. This method resolves the module
  /// path, looks up exported symbols, and imports them into the current scope.
  ///
  /// # Errors
  ///
  /// - `"use statements are only allowed at module scope"` if the use
  ///   statement appears inside a function body.
  /// - `"Symbol 'X' is not exported from module 'Y'"` if a named import
  ///   refers to a non-exported symbol.
  fn visit_use(&mut self, use_stmt: &UseStmt) {
    if self.symbols.current_scope() != &VarScope::Global {
      self.add_error(
        "use statements are only allowed at module scope",
        &use_stmt.location,
      );
      return;
    }

    // Special case: `use all from std.full` (the full stdlib) with pre-loaded signatures.
    // Only the glob form is the full stdlib; a named import such as
    // `use abs from std.full` is a normal selective import handled below.
    if use_stmt.origin.node == ModuleOrigin::Std
      && use_stmt.path.len() == 1
      && use_stmt.path[0].node == "full"
      && matches!(use_stmt.imports, UseImports::All)
    {
      // For stdlib, the symbols should be pre-loaded via load_stdlib_signatures()
      let key = format!("use_std_seen:{}", self.symbols.get_module_path());
      if self.symbols.get_metadata(&key).unwrap_or(None).is_some() {
        self.add_warning(
          "Redundant 'use all from std.full': stdlib symbols are already available",
          &use_stmt.location,
        );
      } else {
        self.symbols.set_metadata(&key, "1");
        // Warn about earlier selective imports that are now redundant
        for (submod, symbols) in self.get_std_submodules_from_db() {
          self.add_warning(
            format!(
              "Redundant import from 'std.{}': symbols '{}' are now available via 'use all from std.full'",
              submod, symbols
            ),
            &use_stmt.location,
          );
        }
      }
      return;
    }

    // Track selective std sub-module imports (and their symbol names) in the database
    if use_stmt.origin.node == ModuleOrigin::Std
      && use_stmt.path.len() == 1
      && matches!(use_stmt.imports, UseImports::Named(_))
    {
      let submod = &use_stmt.path[0].node;
      let symbols = use_stmt.imports.names().join(", ");
      let key = format!("std_submod:{}:{}", self.symbols.get_module_path(), submod);
      self.symbols.set_metadata(&key, &symbols);
    }

    // Warn about redundant imports when `use all from std.full` already covers them
    if use_stmt.origin.node == ModuleOrigin::Std
      && use_stmt.path.len() == 1
      && matches!(use_stmt.imports, UseImports::Named(_))
    {
      let use_std_key = format!("use_std_seen:{}", self.symbols.get_module_path());
      if self
        .symbols
        .get_metadata(&use_std_key)
        .unwrap_or(None)
        .is_some()
      {
        let symbols = use_stmt.imports.names().join(", ");
        self.add_warning(
          format!(
            "Redundant import from 'std.{}': symbols '{}' are already available via 'use all from std.full'",
            use_stmt.path[0].node, symbols
          ),
          &use_stmt.location,
        );
      }
    }

    // For stdlib modules being compiled, skip use statement processing
    // Stdlib modules are analyzed in dependency order, so symbols are already in the database
    if crate::semantic_analysis::module_resolver::is_stdlib_path(&use_stmt.location.source_file) {
      return;
    }

    let resolver = match &self.module_resolver {
      Some(r) => r.clone(),
      None => {
        self.add_error(
          "Internal error: module resolver not initialized for use statement. \
           The compiler was not configured for multi-file compilation.",
          &use_stmt.location,
        );
        return;
      }
    };

    let current_file = match &self.current_module_path {
      Some(p) => p.clone(),
      None => {
        self.add_error(
          "Internal error: current module path not set during use statement analysis. \
           Unable to resolve imported modules.",
          &use_stmt.location,
        );
        return;
      }
    };

    let resolver_ref = resolver.borrow();
    let target_path =
      resolver_ref.resolve_module_path(use_stmt.origin.node, &use_stmt.path, &current_file);
    let canonical_target = match target_path.canonicalize() {
      Ok(canonical) => canonical,
      Err(e) => {
        // Canonicalization failed - log warning but use original path
        eprintln!(
          "WARNING: Failed to canonicalize module path '{}' at {}:{}: {}. \
           Using non-canonical path which may cause module identity issues.",
          target_path.display(),
          use_stmt.location.line,
          use_stmt.location.col,
          e
        );
        target_path.clone()
      }
    };
    let target_id = ModuleId::new(canonical_target);

    // The SQLite symbol manager is the single source of truth for a module's
    // exported symbols. The database keys modules by file name (see
    // `set_current_module_path`), so derive that name from the canonical path.
    let target_module_path = match target_id.path().file_name() {
      Some(name) => name.to_string_lossy().to_string(),
      None => target_id.path().to_string_lossy().to_string(),
    };

    let exports = match self.symbols.get_exports(&target_module_path) {
      Ok(e) => e,
      Err(e) => {
        self.add_error(
          format!("Failed to read exports of module '{}': {}", target_id, e),
          &use_stmt.location,
        );
        return;
      }
    };

    match &use_stmt.imports {
      UseImports::All => {
        for (name, symbol) in &exports {
          self.import_one(name, symbol, &use_stmt.location);
        }
      }
      UseImports::Named(names) => {
        for name_span in names {
          let name = &name_span.node;
          match exports.get(name) {
            Some(symbol) => self.import_one(name, symbol, &name_span.span),
            None => {
              self.add_error(
                format!(
                  "Symbol '{}' is not exported from module '{}'",
                  name, target_id
                ),
                &name_span.span,
              );
            }
          }
        }
      }
    }
  }

  /// Register a type definition.
  ///
  /// Type definitions must be at global scope. Attempting to define
  /// a type inside a function will result in a scope error.
  ///
  /// # Errors
  ///
  /// - `"Type definitions are only allowed at global scope"` if the type
  ///   is defined inside a function body.
  fn visit_type_def(&mut self, type_def: &TypeDefStmt) {
    // Type definitions must be at global scope
    if self.symbols.current_scope() != &VarScope::Global {
      self.add_error(
        "Type definitions are only allowed at global scope",
        &type_def.location,
      );
      return;
    }

    if self.reject_reserved_pi(&type_def.name.node, &type_def.location) {
      return;
    }

    // Register the type in the type definition registry (not symbol table)
    let type_def_name = &type_def.name.node;

    // Extract field information
    let fields: Vec<TypeFieldInfo> = type_def
      .fields
      .iter()
      .map(|field| {
        let field_name = field.name.node.clone();
        let field_type = TypeInference::type_name_to_string(&field.field_type);
        let field_unit = field.unit.as_ref().map(|u| u.raw.clone());
        let field_private = field.is_private;
        (field_name, field_type, field_unit, field_private)
      })
      .collect();

    self
      .symbols
      .define_type_with_fields(type_def_name, type_def.location.clone(), fields);
  }

  /// Register an enum definition.
  ///
  /// Enum definitions must be at global scope. Each variant is stored as
  /// a separate type definition with auto-generated field names (field0, field1, ...).
  /// The enum name itself is also registered as a type for reference.
  /// Constructor functions are registered for each variant so they can be called.
  ///
  /// # Errors
  ///
  /// - `"Enum definitions are only allowed at global scope"` if the enum
  ///   is defined inside a function body.
  fn visit_enum_def(&mut self, enum_def: &EnumDefStmt) {
    // Enum definitions must be at global scope
    if self.symbols.current_scope() != &VarScope::Global {
      self.add_error(
        "Enum definitions are only allowed at global scope",
        &enum_def.location,
      );
      return;
    }

    if self.reject_reserved_pi(&enum_def.name.node, &enum_def.location) {
      return;
    }

    // Register the enum name itself as a type (so lookup_type works)
    let enum_name = &enum_def.name.node;
    self
      .symbols
      .define_type_with_fields(enum_name, enum_def.location.clone(), vec![]);

    // Register each variant as a separate type definition and constructor function
    for variant in &enum_def.variants {
      let variant_type_name = format!("{}__{}", enum_name, variant.name.node);

      // Validate that all field types are known types
      for field in &variant.fields {
        if let BaseType::Custom(type_name) = &field.type_annotation.base_type
          && !self.symbols.lookup_type(type_name).is_some()
        {
          self.add_error(
            format!(
              "Unknown type '{}' in variant field of enum '{}'",
              type_name, enum_name
            ),
            &field.location,
          );
        }
      }

      // Convert variant fields to type fields
      let fields: Vec<TypeFieldInfo> = variant
        .fields
        .iter()
        .map(|field| {
          let field_name = field.name.node.clone();
          let field_type = TypeInference::type_name_to_string(&field.type_annotation);
          let field_unit = field.unit.as_ref().map(|u| u.raw.clone());
          (field_name, field_type, field_unit, false)
        })
        .collect();

      self
        .symbols
        .define_type_with_fields(&variant_type_name, variant.location.clone(), fields);

      // Register the constructor function: variant_type_name() -> variant_type_name
      // Each variant constructor takes the variant fields as parameters and returns the variant type.
      let constructor_params: Vec<Parameter> = variant
        .fields
        .iter()
        .map(|f| Parameter {
          pass_mode: f.pass_mode,
          name: f.name.clone(),
          type_annotation: f.type_annotation.clone(),
          unit: f.unit.clone(),
          location: f.location.clone(),
        })
        .collect();

      // Clone before moving into the first define_function call
      let simple_name_params = constructor_params.clone();

      if let Err(e) = self.symbols.define_function(
        &variant_type_name,
        &variant.location,
        &variant_type_name, // return_type as string
        None,               // return_unit
        FnInfo {
          parameters: constructor_params,
          return_unit: None,
          body: vec![],
        },
        Linkage::Internal,
      ) {
        self.add_error(
          format!("Failed to register function '{}': {}", variant_type_name, e),
          &variant.location,
        );
      }

      // Also register the variant's simple name (e.g. "Circle") as a function
      // with the same parameters as the constructor, so it can be used naturally:
      // `var s as Shape = Circle(5.0)` or `var c as Color = Red`
      // The return type is the enum name (e.g. "Shape"), not the variant type.
      if let Err(e) = self.symbols.define_function(
        &variant.name.node,
        &variant.location,
        enum_name, // return_type: the enum type (e.g. "Shape")
        None,
        FnInfo {
          parameters: simple_name_params,
          return_unit: None,
          body: vec![],
        },
        Linkage::Internal,
      ) {
        self.add_error(
          format!("Failed to register function '{}': {}", variant.name.node, e),
          &variant.location,
        );
      }
    }
  }

  /// Register defined variable and analyze its value.
  ///
  /// Unit assignment rules:
  /// 1. LHS has unit, RHS is unitless → Use declared unit (accepted)
  /// 2. LHS is unitless, RHS has unit → Infer unit from RHS
  /// 3. Both have units → Must match, otherwise error
  /// 4. Both unitless → No unit
  fn visit_var_def(&mut self, def: &VarDefStmt) {
    // Set the current assignment type context for binary operation checking
    let assignment_type = def
      .type_annotation
      .as_ref()
      .map(TypeInference::type_name_to_string);
    let old_context = self.current_assignment_type.take();
    self.current_assignment_type = assignment_type.clone();

    self.visit_expr(&def.value);

    // Restore the old context
    self.current_assignment_type = old_context;

    // Check type for backend restrictions
    if let Some(declared_type) = &def.type_annotation {
      self.visit_type_name(declared_type);
    }

    // Check type compatibility: if a type is declared, RHS must match exactly
    // Exception: numeric literals can infer their type from the declared type
    // Also: fill literals with explicit type annotations don't need type inference
    if let Some(declared_type) = &def.type_annotation {
      let declared_type_str = TypeInference::type_name_to_string(declared_type);
      // Use context-aware type inference: literals infer type from the declaration
      let rhs_type = self.expr_type_with_context(&def.value, Some(&declared_type_str));

      // Check if this is a fill literal (which returns [] from expr_type)
      let is_fill_literal = if let Expr::ArrayLiteral(arr) = &def.value {
        arr.fill.is_some()
      } else {
        false
      };

      // Check if this is a `value at p unsafe bitcast` pattern
      // AST structure: ValueAt(UnsafeBitcast(Identifier))
      // For unsafe bitcast, the type is determined by the assignment context (bit reinterpretation)
      let is_unsafe_bitcast_pattern = if let Expr::Unary(value_at_unary) = &def.value {
        if matches!(value_at_unary.operator, UnaryOp::ValueAt) {
          if let Expr::Unary(unsafe_bitcast_unary) = &*value_at_unary.operand {
            matches!(unsafe_bitcast_unary.operator, UnaryOp::UnsafeBitcast)
          } else {
            false
          }
        } else {
          false
        }
      } else {
        false
      };

      // Check for actual type mismatches that require conversion
      // - Allow numeric literals/introspection to assign to any numeric type (inference)
      // - Allow widening conversions for binary arithmetic (e.g. i64 → f64)
      // - Reject narrowing / same-width signed↔unsigned (e.g. u64 → i64, u64 → u8)
      // - Reject assigning non-numeric expressions to numeric types
      // - Reject assigning numeric to non-numeric types
      // - Allow unsafe bitcast patterns (type determined by declaration context)
      // - Allow fill literals (type determined by annotation)
      let is_numeric_literal = TypeChecker::is_numeric_literal_expr(&def.value);
      let is_float_literal = TypeChecker::is_float_literal(&def.value);
      let is_binary_arithmetic = TypeChecker::is_binary_arithmetic(&def.value);
      let is_declared_numeric = matches!(
        declared_type_str.as_str(),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
      );
      let is_declared_float = matches!(declared_type_str.as_str(), "f16" | "f32" | "f64");

      let types_ok = if is_unsafe_bitcast_pattern {
        // Unsafe bitcast: type is determined by declaration, skip normal type check
        // Bit-width compatibility is checked separately
        true
      } else if is_fill_literal {
        // Fill literal: type is determined by annotation, not the literal
        // Detailed validation happens in validate_array_initialization
        true
      } else if TypeInference::types_compatible(&declared_type_str, &rhs_type) {
        // Exact match or allowed compatibility (arrays, etc.)
        true
      } else if is_declared_numeric
        && ((is_numeric_literal && !is_float_literal)
          || (is_float_literal && is_declared_float)
          || (is_binary_arithmetic
            && is_widening_numeric_conversion(&rhs_type, &declared_type_str)))
      {
        // Integer literal/introspection (infers), float literal to a float target
        // (precision loss by design), or a widening arithmetic result (e.g.
        // i64 → f64): allow.
        true
      } else {
        // Any mismatch: reject (e.g., variable f64 to i32, string to int, etc.)
        false
      };

      if !types_ok {
        self.add_error(
          format!(
            "Type mismatch for '{}': declared type is '{}' but initializer has type '{}'. Use explicit conversion: {} as {}",
            def.name.node, declared_type_str, rhs_type,
            ExpressionAnalyzer::expr_to_string(&def.value), declared_type_str
          ),
          &def.location,
        );
      }

      // Validate that the literal value fits in the declared type's range
      self.validate_literal_range(&declared_type_str, &def.value, &def.location);

      // If the RHS is a `value at p unsafe bitcast` pattern, validate bit-width and unit compatibility
      if is_unsafe_bitcast_pattern {
        self.validate_unsafe_bitcast_compatibility(&declared_type_str, &def.value, &def.location);
      }

      // Validate array initialization
      self.validate_array_initialization(declared_type, &def.value, &def.location);
    } else {
      // No type annotation: check that the RHS is not a bare numeric literal.
      // Bare literals have no type to infer from — the programmer must provide
      // context via `as type` on either the LHS or RHS.
      if is_bare_numeric_literal(&def.value) {
        self.add_error(
          format!(
            "Cannot infer type for ‘{}’: literal has no type without context. Use ‘var {} as <type> = ...’ or ‘var {} = ... as <type>’",
            def.name.node, def.name.node, def.name.node
          ),
          &def.location,
        );
      }
    }

    let rhs_unit = self.expr_unit(&def.value);
    let declared_unit = ExprUnit::from_option(&def.unit);

    match (&declared_unit, &rhs_unit) {
      (ExprUnit::Unit(_), ExprUnit::Unitless) => {
        // LHS has unit, RHS is unitless → accepted, use declared unit
      }
      (ExprUnit::Unitless, ExprUnit::Unit(_)) => {
        // LHS has no unit, RHS has unit → infer unit from RHS
      }
      (ExprUnit::Unit(du), ExprUnit::Unit(ru)) if du != ru => {
        // Both have units but they don't match → error
        self.add_error(
          format!(
            "Unit mismatch for '{}': declared '{}' but initializer has '{}'",
            def.name.node, du, ru
          ),
          &def.location,
        );
      }
      _ => {}
    }

    // Determine the final unit for the symbol:
    // - If declared unit exists, use it
    // - Otherwise, infer from RHS if it has a unit
    let symbol_unit = match (&def.unit, &rhs_unit) {
      (Some(u), _) => Some(u.raw.clone()),
      (None, ExprUnit::Unit(u)) => Some(u.to_string()),
      _ => None,
    };

    // Compute the inferred type if no type annotation was provided
    let inferred_type_str = if def.type_annotation.is_none() {
      self.expr_type(&def.value)
    } else {
      String::new() // Empty - will use declared type below
    };

    // Track the source type if this pointer is created from a `pointer to` expression
    let pointer_source_type = if self.expr_type(&def.value) == "pointer" {
      Self::extract_pointer_source_type(&def.value)
    } else {
      None
    };

    // If no type annotation, use the inferred type by creating a synthetic TypeName
    // This ensures the symbol table has the correct type even for inferred declarations
    let final_type_annotation = if let Some(ta) = &def.type_annotation {
      Some(ta.clone())
    } else if !inferred_type_str.is_empty() {
      // Create a synthetic TypeName from the inferred type string
      Some(Self::type_string_to_type_name(&inferred_type_str))
    } else {
      None
    };

    // Record the definition into the current branch frame (if inside a branch)
    // for definite-assignment analysis.
    let branch_ty = match &final_type_annotation {
      Some(ta) => TypeInference::type_name_to_string(ta),
      None => inferred_type_str.clone(),
    };
    self.record_branch_def(
      &def.name.node,
      &branch_ty,
      symbol_unit.clone(),
      &def.location,
    );

    // A same-named `var` in a sibling branch is a join, not a duplicate: the
    // first branch already registered it, so skip re-registration (the join in
    // `join_branch_defs` verifies type/unit consistency).
    if !self.is_sibling_join(&def.name.node) {
      self.register_variable_decl_with_pointer(
        &def.name.node,
        &def.name.span,
        final_type_annotation.as_ref(),
        symbol_unit,
        def.is_export,
        def.is_import,
        true,
        pointer_source_type,
      );
    }

    // Track pointer provenance for escape checking: a local pointer variable
    // initialized from `pointer to <local>`.
    if let Some(source) = self.pointer_to_local_source(&def.value) {
      self
        .local_pointer_vars
        .insert(def.name.node.clone(), source);
    }

    // Track the initializer's compile-time constant value (if any) for div-zero
    // analysis and future constant propagation.
    //
    // The "constant" designation must be sound: an `export`ed or `import`ed
    // variable is shared with another module (or with C/FFI), so it can be
    // mutated out of our sight and must never be treated as a compile-time
    // constant. Such variables are marked shared rather than given a value.
    //
    // A same-named definition in a later sibling branch is a join: the value may
    // differ across branches, so it is conservatively not a constant (until a
    // later straight-line write re-establishes one).
    if def.is_export || def.is_import {
      self.symbols.mark_variable_shared(&def.name.node);
    } else if self.is_sibling_join(&def.name.node) {
      self
        .symbols
        .update_variable_const_value(&def.name.node, None);
    } else {
      let const_value = self.expr_const_value(&def.value);
      self
        .symbols
        .update_variable_const_value(&def.name.node, const_value);
    }

    // Track allocate() assignments for missing-release detection.
    // If the initializer is allocate(...), record the variable for post-body checking.
    if matches!(&def.value, Expr::Allocate(_)) {
      self
        .allocated_vars_in_scope
        .insert(def.name.node.clone(), def.location.clone());
    }

    // Track T? variables for unchecked-optional warnings.
    if let Some(ta) = &final_type_annotation
      && TypeInference::is_optional_type(&TypeInference::type_name_to_string(ta))
    {
      self
        .optional_vars_in_scope
        .insert(def.name.node.clone(), def.name.span.clone());
    }
  }

  /// Register unsafe variable declaration.
  fn visit_unsafe_decl(&mut self, decl: &UnsafeDeclStmt) {
    // Check type for backend restrictions
    if let Some(declared_type) = &decl.type_annotation {
      self.visit_type_name(declared_type);
    }

    self.register_variable_decl(
      &decl.name.node,
      &decl.name.span,
      decl.type_annotation.as_ref(),
      decl.unit.as_ref().map(|u| u.raw.clone()),
      decl.is_export,
      decl.is_import,
      false,
    );
  }

  /// Analyze assignment statement.
  fn visit_assign(&mut self, assign: &AssignStmt) {
    self.visit_expr(&assign.value);

    for index in &assign.indices {
      self.visit_expr(index);
    }

    let target_name = assign.target.node.first().map(|s| s.as_str()).unwrap_or("");
    if !self.symbols.is_variable_defined(target_name) {
      // Module-qualified assignment: `math.e = value`.
      let is_module_assign = assign.target.node.len() == 2
        && assign.indices.is_empty()
        && self
          .module_export_type(&assign.target.node[0], &assign.target.node[1])
          .is_some();
      if !is_module_assign {
        self.add_error(
          format!("Assignment to undefined variable '{}'", target_name),
          &assign.location,
        );
        return;
      }
      self.mark_symbol_used(&assign.target.node[1]);
      return;
    }

    // Update constant tracking: reassignment sets the variable's constant value
    // to the new RHS (if that is a constant expression), or clears it otherwise.
    // A write inside a branch/loop body leaves the value unknown at the join, so
    // it conservatively clears the constant rather than re-establishing it.
    self.symbols.mark_variable_reassigned(target_name);
    let const_value = if self.sibling_defined_stack.is_empty() {
      self.expr_const_value(&assign.value)
    } else {
      None
    };
    self
      .symbols
      .update_variable_const_value(target_name, const_value);

    // Track allocate() reassignment: if a pointer variable is being
    // reassigned to allocate(...), the previous allocation is lost.
    if matches!(&assign.value, Expr::Allocate(_)) {
      if self.allocated_vars_in_scope.contains_key(target_name)
        && !self.released_vars.contains(target_name)
        && !self.on_exit_released_vars.contains(target_name)
      {
        self.add_warning(
          format!(
            "reassignment to '{}' overwrites a previous allocate() without releasing it first",
            target_name
          ),
          &assign.location,
        );
      }
      self
        .allocated_vars_in_scope
        .insert(target_name.to_string(), assign.location.clone());
      // A new allocate() resets the released status
      self.released_vars.remove(target_name);
      self.on_exit_released_vars.remove(target_name);
    }

    // Check private field access: obj.private_field = value from outside the defining module
    if assign.target.node.len() >= 2 {
      let field_name = assign.target.node.last().map(|s| s.as_str()).unwrap_or("");
      if let Some(type_info) = self.symbols.lookup_type(
        &self
          .symbols
          .lookup_var_type(target_name)
          // Empty string indicates unresolved type — downstream checks will handle
          // or a prior error already reported.
          .unwrap_or_default(),
      ) {
        for (tf_name, _tf_type, _tf_unit, is_private) in &type_info.fields {
          if tf_name == field_name && *is_private {
            let current_module = self
              .current_module_path
              .as_ref()
              .and_then(|p| p.file_name())
              .map(|f| f.to_string_lossy().to_string())
              // When module path has no file stem, treat as non-matching for private access.
              .unwrap_or_default();
            if current_module != type_info.module_path {
              self.add_error(
                format!(
                  "Cannot assign to private field '{}' of type '{}' from outside its defining module '{}'",
                  field_name, target_name, type_info.module_path
                ),
                &assign.location,
              );
            }
          }
        }
      }
    }

    // Check memory safety: pointer-to-local must not escape to global variables
    self.check_escaping_pointer_to_global(&assign.value, target_name, &assign.location);

    // Track pointer provenance for escape checking on subsequent returns/assignments.
    // Only simple targets (`p = ...`) affect a pointer variable's provenance.
    if assign.target.node.len() == 1
      && assign.indices.is_empty()
      && self.symbols.is_local_variable(target_name)
    {
      if let Some(source) = self.pointer_to_local_source(&assign.value) {
        self
          .local_pointer_vars
          .insert(target_name.to_string(), source);
      } else {
        self.local_pointer_vars.remove(target_name);
      }
    }

    // Check type compatibility: assigned value must match target variable type
    // Exception: numeric literals can infer their type from the target variable type
    let var_type = self
      .symbols
      .lookup_var_type(target_name)
      .unwrap_or_else(|| "unknown".to_string());

    // If there are indices, extract the element type instead of the whole array type
    let target_type = if !assign.indices.is_empty() {
      // Array element assignment: extract element type from array type
      // e.g., "i32[3]" -> "i32", "f64[2][3]" -> "f64"
      Self::extract_element_type(&var_type)
    } else {
      // Simple assignment: use the variable's type as-is
      var_type
    };

    let value_type = self.expr_type(&assign.value);

    let is_numeric_literal = TypeChecker::is_numeric_literal_expr(&assign.value);
    let is_float_literal = TypeChecker::is_float_literal(&assign.value);
    let is_binary_arithmetic = TypeChecker::is_binary_arithmetic(&assign.value);
    let is_target_numeric = matches!(
      target_type.as_str(),
      "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64"
    );
    let is_target_float = matches!(target_type.as_str(), "f16" | "f32" | "f64");

    let types_ok = if TypeInference::types_compatible(&target_type, &value_type) {
      // Exact match or allowed compatibility (arrays, etc.)
      true
    } else if is_target_numeric
      && ((is_numeric_literal && !is_float_literal)
        || (is_float_literal && is_target_float)
        || (is_binary_arithmetic && is_widening_numeric_conversion(&value_type, &target_type)))
    {
      // Integer literal/introspection (infers), float literal to a float target
      // (precision loss by design), or a widening arithmetic result (e.g.
      // i64 → f64): allow.
      true
    } else {
      // Any mismatch: reject (e.g., variable f64 to i32, string to int, etc.)
      false
    };

    if !types_ok {
      self.add_error(
        format!(
          "Type mismatch in assignment to '{}': variable has type '{}' but value has type '{}'. Use explicit conversion: {} as {}",
          target_name, target_type, value_type,
          ExpressionAnalyzer::expr_to_string(&assign.value), target_type
        ),
        &assign.location,
      );
    }

    let target_unit = self.symbols.lookup_var_unit(target_name);
    let value_unit = self.expr_unit(&assign.value);

    // For assignment, allow inferring unit if target has Unknown unit
    if target_unit.is_unknown() && !value_unit.is_unknown() {
      // Update the symbol table with the inferred unit
      if value_unit.is_unitless() {
        self.symbols.update_symbol_unit(target_name, None);
      } else if let ExprUnit::Unit(u) = &value_unit {
        self
          .symbols
          .update_symbol_unit(target_name, Some(u.to_string()));
      }
    } else if !ExprUnit::same_unit(&target_unit, &value_unit) {
      self.add_error(
        format!(
          "Unit mismatch in assignment to '{}': variable has {} but value has {}",
          target_name,
          target_unit.display(),
          value_unit.display(),
        ),
        &assign.location,
      );
    }
  }

  /// Analyze compound assignment statement.
  fn visit_compound_assign(&mut self, compound: &CompoundAssignStmt) {
    self.visit_expr(&compound.value);

    // Division-by-zero guard check for /= and %=
    if matches!(
      compound.operator.node,
      CompoundOp::DivAssign | CompoundOp::ModAssign
    ) && !self.is_guarded_nonzero(&compound.value)
    {
      let op_name = if matches!(compound.operator.node, CompoundOp::DivAssign) {
        "division"
      } else {
        "modulo"
      };
      self.add_warning(
        format!(
          "potential {} by zero in compound assignment: divisor '{}' is not provably non-zero. \
           Consider adding a guard such as `when {} != 0` (or a `match` arm guard).\n           note: only structurally identical expressions in surrounding guard conditions are checked.",
          op_name,
          ExpressionAnalyzer::expr_to_string(&compound.value),
          ExpressionAnalyzer::expr_to_string(&compound.value)
        ),
        &compound.location,
      );
    }

    for index in &compound.indices {
      self.visit_expr(index);
    }

    let target_name = compound
      .target
      .node
      .first()
      .map(|s| s.as_str())
      .unwrap_or("");
    if !self.symbols.is_variable_defined(target_name) {
      self.add_error(
        format!(
          "Compound assignment to undefined variable '{}'",
          target_name
        ),
        &compound.location,
      );
      return;
    }

    // Compound assignment changes the variable's value. `is_reassigned` is
    // sticky; the flow-sensitive `const_value` is updated to the folded result
    // when both the current value and the RHS are known constants (feeding the
    // division-by-zero and unsigned-underflow checks).
    self.symbols.mark_variable_reassigned(target_name);
    let new_const = self.fold_compound_assign_const(target_name, compound);
    self
      .symbols
      .update_variable_const_value(target_name, new_const);

    let target_unit = self.symbols.lookup_var_unit(target_name);
    let value_unit = self.expr_unit(&compound.value);

    match compound.operator.node {
      CompoundOp::AddAssign | CompoundOp::SubAssign => {
        if !ExprUnit::same_unit(&target_unit, &value_unit) {
          self.add_error(
            format!(
              "Unit mismatch in compound assignment to '{}': variable has {} but value has {}",
              target_name,
              target_unit.display(),
              value_unit.display(),
            ),
            &compound.location,
          );
        }
      }
      CompoundOp::MulAssign | CompoundOp::DivAssign => {
        if !value_unit.is_unitless() && !value_unit.is_unknown() {
          self.add_error(
            format!(
              "Compound *= or /= requires unitless operand, got {}",
              value_unit.display(),
            ),
            &compound.location,
          );
        }
      }
      CompoundOp::ModAssign => {
        if !target_unit.is_unitless() && !target_unit.is_unknown() {
          self.add_error(
            format!(
              "Modulo assignment requires unitless variable, got {}",
              target_unit.display(),
            ),
            &compound.location,
          );
        }
      }
    }
  }

  fn visit_value_at_assign(&mut self, value_at: &ValueAtAssignStmt) {
    self.visit_expr(&value_at.pointer);
    self.visit_expr(&value_at.value);

    // Use-after-free: writing through a pointer that was already released.
    if let Some(name) = Self::pointer_identifier_name(&value_at.pointer)
      && self.explicitly_released_vars.contains(name)
    {
      self.add_warning(
        format!(
          "`value at {}` used after `release {}` (use-after-free)",
          name, name
        ),
        &value_at.location,
      );
    }
  }

  /// Enter function scope and analyze body.
  ///
  /// Function definitions must be at global scope. Nested function definitions
  /// are not allowed. Attempting to define a function inside another function
  /// will result in a scope error.
  ///
  /// # Errors
  ///
  /// - `"Function definitions are only allowed at global scope"` if the function
  ///   is defined inside another function body.
  fn visit_fn_def(&mut self, fn_def: &FnDefStmt) {
    // Function definitions must be at global scope or test-suite scope.
    if !matches!(
      self.symbols.current_scope(),
      VarScope::Global | VarScope::Suite { .. }
    ) {
      self.add_error(
        "Function definitions are only allowed at module or test-suite scope",
        &fn_def.location,
      );
      return;
    }

    if self.reject_reserved_pi(&fn_def.name.node, &fn_def.name.span) {
      return;
    }

    // 'main' and '_start' are reserved function names (entry points)
    // unless stdlib is disabled
    if self.options.stdlib.is_enabled() {
      match fn_def.name.node.as_str() {
        "main" => {
          self.add_error(
            "'main' is a reserved function name. It is automatically generated as the entry point when using the standard C runtime. \
             If you need a custom entry point, use the `--no-std-lib` CLI option to exclude the standard library.",
            &fn_def.name.span,
          );
          return;
        }
        "_start" => {
          self.add_error(
            "'_start' is a reserved function name. It is automatically generated as the OS entry point when using the standard C runtime. \
             If you need a custom entry point, use the `--no-std-lib` CLI option to exclude the standard library.",
            &fn_def.name.span,
          );
          return;
        }
        _ => {}
      }
    }

    // Check return type for backend restrictions
    if let ReturnTypeKind::Type(return_type_name) = &fn_def.return_type.kind {
      self.visit_type_name(return_type_name);
    }

    // Check parameter types for backend restrictions
    for param in &fn_def.parameters {
      self.reject_reserved_pi(&param.name.node, &param.name.span);
      self.visit_type_name(&param.type_annotation);
    }

    // Warn (once, at the definition site) about aggregate parameters that are
    // copied by value without an explicit modifier. The default is by-value, so
    // a struct/array/str/enum parameter does an O(n) deep copy at every call.
    for param in &fn_def.parameters {
      if param.pass_mode.is_ref() || param.pass_mode.is_explicit_copy() {
        continue;
      }
      if !TypeInference::is_aggregate(&param.type_annotation) {
        continue;
      }
      let type_str = TypeInference::type_name_to_string(&param.type_annotation);
      let name = &param.name.node;
      let fn_name = &fn_def.name.node;
      let message = format!(
        "parameter '{name}' of type '{type_str}' is copied by value at every call.\n\n\
           This performs a deep copy of the whole value (O(n) in its size), which may be\n\
           slow for large arrays, structs, or strings.\n\n\
           If this function only reads the parameter, pass it by reference instead:\n\
               fn {fn_name}(ref {name} as {type_str})\n\n\
           Be aware: a `ref` parameter is mutable — writing to it inside the function\n\
           changes the caller's value after the function returns.\n\n\
           If you deliberately want a copy (so the function cannot affect the caller's\n\
           data), write `copy` to make the intent explicit and silence this warning:\n\
               fn {fn_name}(copy {name} as {type_str})"
      );
      self.add_warning(message, &param.location);
    }

    let return_type = match &fn_def.return_type.kind {
      ReturnTypeKind::Nothing => "nothing".to_string(),
      ReturnTypeKind::Type(t) => TypeInference::type_name_to_string(t),
    };
    let return_unit = fn_def.return_unit.as_ref().map(|u| u.raw.clone());

    // Track defined functions for unused detection (only if not exported/imported)
    if !fn_def.is_export {
      self.mark_symbol_defined(&fn_def.name.node, fn_def.name.span.clone(), true);
    }

    let linkage = if fn_def.is_export {
      Linkage::Export
    } else {
      Linkage::Internal
    };

    // Register the function in the symbol database
    if let Err(e) = self.symbols.define_function(
      &fn_def.name.node,
      &fn_def.name.span,
      &return_type,
      return_unit,
      FnInfo {
        parameters: fn_def.parameters.clone(),
        return_unit: fn_def.return_unit.clone(),
        body: fn_def.body.clone(),
      },
      linkage,
    ) {
      self.add_error(
        format!("Failed to register function '{}': {}", fn_def.name.node, e),
        &fn_def.name.span,
      );
    }

    self.symbols.enter_function(fn_def);

    // Switch arm bindings are function-scoped: save and clear the outer
    // scope's branch-only variables so they don't leak into this function.
    let outer_branch_only_vars = std::mem::take(&mut self.branch_only_vars);

    // Clear T? tracking for the new function scope
    self.optional_vars_in_scope.clear();
    self.checked_optional_vars.clear();

    // Track T? function parameters for unchecked-optional warnings
    for param in &fn_def.parameters {
      let param_type = TypeInference::type_name_to_string(&param.type_annotation);
      if TypeInference::is_optional_type(&param_type) {
        self
          .optional_vars_in_scope
          .insert(param.name.node.clone(), param.name.span.clone());
      }
    }

    // Track function parameters for unused detection
    for param in &fn_def.parameters {
      self.mark_symbol_defined(&param.name.node, param.name.span.clone(), false);
    }

    // Save current scope symbols to check for unused locals
    let scope_start = self.scope_symbols.len();

    self.current_return_unit = fn_def
      .return_unit
      .as_ref()
      .map(|u| ExprUnit::from_string(&u.raw));

    self.current_return_type = Some(return_type);

    for stmt in &fn_def.body {
      self.visit_stmt(stmt);
    }

    // Warn about allocate() calls without matching release / on exit release
    self.emit_missing_release_warnings();

    // Warn about T? variables that were never checked with `has value` / `has no value`
    self.emit_unchecked_optional_warnings();

    // Emit warnings for unused local variables in this function scope
    let local_symbols: Vec<String> = self.scope_symbols.drain(scope_start..).collect();
    self.emit_scope_unused_warnings(local_symbols);

    // Link the symbol table to the AST node before exiting scope
    let fn_table = self
      .symbols
      .get_symbol_table(self.symbols.current_scope().clone());
    if !fn_table.is_empty() {
      *fn_def.symbol_table.borrow_mut() = Some(fn_table);
    }

    self.symbols.exit_function();
    self.current_return_unit = None;
    self.current_return_type = None;

    // Clear allocation tracking for this function scope
    self.allocated_vars_in_scope.clear();
    self.released_vars.clear();
    self.on_exit_released_vars.clear();
    self.explicitly_released_vars.clear();
    self.local_pointer_vars.clear();

    // Restore the outer scope's branch-only variables.
    self.branch_only_vars = outer_branch_only_vars;
  }

  /// Function signature declarations introduce symbols (for imported functions).
  ///
  /// Imported function signatures declare external C functions that will be
  /// resolved at link time. They don't have Lale implementations.
  fn visit_fn_signature(&mut self, fn_signature: &FnSignatureStmt) {
    // Only process imported signatures; regular signatures are forward declarations
    if !fn_signature.is_import {
      return;
    }

    if self.reject_reserved_pi(&fn_signature.name.node, &fn_signature.name.span) {
      return;
    }

    // Check parameter types for backend restrictions
    for param in &fn_signature.parameters {
      self.reject_reserved_pi(&param.name.node, &param.name.span);
      self.visit_type_name(&param.type_annotation);
    }

    // Check return type for backend restrictions
    if let ReturnTypeKind::Type(return_type_name) = &fn_signature.return_type.kind {
      self.visit_type_name(return_type_name);
    }

    let return_type = match &fn_signature.return_type.kind {
      ReturnTypeKind::Nothing => "nothing".to_string(),
      ReturnTypeKind::Type(t) => TypeInference::type_name_to_string(t),
    };
    let return_unit = fn_signature.return_unit.as_ref().map(|u| u.raw.clone());

    // Imported signatures always have Import linkage
    let linkage = Linkage::Import;

    if let Err(e) = self.symbols.define_function(
      &fn_signature.name.node,
      &fn_signature.name.span,
      &return_type,
      return_unit,
      FnInfo {
        parameters: fn_signature.parameters.clone(),
        return_unit: fn_signature.return_unit.clone(),
        body: Vec::new(), // No body for imported functions
      },
      linkage,
    ) {
      self.add_error(
        format!(
          "Failed to register function '{}': {}",
          fn_signature.name.node, e
        ),
        &fn_signature.name.span,
      );
    }
  }

  /// Analyze function call arguments and check parameter units.
  fn visit_fn_call_stmt(&mut self, fn_call: &FnCall) {
    // 'main' and '_start' are entry points and cannot be called by user code
    if let Some(fn_name) = fn_call.target.node.last() {
      match fn_name.as_str() {
        "main" | "_start" => {
          self.add_error(
            format!("'{}' is the entry point and cannot be called by user code. It is invoked by the runtime automatically.", fn_name),
            &fn_call.location,
          );
        }
        _ => {}
      }
    }

    for arg in &fn_call.arguments {
      self.visit_expr(arg);
    }
    self.check_fn_call_param_units(fn_call);
    self.invalidate_ref_passed_vars(fn_call);

    // Mark function as used (resolving a suite function name if present).
    if let Some(fn_name) = fn_call.target.node.last() {
      let resolved = self.resolve_fn_name(fn_name);
      self.mark_symbol_used(&resolved);
    }
  }

  /// Analyze all branches of if/else-if/else statement.
  fn visit_if(&mut self, if_stmt: &IfStmt) {
    self.visit_condition(&if_stmt.condition);

    // else if is no longer supported — use match/when/else instead
    if !if_stmt.else_if_branches.is_empty() {
      let else_if_loc = if_stmt.else_if_branches[0].0.location.clone();
      self.add_error(
        "`else if` is not supported. Use `match / when / else` or `switch / case / default` instead \
         for multi-way branching, which is never ambiguous.",
        &else_if_loc,
      );
    }

    // Extract facts from the primary condition for both polarities
    let then_polarity = !if_stmt.condition.negated;
    let else_polarity = if_stmt.condition.negated;

    // Track definite assignment: capture `var` definitions per branch, then
    // join them so a variable defined in only one branch is flagged branch-only.
    self.enter_construct();
    self.begin_branch();
    self.push_guard_frame(extract_facts(&if_stmt.condition.expr, then_polarity));
    for stmt in &if_stmt.then_branch {
      self.visit_stmt(stmt);
    }
    self.pop_guard_frame();
    let then_defs = self.end_branch();

    for (cond, branch) in &if_stmt.else_if_branches {
      self.visit_condition(cond);
      let ei_polarity = !cond.negated;
      self.push_guard_frame(extract_facts(&cond.expr, ei_polarity));
      for stmt in branch {
        self.visit_stmt(stmt);
      }
      self.pop_guard_frame();
    }

    self.begin_branch();
    if let Some(else_branch) = &if_stmt.else_branch {
      // For else-branch, we combine: facts from original condition being false,
      // plus (to be conservative) the negation of each else-if condition.
      // We only extract facts from the original condition's false polarity;
      // else-if negations are too complex to track precisely.
      self.push_guard_frame(extract_facts(&if_stmt.condition.expr, else_polarity));
      for stmt in else_branch {
        self.visit_stmt(stmt);
      }
      self.pop_guard_frame();
    } else {
      // Exhaustive else enforcement: if requires an else branch.
      self.add_error(
        "if statement requires an else branch. \
         Use `else move on` if no action is needed, \
         or `else missing code` if the branch is not yet implemented. \
         Consider `when` instead of `if` if only the true path matters.",
        &if_stmt.location,
      );
    }
    let else_defs = self.end_branch();

    self.join_branch_defs(vec![then_defs, else_defs]);
    self.exit_construct();
  }

  /// Analyze a when statement: one-sided action with no else.
  fn visit_when(&mut self, when_stmt: &WhenStmt) {
    self.visit_condition(&when_stmt.condition);

    // Reject erroneous else branch with a helpful error message.
    // The grammar accepts `else` in `when` solely to prevent a cryptic
    // parser error; the semantic analyzer provides the user-facing message.
    if when_stmt.else_branch.is_some() {
      self.add_error(
        "`when` does not support an `else` branch. \
         Either use `if` instead of `when` to handle both paths, \
         or remove the `else`.",
        &when_stmt.location,
      );
      // Still analyze the else body so any type errors within it are reported
      if let Some(else_body) = &when_stmt.else_branch {
        for stmt in else_body {
          self.visit_stmt(stmt);
        }
      }
    }

    let polarity = !when_stmt.condition.negated;
    self.enter_construct();
    self.begin_branch();
    self.push_guard_frame(extract_facts(&when_stmt.condition.expr, polarity));
    for stmt in &when_stmt.body {
      self.visit_stmt(stmt);
    }
    self.pop_guard_frame();
    let body_defs = self.end_branch();
    // `when` is one-sided: the body may not execute, so any `var` defined in it
    // is not definitely assigned afterwards.
    self.join_branch_defs(vec![body_defs, BranchDefs::default()]);
    self.exit_construct();
  }

  /// Analyze a move on statement: intentional no-op.
  fn visit_move_on(&mut self, _stmt: &MoveOnStmt) {
    // Nothing to analyze — this is an intentional no-op.
  }

  /// Analyze a missing code statement: deferred implementation.
  /// In release mode, this is a compile error.
  fn visit_missing_code(&mut self, stmt: &MissingCodeStmt) {
    if !self.options.is_debug {
      self.add_error(
        "`missing code` is not allowed in release mode. \
         Replace this placeholder with an implementation before building with --release.",
        &stmt.location,
      );
    }
  }

  /// Visit release statement: `release ptr_expr` — validate the pointer expression.
  fn visit_release(&mut self, stmt: &ReleaseStmt) {
    self.visit_expr(&stmt.pointer);
    // An `on exit release` body is visited through visit_on_exit, where its
    // deferred-release tracking is handled separately. Skip immediate-release
    // tracking here so `on exit release` does not count as an explicit release.
    if self.in_on_exit_body {
      return;
    }
    // Track the released variable for missing-release detection.
    if let Expr::Identifier(ident) = &stmt.pointer
      && ident.path.len() == 1
    {
      let name = &ident.path[0];
      if self.allocated_vars_in_scope.contains_key(name) {
        // Double-free: already released explicitly or via `on exit release`.
        if self.released_vars.contains(name) {
          self.add_warning(
            format!("'{}' is released more than once (double free)", name),
            &stmt.location,
          );
        }
        self.released_vars.insert(name.clone());
        self.explicitly_released_vars.insert(name.clone());
      }
    }
  }

  /// Visit on-exit statement: `on exit <stmt>` — validate the deferred statement.
  fn visit_on_exit(&mut self, stmt: &OnExitStmt) {
    self.in_on_exit_body = true;
    self.visit_stmt(&stmt.body);
    self.in_on_exit_body = false;
    // Track on-exit releases for missing-release detection.
    if let Stmt::Release(release) = &*stmt.body
      && let Expr::Identifier(ident) = &release.pointer
      && ident.path.len() == 1
    {
      let name = &ident.path[0];
      if self.allocated_vars_in_scope.contains_key(name) {
        // Double-free: already released explicitly, so `on exit` would free again.
        if self.released_vars.contains(name) {
          self.add_warning(
            format!("'{}' is released more than once (double free)", name),
            &release.location,
          );
        }
        self.on_exit_released_vars.insert(name.clone());
        // on exit covers all paths, so also mark as explicitly released
        self.released_vars.insert(name.clone());
      }
    }
  }

  /// Visit allocate expression: `allocate(size_expr)` — returns pointer, validate size.
  fn visit_allocate(&mut self, alloc: &AllocateExpr) {
    self.visit_expr(&alloc.size);
  }

  /// Analyze a match statement: ordered conditional branching with multiple arms.
  fn visit_match(&mut self, match_stmt: &MatchStmt) {
    // Detect duplicate guards (dead arms).
    let mut seen_guards: Vec<&Condition> = Vec::new();
    let mut branch_defs: Vec<BranchDefs> = Vec::new();
    self.enter_construct();
    for arm in &match_stmt.arms {
      if seen_guards
        .iter()
        .any(|g| conditions_structurally_equal(g, &arm.guard))
      {
        self.add_error(
          "Duplicate match arm: this condition is identical to an earlier arm (unreachable code)",
          &arm.location,
        );
      }
      seen_guards.push(&arm.guard);

      self.visit_condition(&arm.guard);
      let polarity = !arm.guard.negated;
      self.begin_branch();
      self.push_guard_frame(extract_facts(&arm.guard.expr, polarity));
      for stmt in &arm.body {
        self.visit_stmt(stmt);
      }
      self.pop_guard_frame();
      branch_defs.push(self.end_branch());
    }
    self.begin_branch();
    for stmt in &match_stmt.else_arm {
      self.visit_stmt(stmt);
    }
    branch_defs.push(self.end_branch());

    self.join_branch_defs(branch_defs);
    self.exit_construct();
  }

  /// Analyze a switch statement: exhaustive pattern matching over enum values,
  /// or literal value dispatch over open (non-enum) types.
  ///
  /// Validates:
  ///
  /// - The switch target is an enum or a value type
  /// - Enum cases reference valid variants, are exhaustive, and reject dead defaults
  /// - Value cases are literals whose type and unit match the scrutinee
  /// - Open (non-enum) switches require a `default` arm
  /// - Duplicate case values are rejected
  fn visit_switch(&mut self, switch_stmt: &SwitchStmt) {
    // Visit the switch expression
    self.visit_expr(&switch_stmt.value);

    let scrutinee_type = self.expr_type(&switch_stmt.value);
    if scrutinee_type == "unknown" {
      // Can't determine the switch expression's type — a prior error likely
      // prevented type inference. Skip switch-specific validation to avoid
      // cascading false errors, but still analyze the case bodies.
      for case in &switch_stmt.cases {
        for stmt in &case.body {
          self.visit_stmt(stmt);
        }
      }
      if let Some(default_body) = &switch_stmt.default_case {
        for stmt in default_body {
          self.visit_stmt(stmt);
        }
      }
      return;
    }

    let enum_variants = self.symbols.get_enum_variants(&scrutinee_type);
    if !enum_variants.is_empty() {
      self.visit_enum_switch(switch_stmt, &scrutinee_type, enum_variants);
    } else {
      self.visit_value_switch(switch_stmt, &scrutinee_type);
    }
  }

  /// Analyze loop variable, conditions, and body.
  fn visit_loop(&mut self, loop_stmt: &LoopStmt) {
    if let Some(range) = &loop_stmt.range {
      self.visit_range(range);
      // Variable registration is handled by the synthetic VarDefStmt injected by the AST builder
    }

    if let Some(cond) = &loop_stmt.pre_condition {
      self.visit_condition(cond);
    }

    // A loop body may execute zero times (entry-condition loops, counted loops
    // with a non-positive range, etc.), so any `var` defined only in the body is
    // not definitely assigned after the loop.
    self.enter_construct();
    self.begin_branch();
    for stmt in &loop_stmt.body {
      self.visit_stmt(stmt);
    }
    let body_defs = self.end_branch();
    self.join_branch_defs(vec![body_defs, BranchDefs::default()]);
    self.exit_construct();

    if let Some(cond) = &loop_stmt.post_condition {
      self.visit_condition(cond);
    }
  }

  /// Analyze return value expression.
  fn visit_return(&mut self, ret: &ReturnStmt) {
    let expected_type = self.current_return_type.clone();

    match (&ret.value, &expected_type) {
      (Some(value), Some(expected_type)) => {
        // Return with a value - check type compatibility
        self.visit_expr(value);

        self.check_escaping_pointer_in_return(value, &ret.location);

        let expr_u = self.expr_unit(value);

        if let Some(expected) = &self.current_return_unit.clone()
          && !ExprUnit::same_unit(expected, &expr_u)
        {
          self.add_error(
            format!(
              "Return unit mismatch: function declared to return '{}' but expression has '{}'",
              expected.display(),
              expr_u.display()
            ),
            &ret.location,
          );
        }

        // Type checking for return value
        let value_type = self.expr_type(value);
        let is_nothing_expr = matches!(value, Expr::NothingExpr);
        if value_type != "unknown" && expected_type != "nothing" {
          // If returning optional T?, the value can be concrete T (auto-wrapped)
          if !TypeInference::types_compatible(expected_type, &value_type) {
            self.add_error(
              format!(
                "Return type mismatch: function declared to return '{}' but expression has type '{}'",
                expected_type, value_type
              ),
              &ret.location,
            );
          }
        } else if expected_type == "nothing" && !is_nothing_expr {
          self.add_error(
            format!(
              "Function returns 'nothing' but a value of type '{}' was provided in return statement",
              value_type
            ),
            &ret.location,
          );
        }
      }
      (Some(value), None) => {
        // No expected type (shouldn't happen in valid code)
        self.visit_expr(value);
        self.check_escaping_pointer_in_return(value, &ret.location);
      }
      (None, Some(expected_type)) => {
        // Bare `return` (no value) - only valid when returning `nothing` or `T?`
        if expected_type != "nothing" && !TypeInference::is_optional_type(expected_type) {
          self.add_error(
            format!(
              "Function returns '{}' but return statement has no value. Use 'return nothing' for absences.",
              expected_type
            ),
            &ret.location,
          );
        }
      }
      (None, None) => {
        // No expected type and no return value - no checks needed
      }
    }
  }

  /// Exit program statement (no semantic analysis needed).
  fn visit_exit_program(&mut self, _exit_program: &ExitProgramStmt) {}

  /// Exit loop statement (no semantic analysis needed).
  fn visit_exit_loop(&mut self, _exit_loop: &ExitLoopStmt) {}

  /// Rewind statement (no semantic analysis needed).
  fn visit_rewind(&mut self, _rewind: &RewindStmt) {}

  /// Analyze stdout expression.
  fn visit_stdout(&mut self, stdout: &StdoutStmt) {
    self.visit_output_expr(&stdout.value, &stdout.location, "write");
  }

  /// Analyze stderr expression.
  fn visit_stderr(&mut self, stderr: &StderrStmt) {
    self.visit_output_expr(&stderr.value, &stderr.location, "warn");
  }

  fn visit_log(&mut self, log: &LogStmt) {
    self.visit_output_expr(&log.value, &log.location, "log");
  }

  fn visit_debug(&mut self, debug: &DebugStmt) {
    self.visit_expr(&debug.value);
    self.check_expr_type_is_concrete(&debug.value, &debug.location, "debug");
  }

  /// Stdin statement: validates the target is of type text.
  /// The variable is defined by a synthetic VarDefStmt injected by the AST builder.
  fn visit_stdin(&mut self, stdin: &StdinStmt) {
    let var_name = stdin.target.node.join(".");
    if let Some(symbol) = self.symbols.lookup_var_symbol(&var_name).unwrap_or(None) {
      if symbol.data_type != "text" {
        self.add_error(
          format!(
            "read requires a text variable, but '{}' has type {}",
            var_name, symbol.data_type
          ),
          &stdin.target.span,
        );
      }
    } else {
      ice!(
        "read target '{}' was not pre-defined by synthetic VarDefStmt — this is a compiler bug",
        var_name
      );
    }
  }

  /// Analyze compile-time if/else-if/else statement.
  fn visit_ct_if(&mut self, ct_if: &CtIfStmt) {
    match self.eval_const_condition(&ct_if.condition) {
      Some(true) => {
        for stmt in &ct_if.then_branch {
          self.visit_stmt(stmt);
        }
      }
      Some(false) => {
        let mut branch_taken = false;
        for (cond, branch) in &ct_if.else_if_branches {
          match self.eval_const_condition(cond) {
            Some(true) => {
              for stmt in branch {
                self.visit_stmt(stmt);
              }
              branch_taken = true;
              break;
            }
            Some(false) => {}
            None => {
              self.report_ct_condition_error(cond);
              for stmt in branch {
                self.visit_stmt(stmt);
              }
              branch_taken = true;
              break;
            }
          }
        }

        if !branch_taken && let Some(else_branch) = &ct_if.else_branch {
          for stmt in else_branch {
            self.visit_stmt(stmt);
          }
        }
      }
      None => {
        // Preserve the detailed #if non-boolean hint, and reject unevaluable conditions.
        if is_non_boolean_condition(&ct_if.condition) {
          self.errors.push(SemanticError::new(
            "Compile-time condition must be a boolean expression. \
             Integer literals and arithmetic expressions require explicit comparison. \
             Use '#if x > 0' instead of '#if x'"
              .to_string(),
            ct_if.condition.location.clone(),
          ));
        } else {
          self.errors.push(SemanticError::new(
            "Compile-time condition must be evaluable at compile time (no runtime values)."
              .to_string(),
            ct_if.condition.location.clone(),
          ));
        }

        // Still analyze all branches to surface any additional errors.
        for stmt in &ct_if.then_branch {
          self.visit_stmt(stmt);
        }
        for (_cond, branch) in &ct_if.else_if_branches {
          for stmt in branch {
            self.visit_stmt(stmt);
          }
        }
        if let Some(else_branch) = &ct_if.else_branch {
          for stmt in else_branch {
            self.visit_stmt(stmt);
          }
        }
      }
    }
  }

  /// Compile-time fail: record error without printing (will be printed at end).
  fn visit_ct_fail(&mut self, ct_fail: &CtFailStmt) {
    self.errors.push(SemanticError::new(
      format!("Compile-time failure: {}", ct_fail.message),
      ct_fail.location.clone(),
    ));
  }

  /// Compile-time warn: record warning without printing (will be printed at end).
  fn visit_assert(&mut self, assert: &AssertStmt) {
    self.visit_expr(&assert.condition);
    let ct = self.expr_type(&assert.condition);
    if ct != "bool" {
      self.add_error(
        format!("Assert condition must be boolean, got '{}'", ct),
        &assert.location,
      );
    }
  }

  fn visit_ct_warn(&mut self, ct_warn: &CtWarnStmt) {
    self.warnings.push(SemanticError::new(
      ct_warn.message.clone(),
      ct_warn.location.clone(),
    ));
  }

  /// Analyze a compile-time when statement.
  fn visit_ct_when(&mut self, ct_when: &CtWhenStmt) {
    match self.eval_const_condition(&ct_when.condition) {
      Some(true) => {
        for stmt in &ct_when.body {
          self.visit_stmt(stmt);
        }
      }
      Some(false) => {
        // Body is removed at compile time — nothing to analyze.
      }
      None => {
        self.report_ct_condition_error(&ct_when.condition);
        // Still analyze the body to surface any additional errors.
        for stmt in &ct_when.body {
          self.visit_stmt(stmt);
        }
      }
    }
  }

  /// Analyze a compile-time match statement (ordered condition chain).
  fn visit_ct_match(&mut self, ct_match: &CtMatchStmt) {
    // Detect duplicate guards (dead arms).
    let mut seen_guards: Vec<&Condition> = Vec::new();
    for arm in &ct_match.arms {
      if seen_guards
        .iter()
        .any(|g| conditions_structurally_equal(g, &arm.guard))
      {
        self.add_error(
          "Duplicate '#match' arm: this condition is identical to an earlier arm (unreachable code)",
          &arm.location,
        );
      }
      seen_guards.push(&arm.guard);
    }

    let mut taken = false;
    for arm in &ct_match.arms {
      match self.eval_const_condition(&arm.guard) {
        Some(true) => {
          for stmt in &arm.body {
            self.visit_stmt(stmt);
          }
          taken = true;
          break;
        }
        Some(false) => {}
        None => {
          self.report_ct_condition_error(&arm.guard);
          // Still analyze this arm to surface any additional errors.
          for stmt in &arm.body {
            self.visit_stmt(stmt);
          }
          taken = true;
          break;
        }
      }
    }
    if !taken {
      for stmt in &ct_match.else_arm {
        self.visit_stmt(stmt);
      }
    }
  }

  /// Analyze a compile-time switch statement (literal value dispatch).
  fn visit_ct_switch(&mut self, ct_switch: &CtSwitchStmt) {
    match self.eval_const_value(&ct_switch.value) {
      Some(value) => {
        // Validate all case values: reject unevaluable and duplicate cases.
        let mut seen: std::collections::HashSet<ConstValue> = std::collections::HashSet::new();
        for case in &ct_switch.cases {
          match self.eval_const_value(&case.value) {
            Some(case_value) => {
              if !seen.insert(case_value.clone()) {
                self.errors.push(SemanticError::new(
                  format!("Duplicate case value in '#switch': {:?}", case_value),
                  case.location.clone(),
                ));
              }
            }
            None => {
              self.errors.push(SemanticError::new(
                "Compile-time switch case value must be a compile-time constant \
                 (bool, int, or str)."
                  .to_string(),
                case.location.clone(),
              ));
            }
          }
        }

        // Find the first matching case and analyze only its body (dead cases
        // are not type-checked).
        let mut matched = false;
        for case in &ct_switch.cases {
          if let Some(case_value) = self.eval_const_value(&case.value)
            && case_value == value
          {
            for stmt in &case.body {
              self.visit_stmt(stmt);
            }
            matched = true;
            break;
          }
        }
        if !matched {
          for stmt in &ct_switch.default_case {
            self.visit_stmt(stmt);
          }
        }
      }
      None => {
        self.errors.push(SemanticError::new(
          "Compile-time switch value must be a compile-time constant \
           (bool, int, or str)."
            .to_string(),
          ct_switch.location.clone(),
        ));
        // Still analyze all cases to surface any additional errors.
        for case in &ct_switch.cases {
          for stmt in &case.body {
            self.visit_stmt(stmt);
          }
        }
        for stmt in &ct_switch.default_case {
          self.visit_stmt(stmt);
        }
      }
    }
  }

  /// Doc comment (no semantic analysis needed).
  fn visit_doc(&mut self, _doc: &DocStmt) {}

  /// Code comment (no semantic analysis needed).
  fn visit_comment(&mut self, _comment: &CommentStmt) {}

  /// Track when a `T?` variable is checked with `has value`.
  fn visit_has_value(&mut self, expr: &Expr) {
    self.visit_expr(expr);
    self.record_optional_check(expr);
  }

  /// Track when a `T?` variable is checked with `has no value`.
  fn visit_has_no_value(&mut self, expr: &Expr) {
    self.visit_expr(expr);
    self.record_optional_check(expr);
  }

  /// Try-propagate expression: `expr?`.
  fn visit_try_propagate(&mut self, expr: &Expr) {
    self.visit_expr(expr);
    let operand_type = self.expr_type(expr);
    if !TypeInference::is_optional_type(&operand_type) && operand_type != "unknown" {
      self.add_error(
        format!(
          "`?` operator requires an optional type (T?), got: {}",
          operand_type
        ),
        &self.expr_location(expr),
      );
      return;
    }
    // Check that propagation is valid: must be inside a function returning T? or at top level
    let is_top_level = self.symbols.current_scope() == &VarScope::Global;
    let in_optional_fn = self
      .current_return_type
      .as_ref()
      .map(|t| TypeInference::is_optional_type(t))
      .unwrap_or(false);
    if !is_top_level && !in_optional_fn {
      self.add_error(
        "`?` operator can only be used in a function that returns T? or at the top level"
          .to_string(),
        &self.expr_location(expr),
      );
    }
  }

  /// Analyze both operands of binary operation.
  /// Note: Unit checking is done via expr_unit() called from visit_var_def, visit_return, etc.
  fn visit_binary(&mut self, bin: &BinaryExpr) {
    self.visit_expr(&bin.left);
    self.visit_expr(&bin.right);
    self.check_binary_operand_types(bin);

    // Division-by-zero guard check
    if matches!(bin.operator, BinaryOp::Div | BinaryOp::Mod) && !self.is_guarded_nonzero(&bin.right)
    {
      let op_name = if matches!(bin.operator, BinaryOp::Div) {
        "division"
      } else {
        "modulo"
      };
      self.add_warning(
        format!(
          "potential {} by zero: divisor '{}' is not provably non-zero. \
           Consider adding a guard such as `when {} != 0` (or a `match` arm guard).\n           note: only structurally identical expressions in surrounding guard conditions are checked.",
          op_name,
          ExpressionAnalyzer::expr_to_string(&bin.right),
          ExpressionAnalyzer::expr_to_string(&bin.right)
        ),
        &bin.location,
      );
    }
  }

  /// Analyze unary operation operand.
  /// Note: Unit checking is done via expr_unit() called from visit_var_def, visit_return, etc.
  fn visit_unary(&mut self, un: &UnaryExpr) {
    // Validate UnsafeBitcast: Due to Pratt parser precedence, the AST structure for
    // `value at p unsafe bitcast` is: ValueAt(UnsafeBitcast(Identifier))
    // So when we visit UnsafeBitcast, it should be inside a ValueAt (handled by the parent).
    // We just need to check that UnsafeBitcast's operand is an identifier (the pointer).
    if matches!(un.operator, UnaryOp::UnsafeBitcast) {
      match &*un.operand {
        Expr::Identifier(_) | Expr::MemberAccess(_) => {
          // Valid: `unsafe bitcast` applied to a pointer identifier or struct field access.
          // Member access like `mode.ptr` is a common FFI pattern for accessing
          // pointer fields on structs returned from C functions.
          self.visit_expr(&un.operand);
        }
        Expr::Conversion(conv) => {
          // New: `unsafe bitcast expr as T` — explicit numeric bit reinterpretation.
          // The `as T` supplies the target type; `unsafe bitcast` turns it into a
          // bitcast instead of a value conversion. We must NOT recurse through
          // `visit_conversion`, which would reject a lossy signed↔unsigned
          // conversion — `unsafe bitcast` is the explicit opt-in for that.
          self.visit_unsafe_bitcast_conversion(conv, &un.location);
        }
        _ => {
          self.add_error(
            "`unsafe bitcast` must be applied to a pointer variable in `value at ptr unsafe bitcast` pattern, or to a conversion `unsafe bitcast expr as T`".to_string(),
            &un.location,
          );
          self.visit_expr(&un.operand);
        }
      }
    } else if matches!(un.operator, UnaryOp::ValueAt) {
      // Use-after-free: dereferencing a pointer that was already released.
      if let Some(name) = Self::pointer_identifier_name(&un.operand)
        && self.explicitly_released_vars.contains(name)
      {
        self.add_warning(
          format!(
            "`value at {}` used after `release {}` (use-after-free)",
            name, name
          ),
          &un.location,
        );
      }
      // Check if this is a `value at (unsafe bitcast p)` pattern - validate that unsafe bitcast is used correctly
      if let Expr::Unary(inner) = &*un.operand
        && matches!(inner.operator, UnaryOp::UnsafeBitcast)
      {
        // This is the `value at p unsafe bitcast` pattern (AST: ValueAt -> UnsafeBitcast -> Identifier)
        // Validate bit-width and units, inferring target type from expression context.
        let value_at_expr = Expr::Unary(un.clone());
        let target_type = self.expr_type(&value_at_expr);
        self.validate_unsafe_bitcast_compatibility(&target_type, &value_at_expr, &un.location);
        self.visit_expr(&un.operand);
        return;
      }
      // Regular `value at` without unsafe bitcast - operand must be a pointer type
      let operand_type = self.expr_type(&un.operand);
      if operand_type != "pointer" && operand_type != "unknown" {
        self.add_warning(
          format!(
            "value at operator expects pointer type, got: {}",
            operand_type
          ),
          &un.location,
        );
      }
      self.visit_expr(&un.operand);
    } else if matches!(un.operator, UnaryOp::PointerTo) {
      // Pointer-to operator: creates a pointer from an lvalue
      // Any expression can be made into a pointer (though const values are tricky)
      //
      // Taking a variable's address makes it mutable through that pointer, so it
      // can no longer be treated as a compile-time constant for div-zero analysis.
      if let Some(name) = lvalue_root_name(&un.operand) {
        self.symbols.mark_variable_shared(&name);
      }
      self.visit_expr(&un.operand);
    } else if matches!(un.operator, UnaryOp::ValueOf) {
      // `value of expr`: unwraps an optional type
      // The operand must be an optional type (T?).
      let operand_type = self.expr_type(&un.operand);
      if !TypeInference::is_optional_type(&operand_type) && operand_type != "unknown" {
        self.add_error(
          format!(
            "`value of` requires an optional type (T?), got: {}",
            operand_type
          ),
          &un.location,
        );
      }
      // Warn if `value of` is used on a T? variable without a visible
      // `has value` / `has no value` check in the same scope.
      self.check_unguarded_value_of(&un.operand, &un.location);
      self.visit_expr(&un.operand);
    } else if matches!(
      un.operator,
      UnaryOp::TypeOf | UnaryOp::SizeOf | UnaryOp::UnitOf
    ) {
      // Query operators: valid on any expression
      self.visit_expr(&un.operand);
    } else if un.operator == UnaryOp::CountOf {
      // `#count of` is only valid on arrays.
      let operand_type = self.expr_type_with_context(&un.operand, None);
      if !operand_type.contains('[') {
        self.add_error(
          format!(
            "`#count of` requires an array, got: {}. Use `text.chars` or `text.bytes` for text length.",
            operand_type
          ),
          &un.location,
        );
      }
      self.visit_expr(&un.operand);
    } else {
      // Other unary operators (Neg, Not, Invert, etc.)
      self.visit_expr(&un.operand);
    }
  }

  /// Analyze type conversion operand.
  ///
  /// Type conversion rules:
  /// 1. Units are preserved (not stripped)
  /// 2. Only widening numeric conversions allowed (e.g., i32→i64, f32→f64)
  /// 3. Narrowing conversions are rejected (e.g., f64→f32, i64→i32)
  /// 4. Incompatible type conversions are rejected (str↔numeric, bool↔numeric)
  /// 5. EXCEPTION: Numeric literals can convert to any numeric type if they fit in range
  fn visit_conversion(&mut self, conv: &ConversionExpr) {
    self.visit_expr(&conv.operand);
    // Check target type for backend restrictions
    self.visit_type_name(&conv.target_type);

    let target_type_str = TypeInference::type_name_to_string(&conv.target_type);

    // SPECIAL CASE: integer literals with a known compile-time value
    // These can convert to any numeric type as long as the value fits in range.
    // This allows `1 as u8` to work without being rejected as "narrowing".
    // Float literals are excluded: converting a float to an integer type is a
    // narrowing that drops the fractional part (same rule as a float variable).
    if TypeChecker::is_numeric_valued(&conv.operand)
      && !TypeChecker::is_float_literal(&conv.operand)
      && let Some(int_value) = UnitAnalyzer::try_extract_int_value(&conv.operand)
    {
      // Check if target is an integer numeric type
      if let TypeCategory::Numeric(ref target_info) =
        TypeValidator::get_type_category(&target_type_str)
        && !target_info.is_float
        && let Some((min, max)) = TypeValidator::get_numeric_int_range(target_info)
      {
        let v = int_value as i128;
        if v < min || v > max {
          // Literal doesn't fit into target type
          self.add_error(
            format!(
              "Literal value {} is out of range for type '{}' (valid range: {}..{})",
              int_value, target_type_str, min, max
            ),
            &conv.location,
          );
        }
        // Value fits in range: allow conversion (bypass generic narrowing check)
        return;
      }

      // Check if this numeric literal is being converted to char — validate Unicode range
      if target_type_str == "char" {
        let v = int_value as u32;
        if v > 0x10FFFF {
          self.add_error(
            format!(
              "Literal value {} is out of range for type 'char' (valid Unicode: 0x00000000..0x10FFFF)",
              int_value
            ),
            &conv.location,
          );
        }
        return;
      }
    }

    // Fallback: generic conversion validation.
    // Integer literals infer their type from the target type for validation.
    // Float literals may infer to a *float* target (precision loss is by design)
    // but are validated as `f64` against an integer target so `x as i32` is
    // rejected (float → integer drops the fractional part).
    let source_type = if TypeChecker::is_float_literal(&conv.operand) {
      if matches!(target_type_str.as_str(), "f16" | "f32" | "f64") {
        target_type_str.clone()
      } else {
        "f64".to_string()
      }
    } else if matches!(
      &*conv.operand,
      Expr::IntLiteral(_) | Expr::UintLiteral(_) | Expr::HexLiteral(_)
    ) {
      // For integer literals in conversions, infer type from target if numeric or pointer.
      if matches!(
        TypeValidator::get_type_category(&target_type_str),
        TypeCategory::Numeric(_)
      ) || target_type_str == "pointer"
      {
        target_type_str.clone() // Infer literal type from target for validation
      } else {
        self.expr_type(&conv.operand)
      }
    } else {
      self.expr_type(&conv.operand)
    };

    TypeValidator::validate_type_conversion(
      &source_type,
      &target_type_str,
      &conv.location,
      &mut self.errors,
    );
  }

  /// Check if identifier is defined.
  /// Handles both unqualified names (add) and qualified paths (math -> lib -> add).
  /// If a qualified path is used, it must match exactly.
  /// If an unqualified name is used, it must be unambiguous across imported modules.
  fn visit_identifier(&mut self, id: &IdentifierExpr) {
    // `π` is a reserved built-in constant, not a user symbol. Skip the
    // definition/ambiguity checks that would otherwise flag it as undefined.
    if id.is_pi() {
      return;
    }

    let name = id.name();

    // Check for post-branch access of arm-bound variables
    if self.branch_only_vars.contains_key(name) {
      self.add_error(
        format!(
          "Variable '{}' is only defined in some branches, so it is not definitely assigned here. \
           Define it in every branch with the same type and unit before using it afterwards.",
          name
        ),
        &id.location,
      );
      return;
    }

    if id.is_qualified() {
      // Qualified identifier: must exist with the exact module path
      // For now, just check that the unqualified name exists
      // Full module path checking will be implemented with extended symbol tables
      if !self.symbols.is_variable_defined(name) {
        // Check if this is a zero-argument function (e.g., enum variant via Shape -> Point)
        if let Some(fn_info) = self.symbols.lookup_function(name) {
          if fn_info.parameters.is_empty() {
            // Valid: zero-argument function call (enum variant constructor).
          } else {
            self.add_error(
              format!("Symbol '{}' not found", id.full_path()),
              &id.location,
            );
          }
        } else {
          self.add_error(
            format!("Symbol '{}' not found", id.full_path()),
            &id.location,
          );
        }
      } else {
        self.mark_symbol_used(name);
        let symbol_location = self
          .symbols
          .lookup_var_symbol(name)
          .unwrap_or(None)
          .map(|s| s.source_location.clone());
        if let Some(location) = symbol_location {
          self.mark_symbol_location_used(&location);
        }
      }
    } else {
      // Unqualified identifier: check for ambiguity
      if !self.symbols.is_variable_defined(name) {
        // Check if this is a zero-argument function (e.g., enum variant constructor
        // or a suite function referenced as an identifier).
        if let Some(fn_info) = self.symbols.lookup_function(&self.resolve_fn_name(name)) {
          if fn_info.parameters.is_empty() {
            // Valid: zero-argument function call (enum variant constructor).
            // The IR generator will emit a function call.
          } else {
            self.add_error(format!("Undefined variable '{}'", name), &id.location);
          }
        } else {
          self.add_error(format!("Undefined variable '{}'", name), &id.location);
        }
      } else {
        // Check for ambiguity - if multiple modules provide the same symbol,
        // require the user to use a qualified path.
        let all_symbols = self
          .symbols
          .lookup_all_var_symbols(name)
          .unwrap_or_default();

        // Filter to only imported symbols from different modules
        let mut imported_modules = std::collections::HashSet::new();
        for symbol in &all_symbols {
          if symbol.linkage == Linkage::Import && !symbol.module_path.is_empty() {
            imported_modules.insert(symbol.module_path.clone());
          }
        }

        if imported_modules.len() > 1 {
          let module_list = imported_modules
            .iter()
            .map(|m| format!("'{}'", m))
            .collect::<Vec<_>>()
            .join(", ");
          self.add_error(
            format!(
              "Ambiguous symbol '{}': imported from multiple modules ({}). Use qualified path (e.g., 'module.{}')",
              name, module_list, name
            ),
            &id.location,
          );
        } else {
          self.mark_symbol_used(name);

          // Also try to mark the specific definition location if we can find it
          let symbol_location = self
            .symbols
            .lookup_var_symbol(name)
            .unwrap_or(None)
            .map(|s| s.source_location.clone());
          if let Some(location) = symbol_location {
            self.mark_symbol_location_used(&location);
          }
        }
      }
    }
  }

  /// Numeric literals require no semantic analysis.
  fn visit_int_literal(&mut self, _lit: &IntLiteral) {}
  fn visit_uint_literal(&mut self, _lit: &UintLiteral) {}
  fn visit_float_literal(&mut self, _lit: &FloatLiteral) {}
  fn visit_hex_literal(&mut self, _lit: &HexLiteral) {}
  fn visit_char_literal(&mut self, _lit: &CharLiteral) {}
  fn visit_bool_literal(&mut self, _lit: &BoolLiteral) {}

  /// Analyze expressions inside string embedded values.
  fn visit_string_literal(&mut self, lit: &StringLiteral) {
    for part in &lit.parts {
      if let StringPart::EmbeddedValue(expr) = part {
        self.visit_expr(expr);
      }
    }
  }

  /// Analyze all array elements.
  fn visit_array_literal(&mut self, lit: &ArrayLiteral) {
    for elem in &lit.elements {
      self.visit_expr(elem);
    }
  }

  /// Analyze function call arguments and check parameter units.
  fn visit_fn_call_expr(&mut self, fn_call: &FnCall) {
    // 'main' and '_start' are entry points and cannot be called by user code
    if let Some(fn_name) = fn_call.target.node.last() {
      match fn_name.as_str() {
        "main" | "_start" => {
          self.add_error(
            format!("'{}' is the entry point and cannot be called by user code. It is invoked by the runtime automatically.", fn_name),
            &fn_call.location,
          );
        }
        "text" => {
          // Built-in text constructor must have exactly 3 arguments (pointer, bytes, chars)
          if fn_call.arguments.len() != 3 {
            self.add_error(
              format!(
                "Built-in text() constructor expects 3 arguments (pointer, bytes, chars) but got {}",
                fn_call.arguments.len()
              ),
              &fn_call.location,
            );
          }
        }
        "binary" => {
          // Built-in binary constructor must have exactly 2 arguments (pointer, bytes)
          if fn_call.arguments.len() != 2 {
            self.add_error(
              format!(
                "Built-in binary() constructor expects 2 arguments (pointer, bytes) but got {}",
                fn_call.arguments.len()
              ),
              &fn_call.location,
            );
          }
        }
        "vec2" => {
          if fn_call.arguments.len() != 2 {
            self.add_error(
              format!(
                "vec2 constructor expects 2 arguments but got {}",
                fn_call.arguments.len()
              ),
              &fn_call.location,
            );
          }
        }
        "vec3" => {
          if fn_call.arguments.len() != 3 {
            self.add_error(
              format!(
                "vec3 constructor expects 3 arguments but got {}",
                fn_call.arguments.len()
              ),
              &fn_call.location,
            );
          }
        }
        "vec4" if fn_call.arguments.len() != 4 => {
          self.add_error(
            format!(
              "vec4 constructor expects 4 arguments but got {}",
              fn_call.arguments.len()
            ),
            &fn_call.location,
          );
        }
        _ => {}
      }
    }

    for arg in &fn_call.arguments {
      self.visit_expr(arg);
    }
    self.check_fn_call_param_units(fn_call);
    self.invalidate_ref_passed_vars(fn_call);

    // Validate module-qualified calls (`math.double`): the first path segment must
    // be a module that exports the function. Enum-variant access (`Shape.Point`)
    // is excluded — its first segment is a type, not a module.
    if fn_call.target.node.len() > 1 {
      let module_name = &fn_call.target.node[0];
      let fn_simple = &fn_call.target.node[1];
      if self.symbols.lookup_type(module_name).is_none()
        && self.module_export_type(module_name, fn_simple).is_none()
      {
        self.add_error(
          format!(
            "Symbol '{}' is not exported from module '{}'",
            fn_simple, module_name
          ),
          &fn_call.location,
        );
      }
    }

    // Mark function as used (resolving a suite function name if present).
    if let Some(fn_name) = fn_call.target.node.last() {
      let resolved = self.resolve_fn_name(fn_name);
      self.mark_symbol_used(&resolved);
    }
  }

  /// Analyze the object being accessed.
  fn visit_member_access(&mut self, acc: &MemberAccess) {
    // A `.` on a type name is enum-variant access (`Shape.Point`), not field
    // access. Resolve it directly; report an error if the member is not a
    // zero-argument variant of that type.
    if let Expr::Identifier(id) = &*acc.object {
      let name = id.name().to_string();
      if self.symbols.lookup_type(&name).is_some() {
        if self
          .enum_variant_access_type(&acc.object, &acc.member.node)
          .is_some()
        {
          // Valid zero-argument enum-variant access.
          return;
        }
        self.add_error(
          format!("Cannot access '{}' on type '{}'", acc.member.node, name),
          &acc.location,
        );
        return;
      }
    }

    // A `.` on a module name is module-qualified access (`math.e`).
    if self
      .module_member_access_type(&acc.object, &acc.member.node)
      .is_some()
    {
      self.mark_symbol_used(&acc.member.node);
      return;
    }

    // Normal field access on a value.
    self.visit_expr(&acc.object);

    // Check private field access for reads
    let object_type = self.expr_type(&acc.object);
    if object_type != "text"
      && object_type != "unknown"
      && let Some(type_info) = self.symbols.lookup_type(&object_type)
    {
      for (tf_name, _tf_type, _tf_unit, is_private) in &type_info.fields {
        if tf_name == &acc.member.node && *is_private {
          let current_module = self
            .current_module_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().to_string())
            // When module path has no file stem, treat as non-matching for private access.
            .unwrap_or_default();
          if current_module != type_info.module_path {
            self.add_error(
              format!(
                "Cannot access private field '{}' of type '{}' from outside its defining module '{}'",
                acc.member.node, object_type, type_info.module_path
              ),
              &acc.location,
            );
          }
        }
      }
    }
  }

  /// Analyze array and all index expressions.
  fn visit_array_index(&mut self, idx: &ArrayIndex) {
    self.visit_expr(&idx.array);

    // Try to get the array type to check bounds
    let array_type = self.expr_type(&idx.array);

    for (dim_idx, index) in idx.indices.iter().enumerate() {
      // Check for 1-based indexing violations: literal 0 or negative indices
      match index {
        Expr::IntLiteral(int_lit) => {
          if int_lit.value <= 0 {
            self.add_error(
              format!(
                "Array index must be >= 1 (1-based indexing): got literal {}. Use arr[1] for the first element, not arr[0].",
                int_lit.value
              ),
              &idx.location,
            );
          }

          // Check upper bounds if we can extract dimension size from the array type
          if let Some(dimension_size) = self.extract_dimension_from_type(&array_type, dim_idx)
            && int_lit.value > dimension_size as i64
          {
            self.add_error(
                format!(
                  "Array index out of bounds: dimension {} has size {} but got literal {}. Valid range is 1..{}.",
                  dim_idx + 1, dimension_size, int_lit.value, dimension_size
                ),
                &idx.location,
              );
          }
        }
        Expr::UintLiteral(uint_lit) if uint_lit.value == 0 => {
          // Unsigned literals: 0 is invalid (must be >= 1)
          self.add_error(
            "Array index must be >= 1 (1-based indexing): got literal 0. Use arr[1] for the first element, not arr[0].".to_string(),
            &idx.location,
          );
        }
        Expr::UintLiteral(uint_lit) => {
          // Check upper bounds if we can extract dimension size from the array type
          if let Some(dimension_size) = self.extract_dimension_from_type(&array_type, dim_idx)
            && uint_lit.value > dimension_size as u64
          {
            self.add_error(
                format!(
                  "Array index out of bounds: dimension {} has size {} but got literal {}. Valid range is 1..{}.",
                  dim_idx + 1, dimension_size, uint_lit.value, dimension_size
                ),
                &idx.location,
              );
          }
        }
        _ => {}
      }
      self.visit_expr(index);
    }
  }

  /// Compiler constant (no semantic analysis needed).
  fn visit_compiler_const(&mut self, cc: &CompilerConst) {
    if let CompilerConstKind::Function = cc.kind
      && self.symbols.current_scope() == &VarScope::Global
    {
      self.add_error(
        "`#function_name` is only available inside a function, not at global scope",
        &cc.location,
      );
    }
  }

  /// Nothing expression: no analysis needed.
  fn visit_nothing_expr(&mut self) {}

  /// Has errors expression: no analysis needed.
  fn visit_has_errors(&mut self) {}

  /// Last error expression: no analysis needed.
  fn visit_last_error(&mut self) {}

  /// Add error statement: analyze the value expression.
  fn visit_add_error(&mut self, add_error: &AddErrorStmt) {
    self.visit_expr(&add_error.value);
  }

  /// Alert error messages statement: no analysis needed.
  fn visit_alert_errors(&mut self, _alert_errors: &AlertErrorsStmt) {}

  /// Alert output statement: analyze the value expression.
  fn visit_alert_stmt(&mut self, alert: &AlertStmt) {
    self.visit_expr(&alert.value);
  }

  /// Type name validation: checks for backend-specific restrictions.
  fn visit_type_name(&mut self, _type_name: &TypeName) {}

  /// Parameter (analyzed via enter_function).
  fn visit_parameter(&mut self, _param: &Parameter) {}

  /// Analyze condition expression: must be boolean and unitless.
  fn visit_condition(&mut self, cond: &Condition) {
    self.visit_expr(&cond.expr);

    let expr_type = self.expr_type(&cond.expr);
    if expr_type != "bool" {
      self.add_error(
        format!("Condition must be boolean; got '{}'", expr_type),
        &cond.location,
      );
    }

    let u = self.expr_unit(&cond.expr);
    if matches!(u, ExprUnit::Unit(_)) {
      self.add_error(
        format!("Condition must be unitless; got {}", u.display()),
        &cond.location,
      );
    }
  }

  /// Analyze range bounds and step expressions.
  /// The shadowing check is handled by visit_var_var on the synthetic VarDefStmt.
  fn visit_range(&mut self, range: &Range) {
    self.visit_expr(&range.from);
    self.visit_expr(&range.to);
    if let Some(step) = &range.step {
      self.visit_expr(step);
      // Validate step type matches loop variable type.
      // Use context-aware type inference so float literals like 0.5 can match
      // both f32 and f64 loop variable types.
      let var_type_str =
        crate::semantic_analysis::type_compatibility::TypeInference::type_name_to_string(
          &range.var_type,
        );
      let step_type = self.expr_type_with_context(step, Some(&var_type_str));
      if step_type != var_type_str && step_type != "unknown" {
        self.add_error(
          format!(
            "Step type mismatch: loop variable '{}' is of type '{}' but step expression has type '{}'. \
             The step type must match the loop variable type (e.g., use `step 2` for i32, or change the loop variable to a float type for `step 0.5`).",
            range.variable.node, var_type_str, step_type
          ),
          &range.location,
        );
      }
    }
  }
}

/// Check if operand types are compatible for the given binary operation.
fn check_binary_operand_types_impl(analyzer: &mut SemanticAnalyzer, bin: &BinaryExpr) {
  // Get the current assignment type context (if being checked within an assignment)
  let assignment_context = analyzer.current_assignment_type.clone();

  // Get types with context if available (for assignment context)
  let left_type = assignment_context
    .as_ref()
    .map(|ctx| analyzer.expr_type_with_context(&bin.left, Some(ctx)))
    .unwrap_or_else(|| analyzer.expr_type(&bin.left));
  let right_type = assignment_context
    .as_ref()
    .map(|ctx| analyzer.expr_type_with_context(&bin.right, Some(ctx)))
    .unwrap_or_else(|| analyzer.expr_type(&bin.right));

  // Check for untyped literals in expressions (they return "unknown")
  let left_is_untyped_literal = matches!(
    &*bin.left,
    Expr::IntLiteral(_) | Expr::UintLiteral(_) | Expr::FloatLiteral(_) | Expr::HexLiteral(_)
  ) && left_type == "unknown";
  let right_is_untyped_literal = matches!(
    &*bin.right,
    Expr::IntLiteral(_) | Expr::UintLiteral(_) | Expr::FloatLiteral(_) | Expr::HexLiteral(_)
  ) && right_type == "unknown";

  // Both operands are untyped literals - reject (must have explicit types)
  if left_is_untyped_literal && right_is_untyped_literal {
    analyzer.add_error(
      format!(
        "Literals in arithmetic expressions must have explicit type. Suggest: {} as <type> {} {} as <type>",
        ExpressionAnalyzer::expr_to_string(&bin.left),
        TypeChecker::operator_name(&bin.operator),
        ExpressionAnalyzer::expr_to_string(&bin.right)
      ),
      &bin.location,
    );
    return;
  }

  // If only one operand is an untyped literal, we can infer its type from the other operand
  // Try to do context-aware type inference
  if left_is_untyped_literal && right_type != "unknown" {
    // Special case: pointer arithmetic with literal offset
    // For `ptr + literal` or `ptr - literal`, the literal should infer as u64 (offset), not pointer
    if left_type == "unknown"
      && left_is_untyped_literal
      && right_type.to_lowercase() == "pointer"
      && matches!(bin.operator, BinaryOp::Add | BinaryOp::Sub)
    {
      // This would be u64 + pointer or u64 - pointer, which is invalid
      // Let normal checking handle the error
      return check_binary_type_compatibility(analyzer, bin, "u64", &right_type);
    }
    // Try to re-evaluate left with right's type as context
    let inferred_left_type = analyzer.expr_type_with_context(&bin.left, Some(&right_type));
    if inferred_left_type != "unknown" {
      // Successfully inferred type from context, use it
      return check_binary_type_compatibility(analyzer, bin, &inferred_left_type, &right_type);
    }
  } else if right_is_untyped_literal && left_type != "unknown" {
    // Special case: pointer arithmetic with literal offset
    // For `ptr + literal` or `ptr - literal`, the literal should infer as u64 (offset), not pointer
    if right_type == "unknown"
      && left_type.to_lowercase() == "pointer"
      && matches!(bin.operator, BinaryOp::Add | BinaryOp::Sub)
    {
      // This is pointer +/- literal, infer literal as u64
      let inferred_right_type = analyzer.expr_type_with_context(&bin.right, Some("u64"));
      if inferred_right_type != "unknown" {
        return check_binary_type_compatibility(analyzer, bin, &left_type, &inferred_right_type);
      }
    }
    // Try to re-evaluate right with left's type as context
    let inferred_right_type = analyzer.expr_type_with_context(&bin.right, Some(&left_type));
    if inferred_right_type != "unknown" {
      // Successfully inferred type from context, use it
      return check_binary_type_compatibility(analyzer, bin, &left_type, &inferred_right_type);
    }
    // Inference failed: left type is not numeric, so the literal can't be coerced.
    // This is a type mismatch (e.g., enum_value > 9).
    analyzer.add_error(
      format!(
        "Cannot use '{}' with '{}' in {} operation. The literal cannot infer type from '{}'.",
        ExpressionAnalyzer::expr_to_string(&bin.left),
        ExpressionAnalyzer::expr_to_string(&bin.right),
        TypeChecker::operator_name(&bin.operator),
        left_type
      ),
      &bin.location,
    );
    return;
  }

  // Perform type compatibility check with the determined types
  check_binary_type_compatibility(analyzer, bin, &left_type, &right_type);
}

/// Helper: Check binary operation type compatibility and report errors
fn check_binary_type_compatibility(
  analyzer: &mut SemanticAnalyzer,
  bin: &BinaryExpr,
  left_type: &str,
  right_type: &str,
) {
  // Check for type mismatches
  if !UnitAnalyzer::check_binary_operand_types_ok(left_type, right_type, &bin.operator) {
    let msg = match bin.operator {
      BinaryOp::Dot => {
        let lv = left_type.starts_with("vec");
        let rv = right_type.starts_with("vec");
        if lv && rv {
          format!(
            "Type mismatch in dot product: '{}' and '{}' must be vectors of the same dimension and inner type",
            left_type, right_type
          )
        } else if lv || rv {
          format!(
            "Cannot use dot operator between vector '{}' and scalar '{}'. Use * for scalar-vector multiplication",
            left_type, right_type
          )
        } else {
          format!(
            "Type mismatch in dot product: '{}' and '{}' must be the same type",
            left_type, right_type
          )
        }
      }
      BinaryOp::Cross => {
        if left_type.starts_with("vec2") || right_type.starts_with("vec2") {
          "Cross product is not defined for 2D vectors. Use explicit scalar computation: x1*y2 - y1*x2".to_string()
        } else if left_type.starts_with("vec4") || right_type.starts_with("vec4") {
          "Cross product is only defined for 3D vectors, not vec4".to_string()
        } else if !left_type.starts_with("vec3") || !right_type.starts_with("vec3") {
          format!(
            "Cross product requires two vec3 operands, got '{}' and '{}'",
            left_type, right_type
          )
        } else {
          format!(
            "Cross product operands must have the same inner type, got '{}' and '{}'",
            left_type, right_type
          )
        }
      }
      BinaryOp::Mul if left_type.starts_with("vec") && right_type.starts_with("vec") => {
        "Cannot multiply two vectors with '*'. Use 'dot' for dot product or 'cross' for cross product".to_string()
      }
      _ => {
        if left_type == "byte" || right_type == "byte" {
          format!(
            "Type mismatch in {} operation: 'byte' is an octet, not a number. \
             It supports only bitwise operations (`bitwise and`, `bitwise or`, `bitwise xor`) \
             and equality. To use it as a number, convert it explicitly, e.g. `{} as u8`.",
            TypeChecker::operator_name(&bin.operator),
            ExpressionAnalyzer::expr_to_string(if left_type == "byte" { &bin.left } else { &bin.right })
          )
        } else {
          format!(
            "Type mismatch in {} operation: cannot use '{}' with '{}'. Use explicit conversion: {} as {} or {} as {}",
            TypeChecker::operator_name(&bin.operator),
            left_type,
            right_type,
            ExpressionAnalyzer::expr_to_string(&bin.left),
            right_type,
            ExpressionAnalyzer::expr_to_string(&bin.right),
            left_type
          )
        }
      }
    };
    analyzer.add_error(msg, &bin.location);
    return;
  }

  // Check for unsigned underflow in subtraction
  if bin.operator == BinaryOp::Sub {
    analyzer.check_unsigned_underflow(&bin.left, &bin.right, left_type, right_type, &bin.location);
  }

  // Check for compile-time integer overflow in constant arithmetic.
  // Unsigned subtraction is handled by `check_unsigned_underflow` above, so it is
  // excluded here to avoid two redundant errors on the same expression.
  let is_unsigned_sub =
    bin.operator == BinaryOp::Sub && matches!(left_type, "u8" | "u16" | "u32" | "u64");
  if !is_unsigned_sub
    && matches!(
      bin.operator,
      BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Dot
    )
  {
    analyzer.check_integer_overflow(&Expr::Binary(bin.clone()), left_type, &bin.location);
  }
}

/// The smallest signed integer type that can represent every value of the given
/// unsigned type. Returns `None` for `u64`, which has no wider signed type in
/// Lale (there is no `i128`).
fn widening_signed_type(unsigned: &str) -> Option<&'static str> {
  match unsigned {
    "u8" => Some("i16"),
    "u16" => Some("i32"),
    "u32" => Some("i64"),
    _ => None,
  }
}

/// Whether a numeric expression of type `source` may be assigned to a variable
/// of type `target` without an explicit conversion — i.e. the conversion is
/// value-preserving (widening). This rejects narrowing (e.g. `u64` → `u8`) and
/// same-width signed↔unsigned (e.g. `u64` → `i64`), which require `unsafe bitcast`.
fn is_widening_numeric_conversion(source: &str, target: &str) -> bool {
  match (
    TypeValidator::get_type_category(source),
    TypeValidator::get_type_category(target),
  ) {
    (TypeCategory::Numeric(src), TypeCategory::Numeric(tgt)) => {
      TypeValidator::is_widening_conversion(&src, &tgt)
    }
    _ => false,
  }
}

// Implement check_binary_operand_types as a method on SemanticAnalyzer
impl<'a> SemanticAnalyzer<'a> {
  fn check_binary_operand_types(&mut self, bin: &BinaryExpr) {
    check_binary_operand_types_impl(self, bin);
  }

  /// Report an error for a compile-time condition that is either non-boolean or
  /// not evaluable at compile time (references runtime state).
  fn report_ct_condition_error(&mut self, condition: &Condition) {
    let message = if is_non_boolean_condition(condition) {
      "Compile-time condition must be a boolean expression.".to_string()
    } else {
      "Compile-time condition must be evaluable at compile time (no runtime values).".to_string()
    };
    self
      .errors
      .push(SemanticError::new(message, condition.location.clone()));
  }

  /// Check for potential underflow in unsigned subtraction.
  /// When subtracting two unsigned integers, if the right operand could be larger than the left,
  /// we reject the operation and suggest using signed integers instead.
  fn check_unsigned_underflow(
    &mut self,
    left_expr: &Expr,
    right_expr: &Expr,
    left_type: &str,
    right_type: &str,
    location: &SourceLocation,
  ) {
    // Only check unsigned subtraction
    let is_unsigned_left = matches!(left_type, "u8" | "u16" | "u32" | "u64");
    let is_unsigned_right = matches!(right_type, "u8" | "u16" | "u32" | "u64");

    if !is_unsigned_left || !is_unsigned_right {
      return; // Not unsigned subtraction, no check needed
    }

    // Fold each operand with its declared type (via `expr_const_value_typed` +
    // `const_eval`), so a non-leaf expression such as `a + b` resolves to a value
    // instead of being rejected conservatively.
    let left_value = self
      .expr_const_value_typed(left_expr, Some(left_type))
      .into_value()
      .and_then(|v| v.as_unsigned());
    let right_value = self
      .expr_const_value_typed(right_expr, Some(right_type))
      .into_value()
      .and_then(|v| v.as_unsigned());

    let signed_type = widening_signed_type(left_type);
    let left_str = ExpressionAnalyzer::expr_to_string(left_expr);
    let right_str = ExpressionAnalyzer::expr_to_string(right_expr);

    match (left_value, right_value) {
      (Some(left_val), Some(right_val)) => {
        // Both operands are known constants: check at compile time.
        if right_val > left_val {
          let message = match signed_type {
            Some(signed) => format!(
              "Potential underflow in unsigned subtraction: {} - {} cannot fit in {} \
               (result would be negative but {} cannot represent negative values). \
               Solution: use a wider signed type for the operands: \
               var result as {} = ({} as {}) - ({} as {})",
              left_val,
              right_val,
              left_type,
              left_type,
              signed,
              left_str,
              signed,
              right_str,
              signed
            ),
            None => format!(
              "Potential underflow in unsigned subtraction: {} - {} cannot fit in {} \
               (result would be negative but {} cannot represent negative values). \
               u64 has no wider signed type (Lale has no i128), so the negative result \
               cannot be represented. Subtract the smaller value from the larger, or \
               guard the subtraction with a comparison.",
              left_val, right_val, left_type, left_type
            ),
          };
          self.add_error(message, location);
        }
      }
      _ => {
        // At least one operand's value is unknown: reject conservatively for safety.
        let message = match signed_type {
          Some(signed) => format!(
            "Cannot subtract two unsigned values when result could be negative. \
             Use a wider signed type for the operands: \
             var result as {} = ({} as {}) - ({} as {})",
            signed, left_str, signed, right_str, signed
          ),
          None => "Cannot subtract two unsigned values when result could be negative. \
             u64 has no wider signed type (Lale has no i128), so the negative result \
             cannot be represented. Subtract the smaller value from the larger, or \
             guard the subtraction with a comparison."
            .to_string(),
        };
        self.add_error(message, location);
      }
    }
  }

  /// Report a compile-time error when a constant integer expression provably
  /// overflows/underflows — the same condition the runtime `Checked*` trap would
  /// catch, but reported earlier. Only fires when the operands fold to known
  /// constants; unknown operands remain the responsibility of the runtime trap.
  fn check_integer_overflow(&mut self, expr: &Expr, type_str: &str, location: &SourceLocation) {
    // Float `NaN`, division-by-zero, and non-integer operands are handled elsewhere.
    if !matches!(
      type_str,
      "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
    ) {
      return;
    }
    if let EvalResult::Trap = self.expr_const_value_typed(expr, Some(type_str)) {
      self.add_error(
        format!(
          "integer overflow in constant expression '{}' (type '{}'); \
           use a wider type or an explicit conversion",
          ExpressionAnalyzer::expr_to_string(expr),
          type_str
        ),
        location,
      );
    }
  }

  /// Extract the size of a specific dimension from a type string.
  /// For example, from "i32[5]" or "f64[3][3]", extract the dimension size at position `dimension_index`.
  /// Returns None if the dimension size cannot be determined (e.g., non-literal dimensions).
  fn extract_dimension_from_type(&self, type_str: &str, dimension_index: usize) -> Option<usize> {
    // Parse type strings like "i32[5]" or "f64[3][3]"
    // Extract all [N] groups and return the one at dimension_index

    let mut dimensions = Vec::new();
    let chars = type_str.chars().peekable();
    let mut in_bracket = false;
    let mut current_num = String::new();

    for ch in chars {
      match ch {
        '[' => {
          in_bracket = true;
          current_num.clear();
        }
        ']' => {
          if in_bracket && !current_num.is_empty() {
            if let Ok(size) = current_num.parse::<usize>() {
              dimensions.push(size);
            }
            current_num.clear();
          }
          in_bracket = false;
        }
        _ if in_bracket && ch.is_ascii_digit() => {
          current_num.push(ch);
        }
        _ => {} // Skip other characters
      }
    }

    dimensions.get(dimension_index).copied()
  }

  /// Extract the element type from an array type string.
  /// Examples: "i32[5]" -> "i32", "f64[3][4]" -> "f64"
  fn extract_element_type(type_str: &str) -> String {
    // Find the first '[' and return everything before it
    if let Some(bracket_pos) = type_str.find('[') {
      type_str[..bracket_pos].to_string()
    } else {
      // Not an array type, return as-is
      type_str.to_string()
    }
  }

  /// Evaluate a compile-time condition expression.
  /// Returns Some(bool) if the condition can be evaluated at compile time,
  /// None if it contains runtime expressions that can't be evaluated.
  fn eval_const_condition(&self, condition: &Condition) -> Option<bool> {
    self.eval_const_condition_public(condition)
  }

  /// Public version of eval_const_condition for external use.
  pub fn eval_const_condition_public(&self, condition: &Condition) -> Option<bool> {
    let mut result = self.eval_const_expr(&condition.expr)?;
    if condition.negated {
      result = !result;
    }
    Some(result)
  }

  /// Evaluate a constant expression at compile time.
  /// Returns Some(bool) if evaluable as a boolean, None if it contains runtime expressions or non-boolean types.
  /// Integers and arithmetic are NOT implicitly converted to booleans (Lale principle: no implicit conversions).
  fn eval_const_expr(&self, expr: &Expr) -> Option<bool> {
    match expr {
      Expr::BoolLiteral(lit) => Some(lit.value),
      Expr::CompilerConst(cc) => self.eval_compiler_const(cc),
      Expr::IntLiteral(_) => {
        // Integer literals require explicit comparison (no implicit int-to-bool conversion)
        None
      }
      Expr::StringLiteral(lit) => {
        // Strings with no embedded values are always truthy if non-empty
        let has_text = lit.parts.iter().any(|p| matches!(p, StringPart::Text(_)));
        Some(has_text)
      }
      Expr::Binary(bin) => self.eval_const_binary(bin),
      Expr::Unary(un) => {
        match un.operator {
          UnaryOp::Not => self.eval_const_expr(&un.operand).map(|v| !v),
          _ => None, // Other unary operators not evaluable at compile time
        }
      }
      Expr::Grouped(inner) => self.eval_const_expr(inner),
      _ => None, // Other expressions not evaluable at compile time
    }
  }

  /// Evaluate a compile-time expression to a comparable value (for `#switch`).
  /// Returns None if the expression contains runtime values.
  fn eval_const_value(&self, expr: &Expr) -> Option<ConstValue> {
    match expr {
      Expr::BoolLiteral(lit) => Some(ConstValue::Bool(lit.value)),
      Expr::IntLiteral(lit) => Some(ConstValue::Int(lit.value)),
      Expr::UintLiteral(lit) => Some(ConstValue::Int(lit.value as i64)),
      Expr::StringLiteral(lit) => {
        let mut result = String::new();
        for part in &lit.parts {
          match part {
            StringPart::Text(text) => result.push_str(&text.node),
            StringPart::EmbeddedValue(_) => return None,
          }
        }
        Some(ConstValue::Text(result))
      }
      Expr::CompilerConst(cc) => self.eval_compiler_const(cc).map(ConstValue::Bool),
      Expr::Grouped(inner) => self.eval_const_value(inner),
      Expr::Unary(un) => match un.operator {
        UnaryOp::Neg => self.eval_const_value(&un.operand).and_then(|v| match v {
          ConstValue::Int(i) => Some(ConstValue::Int(-i)),
          _ => None,
        }),
        UnaryOp::Not => self.eval_const_value(&un.operand).and_then(|v| match v {
          ConstValue::Bool(b) => Some(ConstValue::Bool(!b)),
          _ => None,
        }),
        _ => None,
      },
      Expr::Binary(bin) => match bin.operator {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
          self.eval_const_int_binary(bin).map(ConstValue::Int)
        }
        _ => None,
      },
      _ => None,
    }
  }

  /// Evaluate a compile-time integer expression and return its numeric value.
  /// Returns Some(i64) if evaluable as an integer, None otherwise.
  fn eval_const_int(&self, expr: &Expr) -> Option<i64> {
    match expr {
      Expr::IntLiteral(lit) => Some(lit.value),
      Expr::CompilerConst(_) => None, // Compiler constants are not integers for arithmetic
      Expr::Binary(bin) => self.eval_const_int_binary(bin),
      Expr::Unary(un) => {
        if matches!(un.operator, UnaryOp::Neg) {
          self.eval_const_int(&un.operand).map(|v| -v)
        } else {
          None
        }
      }
      Expr::Grouped(inner) => self.eval_const_int(inner),
      _ => None,
    }
  }

  /// Evaluate a compiler constant at compile time.
  fn eval_compiler_const(&self, cc: &CompilerConst) -> Option<bool> {
    match cc.kind {
      CompilerConstKind::Main => Some(true), // #main is always true in the main program
      CompilerConstKind::SourceFile => Some(true), // #source_file is always truthy (never empty)
      CompilerConstKind::SourceLine => Some(true), // #source_line is always truthy
      CompilerConstKind::CompileTime => Some(true), // #compile_time is always true at compile time
      CompilerConstKind::CompilerVersion => Some(true), // #compiler_version is always truthy
      CompilerConstKind::Function => Some(true), // #function_name is truthy if in a function
      CompilerConstKind::Posix => Some(self.is_posix), // #posix is true on POSIX systems
      CompilerConstKind::Windows => Some(self.is_windows), // #windows is true on Windows
      CompilerConstKind::Debug => Some(self.options.is_debug), // #debug is true in debug mode
      CompilerConstKind::Mode => Some(true), // #mode is always a non-empty string
    }
  }

  /// Evaluate a compile-time binary expression.
  fn eval_const_binary(&self, bin: &BinaryExpr) -> Option<bool> {
    match bin.operator {
      BinaryOp::Eq => {
        // Try integer comparison first, fall back to string comparison
        if let (Some(left), Some(right)) = (
          self.eval_const_int(&bin.left),
          self.eval_const_int(&bin.right),
        ) {
          Some(left == right)
        } else {
          let left_str = self.expr_to_string_value(&bin.left)?;
          let right_str = self.expr_to_string_value(&bin.right)?;
          Some(left_str == right_str)
        }
      }
      BinaryOp::NotEq => {
        // Try integer comparison first, fall back to string comparison
        if let (Some(left), Some(right)) = (
          self.eval_const_int(&bin.left),
          self.eval_const_int(&bin.right),
        ) {
          Some(left != right)
        } else {
          let left_str = self.expr_to_string_value(&bin.left)?;
          let right_str = self.expr_to_string_value(&bin.right)?;
          Some(left_str != right_str)
        }
      }
      BinaryOp::Lt => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left < right)
      }
      BinaryOp::LtEq => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left <= right)
      }
      BinaryOp::Gt => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left > right)
      }
      BinaryOp::GtEq => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left >= right)
      }
      BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
        // Arithmetic operations require explicit comparison (no implicit int-to-bool conversion)
        // This enforces the Lale principle: "no implicit conversions"
        None
      }
      BinaryOp::And => {
        let left = self.eval_const_expr(&bin.left)?;
        let right = self.eval_const_expr(&bin.right)?;
        Some(left && right)
      }
      BinaryOp::Or => {
        let left = self.eval_const_expr(&bin.left)?;
        let right = self.eval_const_expr(&bin.right)?;
        Some(left || right)
      }
      BinaryOp::Xor => {
        let left = self.eval_const_expr(&bin.left)?;
        let right = self.eval_const_expr(&bin.right)?;
        Some(left != right)
      }
      _ => None, // Other operators not evaluable
    }
  }

  /// Evaluate a compile-time integer binary expression and return its numeric value.
  fn eval_const_int_binary(&self, bin: &BinaryExpr) -> Option<i64> {
    match bin.operator {
      BinaryOp::Add => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left + right)
      }
      BinaryOp::Sub => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left - right)
      }
      BinaryOp::Mul => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        Some(left * right)
      }
      BinaryOp::Div => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        if right == 0 {
          None // Division by zero is not evaluable
        } else {
          Some(left / right)
        }
      }
      BinaryOp::Mod => {
        let left = self.eval_const_int(&bin.left)?;
        let right = self.eval_const_int(&bin.right)?;
        if right == 0 {
          None // Modulo by zero is not evaluable
        } else {
          Some(left % right)
        }
      }
      _ => None, // Other operators don't have integer results
    }
  }

  /// Convert a compile-time expression to its string value.
  fn expr_to_string_value(&self, expr: &Expr) -> Option<String> {
    match expr {
      Expr::StringLiteral(lit) => {
        // For compile-time evaluation, only handle simple strings without embedded values
        let mut result = String::new();
        for part in &lit.parts {
          match part {
            StringPart::Text(text) => result.push_str(&text.node),
            StringPart::EmbeddedValue(_) => return None, // Can't evaluate runtime expressions
          }
        }
        Some(result)
      }
      Expr::CompilerConst(cc) => self.compiler_const_to_string(cc),
      Expr::Grouped(inner) => self.expr_to_string_value(inner),
      _ => None,
    }
  }

  /// Convert a compiler constant to its string representation.
  fn compiler_const_to_string(&self, cc: &CompilerConst) -> Option<String> {
    match cc.kind {
      CompilerConstKind::Main => Some("main".to_string()),
      CompilerConstKind::SourceFile => Some(cc.location.source_file.clone()),
      CompilerConstKind::SourceLine => Some(cc.location.line.to_string()),
      CompilerConstKind::CompileTime => Some("compile-time".to_string()),
      CompilerConstKind::CompilerVersion => Some(env!("CARGO_PKG_VERSION").to_string()),
      CompilerConstKind::Function => Some("function".to_string()), // Placeholder
      CompilerConstKind::Posix => Some(self.is_posix.to_string()),
      CompilerConstKind::Windows => Some(self.is_windows.to_string()),
      CompilerConstKind::Debug => Some(self.options.is_debug.to_string()),
      CompilerConstKind::Mode => Some(if self.options.test_mode {
        "test".to_string()
      } else {
        "run".to_string()
      }),
    }
  }

  /// Pre-load stdlib function signatures into the analyzer's symbol table.
  /// This enables correct type inference for stdlib functions during semantic analysis.
  pub fn load_stdlib_signatures(&mut self) {
    use crate::semantic_analysis::module_resolver::resolve_stdlib_path;

    // Resolve stdlib path (checks LALE_HOME, installed, and developer locations)
    let stdlib_dir = resolve_stdlib_path();
    let stdlib_path = stdlib_dir.join("full.lale");

    if !stdlib_path.exists() {
      return; // Silently skip if stdlib doesn't exist
    }

    // Load signatures from stdlib and its dependencies
    self.load_stdlib_signatures_recursive(&stdlib_path);
  }

  /// Recursively load function signatures from a stdlib file and its dependencies
  fn load_stdlib_signatures_recursive(&mut self, stdlib_path: &std::path::Path) {
    use std::fs;

    // Check if file exists
    if !stdlib_path.exists() {
      return;
    }

    // Read and parse stdlib
    if let Ok(stdlib_source) = fs::read_to_string(stdlib_path) {
      use crate::{LaleParser, Rule};
      use pest::Parser;

      match LaleParser::parse(Rule::program, &stdlib_source) {
        Ok(pairs) => {
          match crate::ast::builder::build_program(pairs, stdlib_path.to_str().unwrap_or("stdlib"))
          {
            Ok(stdlib_program) => {
              // Process `use` statements first to load dependencies
              for stmt in &stdlib_program.statements {
                if let Stmt::Use(use_stmt) = stmt {
                  // Resolve the module path
                  if let Some(resolver) = &self.module_resolver {
                    let resolver_ref = resolver.borrow();
                    let target_path = resolver_ref.resolve_module_path(
                      use_stmt.origin.node,
                      &use_stmt.path,
                      stdlib_path,
                    );
                    drop(resolver_ref);

                    // Recursively load signatures from the dependency
                    self.load_stdlib_signatures_recursive(&target_path);
                  }
                }
              }

              // Now extract function signatures
              self.extract_fn_signatures_from_statements(&stdlib_program.statements);
            }
            Err(e) => ice!("AST building error in stdlib: {}", e),
          }
        }
        Err(e) => ice!("Parse error in stdlib: {}", e),
      }
    }
  }

  /// Helper: extract function signatures from statements (including those inside #if blocks)
  fn extract_fn_signatures_from_statements(&mut self, statements: &[Stmt]) {
    for stmt in statements {
      match stmt {
        Stmt::FnDef(fn_def) => {
          // Register the function signature in the global symbol table
          // We get the return type from the function definition
          let return_type_str = match &fn_def.return_type.kind {
            crate::ast::definitions::ReturnTypeKind::Nothing => "nothing".to_string(),
            crate::ast::definitions::ReturnTypeKind::Type(ty) => {
              crate::semantic_analysis::type_compatibility::TypeInference::type_name_to_string(ty)
            }
          };

          let return_unit = fn_def.return_unit.as_ref().map(|u| u.raw.clone());

          // Register as a function with the global scope
          // This includes both the FnInfo (for parameter checking) and the symbol table entry
          if let Err(e) = self.symbols.define_function(
            &fn_def.name.node,
            &fn_def.location,
            &return_type_str,
            return_unit,
            crate::semantic_analysis::sqlite_symbol_management::FnInfo {
              parameters: fn_def.parameters.clone(),
              return_unit: fn_def.return_unit.clone(),
              body: fn_def.body.clone(),
            },
            Linkage::Export,
          ) {
            self.add_error(
              format!("Failed to register function '{}': {}", fn_def.name.node, e),
              &fn_def.location,
            );
          }
        }
        Stmt::CtIf(ct_if) => {
          // Recursively extract signatures from #if branches
          self.extract_fn_signatures_from_statements(&ct_if.then_branch);
          for (_, branch_stmts) in &ct_if.else_if_branches {
            self.extract_fn_signatures_from_statements(branch_stmts);
          }
          if let Some(else_stmts) = &ct_if.else_branch {
            self.extract_fn_signatures_from_statements(else_stmts);
          }
        }
        Stmt::CtWhen(ct_when) => {
          self.extract_fn_signatures_from_statements(&ct_when.body);
        }
        Stmt::CtMatch(ct_match) => {
          for arm in &ct_match.arms {
            self.extract_fn_signatures_from_statements(&arm.body);
          }
          self.extract_fn_signatures_from_statements(&ct_match.else_arm);
        }
        Stmt::CtSwitch(ct_switch) => {
          for case in &ct_switch.cases {
            self.extract_fn_signatures_from_statements(&case.body);
          }
          self.extract_fn_signatures_from_statements(&ct_switch.default_case);
        }
        _ => {
          // Other statement types don't contribute function signatures
        }
      }
    }
  }
}

/// Check if a compile-time condition contains a non-boolean type (e.g., integer without comparison).
fn is_non_boolean_condition(condition: &Condition) -> bool {
  is_non_boolean_expr(&condition.expr)
}

/// Helper to check non-boolean expressions recursively.
fn is_non_boolean_expr(expr: &Expr) -> bool {
  match expr {
    // Direct integer or unsigned integer literal without comparison
    Expr::IntLiteral(_) | Expr::UintLiteral(_) => true,
    // Arithmetic operations without comparison
    Expr::Binary(bin) => matches!(
      bin.operator,
      BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
    ),
    // Unary negation on its own
    Expr::Unary(un) => matches!(un.operator, UnaryOp::Neg),
    // Grouped expressions
    Expr::Grouped(inner) => is_non_boolean_expr(inner),
    // Everything else is either boolean or runtime-unevaluable (not an error)
    _ => false,
  }
}

/// Trait for accessing analyzer results regardless of ownership model.
/// This allows code to work with both SemanticAnalyzer and OwnedAnalyzer.
pub trait AnalyzerResults {
  fn get_errors(&self) -> &[SemanticError];
  fn get_warnings(&self) -> &[SemanticError];
  fn get_symbol_table(&self, scope: VarScope) -> SymbolTable;
  fn get_all_symbol_tables(&self) -> HashMap<VarScope, SymbolTable>;
  fn get_defined_symbols(
    &self,
  ) -> &std::collections::HashMap<String, (String, SourceLocation, bool)>;
  fn get_used_symbols(&self) -> &std::collections::HashSet<String>;
  fn get_used_symbol_locations(&self) -> &std::collections::HashSet<String>;
  fn get_unused_symbol_locations(&self) -> &std::collections::HashSet<String>;
  fn lookup_var_type(&self, name: &str) -> Option<String>;
  fn lookup_function_return_type(&self, name: &str) -> Option<String>;
  fn lookup_function(&self, name: &str) -> Option<FnInfo>;

  /// Infer the type of an expression (not available for all implementations, may return empty string)
  fn expr_type(&mut self, _expr: &Expr) -> String {
    // Default implementation returns empty string (not available)
    String::new()
  }
}

/// Wrapper that owns both the manager and the analyzer with its lifetime bound to the owned manager.
/// This allows convenience functions to return an owned, analyzable unit.
pub struct OwnedAnalyzer {
  manager: SqliteSymbolManager,
  // We can't store SemanticAnalyzer<'_> directly because it needs to borrow the manager,
  // and Rust doesn't allow self-referential structs. Instead, we store the analysis results.
  errors: Vec<SemanticError>,
  warnings: Vec<SemanticError>,
  all_defined_symbols: std::collections::HashMap<String, (String, SourceLocation, bool)>,
  used_symbols: std::collections::HashSet<String>,
  used_symbol_locations: std::collections::HashSet<String>,
  unused_symbol_locations: std::collections::HashSet<String>,
  is_debug: bool,
  checked_overflow: bool,
  test_mode: bool,
}

impl OwnedAnalyzer {
  /// Create a new OwnedAnalyzer with the given components.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    manager: SqliteSymbolManager,
    errors: Vec<SemanticError>,
    warnings: Vec<SemanticError>,
    all_defined_symbols: std::collections::HashMap<String, (String, SourceLocation, bool)>,
    used_symbols: std::collections::HashSet<String>,
    used_symbol_locations: std::collections::HashSet<String>,
    unused_symbol_locations: std::collections::HashSet<String>,
    is_debug: bool,
    checked_overflow: bool,
    test_mode: bool,
  ) -> Self {
    OwnedAnalyzer {
      manager,
      errors,
      warnings,
      all_defined_symbols,
      used_symbols,
      used_symbol_locations,
      unused_symbol_locations,
      is_debug,
      checked_overflow,
      test_mode,
    }
  }

  /// Get a reference to the symbol manager.
  pub fn symbols(&self) -> &SqliteSymbolManager {
    &self.manager
  }

  /// Get mutable reference to the symbol manager.
  pub fn symbols_mut(&mut self) -> &mut SqliteSymbolManager {
    &mut self.manager
  }

  /// Returns true if debug mode is active (default).
  pub fn is_debug(&self) -> bool {
    self.is_debug
  }

  /// Returns true if integer overflow should trap (default).
  pub fn checked_overflow(&self) -> bool {
    self.checked_overflow
  }

  /// Returns true in test mode (the `#mode` constant is "test").
  pub fn test_mode(&self) -> bool {
    self.test_mode
  }

  /// Check if analysis was valid (no errors).
  pub fn is_valid(&self) -> bool {
    self.errors.is_empty()
  }

  /// Evaluate a compile-time condition (public API for AST transformation).
  pub fn eval_condition(&self, condition: &Condition) -> Option<bool> {
    let mut result = self.eval_expr(&condition.expr)?;
    if condition.negated {
      result = !result;
    }
    Some(result)
  }

  /// Evaluate a compile-time expression recursively.
  fn eval_expr(&self, expr: &Expr) -> Option<bool> {
    match expr {
      Expr::BoolLiteral(lit) => Some(lit.value),
      Expr::CompilerConst(cc) => self.eval_compiler_const(cc),
      Expr::IntLiteral(_) => None, // Integer literals require explicit comparison
      Expr::StringLiteral(lit) => {
        // Strings with no embedded values are always truthy if non-empty
        let has_text = lit.parts.iter().any(|p| matches!(p, StringPart::Text(_)));
        Some(has_text)
      }
      Expr::Binary(bin) => self.eval_binary(bin),
      Expr::Unary(un) => match un.operator {
        UnaryOp::Not => self.eval_expr(&un.operand).map(|v| !v),
        _ => None,
      },
      Expr::Grouped(inner) => self.eval_expr(inner),
      _ => None,
    }
  }

  /// Evaluate a compile-time binary expression.
  fn eval_binary(&self, bin: &BinaryExpr) -> Option<bool> {
    match bin.operator {
      BinaryOp::Eq => {
        if let (Some(left), Some(right)) = (self.eval_int(&bin.left), self.eval_int(&bin.right)) {
          Some(left == right)
        } else {
          let left_str = self.expr_to_string_value(&bin.left)?;
          let right_str = self.expr_to_string_value(&bin.right)?;
          Some(left_str == right_str)
        }
      }
      BinaryOp::NotEq => {
        if let (Some(left), Some(right)) = (self.eval_int(&bin.left), self.eval_int(&bin.right)) {
          Some(left != right)
        } else {
          let left_str = self.expr_to_string_value(&bin.left)?;
          let right_str = self.expr_to_string_value(&bin.right)?;
          Some(left_str != right_str)
        }
      }
      BinaryOp::Lt => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left < right)
      }
      BinaryOp::LtEq => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left <= right)
      }
      BinaryOp::Gt => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left > right)
      }
      BinaryOp::GtEq => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left >= right)
      }
      BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => None,
      BinaryOp::And => {
        let left = self.eval_expr(&bin.left)?;
        let right = self.eval_expr(&bin.right)?;
        Some(left && right)
      }
      BinaryOp::Or => {
        let left = self.eval_expr(&bin.left)?;
        let right = self.eval_expr(&bin.right)?;
        Some(left || right)
      }
      BinaryOp::Xor => {
        let left = self.eval_expr(&bin.left)?;
        let right = self.eval_expr(&bin.right)?;
        Some(left != right)
      }
      _ => None,
    }
  }

  /// Evaluate a compile-time integer expression.
  fn eval_int(&self, expr: &Expr) -> Option<i64> {
    match expr {
      Expr::IntLiteral(lit) => Some(lit.value),
      Expr::CompilerConst(_) => None,
      Expr::Binary(bin) => self.eval_int_binary(bin),
      Expr::Unary(un) => {
        if matches!(un.operator, UnaryOp::Neg) {
          self.eval_int(&un.operand).map(|v| -v)
        } else {
          None
        }
      }
      Expr::Grouped(inner) => self.eval_int(inner),
      _ => None,
    }
  }

  /// Evaluate a compile-time integer binary expression.
  fn eval_int_binary(&self, bin: &BinaryExpr) -> Option<i64> {
    match bin.operator {
      BinaryOp::Add => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left + right)
      }
      BinaryOp::Sub => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left - right)
      }
      BinaryOp::Mul => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        Some(left * right)
      }
      BinaryOp::Div => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        if right == 0 { None } else { Some(left / right) }
      }
      BinaryOp::Mod => {
        let left = self.eval_int(&bin.left)?;
        let right = self.eval_int(&bin.right)?;
        if right == 0 { None } else { Some(left % right) }
      }
      _ => None,
    }
  }

  /// Evaluate a compiler constant.
  fn eval_compiler_const(&self, cc: &CompilerConst) -> Option<bool> {
    match cc.kind {
      CompilerConstKind::Main => Some(true),
      CompilerConstKind::SourceFile => Some(true),
      CompilerConstKind::SourceLine => Some(true),
      CompilerConstKind::CompileTime => Some(true),
      CompilerConstKind::CompilerVersion => Some(true),
      CompilerConstKind::Function => Some(true),
      // For these, we need to know the platform - default to platform detection
      #[cfg(target_os = "windows")]
      CompilerConstKind::Windows => Some(true),
      #[cfg(target_os = "windows")]
      CompilerConstKind::Posix => Some(false),
      #[cfg(not(target_os = "windows"))]
      CompilerConstKind::Posix => Some(true),
      #[cfg(not(target_os = "windows"))]
      CompilerConstKind::Windows => Some(false),
      CompilerConstKind::Debug => Some(self.is_debug),
      CompilerConstKind::Mode => Some(true), // #mode is always a non-empty string
    }
  }

  /// Convert an expression to its string value.
  fn expr_to_string_value(&self, expr: &Expr) -> Option<String> {
    match expr {
      Expr::StringLiteral(lit) => {
        let mut result = String::new();
        for part in &lit.parts {
          match part {
            StringPart::Text(text) => result.push_str(&text.node),
            StringPart::EmbeddedValue(_) => return None,
          }
        }
        Some(result)
      }
      Expr::CompilerConst(cc) => self.compiler_const_to_string(cc),
      Expr::Grouped(inner) => self.expr_to_string_value(inner),
      _ => None,
    }
  }

  /// Convert a compiler constant to its string representation.
  fn compiler_const_to_string(&self, cc: &CompilerConst) -> Option<String> {
    match cc.kind {
      CompilerConstKind::Main => Some("main".to_string()),
      CompilerConstKind::SourceFile => Some(cc.location.source_file.clone()),
      CompilerConstKind::SourceLine => Some(cc.location.line.to_string()),
      CompilerConstKind::CompileTime => Some("compile-time".to_string()),
      CompilerConstKind::CompilerVersion => Some(env!("CARGO_PKG_VERSION").to_string()),
      CompilerConstKind::Function => Some("function".to_string()),
      #[cfg(target_os = "windows")]
      CompilerConstKind::Windows => Some("true".to_string()),
      #[cfg(target_os = "windows")]
      CompilerConstKind::Posix => Some("false".to_string()),
      #[cfg(not(target_os = "windows"))]
      CompilerConstKind::Posix => Some("true".to_string()),
      #[cfg(not(target_os = "windows"))]
      CompilerConstKind::Windows => Some("false".to_string()),
      CompilerConstKind::Debug => Some(self.is_debug.to_string()),
      CompilerConstKind::Mode => Some(if self.test_mode {
        "test".to_string()
      } else {
        "run".to_string()
      }),
    }
  }

  /// Evaluate a compile-time expression to a comparable value (for `#switch`).
  fn eval_const_value(&self, expr: &Expr) -> Option<ConstValue> {
    match expr {
      Expr::BoolLiteral(lit) => Some(ConstValue::Bool(lit.value)),
      Expr::IntLiteral(lit) => Some(ConstValue::Int(lit.value)),
      Expr::UintLiteral(lit) => Some(ConstValue::Int(lit.value as i64)),
      Expr::StringLiteral(lit) => {
        let mut result = String::new();
        for part in &lit.parts {
          match part {
            StringPart::Text(text) => result.push_str(&text.node),
            StringPart::EmbeddedValue(_) => return None,
          }
        }
        Some(ConstValue::Text(result))
      }
      Expr::CompilerConst(cc) => self.eval_compiler_const(cc).map(ConstValue::Bool),
      Expr::Grouped(inner) => self.eval_const_value(inner),
      Expr::Unary(un) => match un.operator {
        UnaryOp::Neg => self.eval_const_value(&un.operand).and_then(|v| match v {
          ConstValue::Int(i) => Some(ConstValue::Int(-i)),
          _ => None,
        }),
        UnaryOp::Not => self.eval_const_value(&un.operand).and_then(|v| match v {
          ConstValue::Bool(b) => Some(ConstValue::Bool(!b)),
          _ => None,
        }),
        _ => None,
      },
      Expr::Binary(bin) => match bin.operator {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
          self.eval_int_binary(bin).map(ConstValue::Int)
        }
        _ => None,
      },
      _ => None,
    }
  }
}

impl AnalyzerResults for OwnedAnalyzer {
  fn get_errors(&self) -> &[SemanticError] {
    &self.errors
  }

  fn get_warnings(&self) -> &[SemanticError] {
    &self.warnings
  }

  fn get_symbol_table(&self, scope: VarScope) -> SymbolTable {
    self.manager.get_symbol_table(scope)
  }

  fn get_all_symbol_tables(&self) -> HashMap<VarScope, SymbolTable> {
    self.manager.get_all_symbol_tables().unwrap_or_default()
  }

  fn get_defined_symbols(
    &self,
  ) -> &std::collections::HashMap<String, (String, SourceLocation, bool)> {
    &self.all_defined_symbols
  }

  fn get_used_symbols(&self) -> &std::collections::HashSet<String> {
    &self.used_symbols
  }

  fn get_used_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.used_symbol_locations
  }

  fn get_unused_symbol_locations(&self) -> &std::collections::HashSet<String> {
    &self.unused_symbol_locations
  }

  fn lookup_var_type(&self, name: &str) -> Option<String> {
    self.manager.lookup_var_type(name)
  }

  fn lookup_function_return_type(&self, name: &str) -> Option<String> {
    self
      .manager
      .get_symbol_table(VarScope::Global)
      .get(name)
      .map(|sym| sym.data_type.clone())
  }

  fn lookup_function(&self, name: &str) -> Option<FnInfo> {
    self.manager.lookup_function(name)
  }
}

/// Check if an expression is a bare numeric literal (no type context).
fn is_bare_numeric_literal(expr: &Expr) -> bool {
  match expr {
    Expr::IntLiteral(_) | Expr::UintLiteral(_) | Expr::FloatLiteral(_) | Expr::HexLiteral(_) => {
      true
    }
    Expr::Grouped(inner) => is_bare_numeric_literal(inner),
    Expr::Unary(un) if matches!(un.operator, UnaryOp::Neg) => is_bare_numeric_literal(&un.operand),
    _ => false,
  }
}

/// Convenience function to analyze an AST with default options (LLVM backend).
pub fn analyze_ast(program: &Program) -> OwnedAnalyzer {
  analyze_ast_with_options(program, CompilerOptions::default())
}

/// Analyze an AST with specific compiler options.
pub fn analyze_ast_with_options(program: &Program, options: CompilerOptions) -> OwnedAnalyzer {
  let is_debug = options.is_debug;
  let checked_overflow = options.checked_overflow;
  let test_mode = options.test_mode;
  let mut manager = SqliteSymbolManager::new();

  // Register the default module (empty string) to satisfy FOREIGN KEY constraints
  manager.register_module("", "");

  // Register str as a pre-defined type
  manager.define_type_with_fields(
    "text",
    SourceLocation::dummy(),
    vec![
      ("ptr".to_string(), "pointer".to_string(), None, false),
      (
        "bytes".to_string(),
        "u64".to_string(),
        Some("<bytes>".to_string()),
        false,
      ),
      (
        "chars".to_string(),
        "u64".to_string(),
        Some("<chars>".to_string()),
        false,
      ),
    ],
  );

  // Register binary as a pre-defined type
  manager.define_type_with_fields(
    "binary",
    SourceLocation::dummy(),
    vec![
      ("ptr".to_string(), "pointer".to_string(), None, false),
      (
        "bytes".to_string(),
        "u64".to_string(),
        Some("<bytes>".to_string()),
        false,
      ),
    ],
  );

  // Extract results in a separate scope to release the borrow
  let (
    errors,
    warnings,
    all_defined_symbols,
    used_symbols,
    used_symbol_locations,
    unused_symbol_locations,
  ) = {
    let mut analyzer = SemanticAnalyzer::with_mut_manager(&mut manager, options);
    analyzer.visit_program(program);

    // Pipeline validation: after semantic analysis, verify that every expression
    // in the AST has a concrete type. Any "unknown" type means the analyzer failed
    // to infer the type, and the compiler should abort rather than propagate "unknown"
    // into IR generation where it causes misleading debug output or incorrect code.
    analyzer.validate_all_expression_types(program);

    (
      analyzer.errors.clone(),
      analyzer.warnings.clone(),
      analyzer.all_defined_symbols.clone(),
      analyzer.used_symbols.clone(),
      analyzer.used_symbol_locations.clone(),
      analyzer.unused_symbol_locations.clone(),
    )
  }; // analyzer is dropped here, releasing the borrow on manager

  OwnedAnalyzer {
    manager,
    errors,
    warnings,
    all_defined_symbols,
    used_symbols,
    used_symbol_locations,
    unused_symbol_locations,
    is_debug,
    checked_overflow,
    test_mode,
  }
}

/// Process compile-time if directives by filtering the program AST.
/// This function evaluates #if conditions and removes statements from branches that are not taken.
/// NOTE: This only works with SemanticAnalyzer (not OwnedAnalyzer due to lifetime constraints).
/// For OwnedAnalyzer, the program is returned unprocessed.
pub fn process_ct_directives(program: &Program, analyzer: &OwnedAnalyzer) -> Program {
  // Process ct_if directives by evaluating conditions and keeping only selected branches
  let mut new_statements = Vec::new();
  for stmt in &program.statements {
    process_statement(stmt, analyzer, &mut new_statements);
  }
  Program {
    statements: new_statements,
    location: program.location.clone(),
    global_symbol_table: RefCell::new(None),
  }
}

/// Recursively process a statement, handling ct_if directives
fn process_statement(stmt: &Stmt, analyzer: &OwnedAnalyzer, output: &mut Vec<Stmt>) {
  match stmt {
    Stmt::CtIf(ct_if) => match analyzer.eval_condition(&ct_if.condition) {
      Some(true) => {
        for then_stmt in &ct_if.then_branch {
          process_statement(then_stmt, analyzer, output);
        }
      }
      Some(false) => {
        let mut branch_taken = false;
        for (cond, branch) in &ct_if.else_if_branches {
          match analyzer.eval_condition(cond) {
            Some(true) => {
              for branch_stmt in branch {
                process_statement(branch_stmt, analyzer, output);
              }
              branch_taken = true;
              break;
            }
            Some(false) => {}
            None => unreachable!(
              "unevaluable #if else-if condition reached IR generation (should be a compile-time error)"
            ),
          }
        }

        if !branch_taken && let Some(else_branch) = &ct_if.else_branch {
          for else_stmt in else_branch {
            process_statement(else_stmt, analyzer, output);
          }
        }
      }
      None => unreachable!(
        "unevaluable #if condition reached IR generation (should be a compile-time error)"
      ),
    },
    Stmt::CtWhen(ct_when) => match analyzer.eval_condition(&ct_when.condition) {
      Some(true) => {
        for stmt in &ct_when.body {
          process_statement(stmt, analyzer, output);
        }
      }
      Some(false) => {}
      None => unreachable!(
        "unevaluable #when condition reached IR generation (should be a compile-time error)"
      ),
    },
    Stmt::CtMatch(ct_match) => {
      let mut taken = false;
      for arm in &ct_match.arms {
        match analyzer.eval_condition(&arm.guard) {
          Some(true) => {
            for stmt in &arm.body {
              process_statement(stmt, analyzer, output);
            }
            taken = true;
            break;
          }
          Some(false) => {}
          None => unreachable!(
            "unevaluable #match guard reached IR generation (should be a compile-time error)"
          ),
        }
      }
      if !taken {
        for stmt in &ct_match.else_arm {
          process_statement(stmt, analyzer, output);
        }
      }
    }
    Stmt::CtSwitch(ct_switch) => match analyzer.eval_const_value(&ct_switch.value) {
      Some(value) => {
        let mut matched = false;
        for case in &ct_switch.cases {
          match analyzer.eval_const_value(&case.value) {
            Some(case_value) if case_value == value => {
              for stmt in &case.body {
                process_statement(stmt, analyzer, output);
              }
              matched = true;
              break;
            }
            Some(_) => {}
            None => unreachable!(
              "unevaluable #switch case value reached IR generation (should be a compile-time error)"
            ),
          }
        }
        if !matched {
          for stmt in &ct_switch.default_case {
            process_statement(stmt, analyzer, output);
          }
        }
      }
      None => unreachable!(
        "unevaluable #switch value reached IR generation (should be a compile-time error)"
      ),
    },
    // For other statement types, keep them as-is (but they might contain nested ct_ifs in blocks)
    _ => output.push(stmt.clone()),
  }
}

/// Analyze an AST with stdlib function signatures pre-loaded.
/// This ensures that stdlib functions are known during analysis.
pub fn analyze_ast_with_stdlib(program: &Program) -> OwnedAnalyzer {
  let mut manager = SqliteSymbolManager::new();

  // Register the default module (empty string) to satisfy FOREIGN KEY constraints
  manager.register_module("", "");

  // Register str as a pre-defined type
  manager.define_type_with_fields(
    "text",
    SourceLocation::dummy(),
    vec![
      ("ptr".to_string(), "pointer".to_string(), None, false),
      (
        "bytes".to_string(),
        "u64".to_string(),
        Some("<bytes>".to_string()),
        false,
      ),
      (
        "chars".to_string(),
        "u64".to_string(),
        Some("<chars>".to_string()),
        false,
      ),
    ],
  );

  // Register binary as a pre-defined type
  manager.define_type_with_fields(
    "binary",
    SourceLocation::dummy(),
    vec![
      ("ptr".to_string(), "pointer".to_string(), None, false),
      (
        "bytes".to_string(),
        "u64".to_string(),
        Some("<bytes>".to_string()),
        false,
      ),
    ],
  );

  // Extract results in a separate scope to release the borrow
  let (
    errors,
    warnings,
    all_defined_symbols,
    used_symbols,
    used_symbol_locations,
    unused_symbol_locations,
  ) = {
    let mut analyzer = SemanticAnalyzer::with_mut_manager(&mut manager, CompilerOptions::default());

    // Set up module resolver for multi-file compilation
    use std::path::PathBuf;
    let resolver = Rc::new(RefCell::new(crate::semantic_analysis::ModuleResolver::new(
      PathBuf::from("."),
    )));
    analyzer.set_resolver(resolver.clone());
    analyzer.set_current_module_path(PathBuf::from("main.lale"));

    // Pre-load stdlib function signatures
    // This will now handle stdlib module dependencies through load_stdlib_signatures_recursive
    analyzer.load_stdlib_signatures();

    // Now analyze the main program (stdlib functions are now in the symbol table)
    analyzer.visit_program(program);

    (
      analyzer.errors.clone(),
      analyzer.warnings.clone(),
      analyzer.all_defined_symbols.clone(),
      analyzer.used_symbols.clone(),
      analyzer.used_symbol_locations.clone(),
      analyzer.unused_symbol_locations.clone(),
    )
  }; // analyzer is dropped here, releasing the borrow on manager

  OwnedAnalyzer {
    manager,
    errors,
    warnings,
    all_defined_symbols,
    used_symbols,
    used_symbol_locations,
    unused_symbol_locations,
    is_debug: true,
    checked_overflow: true,
    test_mode: false,
  }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
  use super::*;

  // Helper to create an identifier expression
  fn id(name: &str) -> Expr {
    Expr::Identifier(IdentifierExpr {
      path: vec![name.to_string()],
      location: SourceLocation::dummy(),
    })
  }

  // Helper to create an int literal
  fn int(v: i64) -> Expr {
    Expr::IntLiteral(IntLiteral {
      value: v,
      unit: None,
      location: SourceLocation::dummy(),
    })
  }

  // Helper to create a binary expression
  fn bin(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary(BinaryExpr {
      operator: op,
      left: Box::new(left),
      right: Box::new(right),
      location: SourceLocation::dummy(),
    })
  }

  // Helper to create a grouped expression
  fn grouped(inner: Expr) -> Expr {
    Expr::Grouped(Box::new(inner))
  }

  // Helper to create a conversion expression
  fn conv(inner: Expr, target: &str) -> Expr {
    let base_type = match target {
      "f64" => BaseType::F64,
      "i32" => BaseType::I32,
      "u8" => BaseType::U8,
      _ => BaseType::F64,
    };
    Expr::Conversion(ConversionExpr {
      operand: Box::new(inner),
      target_type: TypeName {
        base_type,
        inner_type: None,
        array_dimensions: Vec::new(),
        is_optional: false,
        location: SourceLocation::dummy(),
      },
      location: SourceLocation::dummy(),
    })
  }

  // ==================== structurally_equal tests ====================

  #[test]
  fn test_structurally_equal_identifiers() {
    assert!(structurally_equal(&id("x"), &id("x")));
    assert!(!structurally_equal(&id("x"), &id("y")));
  }

  #[test]
  fn test_structurally_equal_binary_symmetric() {
    // x * 2 should equal x * 2
    let a = bin(BinaryOp::Mul, id("x"), int(2));
    let b = bin(BinaryOp::Mul, id("x"), int(2));
    assert!(structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_binary_different_left() {
    // x * 2 should NOT equal y * 2
    let a = bin(BinaryOp::Mul, id("x"), int(2));
    let b = bin(BinaryOp::Mul, id("y"), int(2));
    assert!(!structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_binary_different_right() {
    // x * 2 should NOT equal x * 3
    let a = bin(BinaryOp::Mul, id("x"), int(2));
    let b = bin(BinaryOp::Mul, id("x"), int(3));
    assert!(!structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_binary_different_operator() {
    // x * 2 should NOT equal x + 2
    let a = bin(BinaryOp::Mul, id("x"), int(2));
    let b = bin(BinaryOp::Add, id("x"), int(2));
    assert!(!structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_with_conversion() {
    // x as f64 * 2 should equal x as f64 * 2
    let a = bin(BinaryOp::Mul, conv(id("x"), "f64"), int(2));
    let b = bin(BinaryOp::Mul, conv(id("x"), "f64"), int(2));
    assert!(structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_with_grouped() {
    let a = grouped(bin(BinaryOp::Mul, id("x"), int(2)));
    let b = grouped(bin(BinaryOp::Mul, id("x"), int(2)));
    assert!(structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_grouped_vs_ungrouped() {
    // (x * 2) should NOT equal x * 2 (different structure)
    let a = grouped(bin(BinaryOp::Mul, id("x"), int(2)));
    let b = bin(BinaryOp::Mul, id("x"), int(2));
    assert!(!structurally_equal(&a, &b));
  }

  #[test]
  fn test_structurally_equal_int_literals() {
    assert!(structurally_equal(&int(0), &int(0)));
    assert!(structurally_equal(&int(42), &int(42)));
    assert!(!structurally_equal(&int(0), &int(1)));
  }

  // ==================== eval_const_value tests ====================

  fn bool_lit(v: bool) -> Expr {
    Expr::BoolLiteral(BoolLiteral {
      value: v,
      location: SourceLocation::dummy(),
    })
  }

  fn uint_lit(v: u64) -> Expr {
    Expr::UintLiteral(UintLiteral {
      value: v,
      unit: None,
      location: SourceLocation::dummy(),
    })
  }

  fn float_lit(v: f64) -> Expr {
    Expr::FloatLiteral(FloatLiteral {
      value: v,
      unit: None,
      location: SourceLocation::dummy(),
    })
  }

  fn str_lit(s: &str) -> Expr {
    Expr::StringLiteral(StringLiteral {
      parts: vec![StringPart::Text(Spanned::new(
        s.to_string(),
        SourceLocation::dummy(),
      ))],
      location: SourceLocation::dummy(),
    })
  }

  #[test]
  fn test_eval_const_value_int() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.eval_const_value(&int(42)),
      Some(ConstValue::Int(42))
    );
  }

  #[test]
  fn test_eval_const_value_uint() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.eval_const_value(&uint_lit(7)),
      Some(ConstValue::Int(7))
    );
  }

  #[test]
  fn test_eval_const_value_bool() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.eval_const_value(&bool_lit(true)),
      Some(ConstValue::Bool(true))
    );
    assert_eq!(
      analyzer.eval_const_value(&bool_lit(false)),
      Some(ConstValue::Bool(false))
    );
  }

  #[test]
  fn test_eval_const_value_string() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.eval_const_value(&str_lit("hello")),
      Some(ConstValue::Text("hello".to_string()))
    );
  }

  #[test]
  fn test_eval_const_value_arithmetic() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.eval_const_value(&bin(BinaryOp::Add, int(2), int(3))),
      Some(ConstValue::Int(5))
    );
    assert_eq!(
      analyzer.eval_const_value(&bin(
        BinaryOp::Mul,
        grouped(bin(BinaryOp::Add, int(2), int(3))),
        int(4)
      )),
      Some(ConstValue::Int(20))
    );
  }

  #[test]
  fn test_eval_const_value_negative() {
    let analyzer = make_analyzer();
    let neg_five = Expr::Unary(UnaryExpr {
      operator: UnaryOp::Neg,
      operand: Box::new(int(5)),
      location: SourceLocation::dummy(),
    });
    assert_eq!(
      analyzer.eval_const_value(&neg_five),
      Some(ConstValue::Int(-5))
    );
  }

  #[test]
  fn test_eval_const_value_unevaluable() {
    let analyzer = make_analyzer();
    assert_eq!(analyzer.eval_const_value(&id("runtime_var")), None);
  }

  // ==================== try_extract_constant_int tests ====================

  #[test]
  fn test_extract_simple_int() {
    assert_eq!(try_extract_constant_int(&int(5)), Some(5));
    assert_eq!(try_extract_constant_int(&int(0)), Some(0));
    assert_eq!(try_extract_constant_int(&int(-3)), Some(-3));
  }

  #[test]
  fn test_extract_through_conversion() {
    assert_eq!(try_extract_constant_int(&conv(int(0), "f64")), Some(0));
    assert_eq!(try_extract_constant_int(&conv(int(5), "f64")), Some(5));
  }

  #[test]
  fn test_extract_through_grouped() {
    assert_eq!(try_extract_constant_int(&grouped(int(0))), Some(0));
  }

  #[test]
  fn test_extract_non_literal() {
    assert_eq!(try_extract_constant_int(&id("x")), None);
    assert_eq!(
      try_extract_constant_int(&bin(BinaryOp::Add, int(1), int(2))),
      None
    );
  }

  // ==================== compare_against_constant tests ====================

  #[test]
  fn test_compare_gt_true_excludes_zero() {
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::Gt, true).is_some());
    assert!(compare_against_constant(&id("x"), 1, &BinaryOp::Gt, true).is_some());
    assert!(compare_against_constant(&id("x"), 42, &BinaryOp::Gt, true).is_some());
  }

  #[test]
  fn test_compare_gt_true_does_not_exclude_zero() {
    // x > -5 → zero NOT excluded (x could be -4..0)
    assert!(compare_against_constant(&id("x"), -5, &BinaryOp::Gt, true).is_none());
  }

  #[test]
  fn test_compare_gt_false_excludes_zero() {
    assert!(compare_against_constant(&id("x"), -1, &BinaryOp::Gt, false).is_some());
  }

  #[test]
  fn test_compare_gteq_true_excludes_zero() {
    assert!(compare_against_constant(&id("x"), 1, &BinaryOp::GtEq, true).is_some());
  }

  #[test]
  fn test_compare_gteq_true_does_not_exclude_zero() {
    // x >= 0 → zero NOT excluded
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::GtEq, true).is_none());
  }

  #[test]
  fn test_compare_lt_true_excludes_zero() {
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::Lt, true).is_some());
    assert!(compare_against_constant(&id("x"), -1, &BinaryOp::Lt, true).is_some());
  }

  #[test]
  fn test_compare_lt_true_does_not_exclude_zero() {
    // x < 5 → zero NOT excluded
    assert!(compare_against_constant(&id("x"), 5, &BinaryOp::Lt, true).is_none());
  }

  #[test]
  fn test_compare_lteq_true_excludes_zero() {
    // x <= -1 → zero excluded
    assert!(compare_against_constant(&id("x"), -1, &BinaryOp::LtEq, true).is_some());
  }

  #[test]
  fn test_compare_lteq_true_does_not_exclude_zero() {
    // x <= 0 → zero NOT excluded
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::LtEq, true).is_none());
  }

  #[test]
  fn test_compare_eq_true_nonzero() {
    // x == 5 → x ≠ 0 (since 5 ≠ 0)
    assert!(compare_against_constant(&id("x"), 5, &BinaryOp::Eq, true).is_some());
  }

  #[test]
  fn test_compare_eq_true_zero() {
    // x == 0 → does NOT prove x ≠ 0
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::Eq, true).is_none());
  }

  #[test]
  fn test_compare_eq_false_zero() {
    // x == 0 is false → x ≠ 0
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::Eq, false).is_some());
  }

  #[test]
  fn test_compare_noteq_true_zero() {
    // x != 0 is true → x ≠ 0
    assert!(compare_against_constant(&id("x"), 0, &BinaryOp::NotEq, true).is_some());
  }

  #[test]
  fn test_compare_noteq_true_nonzero() {
    // x != 5 is true → does NOT prove x ≠ 0
    assert!(compare_against_constant(&id("x"), 5, &BinaryOp::NotEq, true).is_none());
  }

  // ==================== extract_facts tests ====================

  #[test]
  fn test_extract_facts_from_not_eq_zero() {
    let expr = bin(BinaryOp::NotEq, id("x"), int(0));
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &id("x")));
  }

  #[test]
  fn test_extract_facts_from_gt_zero() {
    let expr = bin(BinaryOp::Gt, id("x"), int(0));
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &id("x")));
  }

  #[test]
  fn test_extract_facts_from_not_expr() {
    let inner = bin(BinaryOp::Eq, id("x"), int(0));
    let expr = Expr::Unary(UnaryExpr {
      operator: UnaryOp::Not,
      operand: Box::new(inner),
      location: SourceLocation::dummy(),
    });
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &id("x")));
  }

  #[test]
  fn test_extract_facts_from_and() {
    let left = bin(BinaryOp::NotEq, id("x"), int(0));
    let right = bin(BinaryOp::Gt, id("y"), int(0));
    let expr = bin(BinaryOp::And, left, right);
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 2);
  }

  #[test]
  fn test_extract_facts_from_and_false_gives_none() {
    let left = bin(BinaryOp::NotEq, id("x"), int(0));
    let right = bin(BinaryOp::Gt, id("y"), int(0));
    let expr = bin(BinaryOp::And, left, right);
    let facts = extract_facts(&expr, false);
    assert!(facts.is_empty());
  }

  #[test]
  fn test_extract_facts_with_conversion() {
    let left = conv(id("x"), "f64");
    let expr = bin(BinaryOp::NotEq, left.clone(), int(0));
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &left));
  }

  #[test]
  fn test_extract_facts_constant_plus_expr_conservative() {
    let expr = bin(BinaryOp::NotEq, int(0), id("x"));
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &id("x")));
  }

  #[test]
  fn test_extract_facts_from_gt_five() {
    // x + y > 5 → NotZero(x + y) because 5 > 0, so x + y ∈ [6, +∞]
    let left = bin(BinaryOp::Add, id("x"), id("y"));
    let expr = bin(BinaryOp::Gt, left.clone(), int(5));
    let facts = extract_facts(&expr, true);
    assert_eq!(facts.len(), 1);
    assert!(structurally_equal(&facts[0], &left));
  }

  #[test]
  fn test_extract_facts_irrelevant_condition() {
    // x + y > -5 → zero could be in range (x+y could be -4..0), so no fact
    let left = bin(BinaryOp::Add, id("x"), id("y"));
    let expr = bin(BinaryOp::Gt, left, int(-5));
    let facts = extract_facts(&expr, true);
    assert!(facts.is_empty());
  }

  // ==================== divisor_equivalent_forms tests ====================

  #[test]
  fn test_equivalent_forms_bare_identifier() {
    let x = id("x");
    let forms = divisor_equivalent_forms(&x);
    assert_eq!(forms.len(), 1);
  }

  #[test]
  fn test_equivalent_forms_conversion() {
    let expr = conv(id("x"), "f64");
    let forms = divisor_equivalent_forms(&expr);
    assert_eq!(forms.len(), 2);
    assert!(structurally_equal(forms[0], &expr));
    assert!(structurally_equal(forms[1], &id("x")));
  }

  #[test]
  fn test_equivalent_forms_grouped() {
    let expr = grouped(id("x"));
    let forms = divisor_equivalent_forms(&expr);
    assert_eq!(forms.len(), 2);
    assert!(structurally_equal(forms[0], &expr));
    assert!(structurally_equal(forms[1], &id("x")));
  }

  #[test]
  fn test_equivalent_forms_nested() {
    let inner = conv(id("x"), "f64");
    let expr = grouped(inner.clone());
    let forms = divisor_equivalent_forms(&expr);
    assert_eq!(forms.len(), 3);
    assert!(structurally_equal(forms[0], &expr));
    assert!(structurally_equal(forms[1], &inner));
    assert!(structurally_equal(forms[2], &id("x")));
  }

  // ==================== is_guarded_nonzero tests ====================

  fn make_analyzer() -> SemanticAnalyzer<'static> {
    let manager = Box::leak(Box::new(
      crate::semantic_analysis::sqlite_symbol_management::SqliteSymbolManager::new(),
    ));
    let options = crate::config::CompilerOptions {
      backend: crate::config::Backend::Interpreter,
      source_file: Some("test.lale".to_string()),
      stdlib: crate::config::StdlibLevel::None,
      is_debug: false,
      checked_overflow: true,
      test_mode: false,
    };
    SemanticAnalyzer::with_mut_manager(manager, options)
  }

  #[test]
  fn test_is_guarded_nonzero_constants() {
    let mut analyzer = make_analyzer();
    assert!(analyzer.is_guarded_nonzero(&int(5)));
    assert!(!analyzer.is_guarded_nonzero(&int(0)));
    assert!(analyzer.is_guarded_nonzero(&int(-1)));
  }

  #[test]
  fn test_is_guarded_nonzero_with_guard_stack() {
    let mut analyzer = make_analyzer();
    analyzer.push_guard_frame(vec![id("x")]);
    assert!(analyzer.is_guarded_nonzero(&id("x")));
    assert!(!analyzer.is_guarded_nonzero(&id("y")));
    analyzer.pop_guard_frame();
    assert!(!analyzer.is_guarded_nonzero(&id("x")));
  }

  #[test]
  fn test_is_guarded_nonzero_conversion_transparent() {
    let mut analyzer = make_analyzer();
    analyzer.push_guard_frame(vec![id("x")]);
    // Divisor is x as f64 — should still be guarded
    assert!(analyzer.is_guarded_nonzero(&conv(id("x"), "f64")));
    assert!(!analyzer.is_guarded_nonzero(&conv(id("y"), "f64")));
    analyzer.pop_guard_frame();
  }

  #[test]
  fn test_is_guarded_nonzero_conversion_fact_to_bare_divisor() {
    let mut analyzer = make_analyzer();
    analyzer.push_guard_frame(vec![conv(id("x"), "f64")]);
    // Divisor is bare x — should still be guarded
    assert!(analyzer.is_guarded_nonzero(&id("x")));
    analyzer.pop_guard_frame();
  }

  #[test]
  fn test_is_guarded_nonzero_with_binary_expression() {
    let mut analyzer = make_analyzer();
    let guarded = bin(BinaryOp::Mul, id("x"), int(2));
    analyzer.push_guard_frame(vec![guarded.clone()]);
    assert!(analyzer.is_guarded_nonzero(&guarded));
    // Different expression should not be guarded
    let other = bin(BinaryOp::Mul, id("x"), int(3));
    assert!(!analyzer.is_guarded_nonzero(&other));
    analyzer.pop_guard_frame();
  }

  // ==================== expr_const_value tests ====================

  #[test]
  fn test_expr_const_value_literals() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.expr_const_value(&int(42)),
      Some(ConstValue::Int(42))
    );
    assert_eq!(
      analyzer.expr_const_value(&uint_lit(7)),
      Some(ConstValue::Uint(7))
    );
    assert_eq!(
      analyzer.expr_const_value(&float_lit(2.5)),
      Some(ConstValue::float(2.5))
    );
    assert_eq!(
      analyzer.expr_const_value(&bool_lit(true)),
      Some(ConstValue::Bool(true))
    );
    assert_eq!(
      analyzer.expr_const_value(&str_lit("hello")),
      Some(ConstValue::Text("hello".to_string()))
    );
  }

  #[test]
  fn test_expr_const_value_wrappers_and_neg() {
    let analyzer = make_analyzer();
    assert_eq!(
      analyzer.expr_const_value(&grouped(int(5))),
      Some(ConstValue::Int(5))
    );
    assert_eq!(
      analyzer.expr_const_value(&conv(int(5), "f64")),
      Some(ConstValue::Int(5))
    );
    let neg = Expr::Unary(UnaryExpr {
      operator: UnaryOp::Neg,
      operand: Box::new(int(5)),
      location: SourceLocation::dummy(),
    });
    assert_eq!(analyzer.expr_const_value(&neg), Some(ConstValue::Int(-5)));
  }

  #[test]
  fn test_expr_const_value_identifier() {
    let analyzer = make_analyzer();
    let location = SourceLocation::dummy();
    let _ = analyzer.symbols.define_variable(
      VarScope::Global,
      "pi",
      &location,
      "f64",
      None,
      Linkage::Internal,
      true,
    );
    analyzer
      .symbols
      .update_variable_const_value("pi", Some(ConstValue::float(std::f64::consts::PI)));

    assert_eq!(
      analyzer.expr_const_value(&id("pi")),
      Some(ConstValue::float(std::f64::consts::PI))
    );
    assert_eq!(analyzer.expr_const_value(&id("nope")), None);
  }

  #[test]
  fn test_expr_const_value_binary_not_folded_yet() {
    let analyzer = make_analyzer();
    // Binary arithmetic is intentionally not folded in Phase 1.
    assert_eq!(
      analyzer.expr_const_value(&bin(BinaryOp::Add, int(2), int(3))),
      None
    );
  }

  // ==================== expr_const_value_typed tests ====================

  fn define_const_var(
    analyzer: &mut SemanticAnalyzer<'static>,
    name: &str,
    ty: &str,
    value: ConstValue,
  ) {
    let location = SourceLocation::dummy();
    let _ = analyzer.symbols.define_variable(
      VarScope::Global,
      name,
      &location,
      ty,
      None,
      Linkage::Internal,
      true,
    );
    analyzer
      .symbols
      .update_variable_const_value(name, Some(value));
  }

  #[test]
  fn test_typed_fold_add_literals_with_hint() {
    let mut analyzer = make_analyzer();
    let expr = bin(BinaryOp::Add, int(5), int(10));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, Some("i32")),
      EvalResult::Value(ConstValue::Int(15))
    );
  }

  #[test]
  fn test_typed_fold_add_variables() {
    let mut analyzer = make_analyzer();
    define_const_var(&mut analyzer, "a", "i32", ConstValue::Int(5));
    define_const_var(&mut analyzer, "b", "i32", ConstValue::Int(10));
    let expr = bin(BinaryOp::Add, id("a"), id("b"));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Value(ConstValue::Int(15))
    );
  }

  #[test]
  fn test_typed_fold_unsigned_operands() {
    let mut analyzer = make_analyzer();
    define_const_var(&mut analyzer, "a", "u32", ConstValue::Uint(5));
    define_const_var(&mut analyzer, "b", "u32", ConstValue::Uint(10));
    let expr = bin(BinaryOp::Sub, id("a"), id("b"));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Trap // 5 - 10 underflows u32
    );
  }

  #[test]
  fn test_typed_fold_overflow_traps() {
    let mut analyzer = make_analyzer();
    let expr = bin(BinaryOp::Add, int(127), int(1));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, Some("i8")),
      EvalResult::Trap
    );
  }

  #[test]
  fn test_typed_fold_comparison_uses_operand_type() {
    let mut analyzer = make_analyzer();
    define_const_var(&mut analyzer, "a", "i32", ConstValue::Int(5));
    define_const_var(&mut analyzer, "b", "i32", ConstValue::Int(10));
    let expr = bin(BinaryOp::Lt, id("a"), id("b"));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Value(ConstValue::Bool(true))
    );
  }

  #[test]
  fn test_typed_fold_neg() {
    let mut analyzer = make_analyzer();
    let neg = Expr::Unary(UnaryExpr {
      operator: UnaryOp::Neg,
      operand: Box::new(int(5)),
      location: SourceLocation::dummy(),
    });
    assert_eq!(
      analyzer.expr_const_value_typed(&neg, Some("i32")),
      EvalResult::Value(ConstValue::Int(-5))
    );
  }

  #[test]
  fn test_typed_fold_nested_binary() {
    let mut analyzer = make_analyzer();
    define_const_var(&mut analyzer, "a", "i32", ConstValue::Int(5));
    define_const_var(&mut analyzer, "b", "i32", ConstValue::Int(10));
    define_const_var(&mut analyzer, "c", "i32", ConstValue::Int(2));
    let inner = bin(BinaryOp::Add, id("a"), id("b"));
    let expr = bin(BinaryOp::Mul, inner, id("c"));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Value(ConstValue::Int(30))
    );
  }

  #[test]
  fn test_typed_fold_unknown_operand() {
    let mut analyzer = make_analyzer();
    define_const_var(&mut analyzer, "a", "i32", ConstValue::Int(5));
    let expr = bin(BinaryOp::Add, id("a"), id("missing"));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Unknown
    );
  }

  #[test]
  fn test_typed_fold_trap_propagates_from_operand() {
    let mut analyzer = make_analyzer();
    // Inner `127 + 1` overflows i8; the outer `+ 1` must also trap.
    let inner = bin(BinaryOp::Add, int(127), int(1));
    let expr = bin(BinaryOp::Add, inner, int(1));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, Some("i8")),
      EvalResult::Trap
    );
  }

  #[test]
  fn test_typed_fold_non_numeric_operator_is_unknown() {
    let mut analyzer = make_analyzer();
    // Logical And operates on bools and is not a numeric fold.
    let expr = bin(BinaryOp::And, bool_lit(true), bool_lit(false));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, None),
      EvalResult::Unknown
    );
  }

  #[test]
  fn test_typed_fold_float_coerces_int_tag() {
    let mut analyzer = make_analyzer();
    // `5` stored as Int(5) in an f64 context folds as 5.0 + 0.5.
    let expr = bin(BinaryOp::Add, int(5), float_lit(0.5));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, Some("f64")),
      EvalResult::Value(ConstValue::float(5.5))
    );
  }

  #[test]
  fn test_typed_fold_division_by_zero_traps() {
    let mut analyzer = make_analyzer();
    let expr = bin(BinaryOp::Div, int(5), int(0));
    assert_eq!(
      analyzer.expr_const_value_typed(&expr, Some("i32")),
      EvalResult::Trap
    );
  }
}
