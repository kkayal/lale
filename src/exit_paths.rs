//! Exit-path analysis for the Lale compiler.
//!
//! Walks the AST and identifies every code location that can cause the
//! program to terminate, producing a report suitable for audit and debugging.
//!
//! # Exit sources
//!
//! | Source               | Example                | When it exits        | Tier |
//! |----------------------|------------------------|----------------------|------|
//! | `exit program`       | `exit program 1`       | Always (explicit)    | 1    |
//! | `assert`             | `assert x > 0`         | Condition false      | 1    |
//! | `value of`           | `value of opt`         | Optional is Nothing  | 1    |
//! | function call        | `validate(x)`          | Callee exits         | 2    |
//!
//! "function call" (Tier 2) means any call to a user-defined function whose
//! body (directly or transitively) reaches one of the Tier-1 sources above.
//! `validate(x)` is a placeholder for such a function; Lale has no built-in
//! `validate`.

use crate::ast::definitions::{Expr, FnDefStmt, Program, SourceLocation, Stmt, UnaryOp};
use std::collections::{HashMap, HashSet};

/// Represents a single exit path found in the code.
#[derive(Debug, Clone)]
pub struct ExitPath {
  /// Location in source code.
  pub location: SourceLocation,
  /// Human-readable description of what exits and why.
  pub description: String,
  /// Category for grouping / filtering.
  pub kind: ExitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
  /// `exit program <code>`
  Explicit,
  /// `assert <condition>`
  Assert,
  /// `value of <optional>` — unwrap fails on Nothing
  ValueOf,
  /// A function call that transitively leads to an exit
  Transitive,
}

impl std::fmt::Display for ExitKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ExitKind::Explicit => write!(f, "explicit"),
      ExitKind::Assert => write!(f, "assert"),
      ExitKind::ValueOf => write!(f, "value-of"),
      ExitKind::Transitive => write!(f, "transitive"),
    }
  }
}

/// Collect all exit paths (Tier 1 + Tier 2 transitive) from a program.
///
/// When `is_debug` is false (release mode), assert statements are excluded
/// because they are not compiled in release builds.
pub fn find_exit_paths(program: &Program, is_debug: bool) -> Vec<ExitPath> {
  let mut paths = Vec::new();

  // ---- Tier 1: direct exit sites ----
  collect_from_statements(&program.statements, &mut paths, is_debug);

  // ---- Tier 2: transitive exits through function calls ----
  // Build call graph: fn_name → set of callee names
  let call_graph = build_call_graph(program);

  // Build map: fn_name → whether it has any direct exit path
  let mut fn_has_exit: HashMap<String, bool> = HashMap::new();
  for stmt in &program.statements {
    if let Stmt::FnDef(fn_def) = stmt {
      let mut direct = Vec::new();
      collect_from_statements(&fn_def.body, &mut direct, is_debug);
      fn_has_exit.insert(fn_def.name.node.clone(), !direct.is_empty());
    }
  }

  // Propagate transitively with cycle-safe DFS
  let mut propagating: HashSet<String> = HashSet::new();
  for fn_name in call_graph.keys() {
    let mut visited = HashSet::new();
    let exits = fn_exits_transitively(fn_name, &call_graph, &fn_has_exit, &mut visited);
    if exits {
      propagating.insert(fn_name.clone());
    }
  }

  // For each function that transitively exits, find all call sites in other
  // functions and report them. Skip top-level/external calls (we report
  // the caller's call site, not the callee's definition).
  let mut transitive: Vec<ExitPath> = Vec::new();
  for caller in call_graph.keys() {
    if !propagating.contains(caller) {
      continue;
    }
    // Find call sites inside this function's body
    if let Some(caller_fn) = find_fn_def(program, caller) {
      find_call_sites(
        &caller_fn.body,
        caller,
        &fn_has_exit,
        &propagating,
        &mut transitive,
        &call_graph,
      );
    }
  }

  // Also report call sites at top level
  for stmt in &program.statements {
    if !matches!(stmt, Stmt::FnDef(_)) {
      collect_transitive_from_stmt(
        stmt,
        &propagating,
        &fn_has_exit,
        &mut transitive,
        &call_graph,
      );
    }
  }

  paths.extend(transitive);

  // Sort by location for stable output
  paths.sort_by(|a, b| {
    a.location
      .line
      .cmp(&b.location.line)
      .then(a.location.col.cmp(&b.location.col))
  });
  paths
}

// ---- Call graph ----

type CallGraph = HashMap<String, HashSet<String>>;

/// Build a call graph: fn_name → {callee_names}.
fn build_call_graph(program: &Program) -> CallGraph {
  let mut graph: CallGraph = HashMap::new();
  for stmt in &program.statements {
    if let Stmt::FnDef(fn_def) = stmt {
      let mut callees = HashSet::new();
      collect_callees_from_stmts(&fn_def.body, &mut callees);
      graph.insert(fn_def.name.node.clone(), callees);
    }
  }
  // Also collect top-level call sites
  let mut top_callees = HashSet::new();
  for stmt in &program.statements {
    if !matches!(stmt, Stmt::FnDef(_)) {
      collect_callees_from_stmts(std::slice::from_ref(stmt), &mut top_callees);
    }
  }
  if !top_callees.is_empty() {
    graph.insert("<top-level>".to_string(), top_callees);
  }
  graph
}

fn collect_callees_from_stmts(stmts: &[Stmt], callees: &mut HashSet<String>) {
  for stmt in stmts {
    match stmt {
      Stmt::FnCall(fc) => {
        // Target path is always non-empty for a function call; default is unreachable.
        let name = fc.target.node.last().cloned().unwrap_or_default();
        callees.insert(name);
        for arg in &fc.arguments {
          collect_callees_from_expr(arg, callees);
        }
      }
      Stmt::If(if_stmt) => {
        collect_callees_from_stmts(&if_stmt.then_branch, callees);
        for (_, body) in &if_stmt.else_if_branches {
          collect_callees_from_stmts(body, callees);
        }
        if let Some(else_body) = &if_stmt.else_branch {
          collect_callees_from_stmts(else_body, callees);
        }
        collect_callees_from_expr(&if_stmt.condition.expr, callees);
      }
      Stmt::When(when_stmt) => {
        collect_callees_from_stmts(&when_stmt.body, callees);
        collect_callees_from_expr(&when_stmt.condition.expr, callees);
      }
      Stmt::Loop(loop_stmt) => {
        collect_callees_from_stmts(&loop_stmt.body, callees);
      }
      Stmt::Return(ret) => {
        if let Some(val) = &ret.value {
          collect_callees_from_expr(val, callees);
        }
      }
      Stmt::VarDef(def) => {
        collect_callees_from_expr(&def.value, callees);
      }
      Stmt::Assign(assign) => {
        collect_callees_from_expr(&assign.value, callees);
      }
      Stmt::CompoundAssign(ca) => {
        collect_callees_from_expr(&ca.value, callees);
      }
      Stmt::ValueAtAssign(va) => {
        collect_callees_from_expr(&va.value, callees);
      }
      Stmt::Debug(debug) => {
        collect_callees_from_expr(&debug.value, callees);
      }
      Stmt::Stdout(out) => {
        collect_callees_from_expr(&out.value, callees);
      }
      Stmt::Stderr(err) => {
        collect_callees_from_expr(&err.value, callees);
      }
      Stmt::Alert(alert) => {
        collect_callees_from_expr(&alert.value, callees);
      }
      Stmt::AddError(ae) => {
        collect_callees_from_expr(&ae.value, callees);
      }
      _ => {}
    }
  }
}

fn collect_callees_from_expr(expr: &Expr, callees: &mut HashSet<String>) {
  match expr {
    Expr::FnCallExpr(fc) => {
      // Target path is always non-empty for a function call; default is unreachable.
      let name = fc.target.node.last().cloned().unwrap_or_default();
      callees.insert(name);
      for arg in &fc.arguments {
        collect_callees_from_expr(arg, callees);
      }
    }
    Expr::Binary(bin) => {
      collect_callees_from_expr(&bin.left, callees);
      collect_callees_from_expr(&bin.right, callees);
    }
    Expr::Unary(un) => {
      collect_callees_from_expr(&un.operand, callees);
    }
    Expr::Grouped(inner) => {
      collect_callees_from_expr(inner, callees);
    }
    Expr::Conversion(conv) => {
      collect_callees_from_expr(&conv.operand, callees);
    }
    _ => {}
  }
}

// ---- Transitive analysis ----

/// DFS with cycle detection: does function `fn_name` transitively lead to an exit?
fn fn_exits_transitively(
  fn_name: &str,
  graph: &CallGraph,
  direct_exits: &HashMap<String, bool>,
  visited: &mut HashSet<String>,
) -> bool {
  if !visited.insert(fn_name.to_string()) {
    // Already visited in this DFS path → cycle, no new information
    return false;
  }
  if direct_exits.get(fn_name).copied().unwrap_or(false) {
    return true;
  }
  if let Some(callees) = graph.get(fn_name) {
    for callee in callees {
      if fn_exits_transitively(callee, graph, direct_exits, visited) {
        return true;
      }
    }
  }
  false
}

/// Find all call sites in `stmts` where the called function transitively exits.
fn find_call_sites(
  stmts: &[Stmt],
  caller_name: &str,
  direct_exits: &HashMap<String, bool>,
  propagating: &HashSet<String>,
  paths: &mut Vec<ExitPath>,
  graph: &CallGraph,
) {
  for stmt in stmts {
    match stmt {
      Stmt::FnCall(fc) => {
        // Target path is always non-empty for a function call; default is unreachable.
        let callee = fc.target.node.last().cloned().unwrap_or_default();
        if direct_exits.get(&callee).copied().unwrap_or(false) {
          paths.push(ExitPath {
            location: fc.location.clone(),
            description: format!("{}(...) → has direct exit path", callee),
            kind: ExitKind::Transitive,
          });
        } else if propagating.contains(&callee) && callee != caller_name {
          let chain = describe_chain(&callee, graph, direct_exits, propagating);
          paths.push(ExitPath {
            location: fc.location.clone(),
            description: format!("{}(...) → {}", callee, chain),
            kind: ExitKind::Transitive,
          });
        }
      }
      Stmt::If(if_stmt) => {
        find_call_sites(
          &if_stmt.then_branch,
          caller_name,
          direct_exits,
          propagating,
          paths,
          graph,
        );
        for (_, body) in &if_stmt.else_if_branches {
          find_call_sites(body, caller_name, direct_exits, propagating, paths, graph);
        }
        if let Some(else_body) = &if_stmt.else_branch {
          find_call_sites(
            else_body,
            caller_name,
            direct_exits,
            propagating,
            paths,
            graph,
          );
        }
      }
      Stmt::When(when_stmt) => {
        find_call_sites(
          &when_stmt.body,
          caller_name,
          direct_exits,
          propagating,
          paths,
          graph,
        );
      }
      Stmt::Loop(loop_stmt) => {
        find_call_sites(
          &loop_stmt.body,
          caller_name,
          direct_exits,
          propagating,
          paths,
          graph,
        );
      }
      _ => {}
    }
  }
}

/// Describe the chain: callee → next callee → ... → exit
fn describe_chain(
  start: &str,
  graph: &CallGraph,
  direct_exits: &HashMap<String, bool>,
  propagating: &HashSet<String>,
) -> String {
  if direct_exits.get(start).copied().unwrap_or(false) {
    return "has direct exit path".to_string();
  }
  if let Some(callees) = graph.get(start) {
    for callee in callees {
      if direct_exits.get(callee).copied().unwrap_or(false) {
        return format!("{}(...) (which has direct exit path)", callee);
      }
      if propagating.contains(callee) {
        let deeper = describe_chain(callee, graph, direct_exits, propagating);
        return format!("{}(...) → {}", callee, deeper);
      }
    }
  }
  "unknown chain".to_string()
}

fn collect_transitive_from_stmt(
  stmt: &Stmt,
  propagating: &HashSet<String>,
  direct_exits: &HashMap<String, bool>,
  paths: &mut Vec<ExitPath>,
  graph: &CallGraph,
) {
  match stmt {
    Stmt::If(if_stmt) => {
      find_call_sites(
        &if_stmt.then_branch,
        "<if-branch>",
        direct_exits,
        propagating,
        paths,
        graph,
      );
      for (_, body) in &if_stmt.else_if_branches {
        find_call_sites(body, "<else-if>", direct_exits, propagating, paths, graph);
      }
      if let Some(else_body) = &if_stmt.else_branch {
        find_call_sites(else_body, "<else>", direct_exits, propagating, paths, graph);
      }
    }
    Stmt::When(when_stmt) => {
      find_call_sites(
        &when_stmt.body,
        "<when>",
        direct_exits,
        propagating,
        paths,
        graph,
      );
    }
    Stmt::Loop(loop_stmt) => {
      find_call_sites(
        &loop_stmt.body,
        "<loop>",
        direct_exits,
        propagating,
        paths,
        graph,
      );
    }
    Stmt::FnCall(fc) => {
      // Target path is always non-empty for a function call; default is unreachable.
      let callee = fc.target.node.last().cloned().unwrap_or_default();
      if propagating.contains(&callee) {
        let chain = describe_chain(&callee, graph, direct_exits, propagating);
        paths.push(ExitPath {
          location: fc.location.clone(),
          description: format!("{}(...) → {}", callee, chain),
          kind: ExitKind::Transitive,
        });
      }
    }
    _ => {}
  }
}

// ---- Tier 1: direct exit sites ----

fn collect_from_statements(stmts: &[Stmt], paths: &mut Vec<ExitPath>, is_debug: bool) {
  for stmt in stmts {
    match stmt {
      Stmt::ExitProgram(exit) => {
        let code = exit
          .code
          .map_or_else(|| "unspecified".to_string(), |c| c.to_string());
        paths.push(ExitPath {
          location: exit.location.clone(),
          description: format!("exit program {}", code),
          kind: ExitKind::Explicit,
        });
      }
      Stmt::Assert(a) => {
        if is_debug {
          let cond_text = expr_snippet(&a.condition);
          paths.push(ExitPath {
            location: a.location.clone(),
            description: format!("assert {} (exits if condition is false)", cond_text),
            kind: ExitKind::Assert,
          });
        }
      }
      Stmt::FnDef(fn_def) => {
        collect_from_statements(&fn_def.body, paths, is_debug);
      }
      Stmt::If(if_stmt) => {
        collect_from_statements(&if_stmt.then_branch, paths, is_debug);
        for (_, body) in &if_stmt.else_if_branches {
          collect_from_statements(body, paths, is_debug);
        }
        if let Some(else_body) = &if_stmt.else_branch {
          collect_from_statements(else_body, paths, is_debug);
        }
      }
      Stmt::When(when_stmt) => {
        collect_from_statements(&when_stmt.body, paths, is_debug);
      }
      Stmt::Loop(loop_stmt) => {
        collect_from_statements(&loop_stmt.body, paths, is_debug);
        if let Some(cond) = &loop_stmt.pre_condition {
          collect_from_expr(&cond.expr, paths);
        }
        if let Some(cond) = &loop_stmt.post_condition {
          collect_from_expr(&cond.expr, paths);
        }
      }
      Stmt::Return(ret) => {
        if let Some(val) = &ret.value {
          collect_from_expr(val, paths);
        }
      }
      Stmt::VarDef(def) => {
        collect_from_expr(&def.value, paths);
      }
      Stmt::Assign(assign) => {
        collect_from_expr(&assign.value, paths);
      }
      Stmt::CompoundAssign(ca) => {
        collect_from_expr(&ca.value, paths);
      }
      Stmt::ValueAtAssign(va) => {
        collect_from_expr(&va.pointer, paths);
        collect_from_expr(&va.value, paths);
      }
      Stmt::Stdout(out) => {
        collect_from_expr(&out.value, paths);
      }
      Stmt::Stderr(err) => {
        collect_from_expr(&err.value, paths);
      }
      Stmt::Log(log) => {
        collect_from_expr(&log.value, paths);
      }
      Stmt::Debug(debug) => {
        collect_from_expr(&debug.value, paths);
      }
      Stmt::Alert(alert) => {
        collect_from_expr(&alert.value, paths);
      }
      Stmt::FnCall(fc) => {
        for arg in &fc.arguments {
          collect_from_expr(arg, paths);
        }
      }
      Stmt::AddError(ae) => {
        collect_from_expr(&ae.value, paths);
      }
      Stmt::Use(_)
      | Stmt::TypeDef(_)
      | Stmt::EnumDef(_)
      | Stmt::Switch(_)
      | Stmt::UnsafeDecl(_)
      | Stmt::FnSignature(_)
      | Stmt::ExitLoop(_)
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
      | Stmt::MoveOn(_)
      | Stmt::MissingCode(_)
      | Stmt::AlertErrors(_)
      | Stmt::Release(_)
      | Stmt::OnExit(_)
      | Stmt::Match(_)
      | Stmt::TestSuite(_) => {}
    }
  }
}

fn collect_from_expr(expr: &Expr, paths: &mut Vec<ExitPath>) {
  match expr {
    Expr::Unary(un) => {
      if matches!(un.operator, UnaryOp::ValueOf) {
        let inner_text = expr_snippet(&un.operand);
        paths.push(ExitPath {
          location: un.location.clone(),
          description: format!("value of {} (exits if the value is Nothing)", inner_text),
          kind: ExitKind::ValueOf,
        });
      }
      collect_from_expr(&un.operand, paths);
    }
    Expr::Binary(bin) => {
      collect_from_expr(&bin.left, paths);
      collect_from_expr(&bin.right, paths);
    }
    Expr::FnCallExpr(fc) => {
      for arg in &fc.arguments {
        collect_from_expr(arg, paths);
      }
    }
    Expr::ArrayLiteral(arr) => {
      for elem in &arr.elements {
        collect_from_expr(elem, paths);
      }
      if let Some(val) = &arr.fill {
        collect_from_expr(val, paths);
      }
    }
    Expr::ArrayIndex(ai) => {
      collect_from_expr(&ai.array, paths);
      for idx in &ai.indices {
        collect_from_expr(idx, paths);
      }
    }
    Expr::MemberAccess(ma) => {
      collect_from_expr(&ma.object, paths);
    }
    Expr::Grouped(inner) => {
      collect_from_expr(inner, paths);
    }
    Expr::Conversion(conv) => {
      collect_from_expr(&conv.operand, paths);
    }
    Expr::StringLiteral(sl) => {
      for part in &sl.parts {
        if let crate::ast::definitions::StringPart::EmbeddedValue(inner) = part {
          collect_from_expr(inner, paths);
        }
      }
    }
    Expr::Identifier(_)
    | Expr::IntLiteral(_)
    | Expr::UintLiteral(_)
    | Expr::FloatLiteral(_)
    | Expr::HexLiteral(_)
    | Expr::CharLiteral(_)
    | Expr::BoolLiteral(_)
    | Expr::CompilerConst(_)
    | Expr::NothingExpr
    | Expr::HasValue(_)
    | Expr::HasNoValue(_)
    | Expr::HasErrors
    | Expr::LastError
    | Expr::Allocate(_)
    | Expr::TryPropagate(_) => {}
  }
}

// ---- Helpers ----

fn find_fn_def<'a>(program: &'a Program, name: &str) -> Option<&'a FnDefStmt> {
  for stmt in &program.statements {
    if let Stmt::FnDef(fn_def) = stmt
      && fn_def.name.node == name
    {
      return Some(fn_def);
    }
  }
  None
}

fn expr_snippet(expr: &Expr) -> String {
  match expr {
    Expr::Identifier(id) => id.name().to_string(),
    Expr::Binary(bin) => {
      let op = match bin.operator {
        crate::ast::definitions::BinaryOp::Eq => "==",
        crate::ast::definitions::BinaryOp::NotEq => "!=",
        crate::ast::definitions::BinaryOp::Lt => "<",
        crate::ast::definitions::BinaryOp::Gt => ">",
        crate::ast::definitions::BinaryOp::LtEq => "<=",
        crate::ast::definitions::BinaryOp::GtEq => ">=",
        crate::ast::definitions::BinaryOp::Add => "+",
        crate::ast::definitions::BinaryOp::Sub => "-",
        crate::ast::definitions::BinaryOp::Mul => "*",
        crate::ast::definitions::BinaryOp::Div => "/",
        crate::ast::definitions::BinaryOp::Pow => "^",
        crate::ast::definitions::BinaryOp::And => "and",
        crate::ast::definitions::BinaryOp::Or => "or",
        crate::ast::definitions::BinaryOp::Xor => "xor",
        _ => "·",
      };
      format!(
        "{} {} {}",
        expr_snippet(&bin.left),
        op,
        expr_snippet(&bin.right)
      )
    }
    Expr::Unary(un) => match un.operator {
      UnaryOp::Neg => format!("-{}", expr_snippet(&un.operand)),
      UnaryOp::Not => format!("not {}", expr_snippet(&un.operand)),
      UnaryOp::ValueOf => format!("value of {}", expr_snippet(&un.operand)),
      _ => "...".to_string(),
    },
    Expr::FnCallExpr(fc) => {
      let name = fc.target.node.join(".");
      let args: Vec<_> = fc.arguments.iter().map(expr_snippet).collect();
      format!("{}({})", name, args.join(", "))
    }
    Expr::IntLiteral(l) => l.value.to_string(),
    Expr::UintLiteral(l) => l.value.to_string(),
    Expr::FloatLiteral(l) => l.value.to_string(),
    Expr::StringLiteral(sl) => {
      for part in &sl.parts {
        if let crate::ast::definitions::StringPart::EmbeddedValue(inner) = part {
          return format!("\"...{{{}}}...\"", expr_snippet(inner));
        }
      }
      "\"...\"".to_string()
    }
    Expr::BoolLiteral(b) => b.value.to_string(),
    _ => "...".to_string(),
  }
}

/// Format exit paths for terminal output.
pub fn format_exit_paths(paths: &[ExitPath], source_file: &str) -> String {
  if paths.is_empty() {
    return format!("No exit paths found in {}", source_file);
  }

  let mut report = format!("Exit paths in {}:\n\n", source_file);
  for path in paths {
    report.push_str(&format!(
      "  {}:{}:{}  {:<12}  {}\n",
      path.location.source_file,
      path.location.line,
      path.location.col,
      path.kind.to_string(),
      path.description,
    ));
  }
  report.push('\n');
  report.push_str(&format!("Total: {} exit path(s)\n", paths.len()));
  report
}
