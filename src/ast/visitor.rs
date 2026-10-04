//! Visitor Pattern for the Lale AST
//!
//! This module implements the **Visitor design pattern** for AST traversal and analysis.
//! It allows different compiler passes to operate on the AST without modifying its structure.
//!
//! # Design Pattern
//!
//! The **`AstVisitor<T>` trait** defines methods for visiting every AST node type,
//! where `T` is the return type. This enables:
//!
//! - **Multiple Passes**: Different visitors can analyze the same AST independently
//! - **Separation of Concerns**: Each pass has its own implementation without cross-contamination
//! - **Extensibility**: Add new passes by implementing the trait without touching the AST
//! - **Type Safety**: Rust's type system ensures exhaustive pattern matching on all node types
//!
//! # Key Concept: Default Dispatch Methods
//!
//! The trait includes **default dispatch methods** that pattern-match on enum variants:
//!
//! ```text
//! fn visit_stmt(&mut self, stmt: &Stmt) -> T {
//!     match stmt {
//!         Stmt::VarDef(var_def) => self.visit_var_def(var_def),
//!         Stmt::Assign(assign) => self.visit_assign(assign),
//!         // ... all variants handled
//!     }
//! }
//! ```
//!
//! **Implementation task**: Provide concrete implementations for specific node types
//! (`visit_var_def`, `visit_assign`, etc.). The dispatch methods are already implemented
//! with sensible defaults.
//!
//! # Existing Implementations
//!
//! Three major visitors are currently implemented:
//!
//! | Visitor | Module | Purpose | Return Type |
//! |---------|--------|---------|-------------|
//! | `AstPrinter` | `ast_printer` | Pretty-print AST for debugging | `()` |
//! | `SemanticAnalyzer` | `semantic_analysis::analyzer` | Type/unit/scope checking | `()` |
//! | `CodeGen` | `codegen` | LLVM IR generation | `()` |
//!
//! # Creating a New Visitor
//!
//! Example: Create a "node counter" that counts each node type:
//!
//! ```text
//! use lale::ast_visitor::AstVisitor;
//! use lale::ast::*;
//!
//! pub struct NodeCounter {
//!     stmt_count: usize,
//!     expr_count: usize,
//! }
//!
//! impl NodeCounter {
//!     pub fn new() -> Self {
//!         NodeCounter {
//!             stmt_count: 0,
//!             expr_count: 0,
//!         }
//!     }
//! }
//!
//! impl AstVisitor<()> for NodeCounter {
//!     fn visit_program(&mut self, program: &Program) {
//!         for stmt in &program.statements {
//!             self.visit_stmt(stmt);
//!         }
//!     }
//!
//!     fn visit_var_def(&mut self, _var_def: &VarDefStmt) {
//!         self.stmt_count += 1;
//!     }
//!
//!     fn visit_binary(&mut self, binary: &BinaryExpr) {
//!         self.expr_count += 1;
//!         self.visit_expr(&binary.left);
//!         self.visit_expr(&binary.right);
//!     }
//!
//!     // ... implement other required methods
//! }
//! ```
//!
//! # Visitor Method Categories
//!
//! Methods are organized by node type:
//! - **Program**: Entry point (`visit_program`)
//! - **Statements**: Definitions, assignments, control flow
//! - **Expressions**: Binary/unary ops, literals, function calls
//! - **Types and Units**: Type annotations, unit expressions
//!
//! # Implementation Notes for Developers
//!
//! 1. **Mutable Self**: Visitors take `&mut self` to maintain internal state
//!    (e.g., `SemanticAnalyzer` tracks symbol tables, `AstPrinter` tracks indentation)
//!
//! 2. **Recursive Traversal**: Most visitor methods call other visitor methods
//!    on child nodes to recurse through the AST
//!
//! 3. **Return Types**: The generic `T` parameter allows different return types:
//!    - `()` for side-effect-only passes (printing, collecting errors)
//!    - `Option<T>` for optional results
//!    - Custom types for complex analyses
//!
//! 4. **Default Dispatch**: Don't override dispatch methods like `visit_stmt()`;
//!    implement specific methods like `visit_var_def()` instead

use super::definitions::*;

/// The main visitor trait for traversing and operating on the AST.
///
/// Each visit method corresponds to a specific AST node type. Implement
/// this trait to perform operations like:
/// - Pretty printing
/// - Semantic analysis
/// - Type checking
/// - Code generation
/// - Optimization passes
///
/// # Type Parameters
/// * `T` - The return type of visitor methods
pub trait AstVisitor<T> {
  // ==================== Program ====================
  fn visit_program(&mut self, program: &Program) -> T;

  // ==================== Statements ====================
  fn visit_stmt(&mut self, stmt: &Stmt) -> T {
    match stmt {
      Stmt::Use(use_stmt) => self.visit_use(use_stmt),
      Stmt::TypeDef(type_def) => self.visit_type_def(type_def),
      Stmt::EnumDef(enum_def) => self.visit_enum_def(enum_def),
      Stmt::VarDef(var_def) => self.visit_var_def(var_def),
      Stmt::UnsafeDecl(decl) => self.visit_unsafe_decl(decl),
      Stmt::Assign(assign) => self.visit_assign(assign),
      Stmt::CompoundAssign(compound) => self.visit_compound_assign(compound),
      Stmt::ValueAtAssign(value_at) => self.visit_value_at_assign(value_at),
      Stmt::FnDef(fn_def) => self.visit_fn_def(fn_def),
      Stmt::FnSignature(fn_signature) => self.visit_fn_signature(fn_signature),
      Stmt::FnCall(fn_call) => self.visit_fn_call_stmt(fn_call),
      Stmt::If(if_stmt) => self.visit_if(if_stmt),
      Stmt::When(when_stmt) => self.visit_when(when_stmt),
      Stmt::Match(match_stmt) => self.visit_match(match_stmt),
      Stmt::Switch(switch_stmt) => self.visit_switch(switch_stmt),
      Stmt::Loop(loop_stmt) => self.visit_loop(loop_stmt),
      Stmt::Return(ret) => self.visit_return(ret),
      Stmt::ExitProgram(exit_program) => self.visit_exit_program(exit_program),
      Stmt::ExitLoop(exit_loop) => self.visit_exit_loop(exit_loop),
      Stmt::Rewind(rewind) => self.visit_rewind(rewind),
      Stmt::Stdout(stdout) => self.visit_stdout(stdout),
      Stmt::Stderr(stderr) => self.visit_stderr(stderr),
      Stmt::Log(log) => self.visit_log(log),
      Stmt::Stdin(stdin) => self.visit_stdin(stdin),
      Stmt::Debug(debug) => self.visit_debug(debug),
      Stmt::CtIf(ct_if) => self.visit_ct_if(ct_if),
      Stmt::CtFail(ct_fail) => self.visit_ct_fail(ct_fail),
      Stmt::CtWarn(ct_warn) => self.visit_ct_warn(ct_warn),
      Stmt::CtWhen(ct_when) => self.visit_ct_when(ct_when),
      Stmt::CtMatch(ct_match) => self.visit_ct_match(ct_match),
      Stmt::CtSwitch(ct_switch) => self.visit_ct_switch(ct_switch),
      Stmt::Assert(a) => self.visit_assert(a),
      Stmt::Doc(doc) => self.visit_doc(doc),
      Stmt::Comment(comment) => self.visit_comment(comment),
      Stmt::AddError(add_error) => self.visit_add_error(add_error),
      Stmt::AlertErrors(alert_errors) => self.visit_alert_errors(alert_errors),
      Stmt::Alert(alert) => self.visit_alert_stmt(alert),
      Stmt::MoveOn(move_on) => self.visit_move_on(move_on),
      Stmt::MissingCode(missing) => self.visit_missing_code(missing),
      Stmt::Release(release) => self.visit_release(release),
      Stmt::OnExit(on_exit) => self.visit_on_exit(on_exit),
      Stmt::TestSuite(test_suite) => self.visit_test_suite(test_suite),
    }
  }

  /// Visit module use statement.
  fn visit_use(&mut self, use_stmt: &UseStmt) -> T;
  /// Visit type definition.
  fn visit_type_def(&mut self, type_def: &TypeDefStmt) -> T;
  /// Visit enum definition.
  fn visit_enum_def(&mut self, enum_def: &EnumDefStmt) -> T;
  /// Visit variable definition.
  fn visit_var_def(&mut self, var_def: &VarDefStmt) -> T;
  /// Visit unsafe variable declaration.
  fn visit_unsafe_decl(&mut self, decl: &UnsafeDeclStmt) -> T;
  /// Visit assignment statement.
  fn visit_assign(&mut self, assign: &AssignStmt) -> T;
  /// Visit compound assignment statement.
  fn visit_compound_assign(&mut self, compound: &CompoundAssignStmt) -> T;
  /// Visit value-at assignment statement.
  fn visit_value_at_assign(&mut self, value_at: &ValueAtAssignStmt) -> T;
  /// Visit function definition.
  fn visit_fn_def(&mut self, fn_def: &FnDefStmt) -> T;
  /// Visit function signature.
  fn visit_fn_signature(&mut self, fn_signature: &FnSignatureStmt) -> T;
  /// Visit function call statement.
  fn visit_fn_call_stmt(&mut self, fn_call: &FnCall) -> T;
  /// Visit if statement.
  fn visit_if(&mut self, if_stmt: &IfStmt) -> T;
  /// Visit when statement (one-sided action, no else).
  fn visit_when(&mut self, when_stmt: &WhenStmt) -> T {
    self.visit_condition(&when_stmt.condition);
    for stmt in &when_stmt.body {
      self.visit_stmt(stmt);
    }
    if let Some(else_body) = &when_stmt.else_branch {
      for stmt in else_body {
        self.visit_stmt(stmt);
      }
    }
    self.visit_condition(&when_stmt.condition) // Return the condition result
  }
  /// Visit move on statement (intentional no-op).
  fn visit_move_on(&mut self, _stmt: &MoveOnStmt) -> T {
    // Default: no-op, nothing to visit
    unimplemented!("visit_move_on must be implemented by the visitor")
  }
  /// Visit missing code statement (deferred implementation placeholder).
  fn visit_missing_code(&mut self, _stmt: &MissingCodeStmt) -> T {
    // Default: no-op, nothing to visit
    unimplemented!("visit_missing_code must be implemented by the visitor")
  }
  /// Visit release statement: `release ptr_expr`
  fn visit_release(&mut self, stmt: &ReleaseStmt) -> T {
    self.visit_expr(&stmt.pointer);
    unimplemented!("visit_release must return T")
  }
  /// Visit on-exit statement: `on exit <stmt>`
  fn visit_on_exit(&mut self, stmt: &OnExitStmt) -> T {
    self.visit_stmt(&stmt.body);
    unimplemented!("visit_on_exit must return T")
  }
  /// Visit test suite statement: `test suite <name> ... end test suite`
  fn visit_test_suite(&mut self, test_suite: &TestSuiteStmt) -> T {
    for item in &test_suite.items {
      match item {
        TestSuiteItem::Case(case) => self.visit_test_case(case),
        TestSuiteItem::Declaration(stmt) => self.visit_stmt(stmt.as_ref()),
        TestSuiteItem::Comment(comment) => self.visit_comment(comment),
        TestSuiteItem::Doc(doc) => self.visit_doc(doc),
      };
    }
    unimplemented!("visit_test_suite must be implemented by the visitor")
  }
  /// Visit a single test case: `test case <name> ... end test case`
  fn visit_test_case(&mut self, test_case: &TestCaseStmt) -> T {
    for stmt in &test_case.body {
      self.visit_stmt(stmt);
    }
    unimplemented!("visit_test_case must be implemented by the visitor")
  }
  /// Visit allocate expression: `allocate(size_expr)`
  fn visit_allocate(&mut self, alloc: &AllocateExpr) -> T {
    self.visit_expr(&alloc.size);
    unimplemented!("visit_allocate must return T")
  }
  /// Visit match statement (conditional branching with multiple arms).
  fn visit_match(&mut self, match_stmt: &MatchStmt) -> T {
    for arm in &match_stmt.arms {
      self.visit_condition(&arm.guard);
      for stmt in &arm.body {
        self.visit_stmt(stmt);
      }
    }
    for stmt in &match_stmt.else_arm {
      self.visit_stmt(stmt);
    }
    // Return result from last condition visit
    match_stmt.arms.last().map_or_else(
      || unimplemented!("visit_match must be implemented by the visitor"),
      |arm| self.visit_condition(&arm.guard),
    )
  }
  /// Visit switch statement (exhaustive pattern matching over enums).
  fn visit_switch(&mut self, switch_stmt: &SwitchStmt) -> T;
  /// Visit loop statement.
  fn visit_loop(&mut self, loop_stmt: &LoopStmt) -> T;
  /// Visit return statement.
  fn visit_return(&mut self, ret: &ReturnStmt) -> T;
  /// Visit exit program statement.
  fn visit_exit_program(&mut self, exit_program: &ExitProgramStmt) -> T;
  /// Visit exit loop statement.
  fn visit_exit_loop(&mut self, exit_loop: &ExitLoopStmt) -> T;
  /// Visit rewind statement.
  fn visit_rewind(&mut self, rewind: &RewindStmt) -> T;
  /// Visit stdout statement.
  fn visit_stdout(&mut self, stdout: &StdoutStmt) -> T;
  /// Visit stderr statement.
  fn visit_stderr(&mut self, stderr: &StderrStmt) -> T;
  /// Visit log statement.
  fn visit_log(&mut self, log: &LogStmt) -> T;
  /// Visit debug statement.
  fn visit_debug(&mut self, debug: &DebugStmt) -> T;
  /// Visit stdin statement.
  fn visit_stdin(&mut self, stdin: &StdinStmt) -> T;
  /// Visit compile-time if statement.
  fn visit_ct_if(&mut self, ct_if: &CtIfStmt) -> T;
  /// Visit compile-time fail statement.
  fn visit_ct_fail(&mut self, ct_fail: &CtFailStmt) -> T;
  /// Visit compile-time warn statement.
  fn visit_ct_warn(&mut self, ct_warn: &CtWarnStmt) -> T;
  /// Visit compile-time when statement.
  fn visit_ct_when(&mut self, ct_when: &CtWhenStmt) -> T;
  /// Visit compile-time match statement.
  fn visit_ct_match(&mut self, ct_match: &CtMatchStmt) -> T;
  /// Visit compile-time switch statement.
  fn visit_ct_switch(&mut self, ct_switch: &CtSwitchStmt) -> T;
  /// Visit assert statement.
  fn visit_assert(&mut self, assert: &AssertStmt) -> T;
  /// Visit documentation comment.
  fn visit_doc(&mut self, doc: &DocStmt) -> T;
  /// Visit code comment.
  fn visit_comment(&mut self, comment: &CommentStmt) -> T;
  /// Visit add error statement.
  fn visit_add_error(&mut self, add_error: &AddErrorStmt) -> T;
  /// Visit alert error messages statement.
  fn visit_alert_errors(&mut self, alert_errors: &AlertErrorsStmt) -> T;
  /// Visit alert output statement.
  fn visit_alert_stmt(&mut self, alert: &AlertStmt) -> T;

  // ==================== Expressions ====================
  fn visit_expr(&mut self, expr: &Expr) -> T {
    match expr {
      Expr::Binary(bin) => self.visit_binary(bin),
      Expr::Unary(un) => self.visit_unary(un),
      Expr::Identifier(id) => self.visit_identifier(id),
      Expr::IntLiteral(lit) => self.visit_int_literal(lit),
      Expr::UintLiteral(lit) => self.visit_uint_literal(lit),
      Expr::FloatLiteral(lit) => self.visit_float_literal(lit),
      Expr::HexLiteral(lit) => self.visit_hex_literal(lit),
      Expr::CharLiteral(lit) => self.visit_char_literal(lit),
      Expr::BoolLiteral(lit) => self.visit_bool_literal(lit),
      Expr::StringLiteral(lit) => self.visit_string_literal(lit),
      Expr::ArrayLiteral(lit) => self.visit_array_literal(lit),
      Expr::FnCallExpr(fn_call) => self.visit_fn_call_expr(fn_call),
      Expr::MemberAccess(acc) => self.visit_member_access(acc),
      Expr::ArrayIndex(idx) => self.visit_array_index(idx),
      Expr::CompilerConst(cc) => self.visit_compiler_const(cc),
      Expr::Grouped(inner) => self.visit_expr(inner),
      Expr::Conversion(conv) => self.visit_conversion(conv),
      Expr::HasValue(inner) => self.visit_has_value(inner),
      Expr::HasNoValue(inner) => self.visit_has_no_value(inner),
      Expr::NothingExpr => self.visit_nothing_expr(),
      Expr::HasErrors => self.visit_has_errors(),
      Expr::LastError => self.visit_last_error(),
      Expr::TryPropagate(inner) => self.visit_try_propagate(inner),
      Expr::Allocate(alloc) => self.visit_allocate(alloc),
    }
  }

  /// Visit binary operation expression.
  fn visit_binary(&mut self, bin: &BinaryExpr) -> T;
  /// Visit unary operation expression.
  fn visit_unary(&mut self, un: &UnaryExpr) -> T;
  /// Visit type conversion expression.
  fn visit_conversion(&mut self, conv: &ConversionExpr) -> T;
  /// Visit identifier expression.
  fn visit_identifier(&mut self, id: &IdentifierExpr) -> T;
  /// Visit signed integer literal.
  fn visit_int_literal(&mut self, lit: &IntLiteral) -> T;
  /// Visit unsigned integer literal.
  fn visit_uint_literal(&mut self, lit: &UintLiteral) -> T;
  /// Visit floating-point literal.
  fn visit_float_literal(&mut self, lit: &FloatLiteral) -> T;
  /// Visit hexadecimal literal.
  fn visit_hex_literal(&mut self, lit: &HexLiteral) -> T;
  /// Visit character literal.
  fn visit_char_literal(&mut self, lit: &CharLiteral) -> T;
  /// Visit boolean literal.
  fn visit_bool_literal(&mut self, lit: &BoolLiteral) -> T;
  /// Visit string literal.
  fn visit_string_literal(&mut self, lit: &StringLiteral) -> T;
  /// Visit array literal.
  fn visit_array_literal(&mut self, lit: &ArrayLiteral) -> T;
  /// Visit function call expression.
  fn visit_fn_call_expr(&mut self, fn_call: &FnCall) -> T;
  /// Visit member access expression.
  fn visit_member_access(&mut self, acc: &MemberAccess) -> T;
  /// Visit array indexing expression.
  fn visit_array_index(&mut self, idx: &ArrayIndex) -> T;
  /// Visit compiler constant expression.
  fn visit_compiler_const(&mut self, cc: &CompilerConst) -> T;
  /// Visit nothing expression: `nothing`.
  fn visit_nothing_expr(&mut self) -> T;
  /// Visit has value expression: `expr has value`.
  fn visit_has_value(&mut self, expr: &Expr) -> T {
    self.visit_expr(expr)
  }
  /// Visit has no value expression: `expr has no value`.
  fn visit_has_no_value(&mut self, expr: &Expr) -> T {
    self.visit_expr(expr)
  }
  /// Visit has errors expression: `errors has messages`.
  fn visit_has_errors(&mut self) -> T;
  /// Visit last error expression: `last error`.
  fn visit_last_error(&mut self) -> T;
  /// Visit try-propagate expression: `expr?`.
  fn visit_try_propagate(&mut self, expr: &Expr) -> T {
    self.visit_expr(expr)
  }

  // ==================== Types ====================
  /// Visit type name.
  fn visit_type_name(&mut self, type_name: &TypeName) -> T;
  /// Visit function parameter.
  fn visit_parameter(&mut self, param: &Parameter) -> T;
  /// Visit conditional expression.
  fn visit_condition(&mut self, cond: &Condition) -> T;
  /// Visit loop range specification.
  fn visit_range(&mut self, range: &Range) -> T;
}

/// Walk a program using a visitor.
pub fn walk_program<T>(program: &Program, visitor: &mut dyn AstVisitor<T>) -> T {
  visitor.visit_program(program)
}

/// Walk statements using a visitor.
pub fn walk_statements<T>(stmts: &[Stmt], visitor: &mut dyn AstVisitor<T>) -> Vec<T> {
  stmts.iter().map(|stmt| visitor.visit_stmt(stmt)).collect()
}

/// Walk an expression using a visitor.
pub fn walk_expr<T>(expr: &Expr, visitor: &mut dyn AstVisitor<T>) -> T {
  visitor.visit_expr(expr)
}
