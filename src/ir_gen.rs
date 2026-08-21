//! IR Generation - Converts the AST to Intermediate Representation
//!
//! This module implements the IR generator visitor that transforms the semantic AST
//! into the Lale IR, which serves as a platform-independent intermediate representation
//! suitable for execution by the interpreter and lowering to future backend targets.
//!
//! # Architecture
//!
//! The IR generation process:
//! 1. Create an `IrGenerator` with a reference to the AST and semantic analyzer
//! 2. Traverse the AST recursively, generating IR instructions
//! 3. Track variable bindings and generate code for expressions
//! 4. Return a complete `Module` containing all IR code
//!
//! # Design
//!
//! - Uses SSA form: each variable is assigned exactly once
//! - Maintains a symbol table mapping AST symbols to IR values
//! - Handles type conversions and unit tracking
//! - Generates code for expressions that produces IR values
//! - Generates code for statements that may modify control flow
//!
//! Quick navigation:
//!   line 114   — Constructor & setup (new, with_mode, with_mode_and_stdlib)
//!   line 163   — Scope helpers (push_scope, pop_scope)
//!   line 181   — Variable management (lookup_var, var_ptr, define_var)
//!   line 242   — Type/unit lookups (lookup_expr_type, lookup_expr_unit, etc.)
//!   line 349   — Optional type helpers (optional_struct_name, ensure_optional_struct, etc.)
//!   line 427   — Entry points (try_generate, try_generate_owned)
//!   line 572   — Stdlib loading (load_stdlib_into_module, etc.)
//!   line 838   — Statement dispatch (try_generate_stmt)
//!   line 985   — Variable definitions (try_generate_var_def)
//!   line 1098  — Assignment (try_generate_assign, try_generate_compound_assign)
//!   line 1396  — Function definitions (try_generate_fn_def)
//!   line 1539  — Unsafe decl & value-at assign (generate_unsafe_decl, generate_value_at_assign)
//!   line 1554  — Control flow (generate_if, generate_loop, generate_return, generate_exit_program)
//!   line 1855  — I/O (generate_stdout, generate_stderr, generate_debug, generate_stdin)
//!   line 2061  — Error stack (generate_add_error, generate_drain_errors, generate_alert)
//!   line 2085  — Expression dispatch (try_generate_expr, try_generate_expr_with_resolved_type)
//!   line 2512  — Binary/unary operations (generate_binary_op, generate_unary_expr, generate_unary_op)
//!   line 2771  — Pointer operations (generate_pointer_to, generate_value_at)
//!   line 2915  — Type conversion (generate_type_conversion)
//!   line 3037  — Literals (int, uint, float, hex, char, bool, string)
//!   line 3204  — Function calls (generate_fn_call_expr)
//!   line 3550  — Type helpers (type_name_to_ir_type, type_string_to_ir_type, etc.)
//!   line 3917  — Array operations (compute_array_offset, get_array_element_type, etc.)
//!   line 4008  — Type constructor (generate_type_constructor)
//!   line 4112  — Array initialization/literals (initialize_array_from_literal, generate_array_literal, etc.)

use crate::ast::Spanned;
use crate::ast::definitions::{
  AddErrorStmt, AlertStmt, AllocateExpr, ArrayLiteral, AssertStmt, AssignStmt, BaseType, BinaryOp,
  BoolLiteral, CharLiteral, CompilerConstKind, CompoundAssignStmt, CompoundOp, DebugStmt,
  EnumDefStmt, ExitProgramStmt, Expr, FloatLiteral, FnCall, FnDefStmt, HexLiteral, IfStmt,
  IntLiteral, LoopStmt, MatchStmt, MissingCodeStmt, Program, ReleaseStmt, ReturnStmt,
  ReturnTypeKind, SourceLocation, StderrStmt, StdinStmt, StdoutStmt, Stmt, StringLiteral,
  StringPart, SwitchCase, SwitchPattern, SwitchPatternField, SwitchStmt, TypeDefStmt, TypeName,
  UintLiteral, UnaryExpr, UnaryOp, ValueAtAssignStmt, VarDefStmt, WhenStmt,
};
use crate::error::{CompileResult, IrGenError};
use crate::semantic_analysis::VarScope;
use crate::semantic_analysis::expression_analysis::ExpressionAnalyzer;
use pest::Parser;

/// Output destination for write/warn statements.
#[derive(Clone, Copy)]
enum OutputDest {
  Stdout,
  Stderr,
  Alert,
}
use crate::ir::builder::IrBuilder;
use crate::ir::function::Linkage;
use crate::ir::types::IrType;
use crate::ir::values::{BlockId, GlobalId, ValueId};
use crate::ir::{Instruction, Module};
use crate::semantic_analysis::SqliteSymbolManager;
use std::collections::HashMap;
use std::collections::HashSet;

/// Represents a variable's storage location and type information.
#[derive(Debug, Clone)]
struct VarInfo {
  /// The allocation (pointer to stack space).
  /// For global variables, this holds the GlobalAddr ValueId valid in the defining function.
  allocation: ValueId,
  /// The type of the variable
  var_type: IrType,
  /// For global-scope variables: the IR GlobalId, accessible from any function via GlobalAddr.
  /// For local variables: None.
  global_id: Option<GlobalId>,
}

/// Represents the layout of a type (field names and offsets).
#[derive(Debug, Clone)]
struct TypeLayout {
  /// Field names in order
  field_names: Vec<String>,
  /// Type of each field
  field_types: Vec<IrType>,
  /// Physical unit of each field (None if unitless)
  field_units: Vec<Option<String>>,
  /// Total size of the type in bytes
  #[allow(dead_code)]
  total_size: i64,
}

/// Returns true if a test case should run under the active `--filter` patterns.
/// An empty pattern list (no filter) matches everything. A pattern containing
/// `/` (matching the `Pass: suite / case` output format) matches a substring of
/// the full `suite/case` path; any other pattern matches a substring of the
/// suite name or the case name. A case runs if ANY pattern matches.
fn test_case_matches(suite: &str, case: &str, filters: &[String]) -> bool {
  if filters.is_empty() {
    return true;
  }
  let full = format!("{}/{}", suite, case);
  filters.iter().any(|pattern| {
    if pattern.contains('/') {
      full.contains(pattern)
    } else {
      suite.contains(pattern) || case.contains(pattern)
    }
  })
}

/// Generates IR from an AST.
pub struct IrGenerator<'a> {
  /// The IR builder used for constructing IR
  builder: IrBuilder,
  /// Stack of scopes for variable tracking.
  /// Each scope is a HashMap from variable name to VarInfo (allocation + type).
  /// The first entry is the global scope; a new entry is pushed per function
  /// (including the synthetic `main`) and popped when that function ends.
  /// Lale has exactly two scope levels (global + function-local), so block
  /// bodies (`if`/`when`/`match`/`switch`/`loop`) do not push scopes.
  scope_stack: Vec<HashMap<String, VarInfo>>,
  /// Stack of loop contexts for break/continue handling.
  /// Each entry contains the loop exit block for that loop level.
  loop_stack: Vec<BlockId>,
  /// Stack of loop headers for rewind handling.
  /// Each entry contains the loop header block for that loop level.
  /// Parallel to loop_stack, pushed/popped in sync.
  loop_header_stack: Vec<BlockId>,
  /// Reference to the symbol manager for type lookups
  symbol_manager: Option<&'a crate::semantic_analysis::SqliteSymbolManager>,
  /// Current function name (for __function__ compiler constant)
  current_function: Option<String>,
  /// Current function return type (for optional wrapping in returns)
  current_return_type: Option<IrType>,
  /// The shared epilogue block (single cleanup path for all returns).
  current_epilogue: Option<BlockId>,
  /// Alloca that carries the return value across the branch to the epilogue.
  current_return_slot: Option<ValueId>,
  /// Type layouts: maps type name to field information
  type_layouts: HashMap<String, TypeLayout>,
  /// Set of enum type names (used to dispatch debug formatting)
  enum_types: std::collections::HashSet<String>,
  /// Maps enum type name to list of (variant_name, discriminant_index)
  enum_variants: HashMap<String, Vec<(String, usize)>>,
  /// Mode: "main" for executable, "lib" for library (no main function wrapper)
  mode: String,
  /// Standard library inclusion level
  stdlib: crate::config::StdlibLevel,
  /// True when generating code inside a user-defined function
  /// (as opposed to top-level code in the synthetic main).
  /// Used to distinguish global-scope definitions from function-local ones.
  in_user_function: bool,
  /// Debug mode flag from compiler options. Controls `#debug` compiler constant.
  is_debug: bool,
  /// Whether integer arithmetic traps on overflow by default. When false
  /// (`--unchecked_overflow`), integer `+`/`-`/`*`/neg wrap like the IR's
  /// canonical wrapping semantics.
  checked_overflow: bool,
  /// Execution mode: false = run (default), true = test. Controls the `#mode`
  /// compile-time constant and gates top-level user code vs test suites.
  test_mode: bool,
  /// Optional `lale test --filter <pattern>` list. When non-empty, only test
  /// suites/cases matching any of the patterns are emitted into the IR; all
  /// others are skipped at code-generation time (semantic analysis still
  /// validates them).
  test_filter: Vec<String>,
  /// True while generating the body of a `test case`. Used to make `assert`
  /// report-and-continue in test mode instead of aborting.
  in_test_case: bool,
  /// When generating a test suite, the `suite::` prefix used to mangle suite
  /// globals/functions so they don't collide with module globals of the same
  /// name. `None` outside test suites.
  suite_scope_name: Option<String>,
  /// Suite-scope globals (mangled `suite::name`) whose str data must be freed at
  /// program exit. They live in the popped suite scope rather than the global
  /// scope, so `emit_global_str_auto_free` walks this list in addition to the
  /// global scope.
  suite_globals: Vec<(GlobalId, IrType)>,
  /// Stack of "sibling-defined" variable-name sets, one per open control-flow
  /// construct. When a `var` is defined in a sibling branch (already defined in
  /// a previous branch of the same construct), `try_generate_var_def` reuses the
  /// existing slot instead of allocating a second one (the definite-assignment
  /// join already happened in semantic analysis).
  sibling_defined: Vec<std::collections::HashSet<String>>,
  /// Whether to emit ANSI color/formatting codes in output.
  use_color: bool,
  /// Allocas of str-typed local variables that have been returned or otherwise
  /// transferred (stored in a global, passed to a function that takes ownership).
  /// The auto-free pass skips these variables.
  consumed_str_allocas: HashSet<ValueId>,
  /// Allocas whose str *data* transfers to the caller but whose alloca wrapper
  /// is temporary and should still be reclaimed at function exit (e.g. a local
  /// `str` variable returned by value).
  transferred_str_data: HashSet<ValueId>,
  /// Names of function parameters (str params are owned by the caller, not freed).
  param_names: HashSet<String>,
  /// On-exit statements collected during function body generation.
  /// Emitted before every return and at end-of-function, in registration order.
  on_exit_stmts: Vec<Stmt>,
}

impl<'a> IrGenerator<'a> {
  /// Create a new IR generator in "main" mode (executable).
  pub fn new(module_name: impl Into<String>) -> Self {
    Self::with_mode(module_name, "main")
  }

  /// Create a new IR generator with specified mode.
  /// mode: "main" for executable (generate main wrapper), "lib" for library
  pub fn with_mode(module_name: impl Into<String>, mode: &str) -> Self {
    Self::with_mode_and_stdlib(module_name, mode, crate::config::StdlibLevel::Full)
  }

  /// Create a new IR generator with specified mode and stdlib level.
  pub fn with_mode_and_stdlib(
    module_name: impl Into<String>,
    mode: &str,
    stdlib: crate::config::StdlibLevel,
  ) -> Self {
    Self::with_mode_stdlib_and_module(module_name, mode, stdlib, None)
  }

  /// Create an IR generator using a pre-existing module (already populated
  /// with builtins types). Used when the stdlib is loaded before user code.
  pub fn with_module_and_stdlib(
    module: Module,
    mode: &str,
    stdlib: crate::config::StdlibLevel,
  ) -> Self {
    Self::with_mode_stdlib_and_module(module.name.clone(), mode, stdlib, Some(module))
  }

  /// Shared constructor. If `existing_module` is provided, it's used instead
  /// of creating a new one via `IrBuilder::new()`.
  fn with_mode_stdlib_and_module(
    module_name: impl Into<String>,
    mode: &str,
    stdlib: crate::config::StdlibLevel,
    existing_module: Option<Module>,
  ) -> Self {
    let mut type_layouts = HashMap::new();

    // Bootstrap: pre-register str layout (see Module::new() comment).
    // In the normal pipeline, builtins.lale's `type str` overwrites this.
    type_layouts.insert(
      "str".to_string(),
      TypeLayout {
        field_names: vec!["ptr".to_string(), "len".to_string()],
        field_types: vec![IrType::raw_ptr(), IrType::I64],
        field_units: vec![None, None],
        total_size: 16,
      },
    );

    let builder = if let Some(module) = existing_module {
      IrBuilder::with_module(module)
    } else {
      IrBuilder::new(module_name)
    };

    IrGenerator {
      builder,
      scope_stack: vec![HashMap::new()], // Start with global scope
      loop_stack: Vec::new(),
      loop_header_stack: Vec::new(),
      symbol_manager: None,
      current_function: None,
      current_return_type: None,
      current_epilogue: None,
      current_return_slot: None,
      type_layouts,
      enum_types: std::collections::HashSet::new(),
      enum_variants: HashMap::new(),
      mode: mode.to_string(),
      stdlib,
      in_user_function: false,
      is_debug: true,
      checked_overflow: true,
      test_mode: false,
      test_filter: Vec::new(),
      in_test_case: false,
      suite_scope_name: None,
      suite_globals: Vec::new(),
      sibling_defined: Vec::new(),
      use_color: true,
      consumed_str_allocas: HashSet::new(),
      transferred_str_data: HashSet::new(),
      param_names: HashSet::new(),
      on_exit_stmts: Vec::new(),
    }
  }

  /// Disable ANSI color/formatting codes in debug and output formatting.
  pub fn set_no_color(&mut self, no_color: bool) {
    self.use_color = !no_color;
  }

  /// Restrict emitted test suites/cases to those matching any of `filters`
  /// (the repeatable `lale test --filter <pattern>` option). An empty list runs
  /// everything.
  pub fn set_test_filter(&mut self, filters: Vec<String>) {
    self.test_filter = filters;
  }

  /// ANSI bold-on, or empty when colors are disabled.
  fn a_bold(&self) -> &'static str {
    if self.use_color { "\x1b[1m" } else { "" }
  }
  /// ANSI cyan, or empty when colors are disabled.
  fn a_cyan(&self) -> &'static str {
    if self.use_color { "\x1b[36m" } else { "" }
  }
  /// ANSI reset, or empty when colors are disabled.
  fn a_reset(&self) -> &'static str {
    if self.use_color { "\x1b[0m" } else { "" }
  }
  /// Reset bold, keep cyan: "\x1b[0m\x1b[36m" or empty.
  fn a_rst_bold(&self) -> &'static str {
    if self.use_color {
      "\x1b[0m\x1b[36m"
    } else {
      ""
    }
  }
  /// ANSI red, or empty.
  fn a_red(&self) -> &'static str {
    if self.use_color { "\x1b[31m" } else { "" }
  }
  /// ANSI yellow, or empty.
  fn a_yellow(&self) -> &'static str {
    if self.use_color { "\x1b[33m" } else { "" }
  }

  /// Push a new scope onto the scope stack.
  fn push_scope(&mut self) {
    self.scope_stack.push(HashMap::new());
  }

  /// Pop the current scope from the scope stack.
  fn pop_scope(&mut self) {
    if self.scope_stack.len() > 1 {
      self.scope_stack.pop();
    }
  }

  /// Emit `__lale_free` calls for all local variables in the current
  /// function scope that haven't been returned or otherwise transferred.
  /// Parameters and consumed (returned) str allocas are skipped.
  ///
  /// Frees string data first (via the `.ptr` field), then frees every stack
  /// slot (alloca) in the frame — mirroring how an AOT backend reclaims a
  /// whole stack frame on return.
  fn emit_auto_free(&mut self) {
    self.emit_str_data_auto_free();

    // Second pass: free every stack slot (alloca) in this function's frame.
    // All allocas are hoisted to the entry block, so this is the whole frame.
    // Consumed (returned/transferred) allocas are skipped.
    let allocas = self.collect_function_allocas();
    for alloca in allocas {
      if !self.consumed_str_allocas.contains(&alloca) {
        self
          .builder
          .call_void_named("__lale_free_pointer", vec![alloca]);
      }
    }
  }

  /// Free string data owned by variables in the current scope (the last scope
  /// on the stack). Called once per test case before leaving the case scope,
  /// and as the first pass of `emit_auto_free` at function exit.
  fn emit_str_data_auto_free(&mut self) {
    // Collect candidates first to avoid borrowing self.scope_stack while mutating self.
    let candidates: Vec<_> = if let Some(scope) = self.scope_stack.last() {
      scope
        .iter()
        .filter(|(name, info)| {
          !self.param_names.contains(*name)
            && !self.consumed_str_allocas.contains(&info.allocation)
            && !self.transferred_str_data.contains(&info.allocation)
        })
        .map(|(_name, info)| (info.allocation, info.var_type.clone()))
        .collect()
    } else {
      Vec::new()
    };

    // Free string data for str-typed locals (and str fields nested inside
    // composite locals). This must happen before freeing the allocas
    // themselves, because extracting the `.ptr` reads it from the alloca.
    let mut str_ptrs: Vec<ValueId> = Vec::new();
    for (alloca, ty) in &candidates {
      let value = if matches!(ty, IrType::Struct { .. }) {
        self.builder.load(*alloca, ty.clone())
      } else {
        *alloca
      };
      self.collect_str_allocas(value, ty, &mut str_ptrs);
    }
    for ptr in &str_ptrs {
      self
        .builder
        .call_void_named("__lale_free_pointer", vec![*ptr]);
    }
  }

  /// Collect every `Alloca` destination in the current function.
  ///
  /// `IrBuilder::alloca` hoists all allocas to the function's entry block, so
  /// walking that block captures every stack slot — named and temporary alike.
  fn collect_function_allocas(&self) -> Vec<ValueId> {
    let mut result = Vec::new();
    if let Some(func_id) = self.builder.get_current_func()
      && let Ok(func) = self.builder.module().try_function(func_id)
      && let Some(entry) = func.block(func.entry_block)
    {
      for inst in &entry.instructions {
        if let Instruction::Alloca { dst, .. } = inst {
          result.push(*dst);
        }
      }
    }
    result
  }

  /// Emit __lale_free calls for all str-typed GLOBAL variables at program exit.
  /// Also recursively frees str fields nested inside struct globals.
  fn emit_global_str_auto_free(&mut self) {
    let mut to_free: Vec<ValueId> = Vec::new();
    // Collect candidates first to avoid borrowing scope_stack while mutating self
    let candidates: Vec<_> = if let Some(global_scope) = self.scope_stack.first() {
      global_scope
        .values()
        .map(|info| (info.allocation, info.global_id, info.var_type.clone()))
        .collect()
    } else {
      Vec::new()
    };
    for (allocation, global_id, ty) in &candidates {
      // Re-derive the global's address at the epilogue instead of reusing the
      // allocation value id: the original `globaladdr` may live inside a
      // mode-guarded block (e.g. an auto-defined `write ... to var` target)
      // that is skipped in the other mode, leaving the value undefined.
      let addr = match global_id {
        Some(gid) => self.builder.global_addr(*gid),
        None => *allocation,
      };
      let value = if matches!(ty, IrType::Struct { .. }) {
        self.builder.load(addr, ty.clone())
      } else {
        addr
      };
      self.collect_str_allocas(value, ty, &mut to_free);
    }

    // Suite-level globals are not in the global scope map (the suite scope is
    // popped before the epilogue), so free their str data explicitly here.
    let suite_candidates: Vec<_> = self
      .suite_globals
      .iter()
      .map(|(gid, ty)| (*gid, ty.clone()))
      .collect();
    for (gid, ty) in &suite_candidates {
      let addr = self.builder.global_addr(*gid);
      let value = if matches!(ty, IrType::Struct { .. }) {
        self.builder.load(addr, ty.clone())
      } else {
        addr
      };
      self.collect_str_allocas(value, ty, &mut to_free);
    }

    for ptr in &to_free {
      self
        .builder
        .call_void_named("__lale_free_pointer", vec![*ptr]);
    }
  }

  /// Recursively collect the `.ptr` values of str data owned by a struct value.
  /// For str values, extracts the `.ptr` field directly.
  /// For composite structs, recurses into str-typed fields (including nested
  /// structs and enum variant payloads) by extracting field values.
  fn collect_str_allocas(&mut self, value: ValueId, ty: &IrType, out: &mut Vec<ValueId>) {
    match ty {
      IrType::Struct { name } if name == "str" => {
        let ptr = self
          .builder
          .extract_field(value, "str", 0, IrType::raw_ptr());
        out.push(ptr);
      }
      IrType::Struct { name } => {
        // Use the IR module's struct definition (not `type_layouts`) because it
        // carries the correct field layout, including the discriminant field for
        // enum variants.
        let struct_fields: Vec<(u32, IrType)> = self
          .builder
          .module()
          .struct_def(name)
          .map(|def| {
            def
              .fields
              .iter()
              .enumerate()
              .filter(|(_, (_, ft))| matches!(ft, IrType::Struct { .. }))
              .map(|(i, (_, ft))| (i as u32, ft.clone()))
              .collect()
          })
          .unwrap_or_default();
        for (idx, field_ty) in struct_fields {
          let field_val = self
            .builder
            .extract_field(value, name, idx, field_ty.clone());
          self.collect_str_allocas(field_val, &field_ty, out);
        }
      }
      _ => {}
    }
  }

  /// Emit all collected `on exit` statements.
  /// Called at every function exit point (explicit returns and end-of-function).
  fn emit_on_exit_stmts(&mut self) -> CompileResult<()> {
    let stmts: Vec<Stmt> = self.on_exit_stmts.clone();
    for stmt in stmts {
      self.try_generate_stmt(&stmt)?;
    }
    Ok(())
  }

  /// Panic-wrapper for callers that can't propagate CompileResult.
  /// Used by `generate_*` methods that delegate to `try_generate_*`.
  fn unwrap_or_panic<T>(result: CompileResult<T>, context: &str) -> T {
    result.unwrap_or_else(|e| ice!("IR generation error in {}: {}", context, e))
  }

  /// Look up a variable in the scope stack, searching from innermost to outermost scope.
  fn lookup_var(&self, name: &str) -> Option<VarInfo> {
    for scope in self.scope_stack.iter().rev() {
      if let Some(var_info) = scope.get(name) {
        return Some(var_info.clone());
      }
    }
    None
  }

  /// Resolve a function name to its suite-qualified form when a suite function
  /// with that name exists in the enclosing test suite. Mirrors the analyzer's
  /// `resolve_fn_name` so IR function calls match the mangled function the
  /// analyzer registered.
  fn resolve_suite_fn_name(&self, name: &str) -> String {
    if let Some(prefix) = &self.suite_scope_name {
      let qualified = format!("{}{}", prefix, name);
      if let Some(manager) = self.symbol_manager
        && manager.lookup_function(&qualified).is_some()
      {
        return qualified;
      }
    }
    name.to_string()
  }

  /// Enter a control-flow construct: open a sibling-defined set for its branches.
  fn enter_construct(&mut self) {
    self.sibling_defined.push(std::collections::HashSet::new());
  }

  /// Exit a control-flow construct: drop its sibling-defined set.
  fn exit_construct(&mut self) {
    self.sibling_defined.pop();
  }

  /// True if `name` was defined in a previously-generated sibling branch of the
  /// current construct (so its storage should be reused, not reallocated).
  fn is_sibling_join(&self, name: &str) -> bool {
    self
      .sibling_defined
      .last()
      .is_some_and(|s| s.contains(name))
  }

  /// Get the pointer to a variable's storage.
  /// For global variables, emits a GlobalAddr instruction (valid in any function).
  /// For local variables, returns the stored stack-alloca ValueId directly.
  fn var_ptr(&mut self, var_info: &VarInfo) -> ValueId {
    if let Some(global_id) = var_info.global_id {
      self.builder.global_addr(global_id)
    } else {
      var_info.allocation
    }
  }

  /// Define a variable.
  ///
  /// Globals (`global_id` set) are registered in the global scope (the first
  /// entry); function locals go into the current function scope (the last
  /// entry). This keeps the two-scope model intact even when a synthetic
  /// `main` function is pushed on top of the global scope.
  ///
  /// Exception: suite-scope globals (mangled `suite::name`) are registered in
  /// the current suite scope (the last entry at the time suite declarations are
  /// generated), so case bodies and suite functions resolve them by simple name
  /// while the IR global itself keeps the mangled, collision-free name.
  fn define_var(
    &mut self,
    name: String,
    allocation: ValueId,
    var_type: IrType,
    global_id: Option<GlobalId>,
  ) {
    let is_suite_global = global_id.is_some() && self.suite_scope_name.is_some();
    let scope = if global_id.is_some() && !is_suite_global {
      self.scope_stack.first_mut()
    } else {
      self.scope_stack.last_mut()
    };
    // Record the name into the sibling-defined set so a later sibling branch
    // reuses this slot (definite-assignment join).
    if let Some(sibling) = self.sibling_defined.last_mut() {
      sibling.insert(name.clone());
    }
    if let Some(scope) = scope {
      scope.insert(
        name,
        VarInfo {
          allocation,
          var_type: var_type.clone(),
          global_id,
        },
      );
    }
    // Track suite globals separately so their str data can be freed at program
    // exit even though the suite scope map is popped before the epilogue.
    if is_suite_global && let Some(gid) = global_id {
      self.suite_globals.push((gid, var_type));
    }
  }

  /// Look up pointer type information from the semantic analyzer.
  /// If a pointer variable's target type is known, returns the IR type for that target.
  /// Otherwise returns None.
  fn get_pointer_target_type_from_semantic_analysis(&self, var_name: &str) -> Option<IrType> {
    if let Some(manager) = &self.symbol_manager {
      // Get the symbol for this variable from the symbol manager
      if let Ok(Some(symbol)) = manager.lookup_var_symbol(var_name) {
        // Check if this pointer has a known target type
        if let Some(target_var_name) = &symbol.pointer_to_type {
          // Look up the target variable to get its type
          if let Ok(Some(target_symbol)) = manager.lookup_var_symbol(target_var_name) {
            // Convert the target type string to IR type
            return Some(self.type_string_to_ir_type(&target_symbol.data_type));
          }
        }
      }
    }
    None
  }

  /// Resolve the type of an expression using the symbol manager.
  fn lookup_expr_type(&self, expr: &Expr) -> String {
    // Handle binary expressions
    if let Expr::Binary(bin) = expr {
      // Comparison ops always return bool
      if matches!(
        bin.operator,
        BinaryOp::Eq
          | BinaryOp::NotEq
          | BinaryOp::Lt
          | BinaryOp::Gt
          | BinaryOp::LtEq
          | BinaryOp::GtEq
      ) {
        return "bool".to_string();
      }
      // Logical ops return bool
      if matches!(bin.operator, BinaryOp::Or | BinaryOp::And | BinaryOp::Xor) {
        return "bool".to_string();
      }
      // Dot product: vecN<T> · vecN<T> → T (extract inner scalar type)
      if bin.operator == BinaryOp::Dot {
        let left_type = self.lookup_expr_type(&bin.left);
        if left_type != "unknown" {
          if let Some(inner) = left_type.find('<')
            && left_type.ends_with('>')
          {
            return left_type[inner + 1..left_type.len() - 1].to_string();
          }
          return left_type;
        }
        // Left is unknown, try right side
        let right_type = self.lookup_expr_type(&bin.right);
        if right_type != "unknown" {
          if let Some(inner) = right_type.find('<')
            && right_type.ends_with('>')
          {
            return right_type[inner + 1..right_type.len() - 1].to_string();
          }
          return right_type;
        }
        return "unknown".to_string();
      }
      // Cross product: vec3<T> ⨯ vec3<T> → vec3<T> (same type)
      if bin.operator == BinaryOp::Cross {
        return self.lookup_expr_type(&bin.left);
      }
      // Pow: result type is the base (left) type
      if bin.operator == BinaryOp::Pow {
        return self.lookup_expr_type(&bin.left);
      }
      // Arithmetic and bitwise ops: result type matches operands.
      // Try left first (which is the authoritative side per semantic analysis).
      // If left is unknown (e.g. a bare literal), try right.
      if matches!(
        bin.operator,
        BinaryOp::Mul
          | BinaryOp::Add
          | BinaryOp::Sub
          | BinaryOp::Div
          | BinaryOp::Mod
          | BinaryOp::BitAnd
          | BinaryOp::BitOr
          | BinaryOp::BitXor
          | BinaryOp::UnsignedLeftShift
          | BinaryOp::UnsignedRightShift
          | BinaryOp::SignedLeftShift
          | BinaryOp::SignedRightShift
      ) {
        let left_type = self.lookup_expr_type(&bin.left);
        if left_type != "unknown" {
          return left_type;
        }
        return self.lookup_expr_type(&bin.right);
      }
      // For other binary ops, fall through to identifier lookup
    }
    // Handle function calls: look up the function's return type
    if let Expr::FnCallExpr(fc) = expr
      && let Some(fn_name) = fc.target.node.first()
      && let Some(manager) = self.symbol_manager
      && let Some(symbol) = self.lookup_symbol_any_scope(manager, fn_name)
    {
      return symbol.data_type;
    }
    // Handle optional expressions
    if matches!(expr, Expr::NothingExpr) {
      return "?".to_string();
    }
    // Handle type conversions: type is the target type
    if let Expr::Conversion(conv) = expr {
      return crate::semantic_analysis::type_compatibility::TypeInference::type_name_to_string(
        &conv.target_type,
      );
    }
    // Fallback: extract identifier name and look up across all scopes
    if let Some(name) = self.find_first_identifier(expr)
      && let Some(manager) = self.symbol_manager
      && let Some(symbol) = self.lookup_symbol_any_scope(manager, &name)
    {
      return symbol.data_type;
    }
    "unknown".to_string()
  }

  /// Look up a symbol by name across all scopes (global + function).
  fn lookup_symbol_any_scope(
    &self,
    manager: &crate::semantic_analysis::SqliteSymbolManager,
    name: &str,
  ) -> Option<crate::ast::Symbol> {
    // Try global scope first
    let global_table = manager.get_symbol_table(VarScope::Global);
    if let Some(symbol) = global_table.get(name) {
      return Some(symbol.clone());
    }
    // Search all function scopes
    let all_tables = manager.get_all_symbol_tables().unwrap_or_default();
    for (scope, table) in &all_tables {
      if matches!(scope, VarScope::Function { .. })
        && let Some(symbol) = table.get(name)
      {
        return Some(symbol.clone());
      }
    }
    None
  }

  /// Find the first identifier in an expression tree.
  fn find_first_identifier(&self, expr: &Expr) -> Option<String> {
    let mut result = None;
    self.for_each_identifier(expr, &mut |name| {
      if result.is_none() {
        result = Some(name.to_string());
      }
    });
    result
  }

  /// Visit all identifiers in an expression tree.
  fn for_each_identifier(&self, expr: &Expr, f: &mut dyn FnMut(&str)) {
    match expr {
      Expr::Identifier(id) => f(id.name()),
      Expr::Binary(bin) => {
        self.for_each_identifier(&bin.left, f);
        self.for_each_identifier(&bin.right, f);
      }
      Expr::Unary(un) => self.for_each_identifier(&un.operand, f),
      Expr::Conversion(conv) => self.for_each_identifier(&conv.operand, f),
      Expr::ArrayIndex(ai) => self.for_each_identifier(&ai.array, f),
      Expr::MemberAccess(ma) => self.for_each_identifier(&ma.object, f),
      Expr::FnCallExpr(fc) => {
        for arg in &fc.arguments {
          self.for_each_identifier(arg, f);
        }
      }
      _ => {}
    }
  }

  // ==================== Optional type helpers ====================

  /// Return the struct name for an optional type (e.g., `optional_f64`).
  fn optional_struct_name(inner_type: &IrType) -> String {
    format!("optional_{}", inner_type.to_string().replace('%', ""))
  }

  /// Ensure the optional struct type is registered in the module.
  fn ensure_optional_struct(&mut self, inner_type: &IrType) -> String {
    let name = Self::optional_struct_name(inner_type);
    // Check module's struct map — avoid duplicate registration
    if self.builder.module().struct_def(&name).is_none() {
      let mut def = crate::ir::module::StructDef::new(&name);
      def.add_field("is_present", IrType::Bool);
      def.add_field("value", inner_type.clone());
      self.builder.module_mut().add_struct(def);
    }
    name
  }

  /// Wrap a concrete value as an optional: `{is_present: true, value}`.
  fn build_optional_wrap(&mut self, value: ValueId, inner_type: &IrType) -> ValueId {
    let name = self.ensure_optional_struct(inner_type);
    let is_present = self.builder.const_bool(true);
    self
      .builder
      .build_struct_value(&name, vec![is_present, value])
  }

  /// Build a nothing optional: `{is_present: false, zero_init}`.
  fn build_optional_none(&mut self, inner_type: &IrType) -> ValueId {
    let name = self.ensure_optional_struct(inner_type);
    let is_present = self.builder.const_bool(false);
    let zero_init = self.builder.const_int(inner_type.clone(), 0);
    self
      .builder
      .build_struct_value(&name, vec![is_present, zero_init])
  }

  /// Extract the is_present tag from an optional value.
  fn extract_optional_tag(&mut self, opt_val: ValueId, inner_type: &IrType) -> ValueId {
    let name = self.ensure_optional_struct(inner_type);
    self.builder.extract_field(opt_val, &name, 0, IrType::Bool)
  }

  /// Check if an IR type represents a struct-based optional (struct name starts with "optional_").
  fn is_optional_struct_type(ty: &IrType) -> bool {
    if let IrType::Struct { name } = ty {
      name.starts_with("optional_")
    } else {
      false
    }
  }

  /// Look up the physical unit of an expression.
  fn lookup_expr_unit(&self, expr: &Expr) -> String {
    // Use UnitComputer for full expression unit analysis (handles `v^2`, `m * v`, etc.)
    // Falls back to searching all function scopes via lookup_var_unit when needed.
    if let Some(manager) = self.symbol_manager {
      use crate::semantic_analysis::unit_computer::UnitComputer;
      let mut errors = Vec::new();
      let mut computer = UnitComputer::new(manager, &mut errors);
      let unit = computer.expr_unit(expr);
      return match unit {
        crate::ast::definitions::ExprUnit::Unit(u) => u.to_string(),
        _ => String::new(),
      };
    }
    // Fallback: simple identifier lookup via symbol_manager
    if let Some(name) = self.find_first_identifier(expr)
      && let Some(manager) = &self.symbol_manager
      && let Ok(Some(symbol)) = manager.lookup_var_symbol(&name)
    {
      // Physical unit may legitimately be None for unitless variables.
      return symbol.physical_unit.clone().unwrap_or_default();
    }
    String::new()
  }

  /// Generate IR for a program, returning Result instead of panicking.
  ///
  /// This is the canonical entry point for IR generation. All code paths
  /// should use this or `try_generate_owned`.
  pub fn try_generate(
    mut self,
    program: &Program,
    symbol_manager: &'a SqliteSymbolManager,
    is_debug: bool,
    checked_overflow: bool,
    test_mode: bool,
    include_stdlib: bool,
  ) -> CompileResult<Module> {
    self.symbol_manager = Some(symbol_manager);
    self.is_debug = is_debug;
    self.checked_overflow = checked_overflow;
    self.test_mode = test_mode;
    // Add standard library extern functions
    if include_stdlib {
      self.builder.add_stdlib_externs();
    }

    // Check if there's any top-level code (non-function definitions)
    let has_top_level = program.statements.iter().any(|stmt| {
      matches!(
        stmt,
        Stmt::VarDef(_)
          | Stmt::Assign(_)
          | Stmt::CompoundAssign(_)
          | Stmt::If(_)
          | Stmt::When(_)
          | Stmt::Switch(_)
          | Stmt::Match(_)
          | Stmt::Loop(_)
          | Stmt::Stdout(_)
          | Stmt::Stderr(_)
          | Stmt::Debug(_)
          | Stmt::FnCall(_)
          | Stmt::Return(_)
          | Stmt::ExitProgram(_)
          | Stmt::Stdin(_)
          | Stmt::AddError(_)
          | Stmt::WriteErrors(_)
          | Stmt::WarnErrors(_)
          | Stmt::AlertErrors(_)
          | Stmt::Alert(_)
          | Stmt::TestSuite(_)
      )
    });

    // If there's top-level code, wrap it in a main function (only in "main" mode)
    // (Note: user-defined 'main' functions are rejected by semantic analysis)
    // main returns i32 (0 for success) per C runtime convention
    // In "lib" mode, top-level code is not wrapped in main (library compilation)
    let create_main = has_top_level && self.mode == "main";
    if create_main {
      self._start_function("main", IrType::I32, Linkage::Export);
      self.push_scope();
    }

    // Process all statements in the program. Top-level runtime code and test
    // suites are gated by #mode so both branches stay present in the IR.
    for stmt in &program.statements {
      let in_main = self.builder.get_current_func().is_some();
      if in_main && Self::is_top_level_runtime_stmt(stmt) {
        let merge = self.begin_mode_guard("run");
        self.try_generate_stmt(stmt)?;
        self.end_mode_guard(merge);
      } else if in_main && matches!(stmt, Stmt::TestSuite(_)) {
        let merge = self.begin_mode_guard("test");
        self.try_generate_stmt(stmt)?;
        self.end_mode_guard(merge);
      } else {
        self.try_generate_stmt(stmt)?;
      }
    }

    // Close the main function if we created one.
    // Emit global str cleanup then auto-free for str locals before returning.
    // Return 0 (success) per C runtime convention.
    if create_main {
      if !self.builder.is_current_block_terminated() {
        self.emit_global_str_auto_free();
        self.emit_auto_free();
      }
      let zero = self.builder.const_int(IrType::I32, 0);
      self.builder.ret(zero);
      self.pop_scope();
    }

    // Return the completed module
    Ok(self.builder.build())
  }

  /// Load stdlib functions directly into a module (without self mutation)
  /// Generate IR from an OwnedAnalyzer (which owns the symbol manager).
  /// This wraps try_generate to work with the owned variant.
  /// NOTE: This temporarily creates a borrowed analyzer - the owned variant is only used for ownership,
  /// the actual IR generation uses the symbol manager from the owned analyzer.
  pub fn try_generate_owned<'b>(
    mut self,
    program: &Program,
    analyzer: &'b crate::semantic_analysis::OwnedAnalyzer,
    include_stdlib: bool,
  ) -> CompileResult<Module>
  where
    'b: 'a,
  {
    // Use the OwnedAnalyzer's symbol manager for type lookups during IR generation
    // This allows us to resolve array element types and dimensions correctly
    self.symbol_manager = Some(analyzer.symbols());

    // Propagate debug mode from compiler options
    self.is_debug = analyzer.is_debug();

    // Propagate overflow checking from compiler options
    self.checked_overflow = analyzer.checked_overflow();

    // Propagate execution mode from compiler options
    self.test_mode = analyzer.test_mode();

    // Add standard library extern functions
    if include_stdlib {
      self.builder.add_stdlib_externs();
    }

    // Check if there's any top-level code (non-function definitions)
    let has_top_level = program.statements.iter().any(|stmt| {
      matches!(
        stmt,
        Stmt::VarDef(_)
          | Stmt::Assign(_)
          | Stmt::CompoundAssign(_)
          | Stmt::If(_)
          | Stmt::When(_)
          | Stmt::Switch(_)
          | Stmt::Match(_)
          | Stmt::Loop(_)
          | Stmt::Stdout(_)
          | Stmt::Stderr(_)
          | Stmt::Debug(_)
          | Stmt::FnCall(_)
          | Stmt::Return(_)
          | Stmt::ExitProgram(_)
          | Stmt::Stdin(_)
          | Stmt::AddError(_)
          | Stmt::WriteErrors(_)
          | Stmt::WarnErrors(_)
          | Stmt::AlertErrors(_)
          | Stmt::Alert(_)
          | Stmt::TestSuite(_)
      )
    });

    // If there's top-level code, wrap it in a main function (only in "main" mode)
    let create_main = has_top_level && self.mode == "main";
    if create_main {
      self._start_function("main", IrType::I32, Linkage::Export);
      self.push_scope();
    }

    // Process all statements in the program. Top-level runtime code and test
    // suites are gated by #mode so both branches stay present in the IR.
    for stmt in &program.statements {
      let in_main = self.builder.get_current_func().is_some();
      if in_main && Self::is_top_level_runtime_stmt(stmt) {
        let merge = self.begin_mode_guard("run");
        self.try_generate_stmt(stmt)?;
        self.end_mode_guard(merge);
      } else if in_main && matches!(stmt, Stmt::TestSuite(_)) {
        let merge = self.begin_mode_guard("test");
        self.try_generate_stmt(stmt)?;
        self.end_mode_guard(merge);
      } else {
        self.try_generate_stmt(stmt)?;
      }
    }

    // Finalize the main function if it was created.
    // Emit global str cleanup then auto-free for str locals before returning.
    // Return 0 (success) per C runtime convention.
    if create_main {
      if !self.builder.is_current_block_terminated() {
        self.emit_global_str_auto_free();
        self.emit_auto_free();
      }
      self.pop_scope();
      let zero_val = self.builder.const_int(IrType::I32, 0);
      self.builder.ret(zero_val);
    }

    Ok(self.builder.build())
  }

  /// Load only builtins.lale (type str, FFI declarations, type conversions)
  /// into the target module. Always runs before user code generation.
  /// Builtins are embedded in the compiler binary — no filesystem dependency.
  pub fn load_builtins_into_module(
    target_module: &mut Module,
    checked_overflow: bool,
  ) -> CompileResult<()> {
    let builtins_source = crate::builtins::BUILTINS_SOURCE;
    let builtins_pairs = crate::LaleParser::parse(crate::Rule::program, builtins_source)
      .map_err(|e| format!("Parse error in embedded builtins.lale: {}", e))?;

    let builtins_program = crate::ast::builder::build_program(builtins_pairs, "builtins.lale")?;

    Self::compile_and_merge_program(
      target_module,
      "builtins",
      builtins_program.statements,
      checked_overflow,
    )
  }

  /// Load the standard library (plus builtins, needed for analysis) into the
  /// target module. Only runs when stdlib is enabled. Builtins functions that
  /// already exist in the target (from `load_builtins_into_module`) are remapped
  /// rather than duplicated.
  pub fn load_stdlib_into_module(
    target_module: &mut Module,
    stdlib: crate::config::StdlibLevel,
    checked_overflow: bool,
  ) -> CompileResult<()> {
    use crate::semantic_analysis::module_resolver::resolve_stdlib_path;

    if stdlib == crate::config::StdlibLevel::None {
      return Ok(());
    }

    let stdlib_dir = resolve_stdlib_path();
    let stdlib_path = match stdlib {
      crate::config::StdlibLevel::None => unreachable!(),
      crate::config::StdlibLevel::Core => stdlib_dir.join("core.lale"),
      crate::config::StdlibLevel::Full => stdlib_dir.join("std.lale"),
    };

    if !stdlib_path.exists() {
      return Ok(());
    }

    // Collect builtins (needed for analysis) + stdlib module statements
    let mut all_statements = Vec::new();

    let builtins_source = crate::builtins::BUILTINS_SOURCE;
    let builtins_pairs = crate::LaleParser::parse(crate::Rule::program, builtins_source)
      .map_err(|e| format!("Parse error in embedded builtins.lale: {}", e))?;
    let builtins_program = crate::ast::builder::build_program(builtins_pairs, "builtins.lale")?;
    all_statements.extend(builtins_program.statements);

    Self::collect_stdlib_modules(stdlib_path.as_path(), &mut all_statements)?;

    Self::compile_and_merge_program(target_module, "stdlib", all_statements, checked_overflow)
  }

  /// Shared pipeline: analyze a program, generate IR, and copy functions +
  /// globals into the target module. Used by both `load_builtins_into_module`
  /// and `load_stdlib_into_module`.
  fn compile_and_merge_program(
    target_module: &mut Module,
    source_name: &str,
    statements: Vec<crate::ast::Stmt>,
    checked_overflow: bool,
  ) -> CompileResult<()> {
    if statements.is_empty() {
      return Ok(());
    }

    let program = crate::ast::Program {
      statements,
      location: crate::ast::SourceLocation {
        source_file: source_name.to_string(),
        line: 1,
        col: 1,
        start_pos: 0,
        end_pos: 0,
      },
      global_symbol_table: Default::default(),
    };

    use crate::semantic_analysis::analyzer::{AnalyzerResults, analyze_ast_with_options};
    let analyzer = analyze_ast_with_options(
      &program,
      crate::config::CompilerOptions::default().checked_overflow(checked_overflow),
    );

    if !analyzer.is_valid() {
      let errors: Vec<String> = analyzer
        .get_errors()
        .iter()
        .map(|e| {
          format!(
            "{}:{}:{}: {}",
            source_name, e.location.line, e.location.col, e.message
          )
        })
        .collect();
      return Err(crate::error::CompileError::Semantic(errors.join("\n")));
    }

    let processed = crate::semantic_analysis::process_ct_directives(&program, &analyzer);

    // Generate IR. Use "main" mode so VarDefs are processed.
    // The synthetic main is filtered out during copy.
    let ir_gen = IrGenerator::new(source_name);
    let src_module = ir_gen.try_generate_owned(&processed, &analyzer, false)?;

    Self::copy_module_to_target(target_module, &src_module);
    Ok(())
  }

  /// Copy functions and Internal globals from source module to target module.
  /// Functions that already exist in the target are remapped (not duplicated).
  /// Internal globals referenced by copied functions are also copied with ID remapping.
  fn copy_module_to_target(target_module: &mut Module, src_module: &Module) {
    use crate::ir::instructions::Instruction;

    // First pass: build function ID remap
    let mut id_remap_map = std::collections::HashMap::new();
    let mut funcs_to_add = Vec::new();

    for func in &src_module.functions {
      if func.name == "main" {
        continue;
      }
      if let Some(existing) = target_module.functions.iter().find(|f| f.name == func.name) {
        id_remap_map.insert(func.id, existing.id);
      } else {
        let new_id = target_module.alloc_func_id();
        id_remap_map.insert(func.id, new_id);
        funcs_to_add.push((func.clone(), new_id));
      }
    }

    // Collect Internal globals referenced by copied functions
    let mut global_id_remap_map = std::collections::HashMap::new();
    for (func, _) in &funcs_to_add {
      for block in &func.blocks {
        for instr in &block.instructions {
          if let Instruction::GlobalAddr { global, .. } = instr {
            if global_id_remap_map.contains_key(global)
              || target_module.globals.iter().any(|g| g.id == *global)
            {
              continue;
            }
            if let Some(g) = src_module.globals.iter().find(|gl| gl.id == *global)
              && g.linkage == Linkage::Internal
            {
              let new_id = target_module.add_global(&g.name, g.ty.clone(), g.linkage);
              global_id_remap_map.insert(*global, new_id);
            }
          }
        }
      }
    }

    // Second pass: remap and add functions
    for (mut func, new_id) in funcs_to_add {
      func.id = new_id;
      for block in &mut func.blocks {
        for instr in &mut block.instructions {
          Self::remap_funcref_in_instruction(instr, &id_remap_map);
          Self::remap_globaladdr_in_instruction(instr, &global_id_remap_map);
        }
      }
      target_module.register_func(func.name.clone(), new_id);
      target_module.functions.push(func);
    }
  }

  /// Recursively collect all statements from stdlib modules and their dependencies
  fn collect_stdlib_modules(
    stdlib_path: &std::path::Path,
    all_statements: &mut Vec<crate::ast::Stmt>,
  ) -> CompileResult<()> {
    Self::collect_stdlib_modules_impl(
      stdlib_path,
      all_statements,
      &mut std::collections::HashSet::new(),
    )
  }

  /// Implementation of collect_stdlib_modules with a mutable visited set
  fn collect_stdlib_modules_impl(
    stdlib_path: &std::path::Path,
    all_statements: &mut Vec<crate::ast::Stmt>,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
  ) -> CompileResult<()> {
    use crate::ast::Stmt;
    use std::fs;

    // Canonicalize path to avoid duplicates. Fail on filesystem errors.
    let canonical_path = stdlib_path.canonicalize().map_err(|e| {
      format!(
        "Cannot access stdlib path '{}': {}",
        stdlib_path.display(),
        e
      )
    })?;

    // Skip if already visited
    if visited.contains(&canonical_path) {
      return Ok(());
    }
    visited.insert(canonical_path);

    // Read the source
    let stdlib_source =
      fs::read_to_string(stdlib_path).map_err(|e| IrGenError::InvalidBuilderState {
        reason: format!(
          "Failed to read stdlib file {}: {}",
          stdlib_path.display(),
          e
        ),
      })?;

    // Parse the stdlib
    use super::{LaleParser, Rule};
    use pest::Parser;

    let parsed = LaleParser::parse(Rule::program, &stdlib_source).map_err(|e| {
      IrGenError::InvalidBuilderState {
        reason: format!("Failed to parse stdlib {}: {}", stdlib_path.display(), e),
      }
    })?;

    // Build AST from parsed stdlib
    use crate::ast::builder::build_program;
    let stdlib_program =
      build_program(parsed, stdlib_path.to_str().unwrap_or("stdlib")).map_err(|e| {
        IrGenError::InvalidBuilderState {
          reason: format!("Failed to build stdlib AST: {}", e),
        }
      })?;

    // First, recursively collect from dependencies (so they're processed first)
    for stmt in &stdlib_program.statements {
      if let Stmt::Use(use_stmt) = stmt {
        // Resolve the module path relative to the current file
        let module_name = use_stmt
          .module_path
          .iter()
          .map(|s| s.node.as_str())
          .collect::<Vec<_>>()
          .join("/");
        let stdlib_dir = stdlib_path.parent().ok_or_else(|| {
          format!(
            "Stdlib path '{}' has no parent directory",
            stdlib_path.display()
          )
        })?;
        let dep_path = stdlib_dir.join(format!("{}.lale", module_name));

        if dep_path.exists() {
          Self::collect_stdlib_modules_impl(&dep_path, all_statements, visited)?;
        }
      }
    }

    // Now add all statements from this module
    all_statements.extend(stdlib_program.statements);

    Ok(())
  }

  /// Load stdlib functions for the interpreter and add them to target module
  pub fn load_and_add_stdlib(&mut self, target_module: &mut Module) -> CompileResult<()> {
    Self::load_stdlib_into_module(target_module, self.stdlib, self.checked_overflow)
  }

  /// Remap FuncRef::Id calls in an instruction using the provided ID mapping.
  /// This is used when merging stdlib functions into the target module to update
  /// internal function call references to use the new IDs.
  fn remap_funcref_in_instruction(
    instr: &mut crate::ir::instructions::Instruction,
    id_remap: &std::collections::HashMap<crate::ir::values::FuncId, crate::ir::values::FuncId>,
  ) {
    use crate::ir::instructions::FuncRef;
    use crate::ir::instructions::Instruction;

    match instr {
      Instruction::Call { func, .. } => {
        if let FuncRef::Id(old_id) = func
          && let Some(&new_id) = id_remap.get(old_id)
        {
          *func = FuncRef::Id(new_id);
        }
      }
      Instruction::CallVoid { func, .. } => {
        if let FuncRef::Id(old_id) = func
          && let Some(&new_id) = id_remap.get(old_id)
        {
          *func = FuncRef::Id(new_id);
        }
      }
      _ => {
        // Other instruction types don't have function references
      }
    }
  }

  /// Remap GlobalAddr instruction global IDs within a single instruction.
  /// This is used when merging stdlib globals into the target module to update
  /// internal global references to use the new IDs.
  fn remap_globaladdr_in_instruction(
    instr: &mut crate::ir::instructions::Instruction,
    id_remap: &std::collections::HashMap<crate::ir::values::GlobalId, crate::ir::values::GlobalId>,
  ) {
    use crate::ir::instructions::Instruction;

    if let Instruction::GlobalAddr { global, .. } = instr
      && let Some(&new_id) = id_remap.get(global)
    {
      *global = new_id;
    }
  }

  /// True when a top-level statement is runtime user code that should execute
  /// only in `run` mode (as opposed to a definition or a test suite).
  fn is_top_level_runtime_stmt(stmt: &Stmt) -> bool {
    matches!(
      stmt,
      Stmt::Assign(_)
        | Stmt::CompoundAssign(_)
        | Stmt::ValueAtAssign(_)
        | Stmt::FnCall(_)
        | Stmt::If(_)
        | Stmt::When(_)
        | Stmt::Match(_)
        | Stmt::Switch(_)
        | Stmt::Loop(_)
        | Stmt::Stdout(_)
        | Stmt::Stderr(_)
        | Stmt::Debug(_)
        | Stmt::Stdin(_)
        | Stmt::Assert(_)
        | Stmt::Alert(_)
        | Stmt::AddError(_)
        | Stmt::WriteErrors(_)
        | Stmt::WarnErrors(_)
        | Stmt::AlertErrors(_)
        | Stmt::ExitProgram(_)
    )
  }

  /// Emit a runtime guard `#mode == mode` and position inside its body block.
  /// Returns the merge block to jump to after the guarded statements.
  ///
  /// Both branches stay present in the IR: the comparison is against a
  /// compile-time constant, so the interpreter evaluates it and a future AOT
  /// optimizer dead-code-eliminates the untaken branch.
  fn begin_mode_guard(&mut self, mode: &str) -> BlockId {
    let mode_const = self
      .builder
      .const_string(if self.test_mode { "test" } else { "run" });
    let expected = self.builder.const_string(mode);
    let cond = self.builder.eq(mode_const, expected);

    let body_block = self.builder.create_block("mode.body");
    let merge_block = self.builder.create_block("mode.merge");
    self.builder.cond_br(cond, body_block, merge_block);
    self.builder.position_at(body_block);
    merge_block
  }

  /// Branch to the merge block (if not already terminated) and continue there.
  fn end_mode_guard(&mut self, merge_block: BlockId) {
    if !self.builder.is_current_block_terminated() {
      self.builder.br(merge_block);
    }
    self.builder.position_at(merge_block);
  }

  /// Dispatch a statement to the appropriate generator method.
  ///
  /// This is the entry point for all IR code generation from statements.
  /// Each statement type (var def, assignment, if, loop, fn call, etc.)
  /// has its own `try_generate_*` method. The dispatcher delegates and
  /// propagates errors via `CompileResult<()>`.
  fn try_generate_stmt(&mut self, stmt: &Stmt) -> CompileResult<()> {
    match stmt {
      Stmt::Use(_) => {
        // Use statements are compile-time only; no IR generation needed
      }
      Stmt::TypeDef(type_def) => {
        // Generate constructor function for the type
        self.generate_type_constructor(type_def);
      }
      Stmt::EnumDef(enum_def) => {
        // Generate constructor functions for each enum variant
        self.generate_enum_constructors(enum_def);
      }
      Stmt::Switch(switch_stmt) => {
        self.generate_switch(switch_stmt);
      }
      Stmt::Match(match_stmt) => {
        // In lib mode, skip top-level match statements (require main context)
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_match(match_stmt);
        }
      }
      Stmt::VarDef(var_def) => {
        // In lib mode, skip top-level defs (they would require a main function context)
        // Function-level defs are handled normally
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.try_generate_var_def(var_def)?;
        }
      }
      Stmt::UnsafeDecl(decl) => {
        self.generate_unsafe_decl(decl);
      }
      Stmt::Assign(assign) => {
        // In lib mode, skip top-level statements (require main function context)
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.try_generate_assign(assign)?;
        }
      }
      Stmt::CompoundAssign(compound) => {
        // In lib mode, skip top-level statements (require main function context)
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.try_generate_compound_assign(compound)?;
        }
      }
      Stmt::ValueAtAssign(value_at) => {
        // In lib mode, skip top-level statements (require main function context)
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_value_at_assign(value_at);
        }
      }
      Stmt::FnDef(fn_def) => self.try_generate_fn_def(fn_def)?,
      Stmt::FnSignature(_) => {
        // Function signatures are declarations only; code is in FnDef
      }
      Stmt::FnCall(fn_call) => {
        // In lib mode, skip top-level function calls (require main context)
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_fn_call_expr(fn_call, None);
        }
      }
      Stmt::If(if_stmt) => {
        // In lib mode, skip top-level if statements
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_if(if_stmt);
        }
      }
      Stmt::When(when_stmt) => {
        // In lib mode, skip top-level when statements
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_when(when_stmt);
        }
      }
      Stmt::MoveOn(_) => {
        // Intentional no-op: generate nothing
      }
      Stmt::MissingCode(missing) => {
        // In debug mode, emit a runtime warning
        if self.is_debug && (self.builder.get_current_func().is_some() || self.mode == "main") {
          self.generate_missing_code_warning(missing);
        }
        // In release mode: semantic analysis should have already rejected this.
        // If it somehow reaches IR gen, we silently skip it.
      }
      Stmt::Release(release) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_release(release)?;
        }
      }
      Stmt::OnExit(on_exit) => {
        // Collect on_exit statements — they'll be emitted at function exit points.
        self.on_exit_stmts.push((*on_exit.body).clone());
      }
      Stmt::Loop(loop_stmt) => {
        // In lib mode, skip top-level loops
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_loop(loop_stmt);
        }
      }
      Stmt::Return(ret) => {
        // Return statements should only appear in functions
        if self.builder.get_current_func().is_some() {
          self.generate_return(ret);
        }
      }
      Stmt::ExitProgram(exit) => {
        // In lib mode, skip top-level exit statements
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_exit_program(exit);
        }
      }
      Stmt::ExitLoop(_) => {
        // Branch to the current loop's exit block (must be in loop)
        if let Some(&loop_exit) = self.loop_stack.last() {
          self.builder.br(loop_exit);
        }
      }
      Stmt::Rewind(_) => {
        // Rewind (continue to loop header): branch to innermost loop header
        if let Some(&loop_header) = self.loop_header_stack.last() {
          self.builder.br(loop_header);
        }
        // If rewind outside loop, silently do nothing (semantic analysis validates this)
      }
      Stmt::Stdout(write) => {
        // In lib mode, skip top-level write statements
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_stdout(write);
        }
      }
      Stmt::Stderr(warn) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_stderr(warn);
        }
      }
      Stmt::Debug(debug) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_debug(debug);
        }
      }
      Stmt::Stdin(stdin) => {
        self.generate_stdin(stdin)?;
      }
      Stmt::CtIf(_) => {
        // Compile-time if statements are resolved during semantic analysis
      }
      Stmt::CtFail(_) | Stmt::CtWarn(_) => {
        // Compile-time fail/warn statements are handled during semantic analysis
      }
      Stmt::CtWhen(_) | Stmt::CtMatch(_) | Stmt::CtSwitch(_) => {
        // Compile-time control-flow statements are resolved during semantic analysis
      }
      Stmt::Doc(_) | Stmt::Comment(_) => {
        // Comments don't generate code
      }
      Stmt::AddError(add_error) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_add_error(add_error);
        }
      }
      Stmt::WriteErrors(_) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_drain_errors(false);
        }
      }
      Stmt::WarnErrors(_) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_drain_errors(true);
        }
      }
      Stmt::AlertErrors(_) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_drain_errors_alert();
        }
      }
      Stmt::Assert(assert) => {
        self.generate_assert(assert);
      }
      Stmt::Alert(alert) => {
        if self.builder.get_current_func().is_some() || self.mode == "main" {
          self.generate_alert(alert);
        }
      }
      Stmt::TestSuite(suite) => {
        // Generate suite-level declarations (vars/functions) first, then the
        // test cases. Suite vars become mangled globals shared by every case and
        // reachable from suite functions; suite functions become mangled
        // functions callable from cases. Both are invisible to non-test code
        // because they are generated inside the `#mode == "test"` guard emitted
        // by the top-level loop.
        let was_suite_scope_name = self.suite_scope_name.clone();
        let suite_prefix = format!("{}::", suite.name);
        self.suite_scope_name = Some(suite_prefix.clone());
        self.push_scope(); // suite scope: suite vars keyed by simple name

        for decl in suite.declarations() {
          match decl {
            Stmt::VarDef(var_def) => {
              // Force a global so the value is shared across cases and reachable
              // from suite functions; `suite_scope_name` makes `define_var` key
              // it by its simple name in the suite scope.
              let was_in_user_function = self.in_user_function;
              self.in_user_function = false;
              self.try_generate_var_def(var_def)?;
              self.in_user_function = was_in_user_function;
            }
            Stmt::FnDef(fn_def) => {
              let mut mangled = fn_def.clone();
              mangled.name.node = format!("{}{}", suite_prefix, fn_def.name.node);
              self.try_generate_fn_def(&mangled)?;
            }
            _ => {}
          }
        }

        for case in suite.cases() {
          // `lale test --filter`: skip cases that don't match any pattern.
          if !test_case_matches(&suite.name, &case.name, &self.test_filter) {
            continue;
          }
          self.builder.test_begin(&suite.name, &case.name);
          let was_in_test_case = self.in_test_case;
          let was_in_user_function = self.in_user_function;
          self.in_test_case = true;
          self.in_user_function = true;
          self.push_scope();
          for stmt in &case.body {
            self.try_generate_stmt(stmt)?;
          }
          // Free the case's string data before leaving the case scope so that
          // same-named variables in different cases are each cleaned up.
          self.emit_str_data_auto_free();
          self.pop_scope();
          self.in_user_function = was_in_user_function;
          self.in_test_case = was_in_test_case;
          self.builder.test_end();
        }

        self.pop_scope(); // suite scope
        self.suite_scope_name = was_suite_scope_name;
      }
    }
    Ok(())
  }

  /// Generate IR for a variable definition (`var x as T = expr`).
  ///
  /// Strategy:
  /// 1. Generate the initializer expression into an SSA value.
  /// 2. If the variable has a declared type, optionally insert a type
  ///    conversion if the initializer type differs (widening conversions only).
  /// 3. Allocate stack space (for locals) or a global (for globals) and store
  ///    the value.
  ///
  /// Global vs local detection: if `in_user_function` is false, the variable
  /// becomes an IR global; otherwise it's a stack alloca.
  fn try_generate_var_def(&mut self, var_def: &VarDefStmt) -> CompileResult<()> {
    let var_name = var_def.name.node.clone();

    // Determine type: use annotation if present, else infer from expression value.
    // When no annotation exists, we generate the expression first to extract its IR type,
    // then allocate at that type. This avoids the brittle default-I64 fallback.
    let (mut ir_type, pre_generated_value) = if let Some(type_annotation) = &var_def.type_annotation
    {
      let ty = self.type_name_to_ir_type(type_annotation);
      let val = self.try_generate_expr_with_resolved_type(&var_def.value, Some(&ty))?;
      (ty, Some(val))
    } else {
      // No type annotation: generate expression without context to determine its IR type.
      // This works for self-typing expressions (string literals, type constructors,
      // function calls). Unannotated numeric literals are rejected by semantic analysis.
      let val = self.try_generate_expr(&var_def.value)?;
      let ty = self.get_value_type(val);
      (ty, Some(val))
    };

    // If this is a pointer, try to refine the type using semantic analysis
    // The semantic analyzer tracks pointer_to_type information for each pointer.
    // Note: many pointers are intentionally raw (e.g., `0 as pointer`) and
    // will not have a tracked target type — that's normal, not a bug.
    if matches!(ir_type, IrType::Ptr(_))
      && let Some(target_type) = self.get_pointer_target_type_from_semantic_analysis(&var_name)
    {
      ir_type = IrType::ptr(target_type);
    }

    // Global-scope definitions create IR globals (accessible from any function).
    // Function-local definitions use stack allocas. Synthetic loop variables are
    // function-local even at the top level (they live in the synthetic main).
    let is_global = !self.in_user_function && !var_def.is_loop_var;

    let (allocation, global_id) = if self.is_sibling_join(&var_name) {
      // Sibling join: reuse the slot allocated in a previous branch of the same
      // construct. Semantic analysis already verified the definite-assignment
      // join, so the slot exists.
      match self.lookup_var(&var_name) {
        Some(info) => {
          if let Some(gid) = info.global_id {
            // Globals: re-derive the address in the current block, because the
            // original `global_addr` ValueId may live in a sibling block and be
            // undefined here (same dominance concern as `emit_global_str_auto_free`).
            let ptr = self.builder.global_addr(gid);
            (ptr, Some(gid))
          } else {
            (info.allocation, None)
          }
        }
        None => ice!(
          "sibling join for '{}' has no existing slot — semantic analysis invariant violated",
          var_name
        ),
      }
    } else if is_global {
      let linkage = if var_def.is_export {
        Linkage::Export
      } else {
        Linkage::Internal
      };
      // Suite globals get a `suite::` prefix to avoid colliding with module globals.
      let storage_name = match &self.suite_scope_name {
        Some(prefix) => format!("{}{}", prefix, var_name),
        None => var_name.clone(),
      };
      let gid = self
        .builder
        .add_global(&storage_name, ir_type.clone(), linkage);
      let ptr = self.builder.global_addr(gid);
      (ptr, Some(gid))
    } else {
      let alloc = self.builder.alloca_named(ir_type.clone(), &var_name);
      (alloc, None)
    };

    // Use the pre-generated value (from above) to avoid redundant regeneration.
    // pre_generated_value is always Some — see both branches of the if above.
    let value = pre_generated_value.unwrap_or_else(|| {
      ice!("pre_generated_value is None — invariant violation in try_generate_var_def")
    });

    // Get the actual type of the generated value for type checking
    let value_type = self.get_value_type(value);

    // Handle array initialization (arrays bypass the normal store path)
    if let IrType::Array { .. } = &ir_type {
      if let Expr::ArrayLiteral(arr_lit) = &var_def.value {
        self.initialize_array_from_literal(allocation, &ir_type, arr_lit)?;
      }
    } else {
      // For non-array types: check for type mismatch and convert if needed,
      // then store the value.
      let actual_value = if matches!(&ir_type, IrType::Struct { .. }) {
        if matches!(value_type, IrType::Ptr(_)) && !matches!(ir_type, IrType::Ptr(_)) {
          self.builder.load(value, ir_type.clone())
        } else {
          value
        }
      } else if value_type != ir_type {
        // Skip conversion if value is already a struct-based optional and target is Optional
        if Self::is_optional_struct_type(&value_type) && matches!(&ir_type, IrType::Optional(_)) {
          value
        } else {
          self.generate_type_conversion(value, value_type.clone(), ir_type.clone())
        }
      } else {
        value
      };

      // If declared type is T? but value is T, wrap as optional
      // Check both the original value type AND the actual (possibly converted) value type.
      let actual_value_type = self.get_value_type(actual_value);
      let actual_value = if let IrType::Optional(inner) = &ir_type {
        if !matches!(actual_value_type, IrType::Optional(_))
          && !Self::is_optional_struct_type(&actual_value_type)
        {
          self.build_optional_wrap(actual_value, inner)
        } else {
          actual_value
        }
      } else {
        actual_value
      };

      self.builder.store(actual_value, allocation);
    }

    // Track the allocation in the scope
    self.define_var(var_name, allocation, ir_type, global_id);
    Ok(())
  }

  /// Generate IR for an assignment (`x = expr` or `arr[i] = expr`).
  ///
  /// Strategy:
  /// 1. Generate the RHS expression into an SSA value.
  /// 2. If the target has indices (`arr[i]`), compute the array element
  ///    address via `compute_array_offset` (which emits BoundsCheck).
  /// 3. For simple assignments, look up the target variable's alloca/global
  ///    address and emit a Store instruction.
  /// 4. Insert type conversions if the RHS type differs from the target type.
  fn try_generate_assign(&mut self, assign: &AssignStmt) -> CompileResult<()> {
    // Get the target variable name (simple case: single-element path)
    if assign.target.node.len() == 1 {
      let var_name = &assign.target.node[0];

      // Look up the variable info (allocation and type)
      if let Some(var_info) = self.lookup_var(var_name) {
        let ptr = self.var_ptr(&var_info);
        if assign.indices.is_empty() {
          // Simple variable assignment: arr = [1, 2, 3]
          let value =
            self.try_generate_expr_with_resolved_type(&assign.value, Some(&var_info.var_type))?;
          self.builder.store(value, ptr);
        } else {
          // Array element assignment: arr[i] = value
          // Check if this is an array assignment
          if let Some(array_info) = self.lookup_var(var_name) {
            if matches!(array_info.var_type, IrType::Array { .. }) {
              let element_type = self.get_array_element_type(var_name);
              let value =
                self.try_generate_expr_with_resolved_type(&assign.value, Some(&element_type))?;
              let dimensions = self.get_array_dimensions(var_name);

              // For array types, var_ptr returns the array pointer directly
              let array_ptr = ptr;

              // Compute offset for the array element
              let offset =
                self.compute_array_offset(&assign.indices, &dimensions, &assign.location);

              // Compute element size
              let element_size = self.ir_type_size(&element_type);

              // Multiply offset by element size
              let size_const = self.builder.const_int(IrType::I64, element_size);
              let byte_offset = self.builder.mul(offset, size_const, IrType::I64);

              // Compute target address: array_ptr + byte_offset
              let target_addr = self.builder.add(array_ptr, byte_offset, IrType::raw_ptr());

              // Store the value at the computed address
              self.builder.store(value, target_addr);
            }
          } else {
            return Err(
              IrGenError::UndefinedVariable {
                name: var_name.clone(),
              }
              .into(),
            );
          }
        }
      }
    }
    // Handle nested member assignment: obj.field = value
    if assign.target.node.len() == 2 {
      let obj_name = &assign.target.node[0];
      let field_name = &assign.target.node[1];

      if let Some(var_info) = self.lookup_var(obj_name) {
        let obj_ptr = self.var_ptr(&var_info);

        // Find the field in the type layout
        for (type_name, layout) in self.type_layouts.iter() {
          if let Some(field_idx) = layout.field_names.iter().position(|f| f == field_name) {
            let field_type = layout.field_types[field_idx].clone();
            let field_offset = (0..field_idx)
              .map(|i| self.ir_type_size(&layout.field_types[i]))
              .sum::<i64>();

            let field_ptr = if field_offset == 0 {
              self
                .builder
                .get_field_ptr(obj_ptr, type_name, field_idx as u32)
            } else {
              let offset_val = self.builder.const_int(IrType::I64, field_offset);
              self.builder.add(obj_ptr, offset_val, IrType::raw_ptr())
            };

            let value =
              self.try_generate_expr_with_resolved_type(&assign.value, Some(&field_type))?;
            self.builder.store(value, field_ptr);
            return Ok(());
          }
        }
        return Err(
          IrGenError::UndefinedVariable {
            name: format!("{}.{}", obj_name, field_name),
          }
          .into(),
        );
      }
    }
    Ok(())
  }

  /// Generate code for a compound assignment, returning Result on failure.
  fn try_generate_compound_assign(&mut self, compound: &CompoundAssignStmt) -> CompileResult<()> {
    // Get the target variable name (simple case: single-element path)
    if compound.target.node.len() == 1 {
      let var_name = &compound.target.node[0];

      // Look up the variable info
      if let Some(var_info) = self.lookup_var(var_name) {
        let ptr = self.var_ptr(&var_info);
        if compound.indices.is_empty() {
          // Simple compound assignment: x += 5
          let current_value = self.builder.load(ptr, var_info.var_type.clone());

          // Generate RHS with the variable's resolved type so literals are
          // created directly in the correct type (e.g., i32 instead of i64)
          let rhs_value =
            self.try_generate_expr_with_resolved_type(&compound.value, Some(&var_info.var_type))?;
          self.emit_compound_div_mod_zero_check(compound, rhs_value);

          let new_value = match &compound.operator.node {
            CompoundOp::AddAssign => self.emit_int_add(
              current_value,
              rhs_value,
              var_info.var_type.clone(),
              &compound.location,
            ),
            CompoundOp::SubAssign => self.emit_int_sub(
              current_value,
              rhs_value,
              var_info.var_type.clone(),
              &compound.location,
            ),
            CompoundOp::MulAssign => self.emit_int_mul(
              current_value,
              rhs_value,
              var_info.var_type.clone(),
              &compound.location,
            ),
            CompoundOp::DivAssign => {
              self
                .builder
                .div(current_value, rhs_value, var_info.var_type.clone())
            }
            CompoundOp::ModAssign => {
              self
                .builder
                .rem(current_value, rhs_value, var_info.var_type.clone())
            }
          };

          self.builder.store(new_value, ptr);
        } else {
          // Array element compound assignment: arr[i] += value
          let element_type = self.get_array_element_type(var_name);
          let dimensions = self.get_array_dimensions(var_name);

          // For array types, var_ptr returns the array pointer directly
          let array_ptr = ptr;

          // Compute offset for the array element
          let offset =
            self.compute_array_offset(&compound.indices, &dimensions, &compound.location);

          // Compute element size
          let element_size = self.ir_type_size(&element_type);

          // Multiply offset by element size
          let size_const = self.builder.const_int(IrType::I64, element_size);
          let byte_offset = self.builder.mul(offset, size_const, IrType::I64);

          // Compute target address
          let target_addr = self.builder.add(array_ptr, byte_offset, IrType::raw_ptr());

          // Load current value, apply compound op, store back
          let current_value = self.builder.load(target_addr, element_type.clone());

          // Generate RHS with the element's resolved type so literals are
          // created directly in the correct type
          let rhs_value =
            self.try_generate_expr_with_resolved_type(&compound.value, Some(&element_type))?;
          self.emit_compound_div_mod_zero_check(compound, rhs_value);

          // Perform the operation
          let new_value = match &compound.operator.node {
            CompoundOp::AddAssign => self.emit_int_add(
              current_value,
              rhs_value,
              element_type.clone(),
              &compound.location,
            ),
            CompoundOp::SubAssign => self.emit_int_sub(
              current_value,
              rhs_value,
              element_type.clone(),
              &compound.location,
            ),
            CompoundOp::MulAssign => self.emit_int_mul(
              current_value,
              rhs_value,
              element_type.clone(),
              &compound.location,
            ),
            CompoundOp::DivAssign => {
              self
                .builder
                .div(current_value, rhs_value, element_type.clone())
            }
            CompoundOp::ModAssign => {
              self
                .builder
                .rem(current_value, rhs_value, element_type.clone())
            }
          };

          // Store the result back
          self.builder.store(new_value, target_addr);
        }
      } else {
        return Err(
          IrGenError::UndefinedVariable {
            name: var_name.clone(),
          }
          .into(),
        );
      }
    }
    // Handle nested member compound assignment: obj.field += value
    if compound.target.node.len() == 2 {
      let obj_name = &compound.target.node[0];
      let field_name = &compound.target.node[1];

      if let Some(var_info) = self.lookup_var(obj_name) {
        let obj_ptr = self.var_ptr(&var_info);

        for (type_name, layout) in self.type_layouts.iter() {
          if let Some(field_idx) = layout.field_names.iter().position(|f| f == field_name) {
            let field_type = layout.field_types[field_idx].clone();
            let field_offset = (0..field_idx)
              .map(|i| self.ir_type_size(&layout.field_types[i]))
              .sum::<i64>();

            let field_ptr = if field_offset == 0 {
              self
                .builder
                .get_field_ptr(obj_ptr, type_name, field_idx as u32)
            } else {
              let offset_val = self.builder.const_int(IrType::I64, field_offset);
              self.builder.add(obj_ptr, offset_val, IrType::raw_ptr())
            };

            let current_value = self.builder.load(field_ptr, field_type.clone());
            let rhs_value =
              self.try_generate_expr_with_resolved_type(&compound.value, Some(&field_type))?;
            self.emit_compound_div_mod_zero_check(compound, rhs_value);
            let new_value = match &compound.operator.node {
              CompoundOp::AddAssign => self.emit_int_add(
                current_value,
                rhs_value,
                field_type.clone(),
                &compound.location,
              ),
              CompoundOp::SubAssign => self.emit_int_sub(
                current_value,
                rhs_value,
                field_type.clone(),
                &compound.location,
              ),
              CompoundOp::MulAssign => self.emit_int_mul(
                current_value,
                rhs_value,
                field_type.clone(),
                &compound.location,
              ),
              CompoundOp::DivAssign => {
                self
                  .builder
                  .div(current_value, rhs_value, field_type.clone())
              }
              CompoundOp::ModAssign => {
                self
                  .builder
                  .rem(current_value, rhs_value, field_type.clone())
              }
            };
            self.builder.store(new_value, field_ptr);
            return Ok(());
          }
        }
      }
    }
    Ok(())
  }

  /// Emit a runtime zero-check for compound division/modulo assignments.
  fn emit_compound_div_mod_zero_check(&mut self, compound: &CompoundAssignStmt, rhs: ValueId) {
    if matches!(
      compound.operator.node,
      CompoundOp::DivAssign | CompoundOp::ModAssign
    ) {
      let msg = if matches!(compound.operator.node, CompoundOp::DivAssign) {
        "division by zero"
      } else {
        "modulo by zero"
      };
      self.builder.zero_check(
        rhs,
        msg,
        compound.location.source_file.as_str(),
        compound.location.line as i64,
        compound.location.col as i64,
      );
    }
  }

  /// Generate code for a function definition, returning Result on failure.
  fn try_generate_fn_def(&mut self, fn_def: &FnDefStmt) -> CompileResult<()> {
    let fn_name = fn_def.name.node.clone();

    // Build qualified function name from parameters
    let param_types: Vec<String> = fn_def
      .parameters
      .iter()
      .map(|p| {
        crate::semantic_analysis::type_compatibility::TypeInference::type_name_to_string(
          &p.type_annotation,
        )
      })
      .collect();
    let param_strs: Vec<&str> = param_types.iter().map(|s| s.as_str()).collect();
    let _qualified_name =
      crate::semantic_analysis::QualifiedFunctionName::new(&fn_name, param_strs);

    // Convert return type
    let return_type = match &fn_def.return_type.kind {
      ReturnTypeKind::Nothing => IrType::Void,
      ReturnTypeKind::Type(type_name) => self.type_name_to_ir_type(type_name),
    };

    let linkage = if fn_def.is_export {
      Linkage::Export
    } else {
      Linkage::Internal
    };

    // Save the current function context (in case we're inside main)
    let prev_func = self.builder.get_current_func();
    let prev_block = self.builder.get_current_block();
    let prev_function_name = self.current_function.clone();
    let prev_return_type = self.current_return_type.clone();
    let prev_epilogue = self.current_epilogue;
    let prev_return_slot = self.current_return_slot;

    // Set current function name (use simple name for internal tracking)
    self.current_function = Some(fn_name.clone());
    self.current_return_type = Some(return_type.clone());

    // Use qualified name in IR (includes parameter types for overloading)
    self._start_function(_qualified_name.as_str(), return_type.clone(), linkage);

    // Push new scope for function body
    self.push_scope();

    // Clear str tracking sets for this function
    self.consumed_str_allocas.clear();
    self.transferred_str_data.clear();
    self.param_names.clear();
    self.on_exit_stmts.clear();

    // Create the shared epilogue block and (for non-void) a return-value slot.
    let epilogue_block = self.builder.create_block("epilogue");
    self.current_epilogue = Some(epilogue_block);
    self.current_return_slot = if matches!(&return_type, IrType::Void) {
      None
    } else {
      let slot = self.builder.alloca(return_type.clone());
      // Exclude the return slot from frame reclamation — it carries the result.
      self.consumed_str_allocas.insert(slot);
      Some(slot)
    };

    // Add parameters. Unlike type/enum constructors (which use `add_param` and
    // read params as SSA values), regular function parameters get a stack slot
    // so they can be reassigned and referenced through the scope like any local.
    for param in &fn_def.parameters {
      let param_type = self.type_name_to_ir_type(&param.type_annotation);

      // Track str parameters — they are owned by the caller, not freed at exit
      if matches!(&param_type, IrType::Struct { name } if name == "str") {
        self.param_names.insert(param.name.node.clone());
      }
      // SSA parameter value from function signature
      let param_value_id = self
        .builder
        .add_param(&param.name.node, param_type.clone(), None);

      // Allocate stack storage for this parameter
      let alloc = self.builder.alloca(param_type.clone());
      // Store the incoming parameter value into the stack slot
      self.builder.store(param_value_id, alloc);

      // In scopes, the "allocation" for the variable is the stack slot pointer
      self.define_var(param.name.node.clone(), alloc, param_type, None);
    }

    // Loop stack must be clean at function entry — semantic analysis validates all loops
    if !self.loop_stack.is_empty() {
      ice!(
        "Function '{}' has non-empty loop_stack at entry (len={}) — loop IR gen invariant violated",
        fn_name,
        self.loop_stack.len()
      );
    }

    // Set flag before generating function body so variable definitions
    // inside this function are recognized as function-local (not global).
    let was_in_user_function = self.in_user_function;
    self.in_user_function = true;

    // Generate code for function body
    for stmt in &fn_def.body {
      self.try_generate_stmt(stmt)?;
    }

    // If the body didn't already branch somewhere (e.g. an explicit return),
    // fall through to the shared epilogue.
    if !self.builder.is_current_block_terminated() {
      self.builder.br(epilogue_block);
    }

    // Emit the shared epilogue: on-exit statements, frame reclamation, return.
    self.builder.position_at(epilogue_block);
    self.emit_on_exit_stmts()?;
    self.emit_auto_free();
    match return_type {
      IrType::Void => self.builder.ret_void(),
      rt => {
        let slot = self
          .current_return_slot
          .unwrap_or_else(|| ice!("non-void function '{}' has no return slot", fn_name));
        let val = self.builder.load(slot, rt);
        // Free the return slot now that the value has been extracted.
        self
          .builder
          .call_void_named("__lale_free_pointer", vec![slot]);
        self.builder.ret(val);
      }
    }

    // Loop stack must be clean at function exit — all loops must be properly closed
    if !self.loop_stack.is_empty() {
      ice!(
        "Function '{}' has non-empty loop_stack at exit (len={}) — loop IR gen invariant violated",
        fn_name,
        self.loop_stack.len()
      );
    }

    // Pop function scope
    self.pop_scope();

    // Restore previous function context
    self.builder.set_current_func(prev_func);
    self.builder.set_current_block(prev_block);
    self.current_function = prev_function_name;
    self.current_return_type = prev_return_type;
    self.current_epilogue = prev_epilogue;
    self.current_return_slot = prev_return_slot;
    self.in_user_function = was_in_user_function;
    Ok(())
  }

  /// Generate code for a statement.
  /// Thin wrapper around `try_generate_stmt` for callers that can't propagate errors.
  fn generate_stmt(&mut self, stmt: &Stmt) {
    Self::unwrap_or_panic(self.try_generate_stmt(stmt), "generate_stmt");
  }

  /// Generate code for an unsafe declaration (uninitialized variable).
  ///
  /// Unsafe declarations (e.g., `unsafe x: i64`) allocate stack space without initializing.
  /// Unlike regular definitions, there is no stored value until an assignment occurs.
  ///
  /// Implementation Note:
  /// The unsafe declaration is handled entirely by semantic analysis. When the semantic
  /// analyzer processes an unsafe declaration, it:
  /// 1. Creates a symbol table entry marking the variable as unsafe
  /// 2. Does NOT assign an initial value (unlike regular definitions)
  /// 3. Tracks that the variable requires initialization before use
  ///
  /// At the IR generation level, no explicit code is needed because:
  /// - The variable allocation happens during semantic analysis passes
  /// - Stack space is managed by the scope stack (lookup_var handles retrieval)
  /// - Assignment statements (Stmt::Assign) handle storing actual values
  /// - Use-before-initialization is validated by semantic analysis
  ///
  /// This design follows Lale's philosophy of explicit error checking at compile time
  /// rather than runtime initialization checks.
  fn generate_unsafe_decl(&mut self, _decl: &crate::ast::definitions::UnsafeDeclStmt) {
    // No IR generation needed - semantic analysis handles everything
  }

  fn generate_value_at_assign(&mut self, value_at: &ValueAtAssignStmt) {
    // Evaluate the pointer expression
    let ptr_value = self.generate_expr(&value_at.pointer);

    // Evaluate the value expression
    let value = self.generate_expr(&value_at.value);

    // Store the value at the pointer location
    self.builder.store(value, ptr_value);
  }

  /// Generate code for an if statement.
  fn generate_if(&mut self, if_stmt: &IfStmt) {
    self.enter_construct();
    // Evaluate the condition
    let cond_value = self.generate_expr(&if_stmt.condition.expr);
    let cond_value = if if_stmt.condition.negated {
      self.builder.not(cond_value)
    } else {
      cond_value
    };

    // Create basic blocks for all branches
    let then_block = self.builder.create_block("if.then");

    // Determine else block: either explicit else or first else_if
    let (else_block, merge_block) =
      if !if_stmt.else_if_branches.is_empty() || if_stmt.else_branch.is_some() {
        let else_bb = self.builder.create_block("if.else");
        let merge_bb = self.builder.create_block("if.merge");
        (else_bb, merge_bb)
      } else {
        let merge_bb = self.builder.create_block("if.merge");
        (merge_bb, merge_bb)
      };

    // Emit conditional branch
    self.builder.cond_br(cond_value, then_block, else_block);

    // Generate then branch
    self.builder.position_at(then_block);
    for stmt in &if_stmt.then_branch {
      self.generate_stmt(stmt);
    }
    // Branch to merge if no return/exit already terminated the block
    if !self.builder.is_current_block_terminated() {
      self.builder.br(merge_block);
    }

    // Generate else_if branches
    if !if_stmt.else_if_branches.is_empty() {
      let mut current_else_block = else_block;
      for (i, (cond, branch)) in if_stmt.else_if_branches.iter().enumerate() {
        self.builder.position_at(current_else_block);

        // Evaluate this condition
        let branch_cond = self.generate_expr(&cond.expr);
        let branch_cond = if cond.negated {
          self.builder.not(branch_cond)
        } else {
          branch_cond
        };

        // Create block for this branch body
        let branch_body = self.builder.create_block(format!("if.else_if.{i}.body"));

        // Create next else block or merge
        let next_block = if i + 1 < if_stmt.else_if_branches.len() || if_stmt.else_branch.is_some()
        {
          self.builder.create_block(format!("if.else_if.{i}.next"))
        } else {
          merge_block
        };

        // Conditional branch
        self.builder.cond_br(branch_cond, branch_body, next_block);

        // Generate branch body
        self.builder.position_at(branch_body);
        for stmt in branch {
          self.generate_stmt(stmt);
        }
        // Branch to merge if not already terminated
        if !self.builder.is_current_block_terminated() {
          self.builder.br(merge_block);
        }

        current_else_block = next_block;
      }

      // Generate else clause if present, or jump to merge
      if let Some(else_stmts) = &if_stmt.else_branch {
        self.builder.position_at(current_else_block);
        for stmt in else_stmts {
          self.generate_stmt(stmt);
        }
        // Branch to merge if not already terminated
        if !self.builder.is_current_block_terminated() {
          self.builder.br(merge_block);
        }
      } else if current_else_block != merge_block {
        // Only branch to merge if current_else_block is not already the merge block
        // (if it is the merge block, the condition branch already jumps to it directly)
        self.builder.position_at(current_else_block);
        // Branch to merge if not already terminated
        if !self.builder.is_current_block_terminated() {
          self.builder.br(merge_block);
        }
      }
    } else if let Some(else_stmts) = &if_stmt.else_branch {
      // Generate simple else clause
      self.builder.position_at(else_block);
      for stmt in else_stmts {
        self.generate_stmt(stmt);
      }
      // Branch to merge if not already terminated
      if !self.builder.is_current_block_terminated() {
        self.builder.br(merge_block);
      }
    }

    // Continue at merge point only if it's reachable
    // The merge block is reachable if:
    // 1. There's no else_branch (implicit else jumps to merge)
    // 2. OR at least one branch might not terminate
    // For now, conservatively assume merge block is reachable
    // (a full control flow analysis would determine if all paths return)
    self.exit_construct();
    self.builder.position_at(merge_block);
  }

  /// Generate IR for a when statement: one-sided action with no else.
  fn generate_when(&mut self, when_stmt: &WhenStmt) {
    self.enter_construct();
    // Evaluate the condition
    let cond_value = self.generate_expr(&when_stmt.condition.expr);
    let cond_value = if when_stmt.condition.negated {
      self.builder.not(cond_value)
    } else {
      cond_value
    };

    // Create basic blocks
    let body_block = self.builder.create_block("when.body");
    let merge_block = self.builder.create_block("when.merge");

    // Conditional branch: if true → body, else → merge (skip)
    self.builder.cond_br(cond_value, body_block, merge_block);

    // Generate body
    self.builder.position_at(body_block);
    for stmt in &when_stmt.body {
      self.generate_stmt(stmt);
    }
    if !self.builder.is_current_block_terminated() {
      self.builder.br(merge_block);
    }

    // Continue at merge point
    self.exit_construct();
    self.builder.position_at(merge_block);
  }

  /// Generate a runtime warning for a `missing code` statement in debug mode.
  fn generate_missing_code_warning(&mut self, stmt: &MissingCodeStmt) {
    let msg = format!("missing code at line {}", stmt.location.line);
    let warning_expr = Expr::StringLiteral(StringLiteral {
      parts: vec![StringPart::Text(Spanned {
        node: format!("Warning: {}", msg),
        span: stmt.location.clone(),
      })],
      location: stmt.location.clone(),
    });
    self.generate_output(&warning_expr, false, OutputDest::Stderr);
  }

  /// Generate code for `release ptr_expr` — frees heap memory.
  fn generate_release(&mut self, stmt: &ReleaseStmt) -> CompileResult<()> {
    let ptr = self.try_generate_expr(&stmt.pointer)?;
    self
      .builder
      .call_void_named("__lale_free_pointer", vec![ptr]);
    Ok(())
  }

  /// Generate code for `allocate(size_expr)` — allocates heap memory, returns pointer.
  fn generate_allocate(&mut self, allocate: &AllocateExpr) -> ValueId {
    let size = self.generate_expr_with_resolved_type(&allocate.size, Some(&IrType::U64));
    self.builder.call_named_at(
      "__lale_malloc_u64",
      vec![size],
      IrType::raw_ptr(),
      &allocate.location.source_file,
      allocate.location.line as i64,
      allocate.location.col as i64,
    )
  }

  /// Generate code for a loop statement.
  fn generate_loop(&mut self, loop_stmt: &LoopStmt) {
    let loop_header = self.builder.create_block("loop.header");
    let loop_body = self.builder.create_block("loop.body");
    let loop_exit = self.builder.create_block("loop.exit");

    // Push the exit block onto the loop stack (for break/continue handling)
    // and header block onto loop header stack (for rewind handling)
    self.loop_stack.push(loop_exit);
    self.loop_header_stack.push(loop_header);

    // Handle range-based loops
    if let Some(range) = &loop_stmt.range {
      // Create this block only for range based loops
      let loop_increment = self.builder.create_block("loop.increment");

      // Determine the loop variable IR type
      let var_type = self.type_name_to_ir_type(&range.var_type);

      // Generate end value (start value is generated by try_generate_var_def
      // on the synthetic VarDefStmt injected by the AST builder)
      let end_val = self.generate_expr_with_resolved_type(&range.to, Some(&var_type));

      // Look up the loop variable: the synthetic VarDefStmt has already
      // allocated and initialized it, so we reuse its allocation.
      let var_info = match self.lookup_var(&range.variable.node) {
        Some(info) => info,
        None => panic!(
          "Loop variable '{}' should be registered by synthetic VarDefStmt",
          range.variable.node
        ),
      };
      let loop_var_alloc = self.var_ptr(&var_info);

      // Branch to loop header
      self.builder.br(loop_header);

      // ============ Generate loop header============
      self.builder.position_at(loop_header);

      // Load current loop variable value
      let loop_var_val = self.builder.load(loop_var_alloc, var_type.clone());

      // Check condition BEFORE body: loop_var <= end_val (inclusive upper bound)
      // This ensures the body never executes with values beyond the range.
      let cond = self.builder.le(loop_var_val, end_val);
      self.builder.cond_br(cond, loop_body, loop_exit);

      // ============ Generate loop body ============
      self.builder.position_at(loop_body);
      self.enter_construct();
      for stmt in &loop_stmt.body {
        self.generate_stmt(stmt);
      }
      self.exit_construct();

      // Branch to increment (unconditional — condition already checked in header)
      self.builder.br(loop_increment);

      // ============ Generate loop increment ============
      self.builder.position_at(loop_increment);
      // Increment loop variable by the step amount (defaults to 1)
      let step_val = if let Some(step_expr) = &range.step {
        self.generate_expr_with_resolved_type(step_expr, Some(&var_type))
      } else {
        // Default step = 1, using the correct constant type
        match &var_type {
          IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
            self.builder.const_uint(var_type.clone(), 1)
          }
          IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64 => {
            self.builder.const_int(var_type.clone(), 1)
          }
          IrType::F16 | IrType::F32 | IrType::F64 => {
            self.builder.const_float(var_type.clone(), 1.0)
          }
          other => {
            ice!(
              "Unsupported loop variable type {:?} — semantic analysis should have rejected this",
              other
            );
          }
        }
      };
      let next_val = self.builder.add(loop_var_val, step_val, var_type.clone());
      self.builder.store(next_val, loop_var_alloc);

      // Branch back to header
      self.builder.br(loop_header);
    } else {
      // Infinite loop (or loop with conditions)
      self.builder.br(loop_header);

      self.builder.position_at(loop_header);

      // Handle entry condition if present
      if let Some(pre_cond) = &loop_stmt.pre_condition {
        let cond_val = self.generate_expr(&pre_cond.expr);
        let cond_val = if pre_cond.negated {
          self.builder.not(cond_val)
        } else {
          cond_val
        };

        self.builder.cond_br(cond_val, loop_body, loop_exit);
      } else {
        // No entry condition: always enter body
        self.builder.br(loop_body);
      }

      // ============ Generate loop body============
      self.builder.position_at(loop_body);
      self.enter_construct();
      for stmt in &loop_stmt.body {
        self.generate_stmt(stmt);
      }
      self.exit_construct();

      // Handle post-condition if present
      if let Some(post_cond) = &loop_stmt.post_condition {
        let cond_val = self.generate_expr(&post_cond.expr);
        let cond_val = if post_cond.negated {
          self.builder.not(cond_val)
        } else {
          cond_val
        };

        self.builder.cond_br(cond_val, loop_exit, loop_body);
      } else {
        // No post-condition: loop back to body (entry condition is a one‑time gate)
        self.builder.br(loop_body);
      }
    }

    // Pop loop exit and header blocks from stacks
    self.loop_stack.pop();
    self.loop_header_stack.pop();

    // Continue at loop exit
    self.builder.position_at(loop_exit);
  }

  /// Generate code for a return statement.
  fn generate_return(&mut self, ret: &ReturnStmt) {
    let return_type = self.current_return_type.clone();

    // Compute the value to return (wrapped in optional if needed), or None for void.
    let value_to_store: Option<ValueId> = if let Some(value_expr) = &ret.value {
      // `return nothing` in a void function is a no-op return.
      if matches!(value_expr, Expr::NothingExpr) && matches!(&return_type, Some(IrType::Void)) {
        None
      } else {
        // If the return expression is a simple str-typed identifier, mark its
        // alloca as consumed so frame reclamation skips it (ownership transfers).
        if let Expr::Identifier(ident) = value_expr
          && ident.path.len() == 1
        {
          let var_name = &ident.path[0];
          if let Some(var_info) = self.lookup_var(var_name)
            && matches!(&var_info.var_type, IrType::Struct { name } if name == "str")
          {
            self.transferred_str_data.insert(var_info.allocation);
          }
        }
        // For Optional(T) return types, resolve to the inner type T so that
        // `return 42` in `fn f() returns i64?` generates 42 as i64.
        let resolved_ret_type = return_type.clone().map(|rt| match rt {
          IrType::Optional(inner) => *inner,
          other => other,
        });
        let value = self.generate_expr_with_resolved_type(value_expr, resolved_ret_type.as_ref());
        // If the function returns T?, wrap the value as optional if not already.
        let wrapped = if let Some(IrType::Optional(inner)) = &return_type {
          let value_type = self.get_value_type(value);
          if !matches!(value_type, IrType::Optional(_))
            && !Self::is_optional_struct_type(&value_type)
          {
            self.build_optional_wrap(value, inner)
          } else {
            value
          }
        } else {
          value
        };
        Some(wrapped)
      }
    } else {
      // Bare return: produce `nothing` optional if T?, else void.
      if let Some(IrType::Optional(inner)) = &return_type {
        Some(self.build_optional_none(inner))
      } else {
        None
      }
    };

    // Store the return value into the return slot (non-void functions).
    if let (Some(slot), Some(val)) = (self.current_return_slot, value_to_store) {
      self.builder.store(val, slot);
    }

    // Branch to the shared epilogue, which runs on-exit statements and
    // frame reclamation before returning.
    let epilogue = self
      .current_epilogue
      .unwrap_or_else(|| ice!("return statement outside a function with an epilogue"));
    self.builder.br(epilogue);
  }

  /// Generate code for an exit program statement.
  fn generate_exit_program(&mut self, exit: &ExitProgramStmt) {
    let code = exit.code.unwrap_or(0);
    let code_val = self.builder.const_int(IrType::I32, code as i64);
    self.builder.call_void_named("__lale_exit", vec![code_val]);
  }

  /// Generate code for a stdout write statement.
  /// Generate assert: if condition is false, call __lale_error and exit.
  /// In release mode (is_debug = false), emits nothing.
  fn generate_assert(&mut self, assert: &AssertStmt) {
    if !self.is_debug {
      return;
    }
    // For an equality assert (`assert a == b`), capture the operand values so
    // the failure message can report "expected b, found a". Other conditions
    // keep the location-only message.
    let (cond, expected, found) = match &assert.condition {
      Expr::Binary(bin) if bin.operator == BinaryOp::Eq => {
        let lhs = self.generate_expr(&bin.left);
        let lhs_type = self.get_value_type(lhs);
        let rhs = self.generate_expr_with_resolved_type(&bin.right, Some(&lhs_type));
        let cond = self.builder.eq(lhs, rhs);
        (cond, Some(rhs), Some(lhs))
      }
      _ => {
        let cond = self.generate_expr(&assert.condition);
        (cond, None, None)
      }
    };
    let false_val = self.builder.const_bool(false);
    let is_false = self.builder.eq(cond, false_val);
    let fail_block = self.builder.create_block("assert.fail");
    let cont_block = self.builder.create_block("assert.cont");
    self.builder.cond_br(is_false, fail_block, cont_block);
    self.builder.position_at(fail_block);

    if self.in_test_case {
      // In test mode, `assert` reports the failure for the current test case
      // and continues so remaining cases still execute. The interpreter prints
      // `Fail: suite / case` when it reaches TestFail, and the shared TestEnd
      // suppresses the normal `Pass:` report.
      self.builder.test_fail(
        &assert.location.source_file,
        assert.location.line as i64,
        assert.location.col as i64,
        expected,
        found,
      );
      self.builder.br(cont_block);
    } else {
      let cond_str = ExpressionAnalyzer::expr_to_string(&assert.condition);
      let msg = format!(
        "Assertion failed: assert {} at {}:{}",
        cond_str, assert.location.source_file, assert.location.line
      );
      let msg_len_val = self.builder.const_int(IrType::I64, msg.len() as i64);
      let msg_val = self.builder.const_string(msg);
      let fd = self.builder.const_int(IrType::I32, 2);
      self
        .builder
        .call_named("write", vec![fd, msg_val, msg_len_val], IrType::I64);
      let code = self.builder.const_int(IrType::I32, 1);
      self.builder.call_void_named("__lale_exit", vec![code]);
    }
    self.builder.position_at(cont_block);
  }

  /// Generate a switch statement: enum dispatch or open value dispatch.
  fn generate_switch(&mut self, switch: &SwitchStmt) {
    let scrutinee_type = self.lookup_expr_type(&switch.value);
    if scrutinee_type == "unknown" {
      return;
    }

    self.enter_construct();
    if self.enum_variants.contains_key(&scrutinee_type) {
      self.generate_enum_switch(switch, &scrutinee_type);
    } else {
      self.generate_value_switch(switch, &scrutinee_type);
    }
    self.exit_construct();
  }

  /// Generate a switch over an enum: read discriminant, build IfElse chain.
  fn generate_enum_switch(&mut self, switch: &SwitchStmt, enum_type: &str) {
    let variants = match self.enum_variants.get(enum_type) {
      Some(v) => v.clone(),
      None => return,
    };

    // Build a map from variant name to discriminant index
    let disc_map: std::collections::HashMap<&str, usize> = variants
      .iter()
      .map(|(name, idx)| (name.as_str(), *idx))
      .collect();

    // Generate the enum value as an SSA value (not stored to memory)
    let enum_val = self.generate_expr(&switch.value);

    // Extract discriminant from field 0 using ExtractField (operates on SSA values)
    let first_variant_name = format!("{}__{}", enum_type, variants[0].0);
    let disc_val = self
      .builder
      .extract_field(enum_val, &first_variant_name, 0u32, IrType::I64);

    // After the switch, execution continues here
    let cont_block = self.builder.create_block("switch.cont");

    // Build if-else chain for each case
    let num_cases = switch.cases.len();
    for (case_idx, case) in switch.cases.iter().enumerate() {
      let SwitchPattern::Enum {
        variant_name,
        fields,
        ..
      } = &case.pattern
      else {
        // Rejected in semantic analysis; skip code generation for this arm.
        continue;
      };

      let disc_idx = *disc_map.get(variant_name.node.as_str()).unwrap_or_else(|| {
        ice!(
          "generate_enum_switch: variant '{}' not found in discriminator map for enum",
          variant_name.node
        );
      });
      let is_last_case = case_idx == num_cases - 1 && switch.default_case.is_none();

      if !is_last_case {
        let disc_const = self.builder.const_int(IrType::I64, disc_idx as i64);
        let cmp = self.builder.eq(disc_val, disc_const);
        let then_block = self
          .builder
          .create_block(format!("switch.arm_{}", case_idx));
        let else_block = self
          .builder
          .create_block(format!("switch.arm_{}_next", case_idx));
        self.builder.cond_br(cmp, then_block, else_block);

        // Then block: generate case body
        self.builder.position_at(then_block);
        self.generate_enum_switch_case_body(case, fields, enum_val, enum_type, &variant_name.node);
        self.builder.br(cont_block);

        // Else block: continue to next comparison
        self.builder.position_at(else_block);
      } else {
        // Last case with no default: no comparison needed, just emit body
        self.generate_enum_switch_case_body(case, fields, enum_val, enum_type, &variant_name.node);
        self.builder.br(cont_block);
      }
    }

    // Default case: if present, emit at the end of the chain
    if let Some(default_body) = &switch.default_case {
      for stmt in default_body {
        Self::unwrap_or_panic(self.try_generate_stmt(stmt), "switch default body");
      }
      self.builder.br(cont_block);
    }

    // Continue after the switch
    self.builder.position_at(cont_block);
  }

  /// Generate a switch over an open value type: compare the scrutinee against each literal.
  fn generate_value_switch(&mut self, switch: &SwitchStmt, scrutinee_type: &str) {
    let scrutinee_ir_type = self.type_string_to_ir_type(scrutinee_type);
    let scrutinee_val = self.generate_expr(&switch.value);

    let cont_block = self.builder.create_block("switch.cont");

    let num_cases = switch.cases.len();
    for (case_idx, case) in switch.cases.iter().enumerate() {
      let SwitchPattern::Literal { value, .. } = &case.pattern else {
        // Rejected in semantic analysis; skip code generation for this arm.
        continue;
      };

      let is_last_case = case_idx == num_cases - 1 && switch.default_case.is_none();

      if !is_last_case {
        let case_val = self.generate_expr_with_resolved_type(value, Some(&scrutinee_ir_type));
        let cmp = self.builder.eq(scrutinee_val, case_val);
        let then_block = self
          .builder
          .create_block(format!("switch.arm_{}", case_idx));
        let else_block = self
          .builder
          .create_block(format!("switch.arm_{}_next", case_idx));
        self.builder.cond_br(cmp, then_block, else_block);

        self.builder.position_at(then_block);
        for stmt in &case.body {
          Self::unwrap_or_panic(self.try_generate_stmt(stmt), "switch case body");
        }
        self.builder.br(cont_block);

        self.builder.position_at(else_block);
      } else {
        // This arm only runs without a `default`, which semantic analysis
        // rejects for open value types. Emit the body defensively.
        for stmt in &case.body {
          Self::unwrap_or_panic(self.try_generate_stmt(stmt), "switch case body");
        }
        self.builder.br(cont_block);
      }
    }

    if let Some(default_body) = &switch.default_case {
      for stmt in default_body {
        Self::unwrap_or_panic(self.try_generate_stmt(stmt), "switch default body");
      }
      self.builder.br(cont_block);
    }

    self.builder.position_at(cont_block);
  }

  /// Generate the body of a single enum switch case: extract variant fields and execute statements.
  fn generate_enum_switch_case_body(
    &mut self,
    case: &SwitchCase,
    fields: &[SwitchPatternField],
    enum_val: ValueId,
    enum_type: &str,
    variant_name: &str,
  ) {
    let variant_type_name = format!("{}__{}", enum_type, variant_name);

    // Extract pattern fields from the enum value using ExtractField (SSA, no memory round-trip)
    for (idx, field) in fields.iter().enumerate() {
      match field {
        SwitchPatternField::Bind(name) => {
          // Field index: +1 to skip the discriminant at field 0
          let field_idx = (idx + 1) as u32;
          let field_type = self.get_variant_field_type(&variant_type_name, field_idx);
          let field_val =
            self
              .builder
              .extract_field(enum_val, &variant_type_name, field_idx, field_type.clone());
          let alloca = self.builder.alloca(field_type.clone());
          self.builder.store(field_val, alloca);
          self.define_var(name.clone(), alloca, field_type, None);
        }
        SwitchPatternField::Discard => {
          // _ field — skip, no binding
        }
      }
    }

    // Generate case body statements
    for stmt in &case.body {
      Self::unwrap_or_panic(self.try_generate_stmt(stmt), "switch case body");
    }
  }

  /// Generate a match statement: evaluate guard conditions in order, run first matching arm.
  fn generate_match(&mut self, match_stmt: &MatchStmt) {
    self.enter_construct();
    // Create merge block
    let merge_block = self.builder.create_block("match.merge");

    // For each arm, create a condition check block and body block
    // Chain them: if arm0 matches → body0, else → check arm1, etc.
    // If no arm matches → else_arm or merge
    let mut next_check_block: Option<BlockId> = None; // first arm starts inline

    for (i, arm) in match_stmt.arms.iter().enumerate() {
      let body_block = self.builder.create_block(format!("match.arm{}.body", i));
      let check_block = if i + 1 < match_stmt.arms.len() || !match_stmt.else_arm.is_empty() {
        self.builder.create_block(format!("match.arm{}.check", i))
      } else {
        merge_block
      };

      // Position at the check/start point
      if let Some(prev_check) = next_check_block {
        self.builder.position_at(prev_check);
      }

      // Evaluate guard condition
      let cond = self.generate_expr(&arm.guard.expr);
      let cond = if arm.guard.negated {
        self.builder.not(cond)
      } else {
        cond
      };
      self.builder.cond_br(cond, body_block, check_block);

      // Generate body
      self.builder.position_at(body_block);
      for stmt in &arm.body {
        self.generate_stmt(stmt);
      }
      if !self.builder.is_current_block_terminated() {
        self.builder.br(merge_block);
      }

      next_check_block = Some(check_block);
    }

    // Generate else arm or jump to merge
    if !match_stmt.else_arm.is_empty() {
      if let Some(last_check) = next_check_block {
        self.builder.position_at(last_check);
      }
      for stmt in &match_stmt.else_arm {
        self.generate_stmt(stmt);
      }
      if !self.builder.is_current_block_terminated() {
        self.builder.br(merge_block);
      }
    } else if let Some(last_check) = next_check_block {
      // If last_check is already merge_block, the cond_br from the last arm
      // already handles reaching it — don't emit a self-looping branch.
      if last_check != merge_block {
        self.builder.position_at(last_check);
        if !self.builder.is_current_block_terminated() {
          self.builder.br(merge_block);
        }
      }
    }

    self.exit_construct();
    self.builder.position_at(merge_block);
  }

  /// Get the IR type of a specific field in a variant struct.
  /// field_idx is the 0-based index into the struct (0 = discriminant, 1+ = variant fields).
  fn get_variant_field_type(&self, variant_type_name: &str, field_idx: u32) -> IrType {
    if field_idx == 0 {
      return IrType::I64; // discriminant is always i64
    }
    // field_types stores only variant fields (without discriminant), so subtract 1
    let layout_idx = (field_idx - 1) as usize;
    if let Some(layout) = self.type_layouts.get(variant_type_name)
      && let Some(ft) = layout.field_types.get(layout_idx)
    {
      return ft.clone();
    }
    IrType::I64 // fallback
  }

  fn generate_stdout(&mut self, write: &StdoutStmt) {
    if let Some(target) = &write.target {
      // write expr to var: capture output into the variable (append)
      self.generate_write_to_var(&write.value, &target.node, write.inline, &write.location);
    } else {
      self.generate_output(&write.value, write.inline, OutputDest::Stdout);
    }
  }

  /// Generate code for write/warn ... to var: capture the string value into a variable.
  /// First use auto-allocates the variable; subsequent uses append via concat.
  fn generate_write_to_var(
    &mut self,
    expr: &Expr,
    var_name: &str,
    inline: bool,
    _location: &SourceLocation,
  ) {
    // Generate the expression as a Ptr(str) (same conversion logic as generate_output).
    // All paths produce a pointer to a str struct so that concat can consume them.
    let value = self.generate_expr(expr);
    let value_type = self.get_value_type(value);

    let str_ptr = match &value_type {
      IrType::Ptr(_) => value,
      IrType::Struct { name } if name == "str" => {
        let str_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(value, str_alloc);
        str_alloc
      }
      IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64 => {
        self.call_int_to_str(value, &value_type, false)
      }
      IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
        self.call_int_to_str(value, &value_type, true)
      }
      IrType::F16 | IrType::F32 | IrType::F64 => {
        let f64_val = if value_type == IrType::F64 {
          value
        } else {
          self.builder.fp_ext(value, IrType::F64)
        };
        self.builder.call_named(
          "__lale_f64_to_str_f64",
          vec![f64_val],
          IrType::struct_ref("str"),
        )
      }
      IrType::Bool => self.builder.call_named(
        "__lale_bool_to_str_bool",
        vec![value],
        IrType::struct_ref("str"),
      ),
      IrType::Char => {
        let i32_val = self.builder.bitcast(value, IrType::I32);
        self.builder.call_named(
          "__lale_char_to_str_char",
          vec![i32_val],
          IrType::struct_ref("str"),
        )
      }
      _ => value,
    };

    let str_type = IrType::struct_ref("str");

    // Load the str struct value from the pointer.
    let mut str_val = self.builder.load(str_ptr, str_type.clone());

    // Append newline for non-inline writes (matching stdout/stderr behavior).
    if !inline {
      let val_alloc = self.builder.alloca(str_type.clone());
      self.builder.store(str_val, val_alloc);
      let newline_str = self.builder.const_string("\n");
      let nl_alloc = self.builder.alloca(str_type.clone());
      self.builder.store(newline_str, nl_alloc);
      let with_nl = self.builder.concat(val_alloc, nl_alloc);
      str_val = self.builder.load(with_nl, str_type.clone());
    }

    // Look up or allocate the variable
    if let Some(var_info) = self.lookup_var(var_name) {
      // Subsequent use: load existing, concat with new value, store back
      let existing = self.builder.load(var_info.allocation, str_type.clone());
      let existing_alloc = self.builder.alloca(str_type.clone());
      self.builder.store(existing, existing_alloc);
      let new_alloc = self.builder.alloca(str_type.clone());
      self.builder.store(str_val, new_alloc);
      let concatenated = self.builder.concat(existing_alloc, new_alloc);
      let concat_val = self.builder.load(concatenated, str_type.clone());
      self.builder.store(concat_val, var_info.allocation);
    } else {
      // First use: allocate and store the str value
      let is_global = !self.in_user_function;
      let (allocation, global_id) = if is_global {
        let gid = self
          .builder
          .add_global(var_name, str_type.clone(), Linkage::Internal);
        let ptr = self.builder.global_addr(gid);
        (ptr, Some(gid))
      } else {
        let alloc = self.builder.alloca_named(str_type.clone(), var_name);
        (alloc, None)
      };
      self.builder.store(str_val, allocation);
      self.define_var(var_name.to_string(), allocation, str_type, global_id);
    }
  }

  /// Generate code for a stderr warn statement.
  fn generate_stderr(&mut self, warn: &StderrStmt) {
    if let Some(target) = &warn.target {
      // warn expr to var: capture output into the variable (append)
      self.generate_write_to_var(&warn.value, &target.node, warn.inline, &warn.location);
      return;
    }
    let yellow = self.a_yellow();
    let reset = self.a_reset();
    let warning_expr = Expr::StringLiteral(StringLiteral {
      parts: vec![
        StringPart::Text(Spanned {
          node: format!("{}Warning: ", yellow),
          span: warn.location.clone(),
        }),
        StringPart::EmbeddedValue(Box::new(warn.value.clone())),
        StringPart::Text(Spanned {
          node: reset.to_string(),
          span: warn.location.clone(),
        }),
      ],
      location: warn.location.clone(),
    });
    self.generate_output(&warning_expr, warn.inline, OutputDest::Stderr);
  }

  /// Generate code for a debug statement: `debug expr` → stderr with label, timestamp, and type.
  /// In release mode (is_debug = false), emits nothing.
  fn generate_debug(&mut self, debug: &DebugStmt) {
    if !self.is_debug {
      return;
    }

    // Compute the expression type and unit at compile time
    let expr_type = self.lookup_expr_type(&debug.value);
    let expr_unit = self.lookup_expr_unit(&debug.value);
    let type_label = if expr_unit.is_empty() {
      expr_type.clone()
    } else {
      format!("{} in <{}>", expr_type, expr_unit)
    };

    // Generate an ISO 8601 timestamp (compile-time — when the IR is generated)
    let timestamp = {
      use std::time::SystemTime;
      match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(dur) => {
          let secs = dur.as_secs();
          // Simple ISO 8601: YYYY-MM-DDTHH:MM:SS (UTC)
          let days = secs / 86400;
          let time = secs % 86400;
          let hours = time / 3600;
          let mins = (time % 3600) / 60;
          let secs = time % 60;
          // Compute year/month/day from Unix epoch days
          let total_days = days + 719468; // days since 0000-03-01 (always positive)
          let era = total_days / 146097;
          let doe = total_days - era * 146097;
          let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
          let y = yoe + era * 400;
          let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
          let mp = (5 * doy + 2) / 153;
          let d = doy - (153 * mp + 2) / 5 + 1;
          let m = if mp < 10 { mp + 3 } else { mp - 9 };
          let y = if m <= 2 { y + 1 } else { y };
          format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            y, m, d, hours, mins, secs
          )
        }
        Err(_) => "unknown".to_string(),
      }
    };

    // Convert the expression type string to IrType for resolved type threading.
    // "unknown" is a sentinel from lookup_expr_type — treat as no resolved type.
    let resolved_ir_type = if expr_type == "unknown" || expr_type.is_empty() {
      None
    } else {
      Some(self.type_string_to_ir_type(&expr_type))
    };

    // Check if the debug value is an enum type or enum variant type.
    // Enum types (e.g. "Color") are registered in self.enum_types.
    // Variant types (e.g. "Color__Red") contain "__" in their name.
    let is_enum_type = self.enum_types.contains(&expr_type);
    let is_enum_variant = expr_type.contains("__");

    if is_enum_type || is_enum_variant {
      // For enum types, resolved_ir_type must be Some (expr_type is a known enum name)
      let ir_type = resolved_ir_type.as_ref().unwrap_or_else(|| {
        ice!("debug: enum type '{}' not convertible to IrType", expr_type);
      });
      if is_enum_type && !is_enum_variant {
        // Enum type (e.g. "Color"): generate the value, take its address,
        // and call the dispatch function which reads the discriminant at runtime.
        let enum_val = self.generate_expr_with_resolved_type(&debug.value, Some(ir_type));
        let enum_type = ir_type.clone();
        let enum_alloc = self.builder.alloca(enum_type);
        self.builder.store(enum_val, enum_alloc);

        let dispatch_name = format!("__lale_enum_fmt_{}", expr_type);
        let dispatch_result =
          self
            .builder
            .call_named(&dispatch_name, vec![enum_alloc], IrType::struct_ref("str"));

        // Build the debug string: prefix + formatted_enum + suffix
        let prefix = self.builder.const_string(format!(
          "{}DEBUG: {} = {}",
          self.a_cyan(),
          debug.expr_text,
          self.a_bold()
        ));
        let suffix = self.builder.const_string(format!(
          "{} as {}  ({}:{}, {}){}",
          self.a_rst_bold(),
          type_label,
          debug.location.source_file,
          debug.location.line,
          timestamp,
          self.a_reset()
        ));

        let prefix_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(prefix, prefix_alloc);
        let suffix_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(suffix, suffix_alloc);

        // Concat: prefix + dispatch_result + suffix
        let result = self.builder.concat(prefix_alloc, dispatch_result);
        let result = self.builder.concat(result, suffix_alloc);

        self.generate_output_str(result, false, OutputDest::Stderr);
        return;
      }

      // Variant type (e.g. "Color__Red"): existing code path
      {
        // Generate the variant value
        let variant_val = self.generate_expr_with_resolved_type(&debug.value, Some(ir_type));

        // Determine the variant type and call its formatter
        // Variant types are named like: Shape__Circle
        let helper_name = format!("__lale_enum_fmt_{}", expr_type);
        let variant_struct_type = self.type_string_to_ir_type(&expr_type);
        let variant_alloc = self.builder.alloca(variant_struct_type);
        self.builder.store(variant_val, variant_alloc);

        let helper_result =
          self
            .builder
            .call_named(&helper_name, vec![variant_alloc], IrType::struct_ref("str"));

        // Build the debug string parts manually
        // Build the debug string: prefix + formatted_enum + suffix
        let prefix = self.builder.const_string(format!(
          "{}DEBUG: {} = {}",
          self.a_cyan(),
          debug.expr_text,
          self.a_bold()
        ));
        let suffix = self.builder.const_string(format!(
          "{} as {}  ({}:{}, {}){}",
          self.a_rst_bold(),
          type_label,
          debug.location.source_file,
          debug.location.line,
          timestamp,
          self.a_reset()
        ));

        // Store parts to stack for concat
        let prefix_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(prefix, prefix_alloc);
        let suffix_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(suffix, suffix_alloc);

        // Concat: prefix + helper_result + suffix
        let result = self.builder.concat(prefix_alloc, helper_result);
        let result = self.builder.concat(result, suffix_alloc);

        self.generate_output_str(result, false, OutputDest::Stderr);
        return;
      } // end inner: variant type path
    }

    // For composite (struct) types, generate JSON with type annotations directly
    // (bypass string embedding which omits types)
    let is_composite = self.type_layouts.contains_key(&expr_type) && expr_type != "str";

    if is_composite {
      // Generate the value and get the struct value
      let value = self.generate_expr_with_resolved_type(&debug.value, resolved_ir_type.as_ref());
      let json_result = self.generate_struct_json(value, &expr_type, true);

      // Build the debug output: prefix + json + suffix
      let prefix =
        self
          .builder
          .const_string(format!("{}DEBUG: {} = ", self.a_cyan(), debug.expr_text));
      let suffix = self.builder.const_string(format!(
        "{}  ({}:{}, {}){}",
        self.a_rst_bold(),
        debug.location.source_file,
        debug.location.line,
        timestamp,
        self.a_reset()
      ));

      let prefix_alloc = self.builder.alloca(IrType::struct_ref("str"));
      self.builder.store(prefix, prefix_alloc);
      let suffix_alloc = self.builder.alloca(IrType::struct_ref("str"));
      self.builder.store(suffix, suffix_alloc);

      let result = self.builder.concat(prefix_alloc, json_result);
      let result = self.builder.concat(result, suffix_alloc);

      self.generate_output_str(result, false, OutputDest::Stderr);
      return;
    }

    let debug_expr = Expr::StringLiteral(StringLiteral {
      parts: vec![
        StringPart::Text(Spanned {
          node: format!(
            "{}DEBUG: {} = {}",
            self.a_cyan(),
            debug.expr_text,
            self.a_bold()
          ),
          span: debug.location.clone(),
        }),
        StringPart::EmbeddedValue(Box::new(debug.value.clone())),
        StringPart::Text(Spanned {
          node: format!(
            "{} as {}  ({}:{}, {}){}",
            self.a_rst_bold(),
            type_label,
            debug.location.source_file,
            debug.location.line,
            timestamp,
            self.a_reset()
          ),
          span: debug.location.clone(),
        }),
      ],
      location: debug.location.clone(),
    });
    self.generate_output(&debug_expr, false, OutputDest::Stderr);
  }

  /// Shared implementation for stdout/stderr output.
  /// The string data and temporary newline allocas are freed after writing.
  fn generate_output(&mut self, expr: &Expr, inline: bool, dest: OutputDest) {
    let value = self.generate_expr(expr);
    let value_type = self.get_value_type(value);

    // Extract ptr and len from the str value. For in-register structs
    // (e.g. const_string, build_str_value, fn return values), use
    // extract_field to avoid a 64-byte alloca. For pointer-to-str,
    // use get_field_ptr + load.
    let (ptr, byte_len) = match &value_type {
      IrType::Ptr(_) => {
        let ptr_field = self.builder.get_field_ptr(value, "str", 0);
        let p = self.builder.load(ptr_field, IrType::raw_ptr());
        let len_field = self.builder.get_field_ptr(value, "str", 1);
        let l = self.builder.load(len_field, IrType::U64);
        (p, l)
      }
      IrType::Struct { name } if name == "str" => {
        let p = self
          .builder
          .extract_field(value, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(value, "str", 1, IrType::U64);
        (p, l)
      }
      IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64 => {
        let str_val = self.call_int_to_str(value, &value_type, false);
        let p = self
          .builder
          .extract_field(str_val, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(str_val, "str", 1, IrType::U64);
        (p, l)
      }
      IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
        let str_val = self.call_int_to_str(value, &value_type, true);
        let p = self
          .builder
          .extract_field(str_val, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(str_val, "str", 1, IrType::U64);
        (p, l)
      }
      IrType::F16 | IrType::F32 | IrType::F64 => {
        let f64_val = if value_type == IrType::F64 {
          value
        } else {
          self.builder.fp_ext(value, IrType::F64)
        };
        let str_val = self.builder.call_named(
          "__lale_f64_to_str_f64",
          vec![f64_val],
          IrType::struct_ref("str"),
        );
        let p = self
          .builder
          .extract_field(str_val, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(str_val, "str", 1, IrType::U64);
        (p, l)
      }
      IrType::Bool => {
        let str_val = self.builder.call_named(
          "__lale_bool_to_str_bool",
          vec![value],
          IrType::struct_ref("str"),
        );
        let p = self
          .builder
          .extract_field(str_val, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(str_val, "str", 1, IrType::U64);
        (p, l)
      }
      IrType::Char => {
        let i32_val = self.builder.bitcast(value, IrType::I32);
        let str_val = self.builder.call_named(
          "__lale_char_to_str_char",
          vec![i32_val],
          IrType::struct_ref("str"),
        );
        let p = self
          .builder
          .extract_field(str_val, "str", 0, IrType::raw_ptr());
        let l = self.builder.extract_field(str_val, "str", 1, IrType::U64);
        (p, l)
      }
      _ => {
        // Fallback: treat as pointer-to-str.
        let ptr_field = self.builder.get_field_ptr(value, "str", 0);
        let p = self.builder.load(ptr_field, IrType::raw_ptr());
        let len_field = self.builder.get_field_ptr(value, "str", 1);
        let l = self.builder.load(len_field, IrType::U64);
        (p, l)
      }
    };

    let fd = match dest {
      OutputDest::Stdout => self.builder.const_int(IrType::I32, 1),
      OutputDest::Stderr | OutputDest::Alert => self.builder.const_int(IrType::I32, 2),
    };

    if inline {
      let _ = self
        .builder
        .call_named("write", vec![fd, ptr, byte_len], IrType::I64);
    } else {
      let nl_buf = self.builder.alloca(IrType::U8);
      let newline_val = self.builder.const_int(IrType::U8, 10);
      self.builder.store(newline_val, nl_buf);

      let _ = self
        .builder
        .call_named("write", vec![fd, ptr, byte_len], IrType::I64);
      let one = self.builder.const_int(IrType::I64, 1);
      let _ = self
        .builder
        .call_named("write", vec![fd, nl_buf, one], IrType::I64);
    }

    // Free the string data after writing, but ONLY if the expression
    // produced freshly-allocated data (embedding, concat, fn_call,
    // type-to-str conversion). A str-typed variable reference is owned by
    // the variable and freed at scope exit — freeing it here would be a
    // double-free. A NON-str variable reference still passes through a
    // type-to-str conversion that allocates fresh data, so those must be
    // freed here too (this was the source of the noisy heap leak when
    // writing an integer/float/bool/char variable).
    let needs_free = match &value_type {
      // Type-to-str conversions always produce freshly-allocated data.
      IrType::I8
      | IrType::I16
      | IrType::I32
      | IrType::I64
      | IrType::U8
      | IrType::U16
      | IrType::U32
      | IrType::U64
      | IrType::F16
      | IrType::F32
      | IrType::F64
      | IrType::Bool
      | IrType::Char => true,
      // str-typed values: a plain variable reference is owned elsewhere.
      _ => !matches!(expr, Expr::Identifier(_)),
    };
    if needs_free {
      self
        .builder
        .call_void_named("__lale_free_pointer", vec![ptr]);
    }
  }

  /// Output a pre-built string (str value) to a destination.
  /// Used by enum/composite debug formatting to bypass expression generation.
  fn generate_output_str(&mut self, str_val: ValueId, inline: bool, dest: OutputDest) {
    // Extract ptr and len from the str value. Use extract_field (not
    // get_field_ptr) so an in-register str value doesn't allocate a
    // temporary slot that would leak.
    let ptr = self
      .builder
      .extract_field(str_val, "str", 0, IrType::raw_ptr());
    let byte_len = self.builder.extract_field(str_val, "str", 1, IrType::U64);

    let fd = match dest {
      OutputDest::Stdout => self.builder.const_int(IrType::I32, 1),
      OutputDest::Stderr | OutputDest::Alert => self.builder.const_int(IrType::I32, 2),
    };

    if inline {
      let _ = self
        .builder
        .call_named("write", vec![fd, ptr, byte_len], IrType::I64);
    } else {
      let nl_buf = self.builder.alloca(IrType::U8);
      let newline_val = self.builder.const_int(IrType::U8, 10);
      self.builder.store(newline_val, nl_buf);

      let _ = self
        .builder
        .call_named("write", vec![fd, ptr, byte_len], IrType::I64);
      let one = self.builder.const_int(IrType::I64, 1);
      let _ = self
        .builder
        .call_named("write", vec![fd, nl_buf, one], IrType::I64);
    }

    // Pre-built strings from generate_output_str are always owned (concat results).
    self
      .builder
      .call_void_named("__lale_free_pointer", vec![ptr]);
  }

  /// Generate code for a stdin read statement.
  /// The target variable is allocated and initialized by a synthetic VarDefStmt
  /// injected by the AST builder.
  fn generate_stdin(&mut self, stdin: &StdinStmt) -> CompileResult<()> {
    let var_name = stdin.target.node.join(".");

    let var_info = self
      .lookup_var(&var_name)
      .ok_or_else(|| IrGenError::InvalidBuilderState {
        reason: format!(
          "read target '{}' not found in scope. Synthetic VarDefStmt should have registered it.",
          var_name
        ),
      })?;

    let var_type = var_info.var_type.clone();

    // `read` always produces a str — verify the target is str
    if !matches!(&var_type, IrType::Struct { name } if name == "str") {
      return Err(
        IrGenError::InvalidBuilderState {
          reason: format!(
            "read requires a str variable, but '{}' has type {:?}",
            var_name, var_type
          ),
        }
        .into(),
      );
    }

    // Call __lale_read_line() -> str
    let result = self
      .builder
      .call_named("__lale_read_line", vec![], IrType::struct_ref("str"));

    // Store the result in the target variable
    let ptr = self.var_ptr(&var_info);
    self.builder.store(result, ptr);

    Ok(())
  }

  /// Generate code for an add error statement: `add error expr`.
  fn generate_add_error(&mut self, add_error: &AddErrorStmt) {
    let value = self.generate_expr(&add_error.value);
    self.builder.push_error(value);
  }

  /// Generate code to drain the error stack.
  fn generate_drain_errors(&mut self, to_stderr: bool) {
    self.builder.drain_errors(to_stderr, None);
  }

  /// Generate code to drain the error stack with alert-style red "Error:" prefix.
  fn generate_drain_errors_alert(&mut self) {
    let error_prefix = format!("{}Error: {}", self.a_red(), self.a_reset());
    self.builder.drain_errors(true, Some(error_prefix));
  }

  /// Generate code for an alert output statement: `alert expr` → stderr with red "Error:" prefix.
  /// The ANSI coloring and "Error:" prefix are handled by the stdlib's __lale_alert hook.
  fn generate_alert(&mut self, alert: &AlertStmt) {
    self.generate_output(&alert.value, alert.inline, OutputDest::Alert);
  }

  /// Generate code for an expression without a resolved type.
  /// Thin panic-wrapper around `try_generate_expr`.
  /// Returns the IR ValueId representing the result.
  fn generate_expr(&mut self, expr: &Expr) -> ValueId {
    Self::unwrap_or_panic(self.try_generate_expr(expr), "generate_expr")
  }

  /// Generate code for an expression without a resolved type, returning Result on failure.
  fn try_generate_expr(&mut self, expr: &Expr) -> CompileResult<ValueId> {
    self.try_generate_expr_with_resolved_type(expr, None)
  }

  /// Generate code for an expression, optionally guided by a type hint.
  /// Thin panic-wrapper around `try_generate_expr_with_resolved_type`.
  /// See that function for the meaning of `type_hint`.
  fn generate_expr_with_resolved_type(
    &mut self,
    expr: &Expr,
    type_hint: Option<&IrType>,
  ) -> ValueId {
    Self::unwrap_or_panic(
      self.try_generate_expr_with_resolved_type(expr, type_hint),
      "generate_expr_with_resolved_type",
    )
  }

  /// Generate code for an expression, optionally guided by a type hint.
  ///
  /// `type_hint` is only needed for bare numeric literals (IntLiteral, UintLiteral,
  /// FloatLiteral) which have no self-evident bit width. It must be `Some` for these —
  /// passing `None` for a bare literal causes `ice!()`. For all other expressions
  /// (identifiers, function calls, member access, binary ops, etc.), the type is
  /// determined by what the expression *is*, not by context, and `type_hint` is unused.
  fn try_generate_expr_with_resolved_type(
    &mut self,
    expr: &Expr,
    type_hint: Option<&IrType>,
  ) -> CompileResult<ValueId> {
    Ok(match expr {
      Expr::Binary(binary) => {
        // For binary operations, the left operand determines the type hint
        // for the right operand (e.g., `j > 2` where j is i32 → 2 should be i32)
        let left = self.try_generate_expr_with_resolved_type(&binary.left, type_hint)?;
        let left_type = self.get_value_type(left);
        let right = self.try_generate_expr_with_resolved_type(&binary.right, Some(&left_type))?;
        self.generate_binary_op(left, right, &binary.operator, &binary.location)
      }

      Expr::Unary(unary) => self.generate_unary_expr(unary, type_hint),

      Expr::Conversion(conv) => {
        // Convert the target type first so we can pass it as context to the
        // inner expression. This ensures that `10 as u64` generates `const u64`
        // instead of `const i64`, avoiding mixed-type arithmetic bugs.
        let target_ir_type = self.type_name_to_ir_type(&conv.target_type);
        let src_value =
          self.try_generate_expr_with_resolved_type(&conv.operand, Some(&target_ir_type))?;
        let source_ir_type = self.get_value_type(src_value);
        self.generate_type_conversion(src_value, source_ir_type, target_ir_type)
      }
      Expr::Identifier(ident) => {
        // Look up the variable in our scope stack.
        // For multi-segment paths, if the first segment is an enum type,
        // use only the last segment as the variable/function name.
        let var_name = if ident.path.len() > 1 && self.enum_types.contains(&ident.path[0]) {
          ident.path.last().cloned().unwrap_or_default()
        } else {
          ident.path.join(".")
        };
        if let Some(var_info) = self.lookup_var(&var_name) {
          // Get the pointer to the variable (GlobalAddr for globals, alloca for locals)
          let ptr = self.var_ptr(&var_info);
          // For array types, return the pointer directly (don't load)
          // For other types, load the value
          if matches!(var_info.var_type, IrType::Array { .. }) {
            ptr
          } else {
            self.builder.load(ptr, var_info.var_type)
          }
        } else if self.is_zero_arg_function(&var_name) {
          // Zero-argument function call (e.g., enum variant constructor "Red")
          self.call_zero_arg_function(&var_name)
        } else {
          return Err(IrGenError::UndefinedVariable { name: var_name }.into());
        }
      }
      Expr::IntLiteral(int_lit) => self.generate_int_literal(int_lit, type_hint),
      Expr::UintLiteral(uint_lit) => self.generate_uint_literal(uint_lit, type_hint),
      Expr::FloatLiteral(float_lit) => self.generate_float_literal(float_lit, type_hint),
      Expr::HexLiteral(hex_lit) => self.generate_hex_literal(hex_lit),
      Expr::CharLiteral(char_lit) => self.generate_char_literal(char_lit),
      Expr::BoolLiteral(bool_lit) => self.generate_bool_literal(bool_lit),
      Expr::StringLiteral(str_lit) => self.generate_string_literal(str_lit),
      Expr::ArrayLiteral(arr_lit) => {
        // Generate array literal initialization
        self.generate_array_literal(arr_lit, type_hint)
      }
      Expr::FnCallExpr(fn_call) => self.generate_fn_call_expr(fn_call, type_hint),
      Expr::MemberAccess(member_access) => {
        // Generate code for the object expression
        let object_val = self.try_generate_expr(&member_access.object)?;
        let member_name = &member_access.member.node;

        // Try to determine the type and field index from type layouts
        let mut field_index = 0;
        let mut field_type = IrType::I64;
        let mut type_name = String::new();

        // Look up the field in all type layouts
        for (layout_type_name, layout) in self.type_layouts.iter() {
          if let Some(field_idx) = layout.field_names.iter().position(|f| f == member_name) {
            field_index = field_idx;
            field_type = layout.field_types[field_idx].clone();
            type_name = layout_type_name.clone();
            break;
          }
        }

        // The constructor returns a pointer to the struct
        // Use ExtractField instruction which handles both struct values and pointers
        self
          .builder
          .extract_field(object_val, &type_name, field_index as u32, field_type)
      }
      Expr::ArrayIndex(arr_idx) => {
        // Generate code for the array expression
        let array_val = self.try_generate_expr(&arr_idx.array)?;

        // Check if there are any indices
        if arr_idx.indices.is_empty() {
          return Ok(array_val); // Return array itself if no index
        }

        // Get array name to look up type information
        let array_name = match &*arr_idx.array {
          Expr::Identifier(id) => id.name(),
          _ => "",
        };

        // Get the element type from the semantic analyzer
        let element_type = self.get_array_element_type(array_name);

        // Array is represented as a pointer; compute offset and load
        // Lale uses 1-based indexing like Julia/MATLAB/Fortran
        // User writes arr[1] to access first element
        // Memory access is 0-based, so we subtract 1 from each index

        let element_size = self.ir_type_size(&element_type);

        // Get array dimensions for multi-dimensional array support
        let dimensions = self.get_array_dimensions(array_name);

        // Compute total offset for multi-dimensional arrays
        let mut total_offset = self.builder.const_int(IrType::I64, 0);

        // Collect the indices for bounds checking
        let mut indices = Vec::new();
        for (dim_idx, index_expr) in arr_idx.indices.iter().enumerate() {
          let index_val =
            self.try_generate_expr_with_resolved_type(index_expr, Some(&IrType::I64))?;
          indices.push(index_val);

          // Convert 1-based index to 0-based: subtract 1
          // Get the index type and perform subtraction in that type
          let idx_type = self.get_value_type(index_val);
          let one_typed = match idx_type.clone() {
            IrType::I32 => self.builder.const_int(IrType::I32, 1),
            IrType::I64 => self.builder.const_int(IrType::I64, 1),
            IrType::U32 => self.builder.const_uint(IrType::U32, 1),
            IrType::U64 => self.builder.const_uint(IrType::U64, 1),
            _ => self.builder.const_int(IrType::I64, 1),
          };

          let zero_based_index = self.builder.sub(index_val, one_typed, idx_type.clone());

          // Compute stride for this dimension
          // For dimension i, stride = product of dimensions[i+1..n]
          let mut stride = 1i64;
          for dim in dimensions.iter().skip(dim_idx + 1) {
            stride *= dim;
          }

          // Offset contribution from this dimension: index * stride
          // Convert zero_based_index to i64 for offset computation
          // Properly extend based on signedness
          let zero_based_i64 = match idx_type.clone() {
            IrType::I64 => zero_based_index,
            IrType::U64 => self.builder.bitcast(zero_based_index, IrType::I64),
            IrType::I32 => {
              // Sign extend i32 to i64

              self.builder.sext(zero_based_index, IrType::I64)
            }
            IrType::U32 => {
              // Zero extend u32 to i64 (bitcast to i32 first, then zext)
              let as_i32 = self.builder.bitcast(zero_based_index, IrType::I32);
              self.builder.zext(as_i32, IrType::I64)
            }
            IrType::I16 => self.builder.sext(zero_based_index, IrType::I64),
            IrType::U16 => {
              let as_i16 = self.builder.bitcast(zero_based_index, IrType::I16);
              self.builder.zext(as_i16, IrType::I64)
            }
            IrType::I8 => self.builder.sext(zero_based_index, IrType::I64),
            IrType::U8 => {
              let as_i8 = self.builder.bitcast(zero_based_index, IrType::I8);
              self.builder.zext(as_i8, IrType::I64)
            }
            _ => {
              ice!(
                "Unhandled index type {:?} in array offset computation — semantic analysis should have validated index types",
                idx_type
              );
            }
          };

          let stride_const = self.builder.const_int(IrType::I64, stride);
          let dim_offset = self.builder.mul(zero_based_i64, stride_const, IrType::I64);

          // Add to total offset
          total_offset = self.builder.add(total_offset, dim_offset, IrType::I64);
        }

        // Insert bounds checks for each dimension
        for (dim_idx, index_val) in indices.iter().enumerate() {
          if dim_idx < dimensions.len() {
            let dim_size = self.builder.const_int(IrType::I64, dimensions[dim_idx]);
            let file_name = arr_idx.location.source_file.clone();
            let line_num = arr_idx.location.line as i64;
            let column_num = arr_idx.location.col as i64;

            self.builder.bounds_check(
              *index_val,
              dim_size,
              "array index out of bounds",
              file_name,
              line_num,
              column_num,
            );
          }
        }

        // Multiply total offset by element size
        let size_const = self.builder.const_int(IrType::I64, element_size);
        let offset = self.builder.mul(total_offset, size_const, IrType::I64);

        // Add offset to array pointer: array + offset
        // Result is a pointer, not I64
        let addr = self.builder.add(array_val, offset, IrType::raw_ptr());

        // Load from computed address
        self.builder.load(addr, element_type)
      }
      Expr::CompilerConst(cc) => {
        match cc.kind {
          CompilerConstKind::Function => {
            // Semantic analysis should have rejected #function_name at global scope.
            let fn_name = self.current_function.clone().unwrap_or_else(|| {
              ice!("CompilerConst::Function used outside any function — semantic analysis should have rejected this");
            });
            self.builder.const_string(&fn_name)
          }
          CompilerConstKind::SourceLine => {
            self.builder.const_int(IrType::I32, cc.location.line as i64)
          }
          CompilerConstKind::SourceFile => self.builder.const_string(&cc.location.source_file),
          CompilerConstKind::Main => {
            // For now, always return false (0)
            self.builder.const_bool(false)
          }
          CompilerConstKind::CompileTime => {
            // Generate compilation timestamp (using build-time environment variable)
            // For runtime compile time, use the line number as a placeholder
            let timestamp = format!("line {}", cc.location.line);
            self.builder.const_string(&timestamp)
          }
          CompilerConstKind::CompilerVersion => {
            // Generate compiler version string
            self.builder.const_string(env!("CARGO_PKG_VERSION"))
          }
          CompilerConstKind::Posix => {
            // Generate a boolean constant for the current OS
            #[cfg(target_os = "windows")]
            let is_posix = false;
            #[cfg(not(target_os = "windows"))]
            let is_posix = true;
            self.builder.const_bool(is_posix)
          }
          CompilerConstKind::Windows => {
            // Generate a boolean constant for the current OS
            #[cfg(target_os = "windows")]
            let is_windows = true;
            #[cfg(not(target_os = "windows"))]
            let is_windows = false;
            self.builder.const_bool(is_windows)
          }
          CompilerConstKind::Debug => self.builder.const_bool(self.is_debug),
          CompilerConstKind::Mode => {
            let mode = if self.test_mode { "test" } else { "run" };
            self.builder.const_string(mode)
          }
        }
      }
      Expr::Allocate(allocate) => self.generate_allocate(allocate),
      Expr::Grouped(expr) => self.try_generate_expr(expr)?,
      Expr::NothingExpr => {
        // nothing in expression context: absent optional
        let inner_type = type_hint
          .and_then(|t| match t {
            IrType::Optional(inner) => Some(inner.as_ref().clone()),
            _ => None,
          })
          .or_else(|| {
            // Fallback: infer from current function's return type (e.g., `return nothing` in T? fn)
            self.current_return_type.as_ref().and_then(|t| match t {
              IrType::Optional(inner) => Some(inner.as_ref().clone()),
              _ => None,
            })
          })
          .unwrap_or_else(|| {
            ice!(
              "'nothing' used without optional type context and current function doesn't return T?"
            )
          });
        self.build_optional_none(&inner_type)
      }
      Expr::HasValue(inner) => {
        // has value: extract is_present tag, compare with true
        let val = self.try_generate_expr(inner)?;
        let inner_type = self.get_value_type(val);
        let real_inner = match &inner_type {
          IrType::Optional(ty) => ty.as_ref().clone(),
          _ => return Ok(self.builder.const_bool(true)),
        };
        let tag = self.extract_optional_tag(val, &real_inner);
        let false_val = self.builder.const_bool(false);
        self.builder.ne(tag, false_val)
      }
      Expr::HasNoValue(inner) => {
        let val = self.try_generate_expr(inner)?;
        let inner_type = self.get_value_type(val);
        let real_inner = match &inner_type {
          IrType::Optional(ty) => ty.as_ref().clone(),
          _ => return Ok(self.builder.const_bool(false)),
        };
        let tag = self.extract_optional_tag(val, &real_inner);
        let false_val = self.builder.const_bool(false);
        self.builder.eq(tag, false_val)
      }
      Expr::HasErrors => {
        let count = self.builder.error_count();
        let zero = self.builder.const_int(IrType::I64, 0);
        self.builder.ne(count, zero)
      }
      Expr::LastError => self.builder.pop_error(),
      Expr::TryPropagate(inner) => {
        // expr? — if absent, propagate nothing up (return nothing / exit)
        let val = self.try_generate_expr(inner)?;
        let opt_type = self.get_value_type(val);
        if let IrType::Optional(inner_ty) = &opt_type {
          let struct_name = Self::optional_struct_name(inner_ty);
          self.ensure_optional_struct(inner_ty);
          // Extract is_present tag
          let tag_val = self
            .builder
            .extract_field(val, &struct_name, 0, IrType::Bool);
          let false_val = self.builder.const_bool(false);
          let is_absent = self.builder.eq(tag_val, false_val);

          // Create blocks for propagation
          let propagate_block = self.builder.create_block("try.propagate");
          let continue_block = self.builder.create_block("try.continue");
          let merge_block = self.builder.create_block("try.merge");

          self
            .builder
            .cond_br(is_absent, propagate_block, continue_block);

          // Propagate block: handle the absent case
          self.builder.position_at(propagate_block);
          if self.in_user_function {
            // Inside a user function: return nothing
            if let Some(ret_ty) = &self.current_return_type
              && *ret_ty == IrType::Void
            {
              self.builder.ret_void();
            } else {
              // Build a none optional value and return it
              let none = self.build_optional_none(inner_ty);
              self.builder.ret(none);
            }
          } else {
            // Top level: print error and exit
            let func_name = if let Expr::FnCallExpr(fc) = inner.as_ref() {
              fc.target.node.join(" -> ")
            } else if let Expr::Identifier(id) = inner.as_ref() {
              id.name().to_string()
            } else {
              "<expression>".to_string()
            };
            let release_msg = format!("error: function '{}' returned nothing\n", func_name);
            let debug_msg = format!("propagated nothing from '{}'", func_name);
            if self.is_debug {
              let msg_str = self.builder.const_string(&debug_msg);
              let ptr = self.builder.str_get_ptr(msg_str);
              let msg_len = self.builder.str_get_byte_len(msg_str);
              let fd = self.builder.const_int(IrType::I32, 2);
              let _ = self
                .builder
                .call_named("write", vec![fd, ptr, msg_len], IrType::I64);
            } else {
              let msg_str = self.builder.const_string(&release_msg);
              let ptr = self.builder.str_get_ptr(msg_str);
              let msg_len = self.builder.str_get_byte_len(msg_str);
              let fd = self.builder.const_int(IrType::I32, 2);
              let _ = self
                .builder
                .call_named("write", vec![fd, ptr, msg_len], IrType::I64);
            }
            let code = self.builder.const_int(IrType::I32, 1);
            self.builder.call_void_named("__lale_exit", vec![code]);
          }

          // Continue block: unwrap the value
          self.builder.position_at(continue_block);
          let unwrapped =
            self
              .builder
              .extract_field(val, &struct_name, 1, inner_ty.as_ref().clone());
          self.builder.br(merge_block);

          self.builder.position_at(merge_block);
          unwrapped
        } else {
          // Not optional — pass through (semantic analysis should catch this)
          val
        }
      }
    })
  }

  /// Emit integer addition, choosing checked arithmetic when overflow checking
  /// is enabled and the result type is an integer. Floating-point and pointer
  /// addition always use the plain (wrapping) IR instruction.
  fn emit_int_add(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    location: &SourceLocation,
  ) -> ValueId {
    if self.checked_overflow && ty.is_integer() {
      self.builder.checked_add(
        lhs,
        rhs,
        ty,
        location.source_file.as_str(),
        location.line as i64,
        location.col as i64,
      )
    } else {
      self.builder.add(lhs, rhs, ty)
    }
  }

  /// Emit integer subtraction, choosing checked arithmetic when overflow
  /// checking is enabled and the result type is an integer.
  fn emit_int_sub(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    location: &SourceLocation,
  ) -> ValueId {
    if self.checked_overflow && ty.is_integer() {
      self.builder.checked_sub(
        lhs,
        rhs,
        ty,
        location.source_file.as_str(),
        location.line as i64,
        location.col as i64,
      )
    } else {
      self.builder.sub(lhs, rhs, ty)
    }
  }

  /// Emit integer multiplication, choosing checked arithmetic when overflow
  /// checking is enabled and the result type is an integer.
  fn emit_int_mul(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    location: &SourceLocation,
  ) -> ValueId {
    if self.checked_overflow && ty.is_integer() {
      self.builder.checked_mul(
        lhs,
        rhs,
        ty,
        location.source_file.as_str(),
        location.line as i64,
        location.col as i64,
      )
    } else {
      self.builder.mul(lhs, rhs, ty)
    }
  }

  /// Emit integer negation, choosing checked arithmetic when overflow checking
  /// is enabled and the operand type is an integer.
  fn emit_int_neg(&mut self, src: ValueId, ty: IrType, location: &SourceLocation) -> ValueId {
    if self.checked_overflow && ty.is_integer() {
      self.builder.checked_neg(
        src,
        ty,
        location.source_file.as_str(),
        location.line as i64,
        location.col as i64,
      )
    } else {
      self.builder.neg(src, ty)
    }
  }

  /// Generate code for a binary operation.
  fn generate_binary_op(
    &mut self,
    left: ValueId,
    right: ValueId,
    op: &BinaryOp,
    location: &SourceLocation,
  ) -> ValueId {
    // Get the types of both operands
    let left_type = self.get_value_type(left);
    let right_type = self.get_value_type(right);

    // Special case: pointer - pointer returns i64 (signed difference)
    if matches!(op, BinaryOp::Sub)
      && matches!(left_type, IrType::Ptr(_))
      && matches!(right_type, IrType::Ptr(_))
    {
      return self.builder.ptr_diff(left, right);
    }

    // Use the left operand's type as the result type (override for Dot/Cross)
    let result_type = match op {
      BinaryOp::Dot => match &left_type {
        IrType::Vec2(inner) | IrType::Vec3(inner) | IrType::Vec4(inner) => *inner.clone(),
        _ => left_type.clone(),
      },
      _ => left_type.clone(),
    };

    // Emit zero-check before division or modulo
    if matches!(op, BinaryOp::Div | BinaryOp::Mod) {
      let msg = if matches!(op, BinaryOp::Div) {
        "division by zero"
      } else {
        "modulo by zero"
      };
      self.builder.zero_check(
        right,
        msg,
        location.source_file.as_str(),
        location.line as i64,
        location.col as i64,
      );
    }

    match op {
      BinaryOp::Add => self.emit_int_add(left, right, result_type, location),
      BinaryOp::Sub => self.emit_int_sub(left, right, result_type, location),
      BinaryOp::Mul => self.emit_int_mul(left, right, result_type, location),
      BinaryOp::Div => self.builder.div(left, right, result_type),
      BinaryOp::Mod => self.builder.rem(left, right, result_type),
      BinaryOp::Pow => self.builder.pow(left, right, result_type),
      BinaryOp::Eq => self.builder.eq(left, right),
      BinaryOp::NotEq => self.builder.ne(left, right),
      BinaryOp::Lt => self.builder.lt(left, right),
      BinaryOp::LtEq => self.builder.le(left, right),
      BinaryOp::Gt => self.builder.gt(left, right),
      BinaryOp::GtEq => self.builder.ge(left, right),
      BinaryOp::And => self.builder.and(left, right),
      BinaryOp::Or => self.builder.or(left, right),
      BinaryOp::Xor => self.builder.xor(left, right),
      BinaryOp::BitAnd => self.builder.bit_and(left, right, result_type),
      BinaryOp::BitOr => self.builder.bit_or(left, right, result_type),
      BinaryOp::BitXor => self.builder.bit_xor(left, right, result_type),
      BinaryOp::UnsignedLeftShift => self.builder.shl(left, right, result_type),
      BinaryOp::UnsignedRightShift => self.builder.ushr(left, right, result_type),
      BinaryOp::SignedLeftShift => self.builder.shl(left, right, result_type),
      BinaryOp::SignedRightShift => self.builder.shr(left, right, result_type),
      BinaryOp::Append => {
        // String/Array append: a ~ b
        // Generate a Concat IR instruction handled by the interpreter directly
        // (lines 801-950 in interpreter.rs). Future backends can implement their own
        // optimal concatenation strategy.
        self.builder.concat(left, right)
      }
      BinaryOp::Dot => {
        // Scalar integer dot products are multiplication; route them through the
        // checked/wrapping integer path so they obey the overflow policy. Vector
        // and float dot products keep the dedicated Dot instruction.
        if left_type.is_integer() && right_type.is_integer() {
          self.emit_int_mul(left, right, result_type, location)
        } else {
          self.builder.dot(left, right, result_type)
        }
      }
      BinaryOp::Cross => {
        // Cross product — emits a dedicated Cross IR instruction.
        // The interpreter performs component-wise computation for vec3.
        self.builder.cross(left, right, result_type)
      }
    }
  }

  /// Generate code for a unary expression, handling type queries and pointer operations.
  fn generate_unary_expr(&mut self, unary: &UnaryExpr, resolved_type: Option<&IrType>) -> ValueId {
    match &unary.operator {
      UnaryOp::TypeOf => {
        // Type query: type of x
        // Returns a string representation of the type
        let type_str = self.lookup_expr_type(&unary.operand);
        self.builder.const_string(&type_str)
      }
      UnaryOp::SizeOf => {
        // Size query: size of x in bytes
        // Determine the type and return its size
        let size = {
          let type_str = self.lookup_expr_type(&unary.operand);
          self.compute_type_size(&type_str)
        };
        self.builder.const_uint(IrType::U32, size)
      }
      UnaryOp::UnitOf => {
        // Unit query: unit of x
        // Returns the physical unit as a string
        let unit_str = if let Some(manager) = &self.symbol_manager {
          // Try to extract variable name from operand
          if let Expr::Identifier(id) = &*unary.operand {
            if let Ok(Some(symbol)) = manager.lookup_var_symbol(id.name()) {
              symbol.physical_unit.clone().unwrap_or_else(|| {
                eprintln!(
                  "WARNING: `unit of` operator on variable '{}' with no physical unit \
                     — defaulting to empty string",
                  id.name()
                );
                "".to_string()
              })
            } else {
              ice!(
                "unit of operator applied to undefined variable '{}' — semantic analysis should have resolved all identifiers",
                id.name()
              );
            }
          } else {
            ice!(
              "unit of operator on non-identifier expression — semantic analysis should have validated operand types"
            );
          }
        } else {
          ice!(
            "unit of operator requires symbol manager context — invariant violation in IR generation"
          );
        };
        self.builder.const_string(&unit_str)
      }
      UnaryOp::PointerTo => {
        // Pointer-to: &x
        // Returns the address of the operand (must be lvalue)
        self.generate_pointer_to(&unary.operand)
      }
      UnaryOp::ValueAt => {
        // Value-at (dereference): *x
        // Dereferences a pointer, determining target type.
        //
        // Special case: `value at p unsafe cast` has AST structure
        // ValueAt(UnsafeCast(Identifier)). The `UnsafeCast` inside means
        // the load must use the pointer's ACTUAL target type, then bitcast.
        // Otherwise we'd load e.g. f64 memory as i64 (wrong representation).
        let is_unsafe_cast = matches!(&*unary.operand, Expr::Unary(inner)
          if matches!(inner.operator, UnaryOp::UnsafeCast));
        if is_unsafe_cast {
          let operand = self.generate_expr(&unary.operand);
          // Determine the actual pointer target type.
          // Priority: 1) pointer's IR type (e.g., Ptr(F64) → F64),
          // 2) symbol database pointer tracking, 3) fallback to resolved_type.
          let actual_target = {
            let ptr_type = self.get_value_type(operand);
            if let IrType::Ptr(inner) = &ptr_type
              && !matches!(inner.as_ref(), IrType::Void)
            {
              Some(inner.as_ref().clone())
            } else if let Some(manager) = &self.symbol_manager {
              Self::try_resolve_value_at_type(manager, &unary.operand, self)
            } else {
              None
            }
          };
          let load_type = actual_target
            .clone()
            .unwrap_or_else(|| resolved_type.cloned().unwrap_or(IrType::I64));
          let target_type = resolved_type.cloned().unwrap_or_else(|| load_type.clone());
          // Load with the actual type, then bitcast to the target type if different
          let ptr_type = self.get_value_type(operand);
          let load_val = if let IrType::Ptr(_) = ptr_type {
            self.builder.load(operand, load_type.clone())
          } else {
            operand
          };
          if load_type != target_type {
            self.builder.bitcast(load_val, target_type)
          } else {
            load_val
          }
        } else {
          let operand = self.generate_expr(&unary.operand);
          self.generate_value_at(operand, &unary.operand, resolved_type)
        }
      }
      UnaryOp::UnsafeCast => {
        // Unsafe bit reinterpretation. This arm is only reached for the
        // standalone `unsafe cast <expr>` form (without `value at`).
        // The more common `value at p unsafe cast` is handled by ValueAt above.
        let operand_val = self.generate_expr(&unary.operand);
        if let Some(target_ty) = resolved_type {
          let src_ty = self.get_value_type(operand_val);
          if src_ty != *target_ty {
            return self.builder.bitcast(operand_val, target_ty.clone());
          }
        }
        operand_val
      }
      UnaryOp::ValueOf => {
        // value of expr: unwrap an optional value
        let operand = self.generate_expr(&unary.operand);
        let opt_type = self.get_value_type(operand);
        if let IrType::Optional(inner) = &opt_type {
          let struct_name = Self::optional_struct_name(inner);
          self.ensure_optional_struct(inner);
          let msg = format!(
            "unwrapped absent optional of type {}",
            self.ir_type_to_type_name_string(inner)
          );
          self.builder.unwrap_optional(
            operand,
            &struct_name,
            inner.as_ref().clone(),
            msg,
            &unary.location.source_file,
            unary.location.line as i64,
            unary.location.col as i64,
          )
        } else {
          // Not an optional — pass through (semantic analysis should catch this)
          operand
        }
      }
      // For other unary operators, generate operand and apply operation
      op => {
        let operand = self.generate_expr(&unary.operand);
        self.generate_unary_op(operand, op, &unary.location)
      }
    }
  }

  /// Generate code for a unary operation (basic arithmetic/logic ops).
  fn generate_unary_op(
    &mut self,
    operand: ValueId,
    op: &UnaryOp,
    location: &SourceLocation,
  ) -> ValueId {
    let operand_type = self.get_value_type(operand);

    match op {
      UnaryOp::Neg => self.emit_int_neg(operand, operand_type, location),
      UnaryOp::Not => self.builder.not(operand),
      UnaryOp::Invert => self.builder.bit_not(operand, operand_type),
      UnaryOp::TypeOf
      | UnaryOp::SizeOf
      | UnaryOp::UnitOf
      | UnaryOp::PointerTo
      | UnaryOp::ValueAt
      | UnaryOp::UnsafeCast
      | UnaryOp::ValueOf => {
        // These should be handled by generate_unary_expr
        unreachable!("Type query/pointer ops should be handled by generate_unary_expr")
      }
    }
  }

  /// Compute the size of a type given its type string representation.
  fn compute_type_size(&self, type_str: &str) -> u64 {
    let base_type = self.extract_base_type(type_str);

    match base_type {
      // Signed integers
      "i8" => 1,
      "i16" => 2,
      "i32" => 4,
      "i64" => 8,
      // Unsigned integers
      "u8" | "byte" => 1,
      "u16" => 2,
      "u32" => 4,
      "u64" => 8,
      // Floating point
      "f16" => 2,
      "f32" => 4,
      "f64" => 8,
      // Other types
      "bool" => 1,
      "char" => 4,
      "str" => 24, // String descriptor (ptr + len + capacity on 64-bit)
      "pointer" => 8,
      _ => 8, // Default for unknown types
    }
  }

  /// Compute the size of an IR type.
  fn compute_ir_type_size(&self, ty: &IrType) -> u64 {
    match ty {
      IrType::Void => 0,
      IrType::I8 | IrType::U8 => 1,
      IrType::I16 | IrType::U16 | IrType::F16 => 2,
      IrType::I32 | IrType::U32 | IrType::F32 => 4,
      IrType::I64 | IrType::U64 | IrType::F64 => 8,
      IrType::Bool => 1,
      IrType::Char => 4,
      IrType::Ptr(_) => 8,
      IrType::Array { element, size } => self.compute_ir_type_size(element) * size,
      IrType::Struct { .. } => 8, // Placeholder; actual size requires layout info
      IrType::Optional(_) => 16,  // 8 bytes tag + 8 bytes value
      IrType::Vec2(inner) => self.compute_ir_type_size(inner) * 2,
      IrType::Vec3(inner) => self.compute_ir_type_size(inner) * 3,
      IrType::Vec4(inner) => self.compute_ir_type_size(inner) * 4,
    }
  }

  /// Generate code for pointer-to (&x) operation.
  fn generate_pointer_to(&mut self, operand_expr: &Expr) -> ValueId {
    match operand_expr {
      Expr::Identifier(id) => {
        // Variable: return its allocation (address)
        let var_name = id.name();
        if let Some(var_info) = self.lookup_var(var_name) {
          self.var_ptr(&var_info)
        } else {
          // Undefined variable - return null pointer
          self.builder.const_int(IrType::raw_ptr(), 0)
        }
      }
      Expr::ArrayIndex(arr_idx) => {
        // Array element: compute and return address
        let array_val = self.generate_expr(&arr_idx.array);

        // Get array name for type information
        let array_name = match &*arr_idx.array {
          Expr::Identifier(id) => id.name(),
          _ => "",
        };

        // Get element type
        let element_type = self.get_array_element_type(array_name);

        // Get dimensions
        let dimensions = self.get_array_dimensions(array_name);

        // Element size
        let element_size = self.ir_type_size(&element_type);

        // Compute offset (same logic as array access, but return address instead of loading)
        let mut total_offset = self.builder.const_int(IrType::I64, 0);

        for (dim_idx, index_expr) in arr_idx.indices.iter().enumerate() {
          let index_val = self.generate_expr_with_resolved_type(index_expr, Some(&IrType::I64));
          let idx_type = self.get_value_type(index_val);

          // Convert 1-based to 0-based
          let one_typed = match idx_type.clone() {
            IrType::I32 => self.builder.const_int(IrType::I32, 1),
            IrType::I64 => self.builder.const_int(IrType::I64, 1),
            IrType::U32 => self.builder.const_uint(IrType::U32, 1),
            IrType::U64 => self.builder.const_uint(IrType::U64, 1),
            _ => self.builder.const_int(IrType::I64, 1),
          };

          let zero_based_index = self.builder.sub(index_val, one_typed, idx_type.clone());

          // Compute stride
          let mut stride = 1i64;
          for dim in dimensions.iter().skip(dim_idx + 1) {
            stride *= dim;
          }

          // Convert to i64 for offset computation
          let zero_based_i64 = if idx_type.clone() == IrType::I64 {
            zero_based_index
          } else {
            self.builder.bitcast(zero_based_index, IrType::I64)
          };

          let stride_const = self.builder.const_int(IrType::I64, stride);
          let dim_offset = self.builder.mul(zero_based_i64, stride_const, IrType::I64);

          total_offset = self.builder.add(total_offset, dim_offset, IrType::I64);
        }

        let size_const = self.builder.const_int(IrType::I64, element_size);
        let offset = self.builder.mul(total_offset, size_const, IrType::I64);

        // Return pointer to element
        self.builder.add(array_val, offset, IrType::raw_ptr())
      }
      Expr::MemberAccess(member_access) => {
        unimplemented!(
          "generate_pointer_to: struct field '{}' offset computation not yet implemented",
          member_access.member.node
        );
      }
      _ => {
        // Non-lvalue: semantic analysis should have rejected pointer-to on non-lvalue expressions
        ice!(
          "generate_pointer_to: called on non-lvalue expression — semantic analysis should have rejected this"
        );
      }
    }
  }

  /// Generate code for value-at (*x) operation (dereference).
  fn generate_value_at(
    &mut self,
    pointer: ValueId,
    pointer_expr: &Expr,
    resolved_type: Option<&IrType>,
  ) -> ValueId {
    // Priority: caller-provided type → symbol manager's pointer tracking → error.
    let target_type = if let Some(rt) = resolved_type {
      rt.clone()
    } else if let Some(manager) = &self.symbol_manager {
      Self::resolve_value_at_type(manager, pointer_expr, self)
    } else {
      ice!("value at: no symbol manager available — invariant violation")
    };
    self.builder.load(pointer, target_type)
  }

  /// Resolve the target type for value-at when no caller-provided type is available.
  fn resolve_value_at_type(
    manager: &crate::semantic_analysis::SqliteSymbolManager,
    pointer_expr: &Expr,
    ir: &IrGenerator,
  ) -> IrType {
    // Unwrap `unsafe cast` wrapper: `value at (unsafe cast p)` — resolve p's target type
    let pointer_expr = if let Expr::Unary(un) = pointer_expr {
      if matches!(un.operator, UnaryOp::UnsafeCast) {
        un.operand.as_ref()
      } else {
        pointer_expr
      }
    } else {
      pointer_expr
    };

    if let Expr::Identifier(id) = pointer_expr {
      if let Ok(Some(symbol)) = manager.lookup_var_symbol(id.name()) {
        if let Some(target_type_str) = &symbol.pointer_to_type {
          return ir.type_string_to_ir_type(target_type_str);
        }
        ice!(
          "value at '{}': pointer has no tracked target type",
          id.name()
        )
      }
      ice!(
        "value at: variable '{}' not found in symbol manager",
        id.name()
      )
    }
    if let Expr::MemberAccess(member) = pointer_expr {
      if let Expr::Identifier(obj) = &*member.object
        && let Ok(Some(obj_symbol)) = manager.lookup_var_symbol(obj.name())
      {
        let obj_type = ir.type_string_to_ir_type(&obj_symbol.data_type);
        if let IrType::Struct { name } = &obj_type
          && let Some(layout) = ir.type_layouts.get(name)
          && let Some(field_idx) = layout
            .field_names
            .iter()
            .position(|f| f == &member.member.node)
        {
          let field_type = &layout.field_types[field_idx];
          if let IrType::Ptr(inner) = field_type {
            if matches!(inner.as_ref(), IrType::Void) {
              ice!(
                "value at on '{}.{}': field is a raw (Void) pointer — cannot determine target type. Use 'unsafe cast as T'.",
                obj.name(),
                member.member.node
              )
            }
            return inner.as_ref().clone();
          }
          ice!(
            "value at on '{}.{}': field is not a pointer type",
            obj.name(),
            member.member.node
          )
        }
      }
      ice!("value at: cannot resolve member access target type")
    }
    ice!("value at: cannot determine target type for non-identifier, non-member expression")
  }

  /// Non-panicking variant of `resolve_value_at_type`. Returns `None` when
  /// the pointer type cannot be determined (local variables, raw pointers).
  fn try_resolve_value_at_type(
    manager: &crate::semantic_analysis::SqliteSymbolManager,
    pointer_expr: &Expr,
    ir: &IrGenerator,
  ) -> Option<IrType> {
    // Unwrap `unsafe cast` wrapper: `value at (unsafe cast p)` — resolve p's target type
    let pointer_expr = if let Expr::Unary(un) = pointer_expr {
      if matches!(un.operator, UnaryOp::UnsafeCast) {
        un.operand.as_ref()
      } else {
        pointer_expr
      }
    } else {
      pointer_expr
    };

    if let Expr::Identifier(id) = pointer_expr {
      if let Ok(Some(symbol)) = manager.lookup_var_symbol(id.name())
        && let Some(target_type_str) = &symbol.pointer_to_type
      {
        return Some(ir.type_string_to_ir_type(target_type_str));
      }
      // Local variable or raw pointer: can't determine target type
      return None;
    }
    None
  }

  /// Get the type of an IR value by looking it up in the current function.
  /// Returns I64 as default if the type cannot be determined.
  fn get_value_type(&self, value: ValueId) -> IrType {
    // Get the current function to look up the value type
    if let Some(func_id) = self.builder.get_current_func()
      && let Some(func) = self.builder.module().function(func_id)
      && let Some(ty) = func.value_type(value)
    {
      return ty.clone();
    }
    // Default fallback type
    // NOTE: This can happen with external function calls where the value type isn't tracked
    // in the current function's table. In such cases, I64 is a reasonable default but may
    // not always be correct. A better approach would be to track return types of external calls.
    IrType::I64
  }

  /// Generate a type conversion instruction based on source and target types.
  /// This selects the appropriate conversion instruction (sext, zext, trunc, etc.)
  /// based on the actual source and target types.
  fn generate_type_conversion(
    &mut self,
    src_value: ValueId,
    source_type: IrType,
    target_type: IrType,
  ) -> ValueId {
    // Same type: no conversion needed
    if source_type == target_type {
      return src_value;
    }

    use IrType::*;

    // Helper to check if a type is signed integer
    let is_signed_int = |ty: &IrType| matches!(ty, I8 | I16 | I32 | I64);
    // Helper to check if a type is unsigned integer
    let is_unsigned_int = |ty: &IrType| matches!(ty, U8 | U16 | U32 | U64);
    // Helper to check if a type is any integer
    let is_any_int = |ty: &IrType| is_signed_int(ty) || is_unsigned_int(ty);
    // Helper to check if a type is floating point
    let is_float = |ty: &IrType| matches!(ty, F16 | F32 | F64);

    match (&source_type, &target_type) {
      // Integer to Integer conversions
      (s, t) if is_any_int(s) && is_any_int(t) => {
        let src_bits = Self::int_type_bits(&source_type);
        let tgt_bits = Self::int_type_bits(&target_type);

        if src_bits < tgt_bits {
          // Widening: use sext for signed sources, zext for unsigned
          if is_signed_int(&source_type) {
            self.builder.sext(src_value, target_type)
          } else {
            self.builder.zext(src_value, target_type)
          }
        } else if src_bits > tgt_bits {
          // Narrowing: truncate
          self.builder.trunc(src_value, target_type)
        } else {
          // Same width: no conversion needed (already handled above)
          src_value
        }
      }

      // Float to Float conversions
      (s, t) if is_float(s) && is_float(t) => {
        let src_bits = Self::float_type_bits(&source_type);
        let tgt_bits = Self::float_type_bits(&target_type);

        if tgt_bits > src_bits {
          // Widening: extend
          self.builder.fp_ext(src_value, target_type)
        } else {
          // Narrowing: truncate
          self.builder.fp_trunc(src_value, target_type)
        }
      }

      // Signed Integer to Float
      (s, t) if is_signed_int(s) && is_float(t) => self.builder.si_to_fp(src_value, target_type),

      // Unsigned Integer to Float
      (s, t) if is_unsigned_int(s) && is_float(t) => self.builder.ui_to_fp(src_value, target_type),

      // Float to Signed Integer
      (s, t) if is_float(s) && is_signed_int(t) => self.builder.fp_to_si(src_value, target_type),

      // Float to Unsigned Integer
      (s, t) if is_float(s) && is_unsigned_int(t) => self.builder.fp_to_ui(src_value, target_type),

      // Wrap value in Optional: T → Optional(T) — used for `var x as T? = expr`
      (s, IrType::Optional(inner)) if s == inner.as_ref() => self.builder.some(src_value),

      // Pointer conversions
      (Ptr(_), t) if is_any_int(t) => {
        // Pointer to integer: just pass through (bit pattern reinterpretation)
        src_value
      }

      (s, Ptr(_)) if is_unsigned_int(s) || is_signed_int(s) => {
        // Integer (signed or unsigned) to pointer: use int_to_ptr
        // Note: Negative integers will be reinterpreted as large unsigned pointers
        self.builder.int_to_ptr(src_value)
      }

      (Ptr(_), Ptr(_)) => {
        // Pointer to pointer: no conversion needed
        src_value
      }

      // Conversions that should have been caught by semantic analysis
      _ => {
        ice!(
          "Unsupported type conversion from {:?} to {:?} — semantic analysis should have rejected this",
          source_type,
          target_type
        );
      }
    }
  }

  /// Get the bit width of an integer type.
  fn int_type_bits(ty: &IrType) -> usize {
    match ty {
      IrType::I8 | IrType::U8 => 8,
      IrType::I16 | IrType::U16 => 16,
      IrType::I32 | IrType::U32 => 32,
      IrType::I64 | IrType::U64 => 64,
      _ => 64, // Default
    }
  }

  /// Get the bit width of a float type.
  fn float_type_bits(ty: &IrType) -> usize {
    match ty {
      IrType::F16 => 16,
      IrType::F32 => 32,
      IrType::F64 => 64,
      _ => 64, // Default
    }
  }

  /// Generate code for an integer literal.
  /// Uses the resolved type from semantic analysis when available,
  /// otherwise defaults to i64.
  fn generate_int_literal(&mut self, lit: &IntLiteral, resolved_type: Option<&IrType>) -> ValueId {
    let ir_type = resolved_type.cloned().unwrap_or_else(|| {
      ice!(
        "IntLiteral '{}' at {}:{} generated without resolved type — \
         type must be threaded through the IR gen call chain",
        lit.value,
        lit.location.line,
        lit.location.col
      )
    });
    match &ir_type {
      IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
        self.builder.const_uint(ir_type, lit.value as u64)
      }
      IrType::F16 | IrType::F32 | IrType::F64 => {
        self.builder.const_float(ir_type, lit.value as f64)
      }
      _ => self.builder.const_int(ir_type, lit.value),
    }
  }

  /// Generate code for an unsigned integer literal.
  /// Uses the resolved type from semantic analysis when available,
  /// otherwise defaults to u64.
  fn generate_uint_literal(
    &mut self,
    lit: &UintLiteral,
    resolved_type: Option<&IrType>,
  ) -> ValueId {
    let ir_type = resolved_type.cloned().unwrap_or_else(|| {
      ice!(
        "UintLiteral '{}' at {}:{} generated without resolved type — \
         type must be threaded through the IR gen call chain",
        lit.value,
        lit.location.line,
        lit.location.col
      );
    });
    self.builder.const_uint(ir_type, lit.value)
  }

  /// Generate code for a float literal.
  /// Uses the resolved type from semantic analysis when available,
  /// otherwise defaults to f64.
  /// If the target type is not a float (e.g., u32), converts via fptoui.
  fn generate_float_literal(
    &mut self,
    lit: &FloatLiteral,
    resolved_type: Option<&IrType>,
  ) -> ValueId {
    let ir_type = resolved_type.cloned().unwrap_or_else(|| {
      ice!(
        "FloatLiteral '{}' at {}:{} generated without resolved type — \
         type must be threaded through the IR gen call chain",
        lit.value,
        lit.location.line,
        lit.location.col
      );
    });
    match &ir_type {
      IrType::F16 | IrType::F32 | IrType::F64 => self.builder.const_float(ir_type, lit.value),
      _ => {
        // Non-float target type (e.g., u32 with step 0.5):
        // generate as f64 then convert to the target type
        let float_val = self.builder.const_float(IrType::F64, lit.value);
        self.generate_type_conversion(float_val, IrType::F64, ir_type)
      }
    }
  }

  /// Generate code for a hex literal.
  fn generate_hex_literal(&mut self, lit: &HexLiteral) -> ValueId {
    // Parse the hex string (e.g., "0xFF" -> 255)
    let value = match i64::from_str_radix(lit.value.trim_start_matches("0x"), 16) {
      Ok(v) => v,
      Err(_) => {
        ice!(
          "Failed to parse hex literal '{}' as i64 — parser should have validated this",
          lit.value
        );
      }
    };
    self.builder.const_int(IrType::I64, value)
  }

  /// Generate code for a character literal.
  fn generate_char_literal(&mut self, lit: &CharLiteral) -> ValueId {
    // Characters are represented as u32 codepoints
    self.builder.const_uint(IrType::U32, lit.value as u64)
  }

  /// Generate code for a boolean literal.
  fn generate_bool_literal(&mut self, lit: &BoolLiteral) -> ValueId {
    self.builder.const_bool(lit.value)
  }

  /// Generate code for a string literal.
  fn generate_string_literal(&mut self, lit: &StringLiteral) -> ValueId {
    // Handle string embedded values by concatenating text and embedded expressions
    let mut parts_values = Vec::new();

    for part in &lit.parts {
      match part {
        crate::ast::definitions::StringPart::Text(text) => {
          parts_values.push(self.builder.const_string(&text.node));
        }
        crate::ast::definitions::StringPart::EmbeddedValue(expr) => {
          let type_str = self.lookup_expr_type(expr);
          let resolved = if type_str.is_empty() || type_str == "unknown" {
            None
          } else {
            Some(self.type_string_to_ir_type(&type_str))
          };
          let value = self.generate_expr_with_resolved_type(expr, resolved.as_ref());

          if !type_str.is_empty() && self.enum_types.contains(&type_str) {
            let enum_type = self.type_string_to_ir_type(&type_str);
            let enum_alloc = self.builder.alloca(enum_type);
            self.builder.store(value, enum_alloc);
            let dispatch_name = format!("__lale_enum_name_{}", type_str);
            let dispatch_result =
              self
                .builder
                .call_named(&dispatch_name, vec![enum_alloc], IrType::struct_ref("str"));
            parts_values.push(dispatch_result);
            continue;
          }

          let value_type = self.get_value_type(value);
          let str_value = match &value_type {
            IrType::Struct { name } if name == "str" => {
              // Deep-copy so concat frees the copy, not the live variable.
              self.builder.copy_str(value)
            }
            IrType::Ptr(inner) if matches!(inner.as_ref(), IrType::Struct { name } if name == "str") =>
            {
              let loaded = self.builder.load(value, IrType::struct_ref("str"));
              self.builder.copy_str(loaded)
            }
            IrType::Ptr(_) => value,
            _ => self.value_to_str_ptr(value, &value_type, false),
          };
          parts_values.push(str_value);
        }
      }
    }

    if parts_values.is_empty() {
      return self.builder.const_string("");
    }

    if parts_values.len() == 1 {
      #[allow(clippy::unwrap_used)] // Invariant: len() == 1 guarantees next() returns Some
      return parts_values.into_iter().next().unwrap();
    }

    // Concatenate left-to-right
    let mut result = parts_values[0];
    for part in &parts_values[1..] {
      result = self.builder.concat(result, *part);
    }
    result
  }

  /// Generate code for a function call.
  fn generate_fn_call_expr(&mut self, fn_call: &FnCall, resolved_type: Option<&IrType>) -> ValueId {
    let fn_name =
      if fn_call.target.node.len() > 1 && self.enum_types.contains(&fn_call.target.node[0]) {
        fn_call.target.node.last().cloned().unwrap_or_default()
      } else {
        fn_call.target.node.join("::")
      };
    // Resolve suite-qualified function names so calls to suite helpers emit a
    // call to the mangled function the analyzer registered.
    let fn_name = self.resolve_suite_fn_name(&fn_name);

    // Pre-lookup: try to find parameter types before generating arguments,
    // so that literal arguments get the correct resolved type.
    // Priority: symbol_manager → resolved_type struct → fn_name type constructor
    let pre_param_types: Option<Vec<IrType>> = self
      .symbol_manager
      .and_then(|sm| {
        sm.lookup_function(&fn_name).map(|fi| {
          fi.parameters
            .iter()
            .map(|p| self.type_name_to_ir_type(&p.type_annotation))
            .collect()
        })
      })
      .or_else(|| {
        resolved_type.and_then(|rt| match rt {
          IrType::Struct { name } => self
            .type_layouts
            .get(name)
            .map(|layout| layout.field_types.clone()),
          // Vector constructors: vec2/vec3/vec4 take N x inner_type arguments
          IrType::Vec2(inner) => Some(vec![inner.as_ref().clone(); 2]),
          IrType::Vec3(inner) => Some(vec![inner.as_ref().clone(); 3]),
          IrType::Vec4(inner) => Some(vec![inner.as_ref().clone(); 4]),
          _ => None,
        })
      })
      .or_else(|| {
        self
          .type_layouts
          .get(&fn_name)
          .map(|layout| layout.field_types.clone())
      });

    // Generate code for each argument, with type context if available
    let mut arg_values = Vec::new();
    for (i, arg) in fn_call.arguments.iter().enumerate() {
      let resolved = pre_param_types.as_ref().and_then(|pts| pts.get(i));
      arg_values.push(self.generate_expr_with_resolved_type(arg, resolved));
    }

    // Build qualified function name using available semantic context
    // Priority: symbol_manager > IR module search
    let qualified_fn_name = if let Some(manager) = self.symbol_manager {
      // Query the symbol database directly for the qualified name and linkage
      if let Ok(Some((qualified_name, linkage))) =
        manager.lookup_function_qualified_name_and_linkage(&fn_name)
      {
        // The database has the authoritative qualified name
        // For imported functions, use simple name; otherwise use qualified name from database
        if linkage.to_lowercase() == "import" && !fn_name.starts_with("__lale_") {
          // Imported C function - use simple name, because C does not support function overloading
          // Exception: __lale_ prefixed functions are Lale stdlib stubs that use qualified names
          fn_name.clone()
        } else {
          // User-defined or stdlib function - use qualified name from database
          qualified_name
        }
      } else {
        // Not in symbol database — may be a type constructor or compiler-internal
        // function. These are resolved downstream from type_layouts or the IR module.
        fn_name.clone()
      }
    } else {
      // No semantic context — type constructors, externs, or builtins
      fn_name.clone()
    };

    // Look up the function to get its return type and parameter types
    // Use the qualified name that we built - if not found, try other semantic sources
    let (return_type, param_types) = if let Some(func) =
      self.builder.module().function_by_name(&qualified_fn_name)
    {
      let param_types = func.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>();
      (func.return_type.clone(), Some(param_types))
    } else {
      // Qualified name not in module - try symbol manager
      if let Some(manager) = self.symbol_manager {
        // Use symbol manager for function lookup
        let global_table = manager.get_symbol_table(crate::semantic_analysis::VarScope::Global);
        if let Some(symbol) = global_table.get(&fn_name) {
          let return_type = self.type_string_to_ir_type(&symbol.data_type);
          (return_type, None)
        } else if fn_name == "str" && fn_call.arguments.len() == 2 {
          // Special case: str(ptr, len) constructor - return str struct
          (IrType::struct_ref("str"), None)
        } else if fn_name == "vec2" || fn_name == "vec3" || fn_name == "vec4" {
          // Built-in vector constructor — default to f64 inner type
          let inner = IrType::F64;
          let vec_ty = if fn_name == "vec2" {
            IrType::vec2(inner)
          } else if fn_name == "vec3" {
            IrType::vec3(inner)
          } else {
            IrType::vec4(inner)
          };
          (vec_ty, None)
        } else {
          ice!(
            "Function '{}' at {}:{} not found in symbol table or IR module — semantic analysis incomplete",
            fn_name,
            fn_call.location.line,
            fn_call.location.col
          );
        }
      } else {
        ice!(
          "Cannot perform IR generation without symbol manager. Function '{}' cannot be verified.",
          fn_name
        );
      }
    };

    // Convert arguments to match parameter types if needed
    let final_args = if let Some(param_types) = param_types {
      arg_values
        .into_iter()
        .zip(param_types)
        .map(|(arg, param_ty)| {
          let arg_type = self.get_value_type(arg);
          if arg_type != param_ty {
            // Special case: passing a struct by value where a pointer is expected
            // (e.g., str field in a type constructor). Allocate stack space,
            // store the struct, and pass the pointer.
            if let (IrType::Struct { .. }, IrType::Ptr(inner)) = (&arg_type, &param_ty)
              && let IrType::Struct { .. } = inner.as_ref()
            {
              let tmp = self.builder.alloca(arg_type.clone());
              self.builder.store(arg, tmp);
              return tmp;
            }
            // Normal type conversion
            self.generate_type_conversion(arg, arg_type, param_ty)
          } else {
            arg
          }
        })
        .collect()
    } else {
      arg_values
    };

    // Special handling for str(ptr, len) constructor
    if fn_name == "str" && fn_call.arguments.len() == 2 {
      // str(ptr, len) constructor - build the struct directly
      // First argument is pointer, second is length (u64)
      if final_args.len() == 2 {
        let ptr_val = final_args[0];
        let len_val = final_args[1];

        // Create the str struct directly
        let str_struct = self
          .builder
          .build_struct_value("str", vec![ptr_val, len_val]);
        return str_struct;
      }
    }

    // Special handling for vec2/vec3/vec4 constructors
    if ((fn_name == "vec2" && fn_call.arguments.len() == 2)
      || (fn_name == "vec3" && fn_call.arguments.len() == 3)
      || (fn_name == "vec4" && fn_call.arguments.len() == 4))
      && final_args.len() == fn_call.arguments.len()
    {
      let inner = match &return_type {
        IrType::Vec2(inner) | IrType::Vec3(inner) | IrType::Vec4(inner) => *inner.clone(),
        _ => IrType::F64,
      };

      return match fn_name.as_str() {
        "vec2" => self.builder.build_vec2(final_args[0], final_args[1], inner),
        "vec3" => self
          .builder
          .build_vec3(final_args[0], final_args[1], final_args[2], inner),
        "vec4" => self.builder.build_vec4(
          final_args[0],
          final_args[1],
          final_args[2],
          final_args[3],
          inner,
        ),
        _ => unreachable!(),
      };
    }

    // Emit the function call using qualified name
    // Always use the qualified name if it was successfully built from semantic context
    // (even if not yet in module - it will be there after stdlib is loaded)
    if qualified_fn_name != fn_name {
      // We have a qualified name from semantic analysis - use it
      // This is important for stdlib functions that are loaded after main IR generation
      self.builder.call_named_at(
        &qualified_fn_name,
        final_args,
        return_type,
        &fn_call.location.source_file,
        fn_call.location.line as i64,
        fn_call.location.col as i64,
      )
    } else if self
      .builder
      .module()
      .function_by_name(&qualified_fn_name)
      .is_some()
    {
      // Qualified name matches function in module
      self.builder.call_named_at(
        &qualified_fn_name,
        final_args,
        return_type,
        &fn_call.location.source_file,
        fn_call.location.line as i64,
        fn_call.location.col as i64,
      )
    } else {
      // No qualified name available - use unqualified
      self.builder.call_named_at(
        &fn_name,
        final_args,
        return_type,
        &fn_call.location.source_file,
        fn_call.location.line as i64,
        fn_call.location.col as i64,
      )
    }
  }

  /// Universal qualified name lookup without fallbacks
  /// Finds any function by matching argument types to build a qualified name.
  /// This works for all functions (builtins, stdlib, user-defined), not just builtins.
  /// Returns Some(qualified_name) if function is found, None if not found.
  /// Does NOT fall back to unqualified names or fuzzy matching.
  fn find_qualified_function_by_args(
    &self,
    unqualified_name: &str,
    arg_values: &[ValueId],
  ) -> Option<String> {
    // Get the types of the arguments
    let arg_types: Vec<String> = arg_values
      .iter()
      .map(|val| {
        let ir_type = self.get_value_type(*val);
        self.ir_type_to_type_name_string(&ir_type)
      })
      .collect();

    // Build a qualified name by appending parameter type names
    if arg_types.is_empty() {
      // No arguments - look for unqualified function
      if self
        .builder
        .module()
        .function_by_name(unqualified_name)
        .is_some()
      {
        return Some(unqualified_name.to_string());
      }
      return None;
    }

    // Build the qualified name (e.g., __lale_i64_to_str)
    let qualified_name = format!("{}_{}", unqualified_name, arg_types.join("_"));

    if self
      .builder
      .module()
      .function_by_name(&qualified_name)
      .is_some()
    {
      return Some(qualified_name);
    }

    // REMOVED: Dangerous fallbacks to unqualified names or fuzzy matching
    // If the qualified name isn't found, the function call is invalid.
    // All legitimate calls should have their exact signature in the module.
    None
  }

  /// Deprecated: This method had dangerous fallbacks that masked errors.
  /// Use find_qualified_function_by_args() instead.
  #[deprecated(
    since = "0.1.0",
    note = "Use find_qualified_function_by_args() - this had dangerous fallbacks"
  )]
  #[allow(dead_code)]
  fn find_builtin_function_by_args(
    &self,
    unqualified_name: &str,
    arg_values: &[ValueId],
  ) -> Option<String> {
    // Delegate to universal function lookup
    self.find_qualified_function_by_args(unqualified_name, arg_values)
  }

  /// Convert an IR type to a type name string for qualification purposes.
  /// Used to build qualified function names like __lale_i64_to_str.
  fn ir_type_to_type_name_string(&self, ir_type: &IrType) -> String {
    match ir_type {
      IrType::I8 => "i8".to_string(),
      IrType::U8 => "u8".to_string(),
      IrType::I16 => "i16".to_string(),
      IrType::U16 => "u16".to_string(),
      IrType::I32 => "i32".to_string(),
      IrType::U32 => "u32".to_string(),
      IrType::I64 => "i64".to_string(),
      IrType::U64 => "u64".to_string(),
      IrType::F16 => "f16".to_string(),
      IrType::F32 => "f32".to_string(),
      IrType::F64 => "f64".to_string(),
      IrType::Bool => "bool".to_string(),
      IrType::Char => "char".to_string(),
      IrType::Ptr(_) => "pointer".to_string(),
      IrType::Array { .. } => "array".to_string(),
      IrType::Void => "void".to_string(),
      IrType::Struct { name } => name.clone(),
      IrType::Optional(inner) => format!("{}?", self.ir_type_to_type_name_string(inner)),
      IrType::Vec2(_) => "vec2".to_string(),
      IrType::Vec3(_) => "vec3".to_string(),
      IrType::Vec4(_) => "vec4".to_string(),
    }
  }

  /// Check if a name refers to a zero-argument function in the IR module.
  /// Used as a fallback when variable lookup fails for an `Expr::Identifier`.
  fn is_zero_arg_function(&self, name: &str) -> bool {
    for func in &self.builder.module().functions {
      if func.name == name && func.params.is_empty() {
        return true;
      }
    }
    for func in &self.builder.module().extern_funcs {
      if func.name == name && func.params.is_empty() {
        return true;
      }
    }
    false
  }

  /// Call a zero-argument function by name, returning its result value.
  fn call_zero_arg_function(&mut self, name: &str) -> ValueId {
    let return_type = self
      .builder
      .module()
      .functions
      .iter()
      .find(|f| f.name == name)
      .map(|f| f.return_type.clone())
      .or_else(|| {
        self
          .builder
          .module()
          .extern_funcs
          .iter()
          .find(|f| f.name == name)
          .map(|f| f.return_type.clone())
      })
      .unwrap_or_else(|| {
        ice!("call_zero_arg_function: function '{}' not found in module functions or extern_funcs — semantic analysis should have caught this", name);
      });
    self.builder.call_named(name, vec![], return_type)
  }

  /// Convert a type name to an IR type.
  fn type_name_to_ir_type(&self, type_name: &TypeName) -> IrType {
    let mut base = match &type_name.base_type {
      BaseType::U8 => IrType::U8,
      BaseType::I8 => IrType::I8,
      BaseType::U16 => IrType::U16,
      BaseType::I16 => IrType::I16,
      BaseType::U32 => IrType::U32,
      BaseType::I32 => IrType::I32,
      BaseType::U64 => IrType::U64,
      BaseType::I64 => IrType::I64,
      BaseType::F16 => IrType::F16,
      BaseType::F32 => IrType::F32,
      BaseType::F64 => IrType::F64,
      BaseType::Bool => IrType::Bool,
      BaseType::Str => IrType::struct_ref("str"), // "str" is a type defined in builtins.lale
      BaseType::Byte => IrType::U8,
      BaseType::Char => IrType::U32,
      BaseType::Pointer => IrType::raw_ptr(),
      BaseType::Vec2 => {
        let inner = type_name
          .inner_type
          .as_ref()
          .map(|t| self.base_type_to_ir_type(t))
          .unwrap_or_else(|| ice!("vec2 type missing inner_type — incomplete type definition"));
        IrType::vec2(inner)
      }
      BaseType::Vec3 => {
        let inner = type_name
          .inner_type
          .as_ref()
          .map(|t| self.base_type_to_ir_type(t))
          .unwrap_or_else(|| ice!("vec3 type missing inner_type — incomplete type definition"));
        IrType::vec3(inner)
      }
      BaseType::Vec4 => {
        let inner = type_name
          .inner_type
          .as_ref()
          .map(|t| self.base_type_to_ir_type(t))
          .unwrap_or_else(|| ice!("vec4 type missing inner_type — incomplete type definition"));
        IrType::vec4(inner)
      }
      BaseType::Custom(name) => {
        // Custom types (records) are represented as struct values directly
        IrType::struct_ref(name)
      }
    };

    // Handle array dimensions if present
    for dim_expr in type_name.array_dimensions.iter().rev() {
      // Extract the dimension size from the expression (should be a literal)
      let size = match dim_expr {
        Expr::UintLiteral(lit) => lit.value,
        Expr::IntLiteral(lit) => lit.value as u64,
        _ => {
          ice!(
            "Array dimension is not a compile-time constant — semantic analysis should have validated array dimensions"
          );
        }
      };
      base = IrType::array(base, size);
    }

    // Wrap in Optional if the type is marked optional
    if type_name.is_optional {
      base = IrType::optional(base);
    }

    base
  }

  /// Convert a bare BaseType to an IR type (without TypeName wrapper).
  fn base_type_to_ir_type(&self, base_type: &BaseType) -> IrType {
    match base_type {
      BaseType::U8 => IrType::U8,
      BaseType::I8 => IrType::I8,
      BaseType::U16 => IrType::U16,
      BaseType::I16 => IrType::I16,
      BaseType::U32 => IrType::U32,
      BaseType::I32 => IrType::I32,
      BaseType::U64 => IrType::U64,
      BaseType::I64 => IrType::I64,
      BaseType::F16 => IrType::F16,
      BaseType::F32 => IrType::F32,
      BaseType::F64 => IrType::F64,
      BaseType::Bool => IrType::Bool,
      BaseType::Str => IrType::struct_ref("str"),
      BaseType::Byte => IrType::U8,
      BaseType::Char => IrType::U32,
      BaseType::Pointer => IrType::raw_ptr(),
      // Vec2/3/4 should use type_name_to_ir_type (needs inner_type from TypeName).
      // If reached here, the caller passed a bare Vec2 BaseType without context.
      BaseType::Vec2 => {
        ice!("bare vec2 reached base_type_to_ir_type — use type_name_to_ir_type instead")
      }
      BaseType::Vec3 => {
        ice!("bare vec3 reached base_type_to_ir_type — use type_name_to_ir_type instead")
      }
      BaseType::Vec4 => {
        ice!("bare vec4 reached base_type_to_ir_type — use type_name_to_ir_type instead")
      }
      BaseType::Custom(name) => IrType::struct_ref(name),
    }
  }

  /// Compute the byte size of an IR type (for struct layout).
  fn ir_type_size(&self, ty: &IrType) -> i64 {
    match ty {
      IrType::Array { element, size } => self.ir_type_size(element) * (*size as i64),
      IrType::Struct { name } => {
        if let Some(layout) = self.type_layouts.get(name) {
          layout
            .field_types
            .iter()
            .map(|ft| self.ir_type_size(ft))
            .sum()
        } else {
          ice!("ir_type_size: struct '{}' not found in type_layouts", name);
        }
      }
      IrType::I8 | IrType::U8 | IrType::Bool => 1,
      IrType::I16 | IrType::U16 | IrType::F16 => 2,
      IrType::I32 | IrType::U32 | IrType::F32 | IrType::Char => 4,
      IrType::I64 | IrType::U64 | IrType::F64 | IrType::Ptr(_) => 8,
      IrType::Void => 0,
      IrType::Optional(_) => 16, // 8 bytes for tag (is_some boolean) + 8 bytes for value
      IrType::Vec2(inner) => self.ir_type_size(inner) * 2,
      IrType::Vec3(inner) => self.ir_type_size(inner) * 3,
      IrType::Vec4(inner) => self.ir_type_size(inner) * 4,
    }
  }

  /// Generate a call to the appropriate int-to-string conversion function.
  /// For signed integers (is_unsigned=false): sign-extends to i64, calls __lale_i64_to_str_i64.
  /// For unsigned integers (is_unsigned=true): zero-extends to u64, calls __lale_u64_to_str_u64.
  fn call_int_to_str(&mut self, value: ValueId, value_type: &IrType, is_unsigned: bool) -> ValueId {
    if is_unsigned {
      let target = IrType::U64;
      let converted = if *value_type == target {
        value
      } else {
        self.builder.zext(value, target.clone())
      };
      self.builder.call_named(
        "__lale_u64_to_str_u64",
        vec![converted],
        IrType::struct_ref("str"),
      )
    } else {
      let target = IrType::I64;
      let converted = if *value_type == target {
        value
      } else {
        self.builder.sext(value, target.clone())
      };
      self.builder.call_named(
        "__lale_i64_to_str_i64",
        vec![converted],
        IrType::struct_ref("str"),
      )
    }
  }

  /// Start building a new function.
  fn _start_function(&mut self, name: &str, return_type: IrType, linkage: Linkage) {
    self.builder.start_function(name, return_type, linkage);
  }

  /// Extract the base type from a type string.
  /// Examples: "i32" -> "i32", "i32[5]" -> "i32", "f64[3][3]" -> "f64"
  fn extract_base_type<'b>(&self, type_str: &'b str) -> &'b str {
    // Find the position of the first bracket
    type_str.split('[').next().unwrap_or(type_str).trim()
  }

  /// Public version of extract_base_type for testing
  #[doc(hidden)]
  pub fn extract_base_type_pub<'b>(&self, type_str: &'b str) -> &'b str {
    self.extract_base_type(type_str)
  }

  /// Public version of type_string_to_ir_type for testing
  #[doc(hidden)]
  pub fn type_string_to_ir_type_pub(&self, type_str: &str) -> IrType {
    self.type_string_to_ir_type(type_str)
  }

  /// Convert a type string to IrType.
  /// Handles base types like "i32", "f64", "u8", etc.
  fn type_string_to_ir_type(&self, type_str: &str) -> IrType {
    match type_str {
      "u8" => IrType::U8,
      "i8" => IrType::I8,
      "u16" => IrType::U16,
      "i16" => IrType::I16,
      "u32" => IrType::U32,
      "i32" => IrType::I32,
      "u64" => IrType::U64,
      "i64" => IrType::I64,
      "f16" => IrType::F16,
      "f32" => IrType::F32,
      "f64" => IrType::F64,
      "bool" => IrType::Bool,
      "str" => IrType::struct_ref("str"),
      "nothing" | "void" => IrType::Void,
      "byte" => IrType::U8,
      "char" => IrType::U32,
      "ptr" | "pointer" => IrType::raw_ptr(),
      other => {
        // Check if this is a custom struct type (registered via type layouts)
        if self.type_layouts.contains_key(other) || self.enum_types.contains(other) {
          return IrType::struct_ref(other);
        }
        // Handle optional types: "f64?" → Optional(F64)
        if let Some(inner) = other.strip_suffix('?') {
          return IrType::Optional(Box::new(self.type_string_to_ir_type(inner)));
        }
        // Handle vector types: "vec3<f64>" → Vec3(F64)
        if let Some(inner) = other.strip_prefix("vec2<")
          && let Some(inner_type) = inner.strip_suffix('>')
        {
          return IrType::vec2(self.type_string_to_ir_type(inner_type));
        }
        if let Some(inner) = other.strip_prefix("vec3<")
          && let Some(inner_type) = inner.strip_suffix('>')
        {
          return IrType::vec3(self.type_string_to_ir_type(inner_type));
        }
        if let Some(inner) = other.strip_prefix("vec4<")
          && let Some(inner_type) = inner.strip_suffix('>')
        {
          return IrType::vec4(self.type_string_to_ir_type(inner_type));
        }
        // Bare vector types without inner type — semantic analysis should have inserted the inner type
        if other == "vec2" {
          ice!(
            "Bare 'vec2' type string without inner type — semantic analysis should have resolved this"
          );
        }
        if other == "vec3" {
          ice!(
            "Bare 'vec3' type string without inner type — semantic analysis should have resolved this"
          );
        }
        if other == "vec4" {
          ice!(
            "Bare 'vec4' type string without inner type — semantic analysis should have resolved this"
          );
        }
        // Handle array types: "f64[5]", "i32[3][4]"
        if other.contains('[') && other.ends_with(']') {
          return self.array_type_string_to_ir_type(other);
        }
        // "unknown" is a sentinel from lookup_expr_type — callers should guard before here
        if other == "unknown" {
          ice!(
            "type_string_to_ir_type called with 'unknown' — caller must check for unknown before type conversion"
          );
        }
        ice!(
          "Unknown type '{}' in type_string_to_ir_type — semantic analysis should have rejected unknown types",
          other
        );
      }
    }
  }

  /// Extract array dimensions from a type string.
  /// Examples: "i32[5]" -> vec![5], "f64[3][4]" -> vec![3, 4]
  pub fn extract_array_dimensions_pub(&self, type_str: &str) -> Vec<i64> {
    self.extract_array_dimensions(type_str)
  }

  /// Convert an array type string like "f64[5]" or "i32[3][4]" to IrType.
  fn array_type_string_to_ir_type(&self, type_str: &str) -> IrType {
    let base_end = type_str.find('[').unwrap_or(type_str.len());
    let base = &type_str[..base_end];
    let base_type = self.type_string_to_ir_type(base);
    let dims = self.extract_array_dimensions(type_str);
    let mut result = base_type;
    for dim in dims.iter().rev() {
      result = IrType::array(result, *dim as u64);
    }
    result
  }

  /// Extract array dimensions from a type string.
  /// Examples: "i32[5]" -> vec![5], "f64[3][4]" -> vec![3, 4]
  fn extract_array_dimensions(&self, type_str: &str) -> Vec<i64> {
    let mut dimensions = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in type_str.chars() {
      match ch {
        '[' => in_bracket = true,
        ']' => {
          if in_bracket && !current.is_empty() {
            if let Ok(dim) = current.parse::<i64>() {
              dimensions.push(dim);
            }
            current.clear();
          }
          in_bracket = false;
        }
        _ if in_bracket => current.push(ch),
        _ => {}
      }
    }

    dimensions
  }

  /// Get the element type from an array variable name by looking it up in the symbol manager.
  fn get_array_element_type(&self, array_name: &str) -> IrType {
    // Try to get type from symbol manager
    if let Some(manager) = &self.symbol_manager
      && let Some(type_str) = manager.lookup_var_type(array_name)
    {
      let base_type = self.extract_base_type(&type_str);
      return self.type_string_to_ir_type(base_type);
    }

    // Final fallback to i64 if type lookup fails
    IrType::I64
  }

  /// Get array dimensions from an array variable name.
  fn get_array_dimensions(&self, array_name: &str) -> Vec<i64> {
    // Try to get dimensions from symbol manager
    if let Some(manager) = &self.symbol_manager
      && let Some(type_str) = manager.lookup_var_type(array_name)
    {
      return self.extract_array_dimensions(&type_str);
    }

    Vec::new()
  }

  /// Compute the offset (in elements) for array indexing.
  /// Handles multi-dimensional arrays using row-major order.
  /// Converts 1-based indices (Lale convention) to 0-based offsets (memory convention).
  fn compute_array_offset(
    &mut self,
    indices: &[Expr],
    dimensions: &[i64],
    location: &SourceLocation,
  ) -> ValueId {
    let mut total_offset = self.builder.const_int(IrType::I64, 0);

    for (dim_idx, index_expr) in indices.iter().enumerate() {
      let index_val = self.generate_expr_with_resolved_type(index_expr, Some(&IrType::I64));

      // Bounds check: verify 1..length (Lale uses 1-based indexing)
      if dim_idx < dimensions.len() {
        let dim_size = self.builder.const_int(IrType::I64, dimensions[dim_idx]);
        self.builder.bounds_check(
          index_val,
          dim_size,
          "array index out of bounds",
          location.source_file.clone(),
          location.line as i64,
          location.col as i64,
        );
      }

      // Convert 1-based index to 0-based: subtract 1
      let idx_type = self.get_value_type(index_val);
      let one_typed = match idx_type.clone() {
        IrType::I32 => self.builder.const_int(IrType::I32, 1),
        IrType::I64 => self.builder.const_int(IrType::I64, 1),
        IrType::U32 => self.builder.const_uint(IrType::U32, 1),
        IrType::U64 => self.builder.const_uint(IrType::U64, 1),
        _ => self.builder.const_int(IrType::I64, 1),
      };

      let zero_based_index = self.builder.sub(index_val, one_typed, idx_type.clone());

      // Compute stride for this dimension
      // For dimension i, stride = product of dimensions[i+1..n]
      let mut stride = 1i64;
      for dim in dimensions.iter().skip(dim_idx + 1) {
        stride *= dim;
      }

      // Convert zero_based_index to i64 for offset computation
      // Properly extend based on signedness
      let zero_based_i64 = match idx_type.clone() {
        IrType::I64 => zero_based_index,
        IrType::U64 => self.builder.bitcast(zero_based_index, IrType::I64),
        IrType::I32 => {
          // Sign extend i32 to i64

          self.builder.sext(zero_based_index, IrType::I64)
        }
        IrType::U32 => {
          // Zero extend u32 to i64 (bitcast to i32 first, then zext)
          let as_i32 = self.builder.bitcast(zero_based_index, IrType::I32);
          self.builder.zext(as_i32, IrType::I64)
        }
        IrType::I16 => self.builder.sext(zero_based_index, IrType::I64),
        IrType::U16 => {
          let as_i16 = self.builder.bitcast(zero_based_index, IrType::I16);
          self.builder.zext(as_i16, IrType::I64)
        }
        IrType::I8 => self.builder.sext(zero_based_index, IrType::I64),
        IrType::U8 => {
          let as_i8 = self.builder.bitcast(zero_based_index, IrType::I8);
          self.builder.zext(as_i8, IrType::I64)
        }
        _ => {
          ice!(
            "Unhandled index type {:?} in array offset computation — semantic analysis should have validated index types",
            idx_type
          );
        }
      };

      let stride_const = self.builder.const_int(IrType::I64, stride);
      let dim_offset = self.builder.mul(zero_based_i64, stride_const, IrType::I64);

      // Add to total offset
      total_offset = self.builder.add(total_offset, dim_offset, IrType::I64);
    }

    total_offset
  }

  /// Generate constructor function for a type.
  /// This creates a function that takes all fields as parameters and returns the type.
  fn generate_type_constructor(&mut self, type_def: &TypeDefStmt) {
    let type_name = &type_def.name.node;

    // Build layout for type fields
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();
    let mut struct_fields = Vec::new();
    let mut current_offset = 0i64;

    for field in &type_def.fields {
      field_names.push(field.name.node.clone());
      let ir_type = self.type_name_to_ir_type(&field.field_type);

      // Store all field types directly (including str by value)
      field_types.push(ir_type.clone());
      struct_fields.push((field.name.node.clone(), ir_type.clone()));
      current_offset += self.ir_type_size(&ir_type);
    }

    let total_size = current_offset;
    let field_units: Vec<Option<String>> = type_def
      .fields
      .iter()
      .map(|f| f.unit.as_ref().map(|u| u.raw.clone()))
      .collect();

    // Add the struct definition to the IR module
    let mut struct_def = crate::ir::module::StructDef::new(type_name.clone());
    struct_def.fields = struct_fields;
    self.builder.module_mut().add_struct(struct_def);

    // Store the layout for later use in field access
    self.type_layouts.insert(
      type_name.clone(),
      TypeLayout {
        field_names: field_names.clone(),
        field_types: field_types.clone(),
        field_units,
        total_size,
      },
    );

    // Save the current function context (in case we're inside main)
    let prev_func = self.builder.get_current_func();
    let prev_block = self.builder.get_current_block();

    // Start a new function for the constructor
    // The constructor returns the struct value directly (the interpreter will handle the calling convention)
    let struct_type = IrType::struct_ref(type_name);
    self._start_function(type_name, struct_type.clone(), Linkage::Internal);
    self.push_scope();

    // Add parameters for each field and track them
    // For str fields: parameter is ptr<str> but we need to load the str value
    let mut field_values = Vec::new();
    for (field_idx, field) in type_def.fields.iter().enumerate() {
      let field_name = field.name.node.clone();
      let field_ir_type = self.type_name_to_ir_type(&field.field_type);
      let is_str_field = matches!(&field_ir_type, IrType::Struct { name } if name == "str");

      // For str fields, the parameter is Ptr(str) because strings are
      // passed as pointers to stack-allocated str structs
      let param_type = if is_str_field {
        IrType::ptr(IrType::struct_ref("str"))
      } else {
        field_ir_type.clone()
      };

      self
        .builder
        .add_param(&field_name, param_type.clone(), None);

      // Get the param value ID at the specific index
      let param_val = if let Some(func) = self.builder.get_current_func()
        && let Some(fn_def) = self
          .builder
          .module()
          .functions
          .iter()
          .find(|f| f.id == func)
        && let Some(param) = fn_def.params.get(field_idx)
      {
        param.value_id
      } else {
        // Fallback: create a placeholder zero value
        self.builder.const_int(field_ir_type.clone(), 0)
      };

      // For str fields, load the str value from the pointer
      if is_str_field {
        let loaded_str = self.builder.load(param_val, IrType::struct_ref("str"));
        field_values.push(loaded_str);
      } else {
        field_values.push(param_val);
      }
    }

    // Build the struct directly from field values and return it
    let struct_value = self.builder.build_struct_value(type_name, field_values);
    self.builder.ret(struct_value);

    self.pop_scope();

    // Restore previous function context
    self.builder.set_current_func(prev_func);
    self.builder.set_current_block(prev_block);
  }

  /// Generate constructor functions for each enum variant.
  ///
  /// For an enum `Shape` with variants `Circle(f64)` and `Point`:
  /// - Registers `Shape` as a struct with fields {tag: i32, payload: Shape__Variant}
  /// - Creates constructors `Shape__Circle(field0 as f64) returns Shape` and `Shape__Point() returns Shape`
  /// - The discriminant is the 0-based position in the variant list.
  fn generate_enum_constructors(&mut self, enum_def: &EnumDefStmt) {
    let enum_name = &enum_def.name.node;
    let num_variants = enum_def.variants.len();

    // Track this as an enum type for debug output dispatch
    self.enum_types.insert(enum_name.to_string());
    // Track variant names and their discriminant indices for branch statement lowering
    self.enum_variants.insert(
      enum_name.to_string(),
      enum_def
        .variants
        .iter()
        .enumerate()
        .map(|(idx, v)| (v.name.node.clone(), idx))
        .collect(),
    );

    if num_variants == 0 {
      return;
    }

    // Compute the maximum field count across all variants (including discriminant)
    // and register the enum type name itself as a struct with that many fields
    // so the interpreter can load/store enum values correctly.
    let max_fields = 1
      + enum_def
        .variants
        .iter()
        .map(|v| v.fields.len())
        .max()
        .unwrap_or(0);
    let enum_struct_fields: Vec<(String, IrType)> = (0..max_fields)
      .map(|i| (format!("__f{}", i), IrType::I64))
      .collect();
    let mut enum_struct_def = crate::ir::module::StructDef::new(enum_name.to_string());
    enum_struct_def.fields = enum_struct_fields.clone();
    self.builder.module_mut().add_struct(enum_struct_def);
    self.type_layouts.insert(
      enum_name.to_string(),
      TypeLayout {
        field_names: enum_struct_fields.iter().map(|(n, _)| n.clone()).collect(),
        field_types: enum_struct_fields.iter().map(|(_, t)| t.clone()).collect(),
        field_units: vec![None; max_fields],
        total_size: (max_fields as i64) * 8,
      },
    );

    // Register each variant as a type with a constructor (like regular type_defs)
    for (disc_idx, variant) in enum_def.variants.iter().enumerate() {
      let variant_type_name = format!("{}__{}", enum_name, variant.name.node);

      // Build the qualified constructor name (matching find_qualified_function_by_args)
      let param_type_names: Vec<String> = variant
        .fields
        .iter()
        .map(|f| {
          let ir_type = self.type_name_to_ir_type(&f.type_annotation);
          self.ir_type_to_type_name_string(&ir_type)
        })
        .collect();
      let qualified_ctor_name = if param_type_names.is_empty() {
        variant_type_name.clone()
      } else {
        format!("{}_{}", variant_type_name, param_type_names.join("_"))
      };

      // Build layout for variant fields (field 0 = __discriminant: i64)
      let mut field_names = Vec::new();
      let mut field_types = Vec::new();
      let mut field_units: Vec<Option<String>> = vec![None]; // discriminant has no unit
      let mut struct_fields: Vec<(String, IrType)> =
        vec![("__discriminant".to_string(), IrType::I64)];
      let mut current_offset = 8i64; // discriminant is 8 bytes

      for field in &variant.fields {
        let ir_type = self.type_name_to_ir_type(&field.type_annotation);
        field_names.push(field.name.node.clone());
        field_types.push(ir_type.clone());
        field_units.push(field.unit.as_ref().map(|u| u.raw.clone()));
        struct_fields.push((field.name.node.clone(), ir_type.clone()));
        current_offset += self.ir_type_size(&ir_type);
      }

      // Add the variant struct definition to the IR module
      let mut struct_def = crate::ir::module::StructDef::new(variant_type_name.clone());
      struct_def.fields = struct_fields;
      self.builder.module_mut().add_struct(struct_def);

      // Store the layout for later use in field access
      let total_size = current_offset;
      self.type_layouts.insert(
        variant_type_name.clone(),
        TypeLayout {
          field_names: field_names.clone(),
          field_types: field_types.clone(),
          field_units: field_units.clone(),
          total_size,
        },
      );

      // Save the current function context
      let prev_func = self.builder.get_current_func();
      let prev_block = self.builder.get_current_block();

      // Start a new function for the variant constructor
      // Returns the variant struct value
      let variant_struct_type = IrType::struct_ref(&variant_type_name);
      self._start_function(
        &qualified_ctor_name,
        variant_struct_type.clone(),
        Linkage::Internal,
      );
      self.push_scope();

      // Add parameters for each field
      let mut field_values = Vec::new();
      for (field_idx, field) in variant.fields.iter().enumerate() {
        let field_ir_type = self.type_name_to_ir_type(&field.type_annotation);
        let is_str_field = matches!(&field_ir_type, IrType::Struct { name } if name == "str");

        let param_type = if is_str_field {
          IrType::ptr(IrType::struct_ref("str"))
        } else {
          field_ir_type.clone()
        };

        self
          .builder
          .add_param(&field.name.node, param_type.clone(), None);

        let param_val = if let Some(func) = self.builder.get_current_func()
          && let Some(fn_def) = self
            .builder
            .module()
            .functions
            .iter()
            .find(|f| f.id == func)
          && let Some(param) = fn_def.params.get(field_idx)
        {
          param.value_id
        } else {
          self.builder.const_int(field_ir_type.clone(), 0)
        };

        if is_str_field {
          let loaded_str = self.builder.load(param_val, IrType::struct_ref("str"));
          field_values.push(loaded_str);
        } else {
          field_values.push(param_val);
        }
      }

      // Prepend discriminant value (variant index as i64) to field_values
      let disc_val = self.builder.const_int(IrType::I64, disc_idx as i64);
      let mut all_field_values = vec![disc_val];
      all_field_values.extend(field_values);

      // Build the variant struct and return it
      let struct_value = self
        .builder
        .build_struct_value(&variant_type_name, all_field_values);
      self.builder.ret(struct_value);

      self.pop_scope();

      // Restore previous function context
      self.builder.set_current_func(prev_func);
      self.builder.set_current_block(prev_block);

      // Generate a simple-name wrapper function (e.g. "Red" → calls "Color__Red",
      // "Circle_f64" → calls "Shape__Circle_f64"). The wrapper's IR name
      // uses the same qualified-name convention as the semantic analyzer so
      // function calls resolve correctly.
      {
        let simple_name = &variant.name.node;
        // Compute qualified name the same way define_function does
        let simple_qname = if variant.fields.is_empty() {
          simple_name.clone()
        } else {
          let param_names: Vec<String> = variant
            .fields
            .iter()
            .map(|f| {
              let ir_type = self.type_name_to_ir_type(&f.type_annotation);
              self.ir_type_to_type_name_string(&ir_type)
            })
            .collect();
          format!("{}_{}", simple_name, param_names.join("_"))
        };

        let wrapper_prev_func = self.builder.get_current_func();
        let wrapper_prev_block = self.builder.get_current_block();

        self._start_function(
          &simple_qname,
          variant_struct_type.clone(),
          Linkage::Internal,
        );
        self.push_scope();

        // Add the same parameters as the variant constructor
        let mut wrapper_args = Vec::new();
        for (field_idx, field) in variant.fields.iter().enumerate() {
          let field_ir_type = self.type_name_to_ir_type(&field.type_annotation);
          let is_str_field = matches!(&field_ir_type, IrType::Struct { name } if name == "str");
          let param_type = if is_str_field {
            IrType::ptr(IrType::struct_ref("str"))
          } else {
            field_ir_type.clone()
          };
          self.builder.add_param(&field.name.node, param_type, None);

          // Get the parameter value for forwarding
          if let Some(func) = self.builder.get_current_func()
            && let Some(fn_def) = self
              .builder
              .module()
              .functions
              .iter()
              .find(|f| f.id == func)
            && let Some(param) = fn_def.params.get(field_idx)
          {
            if is_str_field {
              let loaded = self.builder.load(param.value_id, IrType::struct_ref("str"));
              wrapper_args.push(loaded);
            } else {
              wrapper_args.push(param.value_id);
            }
          }
        }

        // Call the qualified constructor and return its result
        let ctor_result = self.builder.call_named(
          &qualified_ctor_name,
          wrapper_args,
          variant_struct_type.clone(),
        );
        self.builder.ret(ctor_result);

        self.pop_scope();
        self.builder.set_current_func(wrapper_prev_func);
        self.builder.set_current_block(wrapper_prev_block);
      }
    }

    // Generate per-variant enum-to-string debug helper functions
    self.generate_enum_to_string_helper(enum_name, &enum_def.variants);
    // Generate the dispatch function that reads the discriminant at runtime
    // and routes to the correct per-variant formatter (for debug output)
    self.generate_enum_dispatch_fmt(enum_name, &enum_def.variants);
    // Generate the name-only dispatch (for string embedding)
    self.generate_enum_name_dispatch(enum_name, &enum_def.variants);
  }

  /// Generate a helper function that converts an enum value to a debug string.
  ///
  /// For Phase 1, each variant is stored as a named struct `{Enum}__{Variant}`.
  /// The debug output extracts the variant name from the type pattern and formats
  /// the fields inline. This function generates a lookup helper for each variant
  /// that the debug code can call, taking the variant struct and returning a
  /// formatted Ptr(str).
  fn generate_enum_to_string_helper(
    &mut self,
    enum_name: &str,
    variants: &[crate::ast::definitions::EnumVariant],
  ) {
    for variant in variants {
      let variant_type_name = format!("{}__{}", enum_name, variant.name.node);
      let helper_name = format!("__lale_enum_fmt_{}", variant_type_name);

      // Save the current function context
      let prev_func = self.builder.get_current_func();
      let prev_block = self.builder.get_current_block();

      // Start the helper function: takes Ptr(variant_struct), returns str by value.
      self._start_function(&helper_name, IrType::struct_ref("str"), Linkage::Internal);
      self.push_scope();

      // Parameter: pointer to the variant struct value
      let variant_struct_type = IrType::struct_ref(&variant_type_name);
      self
        .builder
        .add_param("val", IrType::ptr(variant_struct_type.clone()), None);

      let func_id = self.builder.get_current_func();
      let param_val = if let Some(func) = func_id
        && let Some(fn_def) = self
          .builder
          .module()
          .functions
          .iter()
          .find(|f| f.id == func)
        && let Some(param) = fn_def.params.first()
      {
        param.value_id
      } else {
        self.pop_scope();
        self.builder.set_current_func(prev_func);
        self.builder.set_current_block(prev_block);
        continue;
      };

      // Load the variant struct from the pointer
      let variant_struct = self.builder.load(param_val, variant_struct_type.clone());

      // Build the formatted string
      let formatted = self.build_variant_format_string(
        &variant_type_name,
        &variant.name.node,
        &variant.fields,
        variant_struct,
      );

      // Return the formatted string by value. Intermediate concat allocas and
      // their .ptr data are freed by the auto-free pass below.
      self.emit_auto_free();
      self.builder.ret(formatted);

      self.pop_scope();

      // Restore previous function context
      self.builder.set_current_func(prev_func);
      self.builder.set_current_block(prev_block);
    }
  }

  /// Generate a per-enum dispatch function that reads the discriminant at runtime
  /// and calls the appropriate per-variant formatter. Each branch returns directly.
  fn generate_enum_dispatch_fmt(
    &mut self,
    enum_name: &str,
    variants: &[crate::ast::definitions::EnumVariant],
  ) {
    let helper_name = format!("__lale_enum_fmt_{}", enum_name);

    // Save the current function context
    let prev_func = self.builder.get_current_func();
    let prev_block = self.builder.get_current_block();

    // Start dispatch function: takes a raw pointer, returns str by value.
    self._start_function(&helper_name, IrType::struct_ref("str"), Linkage::Internal);
    self.push_scope();

    // Parameter: raw pointer to the enum value
    self.builder.add_param("enum_ptr", IrType::raw_ptr(), None);

    let param_val = {
      let func_id = self.builder.get_current_func();
      let mut val = None;
      if let Some(func) = func_id {
        for f in &self.builder.module().functions {
          if f.id == func {
            if let Some(param) = f.params.first() {
              val = Some(param.value_id);
            }
            break;
          }
        }
      }
      match val {
        Some(v) => v,
        None => {
          self.pop_scope();
          self.builder.set_current_func(prev_func);
          self.builder.set_current_block(prev_block);
          return;
        }
      }
    };

    // Read discriminant from field 0 (all variant structs share offset 0 for i64 discriminant).
    // Use the first variant's struct name for field layout lookup.
    let first_variant_name = format!("{}__{}", enum_name, variants[0].name.node);
    let disc_ptr = self
      .builder
      .get_field_ptr(param_val, &first_variant_name, 0u32);
    let disc_val = self.builder.load(disc_ptr, IrType::I64);

    // Build if-else chain: for each variant (except last), compare discriminant
    // and branch to the matching formatter or the next check. Last variant
    // has no comparison — it's the fallthrough.
    for (disc_idx, variant) in variants.iter().enumerate() {
      let variant_type_name = format!("{}__{}", enum_name, variant.name.node);
      let variant_fmt_name = format!("__lale_enum_fmt_{}", variant_type_name);
      let is_last = disc_idx == variants.len() - 1;

      if !is_last {
        let disc_const = self.builder.const_int(IrType::I64, disc_idx as i64);
        let cmp = self.builder.eq(disc_val, disc_const);
        let then_block = self.builder.create_block(format!("enum_fmt_{}", disc_idx));
        let else_block = self
          .builder
          .create_block(format!("enum_fmt_{}_next", disc_idx));
        self.builder.cond_br(cmp, then_block, else_block);

        // Then block: call variant formatter and return its result directly
        self.builder.position_at(then_block);
        let result = self.builder.call_named(
          &variant_fmt_name,
          vec![param_val],
          IrType::struct_ref("str"),
        );
        self.builder.ret(result);

        // Else block: continue to next comparison
        self.builder.position_at(else_block);
      } else {
        // Last variant: no comparison needed, call formatter and return
        let result = self.builder.call_named(
          &variant_fmt_name,
          vec![param_val],
          IrType::struct_ref("str"),
        );
        self.builder.ret(result);
      }
    }

    self.pop_scope();
    self.builder.set_current_func(prev_func);
    self.builder.set_current_block(prev_block);
  }

  /// Generate a per-enum name-only dispatch function that returns just the variant
  /// name (no field data). Used by string embedding `write "{enum_val}"`.
  fn generate_enum_name_dispatch(
    &mut self,
    enum_name: &str,
    variants: &[crate::ast::definitions::EnumVariant],
  ) {
    let helper_name = format!("__lale_enum_name_{}", enum_name);

    let prev_func = self.builder.get_current_func();
    let prev_block = self.builder.get_current_block();

    self._start_function(&helper_name, IrType::struct_ref("str"), Linkage::Internal);
    self.push_scope();

    self.builder.add_param("enum_ptr", IrType::raw_ptr(), None);

    let param_val = {
      let func_id = self.builder.get_current_func();
      let mut val = None;
      if let Some(func) = func_id {
        for f in &self.builder.module().functions {
          if f.id == func {
            if let Some(param) = f.params.first() {
              val = Some(param.value_id);
            }
            break;
          }
        }
      }
      match val {
        Some(v) => v,
        None => {
          self.pop_scope();
          self.builder.set_current_func(prev_func);
          self.builder.set_current_block(prev_block);
          return;
        }
      }
    };

    // Read discriminant from field 0
    let first_variant_name = format!("{}__{}", enum_name, variants[0].name.node);
    let disc_ptr = self
      .builder
      .get_field_ptr(param_val, &first_variant_name, 0u32);
    let disc_val = self.builder.load(disc_ptr, IrType::I64);

    for (disc_idx, variant) in variants.iter().enumerate() {
      let is_last = disc_idx == variants.len() - 1;

      if !is_last {
        let disc_const = self.builder.const_int(IrType::I64, disc_idx as i64);
        let cmp = self.builder.eq(disc_val, disc_const);
        let then_block = self.builder.create_block(format!("enum_name_{}", disc_idx));
        let else_block = self
          .builder
          .create_block(format!("enum_name_{}_next", disc_idx));
        self.builder.cond_br(cmp, then_block, else_block);

        self.builder.position_at(then_block);
        // Return just the variant name as a str value.
        let name_str = self.builder.const_string(&variant.name.node);
        self.builder.ret(name_str);

        self.builder.position_at(else_block);
      } else {
        let name_str = self.builder.const_string(&variant.name.node);
        self.builder.ret(name_str);
      }
    }

    self.pop_scope();
    self.builder.set_current_func(prev_func);
    self.builder.set_current_block(prev_block);
  }

  /// Build a formatted string for a single enum variant: `VariantName` or `VariantName(field0, field1, ...)`.
  /// variant_struct is the loaded struct value of the variant type.
  fn build_variant_format_string(
    &mut self,
    variant_type_name: &str,
    variant_name: &str,
    fields: &[crate::ast::definitions::Parameter],
    variant_struct: ValueId,
  ) -> ValueId {
    if fields.is_empty() {
      // Return the variant name as a str value (no alloca needed).
      return self.builder.const_string(variant_name);
    }

    // Build: name + "(" + field0 + ", " + field1 + ... + ")"
    let name_str = self.builder.const_string(variant_name);
    let name_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(name_str, name_alloc);

    let open_paren = self.builder.const_string("(");
    let open_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(open_paren, open_alloc);

    let mut result = self.builder.concat(name_alloc, open_alloc);

    for (idx, field) in fields.iter().enumerate() {
      if idx > 0 {
        let sep = self.builder.const_string(", ");
        let sep_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(sep, sep_alloc);
        result = self.builder.concat(result, sep_alloc);
      }

      // Extract field value from variant struct (idx+1 skips discriminant field 0).
      // Use extract_field because variant_struct is an in-register value, not a pointer.
      let field_ir_type = self.type_name_to_ir_type(&field.type_annotation);
      let field_value = self.builder.extract_field(
        variant_struct,
        variant_type_name,
        (idx + 1) as u32,
        field_ir_type.clone(),
      );

      // Convert field value to str (Ptr(str))
      let field_str = self.value_to_str_ptr(field_value, &field_ir_type, true);
      result = self.builder.concat(result, field_str);
    }

    // Add closing paren
    let close_paren = self.builder.const_string(")");
    let close_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(close_paren, close_alloc);
    self.builder.concat(result, close_alloc)
  }

  /// Format a vector value (Vec2/Vec3/Vec4) as a string like "vec3(x, y, z)".
  fn format_vec_value(&mut self, value: ValueId, dim: u32, inner: &IrType) -> ValueId {
    // Extract vector components
    let components: Vec<ValueId> = (0..dim)
      .map(|i| self.builder.extract_vec_element(value, i, inner.clone()))
      .collect();

    // Build prefix like "vec2(" or "vec3("
    let prefix_str = match dim {
      2 => "vec2(".to_string(),
      3 => "vec3(".to_string(),
      4 => "vec4(".to_string(),
      _ => "vec?(".to_string(),
    };
    let prefix = self.builder.const_string(&prefix_str);
    let prefix_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(prefix, prefix_alloc);

    let mut result = prefix_alloc;

    // Format each component and join with ", "
    for (i, comp) in components.iter().enumerate() {
      if i > 0 {
        let sep = self.builder.const_string(", ");
        let sep_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(sep, sep_alloc);
        result = self.builder.concat(result, sep_alloc);
      }
      let comp_str = self.value_to_str_ptr(*comp, inner, false);
      result = self.builder.concat(result, comp_str);
    }

    // Closing paren
    let close = self.builder.const_string(")");
    let close_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(close, close_alloc);
    result = self.builder.concat(result, close_alloc);

    // Return as Ptr(str)
    result
  }

  /// Generate a JSON representation of a struct value.
  /// Used for both debug output and string embedding.
  /// `include_types`: if true, adds "type" annotation per field (for debug).
  fn generate_struct_json(
    &mut self,
    struct_value: ValueId,
    type_name: &str,
    include_types: bool,
  ) -> ValueId {
    let layout = match self.type_layouts.get(type_name) {
      Some(l) => l.clone(),
      None => {
        let empty = self.builder.const_string("?");
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(empty, alloc);
        return alloc;
      }
    };

    // Build: {"TypeName": {"field1": {...}, "field2": {...}}}
    // When include_types (debug mode), start with a color reset to cancel any outer
    // bold before structural text, keeping only values bold via per-field markers.
    let open_str = if include_types {
      format!("{}{{\"{}\": {{", self.a_rst_bold(), type_name)
    } else {
      format!("{{\"{}\": {{", type_name)
    };
    let open = self.builder.const_string(&open_str);
    let open_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(open, open_alloc);
    let mut result = open_alloc;

    for (idx, ((field_name, field_type), field_unit)) in layout
      .field_names
      .iter()
      .zip(layout.field_types.iter())
      .zip(layout.field_units.iter())
      .enumerate()
    {
      if idx > 0 {
        let sep = self.builder.const_string(", ");
        let sep_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(sep, sep_alloc);
        result = self.builder.concat(result, sep_alloc);
      }

      let field_value =
        self
          .builder
          .extract_field(struct_value, type_name, idx as u32, field_type.clone());

      // Build the field JSON: {"fieldName": {"value": ..., ...}}
      let field_open = self
        .builder
        .const_string(format!("{{\"{}\": {{", field_name));
      let field_open_alloc = self.builder.alloca(IrType::struct_ref("str"));
      self.builder.store(field_open, field_open_alloc);
      result = self.builder.concat(result, field_open_alloc);

      // "value": <formatted>
      let value_label = self.builder.const_string("\"value\": ");
      let value_label_alloc = self.builder.alloca(IrType::struct_ref("str"));
      self.builder.store(value_label, value_label_alloc);
      result = self.builder.concat(result, value_label_alloc);

      // Format the field value based on its type
      let formatted_val = if let IrType::Struct { name } = field_type {
        if name == "str" {
          // String field: wrap the str value in quotes
          let quote_open = self.builder.const_string("\"");
          let quote_open_alloc = self.builder.alloca(IrType::struct_ref("str"));
          self.builder.store(quote_open, quote_open_alloc);
          // Deep-copy the field's data so concat frees the copy, not the
          // live field value stored in the parent struct.
          let field_copy = self.builder.copy_str(field_value);
          let str_alloc = self.builder.alloca(IrType::struct_ref("str"));
          self.builder.store(field_copy, str_alloc);
          let quote_close = self.builder.const_string("\"");
          let quote_close_alloc = self.builder.alloca(IrType::struct_ref("str"));
          self.builder.store(quote_close, quote_close_alloc);
          let with_open = self.builder.concat(quote_open_alloc, str_alloc);
          self.builder.concat(with_open, quote_close_alloc)
        } else if self.enum_types.contains(name) || self.type_layouts.contains_key(name) {
          // Nested composite type: recurse
          self.generate_struct_json(field_value, name, include_types)
        } else {
          self.value_to_str_ptr(field_value, field_type, include_types)
        }
      } else {
        self.value_to_str_ptr(field_value, field_type, include_types)
      };

      // In debug mode, wrap only the data value with bold markers so
      // structural JSON (field names, brackets) stays non-bold cyan.
      if include_types {
        let bold_on = self.builder.const_string(self.a_bold());
        let bold_on_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(bold_on, bold_on_alloc);
        result = self.builder.concat(result, bold_on_alloc);
        result = self.builder.concat(result, formatted_val);
        let bold_off = self.builder.const_string(self.a_rst_bold());
        let bold_off_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(bold_off, bold_off_alloc);
        result = self.builder.concat(result, bold_off_alloc);
      } else {
        result = self.builder.concat(result, formatted_val);
      }

      // "type": "..." (debug only)
      if include_types {
        let type_sep = self.builder.const_string(", ");
        let type_sep_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(type_sep, type_sep_alloc);
        result = self.builder.concat(result, type_sep_alloc);

        let type_label = self.builder.const_string("\"type\": \"");
        let type_label_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(type_label, type_label_alloc);
        result = self.builder.concat(result, type_label_alloc);

        let type_name_str = self.ir_type_to_type_name_string(field_type);
        let type_val = self.builder.const_string(&type_name_str);
        let type_val_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(type_val, type_val_alloc);
        result = self.builder.concat(result, type_val_alloc);

        let type_close = self.builder.const_string("\"");
        let type_close_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(type_close, type_close_alloc);
        result = self.builder.concat(result, type_close_alloc);
      }

      // "unit": "..." (if present, both modes)
      if let Some(unit_str) = field_unit {
        let unit_sep = self.builder.const_string(", ");
        let unit_sep_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(unit_sep, unit_sep_alloc);
        result = self.builder.concat(result, unit_sep_alloc);

        let unit_label = self.builder.const_string("\"unit\": \"");
        let unit_label_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(unit_label, unit_label_alloc);
        result = self.builder.concat(result, unit_label_alloc);

        let unit_val = self.builder.const_string(unit_str);
        let unit_val_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(unit_val, unit_val_alloc);
        result = self.builder.concat(result, unit_val_alloc);

        let unit_close = self.builder.const_string("\"");
        let unit_close_alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(unit_close, unit_close_alloc);
        result = self.builder.concat(result, unit_close_alloc);
      }

      // Close the field object: }}
      let field_close = self.builder.const_string("}}");
      let field_close_alloc = self.builder.alloca(IrType::struct_ref("str"));
      self.builder.store(field_close, field_close_alloc);
      result = self.builder.concat(result, field_close_alloc);
    }

    // Close the type and outer object: }}
    let type_close = self.builder.const_string("}}");
    let type_close_alloc = self.builder.alloca(IrType::struct_ref("str"));
    self.builder.store(type_close, type_close_alloc);
    self.builder.concat(result, type_close_alloc)
  }

  /// Convert an IR value to a Ptr(str) for string concatenation.
  /// `include_types`: if true, adds type annotation for composite fields (debug mode).
  fn value_to_str_ptr(
    &mut self,
    value: ValueId,
    value_type: &IrType,
    include_types: bool,
  ) -> ValueId {
    match value_type {
      IrType::Struct { name } if name == "str" => {
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(value, alloc);
        alloc
      }
      IrType::Ptr(_) => value,
      IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64 => {
        let str_val = self.call_int_to_str(value, value_type, false);
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(str_val, alloc);
        alloc
      }
      IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
        let str_val = self.call_int_to_str(value, value_type, true);
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(str_val, alloc);
        alloc
      }
      IrType::F16 | IrType::F32 | IrType::F64 => {
        let f64_val = if *value_type == IrType::F64 {
          value
        } else {
          self.builder.fp_ext(value, IrType::F64)
        };
        let str_val = self.builder.call_named(
          "__lale_f64_to_str_f64",
          vec![f64_val],
          IrType::struct_ref("str"),
        );
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(str_val, alloc);
        alloc
      }
      IrType::Bool => {
        let str_val = self.builder.call_named(
          "__lale_bool_to_str_bool",
          vec![value],
          IrType::struct_ref("str"),
        );
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(str_val, alloc);
        alloc
      }
      IrType::Char => {
        let i32_val = self.builder.bitcast(value, IrType::I32);
        let str_val = self.builder.call_named(
          "__lale_char_to_str_char",
          vec![i32_val],
          IrType::struct_ref("str"),
        );
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(str_val, alloc);
        alloc
      }
      IrType::Struct { name } if self.enum_types.contains(name) => {
        // Enum value — call the enum formatter (returns str by value).
        let enum_alloc = self.builder.alloca(value_type.clone());
        self.builder.store(value, enum_alloc);
        let dispatch_name = format!("__lale_enum_fmt_{}", name);
        self
          .builder
          .call_named(&dispatch_name, vec![enum_alloc], IrType::struct_ref("str"))
      }
      IrType::Vec2(inner) | IrType::Vec3(inner) | IrType::Vec4(inner) => {
        // Format vector as vecN(x, y, [z, [w]])
        let dim = match value_type {
          IrType::Vec2(_) => 2,
          IrType::Vec3(_) => 3,
          IrType::Vec4(_) => 4,
          _ => unreachable!(),
        };
        self.format_vec_value(value, dim, inner)
      }
      _ => {
        // Composite type: generate JSON representation
        if let IrType::Struct { name } = value_type
          && self.type_layouts.contains_key(name)
          && name != "str"
        {
          return self.generate_struct_json(value, name, include_types);
        }
        let empty = self.builder.const_string("?");
        let alloc = self.builder.alloca(IrType::struct_ref("str"));
        self.builder.store(empty, alloc);
        alloc
      }
    }
  }

  /// Initialize an array allocation with values from an array literal.
  fn initialize_array_from_literal(
    &mut self,
    array_ptr: ValueId,
    array_type: &IrType,
    arr_lit: &ArrayLiteral,
  ) -> CompileResult<()> {
    // Handle fill pattern if present: [fill with value]
    if let Some(value_expr) = &arr_lit.fill {
      // Get element type for generating fill values
      let elem = if let IrType::Array { element, .. } = array_type {
        element.as_ref()
      } else {
        return Err("initialize_array_from_literal: expected Array type".into());
      };
      let value = self.generate_expr_with_resolved_type(value_expr, Some(elem));

      // Count comes from the array type annotation, not from the fill syntax.
      if let IrType::Array { element, size } = array_type {
        let elem_size = self.get_type_size(element);
        for i in 0..*size {
          let offset = self
            .builder
            .const_int(IrType::I64, (i as i64) * (elem_size as i64));
          let elem_ptr = self.builder.add(array_ptr, offset, IrType::I64);
          self.builder.store(value, elem_ptr);
        }
      }
      return Ok(());
    }

    // Regular array literal: [e1, e2, e3, ...]
    if let IrType::Array { element, size } = array_type {
      let elem_size = self.compute_ir_type_size(element);

      for (index, elem_expr) in arr_lit.elements.iter().enumerate() {
        if index >= *size as usize {
          break; // Don't write past the array bounds
        }

        // Calculate offset: index * element_size
        let offset_val = self
          .builder
          .const_int(IrType::I64, (index as i64) * (elem_size as i64));

        // Get pointer to element: array_ptr + offset
        let elem_ptr = self.builder.add(array_ptr, offset_val, IrType::I64);

        // Check if element is itself an array (nested array literal)
        if let IrType::Array { .. } = &**element {
          // For nested arrays, recursively initialize the element
          if let Expr::ArrayLiteral(inner_lit) = elem_expr {
            self.initialize_array_from_literal(elem_ptr, element, inner_lit)?;
          } else {
            // If not an array literal, generate and store as usual
            let elem_val = self.generate_expr_with_resolved_type(elem_expr, Some(element));
            self.builder.store(elem_val, elem_ptr);
          }
        } else {
          // For scalar elements, generate value and store
          let mut elem_val = self.generate_expr_with_resolved_type(elem_expr, Some(element));

          // Cast value to target element type if needed
          let val_type = self.get_value_type(elem_val);
          if val_type != **element {
            // For simple literals, we can just cast if the value represents a number
            if matches!(
              **element,
              IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64
            ) {
              // Cast unsigned literals to signed types as needed
              elem_val = self.builder.bitcast(elem_val, (**element).clone());
            }
          }

          // Store the element value
          self.builder.store(elem_val, elem_ptr);
        }
      }
    }

    Ok(())
  }

  /// Infer the array type from a nested array literal structure.
  /// For [[1,2], [3,4]], returns [2 x i32] (inner array type)
  fn infer_array_type_from_literal(&self, arr_lit: &ArrayLiteral) -> IrType {
    if arr_lit.elements.is_empty() {
      return IrType::I32; // Default fallback
    }

    if let Expr::ArrayLiteral(inner_lit) = &arr_lit.elements[0] {
      // Recursively infer for nested arrays
      let inner_type = self.infer_array_type_from_literal(inner_lit);
      IrType::array(inner_type, arr_lit.elements.len() as u64)
    } else {
      // Base case: scalar element type
      // Try to infer from first element
      match &arr_lit.elements[0] {
        Expr::IntLiteral(_) => IrType::array(IrType::I32, arr_lit.elements.len() as u64),
        Expr::UintLiteral(_) => IrType::array(IrType::U32, arr_lit.elements.len() as u64),
        Expr::FloatLiteral(_) => IrType::array(IrType::F64, arr_lit.elements.len() as u64),
        Expr::BoolLiteral(_) => IrType::array(IrType::Bool, arr_lit.elements.len() as u64),
        _ => IrType::array(IrType::I32, arr_lit.elements.len() as u64),
      }
    }
  }

  /// Generate code for an array literal expression.
  /// Returns a pointer to the array data.
  fn generate_array_literal(
    &mut self,
    arr_lit: &ArrayLiteral,
    resolved_type: Option<&IrType>,
  ) -> ValueId {
    // Handle fill pattern if present: [fill with value]
    if let Some(value_expr) = &arr_lit.fill {
      return self.generate_array_fill(value_expr, resolved_type);
    }

    // Regular array literal: [e1, e2, e3, ...]
    if arr_lit.elements.is_empty() {
      return self.builder.const_null();
    }

    // Determine the array element type:
    // 1. From resolved_type if it's an Array (preferred — carries type context)
    // 2. From the first element's type (fallback)
    let elem_type = if let Some(IrType::Array { element, .. }) = resolved_type {
      element.as_ref().clone()
    } else if let Expr::ArrayLiteral(inner_lit) = &arr_lit.elements[0] {
      self.infer_array_type_from_literal(inner_lit)
    } else {
      // For scalar elements, generate to get the type
      let first_elem = self.generate_expr_with_resolved_type(&arr_lit.elements[0], resolved_type);
      self.get_value_type(first_elem)
    };

    // Create an array type for the literal
    let array_size = arr_lit.elements.len() as u64;
    let array_type = IrType::array(elem_type, array_size);
    let array_type_copy = array_type.clone();

    // Allocate space for the array
    let array_ptr = self.builder.alloca_named(array_type, "array_literal");

    // Initialize the array (ignore result, if there's an error, initialization just won't complete)
    let _ = self.initialize_array_from_literal(array_ptr, &array_type_copy, arr_lit);

    // Return pointer to the array
    array_ptr
  }

  /// Generate code for fill pattern: [value fill count]
  fn generate_array_fill(&mut self, value_expr: &Expr, resolved_type: Option<&IrType>) -> ValueId {
    let elem_type = if let Some(IrType::Array { element, .. }) = resolved_type {
      element.as_ref().clone()
    } else {
      IrType::I64
    };
    let value = self.generate_expr_with_resolved_type(value_expr, Some(&elem_type));

    // Count comes from the resolved array type, not from the fill syntax.
    let count = if let Some(IrType::Array { size, .. }) = resolved_type {
      *size
    } else {
      1
    };

    // Create array type
    let array_type = IrType::array(elem_type.clone(), count);

    // Allocate array
    let array_ptr = self.builder.alloca_named(array_type, "array_fill");

    // Initialize each element with the same value
    let elem_size = self.get_type_size(&elem_type);
    for i in 0..count {
      let offset = self
        .builder
        .const_int(IrType::I64, (i as i64) * (elem_size as i64));
      let elem_ptr = self.builder.add(array_ptr, offset, IrType::I64);
      self.builder.store(value, elem_ptr);
    }

    array_ptr
  }

  /// Get the size in bytes of a type
  fn get_type_size(&self, ty: &IrType) -> u64 {
    match ty {
      IrType::I8 | IrType::U8 | IrType::Bool => 1,
      IrType::I16 | IrType::U16 => 2,
      IrType::I32 | IrType::U32 | IrType::F32 => 4,
      IrType::I64 | IrType::U64 | IrType::F64 | IrType::Ptr(_) => 8,
      IrType::F16 => 2,
      _ => 8, // Default
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_collect_stdlib_nonexistent_path() {
    let mut statements = Vec::new();
    let result = IrGenerator::collect_stdlib_modules_impl(
      std::path::Path::new("/nonexistent/path/to/stdlib.lale"),
      &mut statements,
      &mut std::collections::HashSet::new(),
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
      err.contains("Cannot access stdlib path"),
      "Expected access error, got: {}",
      err
    );
  }

  #[test]
  fn test_collect_stdlib_valid_file() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("test.lale");
    std::fs::write(&file, "var x as i32 = 1\n").unwrap();

    let mut statements = Vec::new();
    let result = IrGenerator::collect_stdlib_modules_impl(
      &file,
      &mut statements,
      &mut std::collections::HashSet::new(),
    );
    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
  }
}
