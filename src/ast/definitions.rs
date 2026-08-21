//! Abstract Syntax Tree (AST) type definitions for the Lale programming language.
//!
//! This module defines the semantic structure of Lale programs. Each type
//! corresponds to a grammar construct (statements, expressions, types, etc.)
//! and provides a clean, typed representation for compiler passes.
//!
//! The AST is built from the pest parse tree by `ast::builder::build_program()`
//! and can be traversed using the `AstVisitor` trait.
//!
//! Note that the struct Program **is** the root of the AST.
//! We call it Program instead of AST, because it matches the program rule
//! in the grammar, which is the top-level rule.
//!
//! ## Location Tracking with `Spanned<T>`
//!
//! The AST uses the [`Spanned<T>`] wrapper type to associate source locations
//! with individual tokens and values. This provides:
//!
//! - **Type safety**: Location information is part of the type, making it
//!   impossible to forget tracking a location.
//! - **Consistency**: All located values use the same pattern.
//! - **Clarity**: `Spanned<String>` clearly indicates a string with location.
//!
//! ### When to use Spanned vs plain location field
//!
//! - Use `Spanned<T>` for individual tokens: identifiers, operators, literals
//! - Use plain `location: SourceLocation` for compound nodes that span multiple
//!   tokens (like the overall statement location)
//!
//! ## Symbol Table Linkage
//!
//! Function definitions (`FnDefStmt`) contain a `symbol_table` field that is
//! populated during semantic analysis. This links AST nodes directly to their
//! corresponding symbol information, enabling code generation without needing
//! to carry the semantic analyzer around.

use std::cell::RefCell;
use std::collections::HashMap;

// Re-export location types from parent module
pub use super::{SourceLocation, Spanned};

// ==================== SYMBOL TABLE TYPES ====================
// These types define the symbol table structure. They are defined here in ast.rs
// to allow FnDefStmt to reference SymbolTable without circular dependencies.
// The semantic_analyzer module re-exports these types.

/// Linkage determines symbol visibility across compilation units.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Linkage {
  /// Symbol is only visible within the current compilation unit.
  #[default]
  Internal,
  /// Symbol is imported from another compilation unit.
  Import,
  /// Symbol is exported to other compilation units.
  Export,
}

/// Visibility determines access control within the source code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Visibility {
  /// Symbol is accessible from anywhere.
  #[default]
  Public,
  /// Symbol is only accessible within the defining module.
  Private,
}

/// Storage class determines how and where a variable is stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StorageClass {
  /// Default single-threaded storage.
  #[default]
  Default,
  /// Thread-local storage (for future multi-threading support).
  ThreadLocal,
}

/// Distinguishes between variable and function symbols.
#[derive(Debug, Clone)]
pub enum SymbolKind {
  Variable,
  Parameter,
  Function,
  TypeDef,
}

/// A symbol entry in the symbol table.
#[derive(Debug, Clone)]
pub struct Symbol {
  pub source_location: SourceLocation,
  pub data_type: String,
  pub physical_unit: Option<String>,
  pub kind: SymbolKind,
  pub linkage: Linkage,
  pub visibility: Visibility,
  pub storage_class: StorageClass,
  pub is_definition: bool,
  pub is_initialized: bool,
  /// For pointers: the type that this pointer points to (if known).
  /// For non-pointers: None.
  /// Used for compile-time type checking of `unsafe cast value at ptr`.
  pub pointer_to_type: Option<String>,
  /// Module path where this symbol was defined (empty string for current file).
  pub module_path: String,
}

/// A symbol table mapping names to symbols.
pub type SymbolTable = HashMap<String, Symbol>;

// ==================== AST TYPES ====================

/// A complete Lale program.
///
/// The `global_symbol_table` field is populated during semantic analysis and contains
/// all global symbols (top-level variables and functions).
#[derive(Debug, Clone)]
pub struct Program {
  pub statements: Vec<Stmt>,
  pub location: SourceLocation,
  /// Global symbol table. Populated during semantic analysis.
  /// Uses RefCell to allow mutation during analysis while AST is borrowed immutably.
  pub global_symbol_table: RefCell<Option<SymbolTable>>,
}

/// Statement types in Lale.
#[derive(Debug, Clone)]
pub enum Stmt {
  /// Module use statement: `use math.lib: add`
  Use(UseStmt),
  /// Type definition: `type Person ... end type`
  TypeDef(TypeDefStmt),
  /// Enum definition: `enum Shape Circle(f64) Point end enum`
  EnumDef(EnumDefStmt),
  /// Variable definition: `var x as u8 = 5`
  VarDef(VarDefStmt),
  /// Unsafe variable declaration: `unsafe decl x as u8`
  UnsafeDecl(UnsafeDeclStmt),
  /// Assignment to existing variable: `x = 5`
  Assign(AssignStmt),
  /// Compound assignment: `x += 5`
  CompoundAssign(CompoundAssignStmt),
  /// Assignment through pointer: `value at ptr = value`
  ValueAtAssign(ValueAtAssignStmt),
  /// Function definition: `fn foo() returns nothing ... end fn`
  FnDef(FnDefStmt),
  /// Function signature (declaration without body)
  FnSignature(FnSignatureStmt),
  /// Function call as statement: `foo()`
  FnCall(FnCall),
  /// If statement: `if ... end if`
  If(IfStmt),
  /// Switch statement: exhaustive pattern matching over enums
  Switch(SwitchStmt),
  /// Loop statement: `loop ... end loop`
  Loop(Box<LoopStmt>),
  /// Return statement: `return expr`
  Return(ReturnStmt),
  /// Exit program statement: `exit program`
  ExitProgram(ExitProgramStmt),
  /// Exit loop statement: `exit loop`
  ExitLoop(ExitLoopStmt),
  /// Rewind statement: `rewind`
  Rewind(RewindStmt),
  /// Write to stdout: `write expr`
  Stdout(StdoutStmt),
  /// Write to stderr: `warn expr`
  Stderr(StderrStmt),
  /// Debug output to stderr: `debug expr`
  Debug(DebugStmt),
  /// Read from stdin: `read var`
  Stdin(StdinStmt),
  /// Compile-time if: `#if ... #end if`
  CtIf(CtIfStmt),
  /// Compile-time fail: `#fail "message"`
  CtFail(CtFailStmt),
  /// Compile-time warn: `#warn "message"`
  CtWarn(CtWarnStmt),
  /// Compile-time when: `#when ... #end when`
  CtWhen(CtWhenStmt),
  /// Compile-time match: `#match ... #end match`
  CtMatch(CtMatchStmt),
  /// Compile-time switch: `#switch ... #end switch`
  CtSwitch(CtSwitchStmt),
  Assert(AssertStmt),
  /// Documentation comment
  Doc(DocStmt),
  /// Regular comment
  Comment(CommentStmt),
  /// Add error to error stack: `add error expr`
  AddError(AddErrorStmt),
  /// Write error messages to stdout: `write error messages`
  WriteErrors(WriteErrorsStmt),
  /// Write error messages to stderr: `warn error messages`
  WarnErrors(WarnErrorsStmt),
  /// Alert error messages: `alert error messages`
  AlertErrors(AlertErrorsStmt),
  /// Alert output: `alert expr` / `alert inline expr`
  Alert(AlertStmt),
  /// When statement: one-sided action, no else
  When(WhenStmt),
  /// Match statement: conditional branching with multiple arms
  Match(MatchStmt),
  /// Move on: intentional no-op placeholder
  MoveOn(MoveOnStmt),
  /// Missing code: deferred implementation placeholder
  MissingCode(MissingCodeStmt),
  /// Release heap memory: `release ptr_expr`
  Release(ReleaseStmt),
  /// On-exit deferred statement: `on exit <stmt>`
  OnExit(OnExitStmt),
  /// Test suite: `test suite <name> ... end test suite`
  TestSuite(TestSuiteStmt),
}

/// Release statement: `release pointer_expr`
#[derive(Debug, Clone)]
pub struct ReleaseStmt {
  /// The pointer expression to release
  pub pointer: Expr,
  /// Location spanning the entire statement
  pub location: SourceLocation,
  /// Comments attached to this statement
  pub comments: AttachedComments,
}

/// On-exit statement: `on exit <deferred_stmt>`
#[derive(Debug, Clone)]
pub struct OnExitStmt {
  /// The statement to execute at function exit
  pub body: Box<Stmt>,
  /// Location spanning the entire statement
  pub location: SourceLocation,
}

/// Module use statement.
///
/// Imports symbols from another Lale module at compile time.
/// This is distinct from the `import` modifier which marks FFI/extern symbols.
///
/// # Example
///
/// ```lale
/// use math.lib: add, subtract
/// use ../sibling: name
/// use utils/log: *
/// ```
#[derive(Debug, Clone)]
pub struct UseStmt {
  /// The parsed module path, broken into segments.
  /// E.g. "math.lib" -> ["math", "lib"]; "../sibling" -> ["..", "sibling"].
  pub module_path: Vec<Spanned<String>>,
  /// Whether this is a relative path (starts with "..").
  pub is_relative: bool,
  /// Imported symbol names: identifiers or "*" for glob.
  pub imports: UseImports,
  /// Location spanning the entire use statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Specifies which symbols to import from a module.
#[derive(Debug, Clone)]
pub enum UseImports {
  /// Import all exports: `use foo` (no colon, default behavior)
  All,
  /// Import specific symbols: `use foo: a, b, c`
  Named(Vec<Spanned<String>>),
}

/// Type definition.
///
/// A type is a composite type that groups multiple typed fields together.
/// Types bundle related data without methods or inheritance (initial implementation).
///
/// # Example
///
/// ```lale
/// type Person
///     name as str
///     age as i32
/// end type
///
/// export type Point
///     x as f64
///     y as f64
/// end type
/// ```
#[derive(Debug, Clone)]
pub struct TypeDefStmt {
  /// Whether this type is exported to other modules.
  pub is_export: bool,
  /// The type name with its source location.
  pub name: Spanned<String>,
  /// The fields in this type.
  pub fields: Vec<TypeField>,
  /// Location spanning the entire type definition.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Enum definition.
///
/// An enum is a tagged union type where variants can carry typed fields.
/// Each variant maps a name to an optional set of typed fields.
///
/// # Example
///
/// ```lale
/// enum Shape
///     Circle(f64)
///     Rectangle(f64, f64)
///     Point
/// end enum
/// ```
#[derive(Debug, Clone)]
pub struct EnumDefStmt {
  /// Whether this enum is exported to other modules.
  pub is_export: bool,
  /// The enum name with its source location.
  pub name: Spanned<String>,
  /// The variants of this enum.
  pub variants: Vec<EnumVariant>,
  /// Location spanning the entire enum definition.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// A single variant in an enum definition.
///
/// Each variant has a name and optional typed fields.
/// The discriminant is the 0-based position in the variant list.
///
/// # Example
///
/// ```lale
/// Circle(f64)       // name="Circle", fields=[field0: f64]
/// Rectangle(f64, f64) // name="Rectangle", fields=[field0: f64, field1: f64]
/// Point            // name="Point", fields=[]
/// ```
#[derive(Debug, Clone)]
pub struct EnumVariant {
  /// The variant name with its source location.
  pub name: Spanned<String>,
  /// The fields of this variant (typed but unnamed; auto-named field0, field1, ...).
  pub fields: Vec<Parameter>,
  /// Location spanning this variant definition.
  pub location: SourceLocation,
}

/// A field in a type definition.
#[derive(Debug, Clone)]
pub struct TypeField {
  /// Whether this field is private (only accessible within its defining module).
  pub is_private: bool,
  /// The field name with its source location.
  pub name: Spanned<String>,
  /// The type of this field.
  pub field_type: TypeName,
  /// Optional physical unit for this field.
  pub unit: Option<Unit>,
  /// Location spanning this field definition.
  pub location: SourceLocation,
  /// Comments attached to this field.
  pub comments: AttachedComments,
}

/// Variable definition (with initialization).
///
/// Represents a `var` statement that declares and initializes a variable.
///
/// # Example
///
/// ```lale
/// var x as u32 = 42
/// var velocity as f64 in <m/s> = 9.8
/// export var PI as f64 = 3.14159
/// ```
#[derive(Debug, Clone)]
pub struct VarDefStmt {
  /// Whether this definition is exported to other modules.
  pub is_export: bool,
  /// Whether this definition is imported from another module.
  pub is_import: bool,
  /// True when this is a synthetic loop-variable definition. Such a variable
  /// is function-local even at the top level (it lives in the synthetic main),
  /// not a global.
  pub is_loop_var: bool,
  /// The variable name with its source location.
  pub name: Spanned<String>,
  /// Optional type annotation (inferred if not present).
  pub type_annotation: Option<TypeName>,
  /// Optional physical unit for dimensional analysis.
  pub unit: Option<Unit>,
  /// The initialization expression.
  pub value: Expr,
  /// Location spanning the entire definition statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Unsafe variable declaration (no initialization).
///
/// This is explicitly marked unsafe because uninitialized variables are dangerous.
/// The programmer must explicitly acknowledge the risk.
///
/// # Example
///
/// ```lale
/// unsafe decl buffer as u8[1024]
/// ```
#[derive(Debug, Clone)]
pub struct UnsafeDeclStmt {
  /// Whether this declaration is exported to other modules.
  pub is_export: bool,
  /// Whether this declaration is imported from another module.
  pub is_import: bool,
  /// The variable name with its source location.
  pub name: Spanned<String>,
  /// Optional type annotation.
  pub type_annotation: Option<TypeName>,
  /// Optional physical unit for dimensional analysis.
  pub unit: Option<Unit>,
  /// Location spanning the entire declaration statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Assignment to an existing variable.
///
/// # Example
///
/// ```lale
/// x = 42
/// obj.field = "hello"
/// arr[0] = 1
/// ```
#[derive(Debug, Clone)]
pub struct AssignStmt {
  /// The assignment target path (e.g., ["obj", "field"] for obj.field).
  /// The span covers the first identifier in the path.
  pub target: Spanned<Vec<String>>,
  /// Array indices for indexed assignment (e.g., arr[i][j]).
  pub indices: Vec<Expr>,
  /// The value being assigned.
  pub value: Expr,
  /// Location spanning the entire assignment statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compound assignment operators.
#[derive(Debug, Clone, PartialEq)]
pub enum CompoundOp {
  /// Addition assignment: `+=`
  AddAssign,
  /// Subtraction assignment: `-=`
  SubAssign,
  /// Multiplication assignment: `*=`
  MulAssign,
  /// Division assignment: `/=`
  DivAssign,
  /// Modulo assignment: `%=`
  ModAssign,
}

/// Compound assignment to an existing variable.
///
/// # Example
///
/// ```lale
/// x += 5
/// counter -= 1
/// ```
#[derive(Debug, Clone)]
pub struct CompoundAssignStmt {
  /// The assignment target path with its source location.
  pub target: Spanned<Vec<String>>,
  /// Array indices for indexed assignment.
  pub indices: Vec<Expr>,
  /// The compound operator with its source location.
  pub operator: Spanned<CompoundOp>,
  /// The value being combined.
  pub value: Expr,
  /// Location spanning the entire compound assignment statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Assignment through a pointer: `value at ptr = value`.
///
/// Writes a value to a memory location pointed to by an expression.
///
/// # Example
///
/// ```lale
/// var p as pointer = pointer to x
/// value at p = 42
/// value at (buf + 1 as u64) = (0 as i8)
/// ```
#[derive(Debug, Clone)]
pub struct ValueAtAssignStmt {
  /// The pointer expression (can be complex like ptr + offset).
  pub pointer: Expr,
  /// The value being written to memory.
  pub value: Expr,
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Function definition.
///
/// # Example
///
/// ```lale
/// fn add(a as i32, b as i32) returns i32
///     return a + b
/// end fn
/// ```
#[derive(Debug, Clone)]
pub struct FnDefStmt {
  /// Whether this function is exported to other modules.
  pub is_export: bool,
  /// The function name with its source location.
  pub name: Spanned<String>,
  /// Function parameters.
  pub parameters: Vec<Parameter>,
  /// Return type specification.
  pub return_type: ReturnType,
  /// Optional physical unit for the return value.
  pub return_unit: Option<Unit>,
  /// Function body statements.
  pub body: Vec<Stmt>,
  /// Location spanning the entire function definition.
  pub location: SourceLocation,
  /// Symbol table for this function's scope. Populated during semantic analysis.
  /// Uses RefCell to allow mutation during analysis while AST is borrowed immutably.
  pub symbol_table: RefCell<Option<SymbolTable>>,
  /// Comments attached to this function.
  pub comments: AttachedComments,
}

/// Function signature (declaration without body).
///
/// Used for forward declarations and external C function declarations.
/// Function signatures cannot be exported; to export a function, use `fn var` with a body.
///
/// # Example
///
/// ```lale
/// fn signature add(x as i32, y as i32) returns i32
/// import fn signature sqrt(x as f64) returns f64
/// ```
#[derive(Debug, Clone)]
pub struct FnSignatureStmt {
  /// Whether this signature is imported from external linkage (C library, etc).
  /// Imported signatures are resolved at link time by the linker.
  pub is_import: bool,
  /// The function name with its source location.
  pub name: Spanned<String>,
  /// Function parameters.
  pub parameters: Vec<Parameter>,
  /// Return type specification.
  pub return_type: ReturnType,
  /// Optional physical unit for the return value.
  pub return_unit: Option<Unit>,
  /// Location spanning the entire function signature.
  pub location: SourceLocation,
  /// Comments attached to this signature.
  pub comments: AttachedComments,
}

/// Function call (both statement and expression forms).
#[derive(Debug, Clone)]
pub struct FnCall {
  /// Target function path (e.g., ["module", "func"] for module.func).
  /// Single-element for simple function calls.
  pub target: Spanned<Vec<String>>,
  /// Function call arguments.
  pub arguments: Vec<Expr>,
  /// Optional unit annotation (e.g., vec3(1,2,3)<m>).
  pub unit: Option<Unit>,
  /// Location spanning the entire function call.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
  /// Inline comments (for pretty printing).
  pub inline_comments: Vec<AttachedComment>,
}

/// If statement.
///
/// # Example
///
/// ```lale
/// if x > 0
///     write "positive"
/// else if x < 0
///     write "negative"
/// else
///     write "zero"
/// end if
/// ```
#[derive(Debug, Clone)]
pub struct IfStmt {
  /// The condition to evaluate.
  pub condition: Condition,
  /// Statements in the `then` branch.
  pub then_branch: Vec<Stmt>,
  /// Conditions and statements in `else if` branches.
  pub else_if_branches: Vec<(Condition, Vec<Stmt>)>,
  /// Statements in the `else` branch (if present).
  pub else_branch: Option<Vec<Stmt>>,
  /// Location spanning the entire if statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// When statement: a one-sided action or guard.
///
/// Unlike `if`, `when` has no `else` branch — there is no "No" side.
/// The `else_branch` field exists only so the parser can accept a mistaken
/// `else` and the semantic analyzer can produce a helpful error message.
///
/// # Example
///
/// ```lale
/// when battery_level < 10
///     write "Warning: Please plug in your charger!"
/// end when
/// ```
#[derive(Debug, Clone)]
pub struct WhenStmt {
  /// The condition to evaluate.
  pub condition: Condition,
  /// Statements in the body.
  pub body: Vec<Stmt>,
  /// Erroneous else branch — rejected by the semantic analyzer with a helpful message.
  pub else_branch: Option<Vec<Stmt>>,
  /// Location spanning the entire when statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Move on: intentional no-op placeholder.
///
/// Signals that a code path was considered and deliberately left empty.
#[derive(Debug, Clone)]
pub struct MoveOnStmt {
  /// Location in source code.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Missing code: deferred implementation placeholder.
///
/// Signals that a code path is acknowledged but not yet implemented.
/// In debug mode this emits a runtime warning; in release mode the
/// compiler rejects it as an error.
#[derive(Debug, Clone)]
pub struct MissingCodeStmt {
  /// Location in source code.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Switch statement: exhaustive pattern matching over enums.
///
/// # Example
///
/// ```lale
/// switch s
///     case Circle(r):
///         write "radius: {r}"
///     case Rectangle(w, h):
///         write "{w} x {h}"
///     default:
///         write "unknown shape"
/// end switch
/// ```
#[derive(Debug, Clone)]
pub struct SwitchStmt {
  /// The expression to match against (must evaluate to an enum type).
  pub value: Expr,
  /// The cases of the switch (one per variant).
  pub cases: Vec<SwitchCase>,
  /// Optional default case for catching unmatched cases.
  pub default_case: Option<Vec<Stmt>>,
  /// Location of the `default` keyword (if present).
  pub default_location: Option<SourceLocation>,
  /// Location spanning the entire switch statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// A single case in a switch statement.
///
/// Each case matches a specific variant and binds its fields.
#[derive(Debug, Clone)]
pub struct SwitchCase {
  /// The pattern to match against (variant name + field bindings).
  pub pattern: SwitchPattern,
  /// The body of this case.
  pub body: Vec<Stmt>,
  /// Location spanning this case.
  pub location: SourceLocation,
}

/// A pattern in a switch case: either an enum variant or a literal value.
///
/// # Example
///
/// ```lale
/// Circle(r)       // Enum { variant_name: "Circle", fields: [Bind("r")] }
/// Point           // Enum { variant_name: "Point", fields: [] }
/// Rectangle(_, _) // Enum { variant_name: "Rectangle", fields: [Discard, Discard] }
/// 200             // Literal { value: IntLiteral(200) }
/// "Saturday"      // Literal { value: StringLiteral("Saturday") }
/// ```
#[derive(Debug, Clone)]
pub enum SwitchPattern {
  /// An enum variant pattern: `Circle(r)`, `Point`, `Rectangle(_, _)`.
  Enum {
    /// The variant name being matched.
    variant_name: Spanned<String>,
    /// Field bindings/discards for the variant's fields.
    fields: Vec<SwitchPatternField>,
    /// Location spanning this pattern.
    location: SourceLocation,
  },
  /// A literal value pattern: `200`, `"Saturday"`, `true`, `'x'`.
  Literal {
    /// The literal value being matched.
    value: Box<Expr>,
    /// Location spanning this pattern.
    location: SourceLocation,
  },
}

impl SwitchPattern {
  /// The source location of this pattern.
  pub fn location(&self) -> &SourceLocation {
    match self {
      SwitchPattern::Enum { location, .. } | SwitchPattern::Literal { location, .. } => location,
    }
  }
}

/// A field in a switch pattern — either a binding or a discard.
#[derive(Debug, Clone)]
pub enum SwitchPatternField {
  /// Bind this field to a local variable: `name`
  Bind(String),
  /// Discard this field: `_`
  Discard,
}

/// Match statement: conditional branching with multiple arms.
///
/// # Example
///
/// ```lale
/// match
///     case x > 0:
///         write "positive"
///     case x < 0:
///         write "negative"
///     default:
///         write "zero"
/// end match
/// ```
#[derive(Debug, Clone)]
pub struct MatchStmt {
  /// The arms of the match (each with a guard condition).
  pub arms: Vec<MatchArm>,
  /// Optional else/default arm for when no guards match.
  pub else_arm: Vec<Stmt>,
  /// Location spanning the entire match statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// A single arm in a match statement.
#[derive(Debug, Clone)]
pub struct MatchArm {
  /// The guard condition for this arm.
  pub guard: Condition,
  /// The body of this arm.
  pub body: Vec<Stmt>,
  /// Location spanning this arm.
  pub location: SourceLocation,
}

/// A single test case within a test suite.
#[derive(Debug, Clone)]
pub struct TestCaseStmt {
  /// The case name (from the string literal).
  pub name: String,
  /// The body of this test case.
  pub body: Vec<Stmt>,
  /// Location spanning this case.
  pub location: SourceLocation,
  /// Comments attached to this case.
  pub comments: AttachedComments,
}

/// An element inside a `test suite` body: a test case, a suite-level `var`/`fn`
/// declaration, or a standalone comment/doc node. Keeping these in a single
/// ordered list preserves source fidelity — a comment between two cases stays
/// in position instead of being dropped.
#[derive(Debug, Clone)]
pub enum TestSuiteItem {
  Case(TestCaseStmt),
  /// A suite-level declaration (`Stmt::VarDef` or `Stmt::FnDef`), shared by
  /// every case in the suite and invisible outside it. Boxed to keep the enum
  /// small (Stmt is large).
  Declaration(Box<Stmt>),
  Comment(CommentStmt),
  Doc(DocStmt),
}

/// Test suite statement: a named group of test cases.
#[derive(Debug, Clone)]
pub struct TestSuiteStmt {
  /// The suite name (from the string literal).
  pub name: String,
  /// The ordered body: test cases interleaved with standalone comments/docs.
  pub items: Vec<TestSuiteItem>,
  /// Location spanning the entire suite.
  pub location: SourceLocation,
  /// Comments attached to this suite.
  pub comments: AttachedComments,
}

impl TestSuiteStmt {
  /// Iterate over just the test cases, skipping standalone comment/doc nodes.
  pub fn cases(&self) -> impl Iterator<Item = &TestCaseStmt> {
    self.items.iter().filter_map(|item| match item {
      TestSuiteItem::Case(case) => Some(case),
      _ => None,
    })
  }

  /// Iterate over just the suite-level `var`/`fn` declarations, skipping cases
  /// and standalone comment/doc nodes.
  pub fn declarations(&self) -> impl Iterator<Item = &Stmt> {
    self.items.iter().filter_map(|item| match item {
      TestSuiteItem::Declaration(stmt) => Some(stmt.as_ref()),
      _ => None,
    })
  }
}

/// Loop statement.
///
/// Supports optional iteration ranges and pre/post conditions.
///
/// # Example
///
/// ```lale
/// loop i as i32 from 1 to 10
///     write i
/// end loop
///
/// loop while x < 100
///     x = x * 2
/// end loop
/// ```
#[derive(Debug, Clone)]
pub struct LoopStmt {
  /// Optional iteration range (e.g., `i as i32 from 1 to 10`).
  pub range: Option<Range>,
  /// Optional entry condition (checked once before entering the loop).
  pub pre_condition: Option<Condition>,
  /// Loop body statements.
  pub body: Vec<Stmt>,
  /// Optional post-condition (evaluated after each iteration).
  pub post_condition: Option<Condition>,
  /// Location spanning the entire loop statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Return statement.
///
/// # Example
///
/// ```lale
/// return 42
/// return "done"
/// ```
#[derive(Debug, Clone)]
pub struct ReturnStmt {
  /// The value to return (if present).
  pub value: Option<Expr>,
  /// Location of the return statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Exit program statement: `exit program`
#[derive(Debug, Clone)]
pub struct ExitProgramStmt {
  /// Optional exit code.
  pub code: Option<i32>,
  /// Location of the exit program statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Exit loop statement: `exit loop`
#[derive(Debug, Clone)]
pub struct ExitLoopStmt {
  /// Location of the exit loop statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Rewind statement: `rewind`
#[derive(Debug, Clone)]
pub struct RewindStmt {
  /// Location of the rewind statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Write to stdout: `write expr`
#[derive(Debug, Clone)]
pub struct StdoutStmt {
  /// Whether this is an inline write (`write inline expr`).
  pub inline: bool,
  /// The expression to write.
  pub value: Expr,
  /// Optional target (output redirection).
  pub target: Option<Spanned<String>>,
  /// Location spanning the entire write statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Write to stderr: `warn expr`
#[derive(Debug, Clone)]
pub struct StderrStmt {
  /// Whether this is an inline warn (`warn inline expr`).
  pub inline: bool,
  /// The expression to write.
  pub value: Expr,
  /// Optional target (output redirection).
  pub target: Option<Spanned<String>>,
  /// Location spanning the entire warn statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Debug output to stderr: `debug expr` — expands to `warn "DEBUG: expr = {expr}"`
#[derive(Debug, Clone)]
pub struct DebugStmt {
  /// The expression to debug.
  pub value: Expr,
  /// Source text of the expression (for the label).
  pub expr_text: String,
  /// Location spanning the entire debug statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Read from stdin: `read var`
#[derive(Debug, Clone)]
pub struct StdinStmt {
  /// Target variable path.
  pub target: Spanned<Vec<String>>,
  /// Location spanning the entire read statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Error stack push statement: `add error expr`.
#[derive(Debug, Clone)]
pub struct AddErrorStmt {
  /// The expression whose string value is pushed onto the error stack.
  pub value: Expr,
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Write error messages to stdout: `write error messages`.
#[derive(Debug, Clone)]
pub struct WriteErrorsStmt {
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Warn error messages statement: `warn error messages`.
#[derive(Debug, Clone)]
pub struct WarnErrorsStmt {
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Alert error messages statement: `alert error messages`.
#[derive(Debug, Clone)]
pub struct AlertErrorsStmt {
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Alert output statement: `alert expr` / `alert inline expr`.
/// Prints to stderr with a red "Error:" prefix.
#[derive(Debug, Clone)]
pub struct AlertStmt {
  /// Whether this is inline (no trailing newline).
  pub inline: bool,
  /// The expression to output.
  pub value: Expr,
  /// Location spanning the entire statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time if statement.
#[derive(Debug, Clone)]
pub struct CtIfStmt {
  /// The compile-time condition.
  pub condition: Condition,
  /// Statements in the then branch.
  pub then_branch: Vec<Stmt>,
  /// Else-if branches.
  pub else_if_branches: Vec<(Condition, Vec<Stmt>)>,
  /// Optional else branch.
  pub else_branch: Option<Vec<Stmt>>,
  /// Location of the compile-time if.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time fail: `#fail "message"`
#[derive(Debug, Clone)]
pub struct CtFailStmt {
  /// The error message.
  pub message: String,
  /// Location of the fail statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time warn: `#warn "message"`
#[derive(Debug, Clone)]
pub struct CtWarnStmt {
  /// The warning message.
  pub message: String,
  /// Location of the warn statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time when: one-sided guard evaluated at compile time.
#[derive(Debug, Clone)]
pub struct CtWhenStmt {
  /// The compile-time condition.
  pub condition: Condition,
  /// Statements in the body.
  pub body: Vec<Stmt>,
  /// Location spanning the entire when statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time match: ordered condition chain evaluated at compile time.
#[derive(Debug, Clone)]
pub struct CtMatchStmt {
  /// The arms (each with a guard condition).
  pub arms: Vec<MatchArm>,
  /// Optional else/default arm.
  pub else_arm: Vec<Stmt>,
  /// Location spanning the entire match statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// Compile-time switch: value dispatch over literal cases at compile time.
#[derive(Debug, Clone)]
pub struct CtSwitchStmt {
  /// The expression to match against.
  pub value: Expr,
  /// The literal cases.
  pub cases: Vec<CtSwitchCase>,
  /// Optional default case.
  pub default_case: Vec<Stmt>,
  /// Location spanning the entire switch statement.
  pub location: SourceLocation,
  /// Comments attached to this statement.
  pub comments: AttachedComments,
}

/// A single compile-time switch case (`#case <literal>:`).
#[derive(Debug, Clone)]
pub struct CtSwitchCase {
  /// The literal to compare against.
  pub value: Expr,
  /// The body of this case.
  pub body: Vec<Stmt>,
  /// Location spanning this case.
  pub location: SourceLocation,
}

/// Documentation comment (captured as a statement).
#[derive(Debug, Clone)]
pub struct DocStmt {
  /// The comment content (without `///` prefix).
  pub content: String,
  /// Location of the comment.
  pub location: SourceLocation,
}

/// Regular comment (captured as a statement for structure preservation).
#[derive(Debug, Clone)]
pub struct CommentStmt {
  /// The comment content (without `//` prefix).
  pub content: String,
  /// Location of the comment.
  pub location: SourceLocation,
}

/// Comments attached to an AST node.
#[derive(Debug, Clone, Default)]
pub struct AttachedComments {
  /// Leading comments (before the node).
  pub leading: Vec<AttachedComment>,
  /// Trailing comment (after the node, on the same or next line).
  pub trailing: Option<AttachedComment>,
}

/// A single attached comment (either doc or regular).
#[derive(Debug, Clone)]
pub enum AttachedComment {
  /// A documentation comment (starts with `///`).
  Doc(DocStmt),
  /// A regular comment (starts with `//`).
  Comment(CommentStmt),
}

/// Type name with optional array dimensions.
#[derive(Debug, Clone)]
/// A fully-qualified type in the Lale type system.
///
/// Represents a type annotation as written in source code, including the base
/// primitive or user-defined type, optional inner type for vectors (`vec3 of f64`
/// stores `f64` in `inner_type`), array dimensions (`i32[5][3]` stores `[5, 3]`),
/// and the optional marker (`T?` sets `is_optional`).
///
/// # Examples
///
/// ```text
/// i32              -> base_type = I32, no inner, no dims, not optional
/// vec3 of f64      -> base_type = Vec3, inner_type = F64, no dims, not optional
/// i32[5]           -> base_type = I32, no inner, dims = [5], not optional
/// f64?             -> base_type = F64, no inner, no dims, is_optional = true
/// vec2 of i32[3]?  -> base_type = Vec2, inner_type = I32, dims = [3], optional
/// ```
pub struct TypeName {
  /// The base type.
  pub base_type: BaseType,
  /// For Vec2/Vec3/Vec4 — the inner element type. None for other types.
  pub inner_type: Option<Box<BaseType>>,
  /// Array dimensions (each element is a dimension size expression).
  pub array_dimensions: Vec<Expr>,
  /// Whether this is an optional type (`T?`).
  pub is_optional: bool,
  /// Location of the type name.
  pub location: SourceLocation,
}

impl TypeName {
  /// Returns true if this is a pointer type.
  pub fn is_pointer(&self) -> bool {
    matches!(self.base_type, BaseType::Pointer)
  }
}

/// Base types in Lale.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseType {
  U8,
  I8,
  U16,
  I16,
  U32,
  I32,
  U64,
  I64,
  F16,
  F32,
  F64,
  Str,
  Bool,
  Byte,
  Char,
  /// Raw pointer type.
  Pointer,
  /// 2‑component vector (inner type stored on TypeName for now).
  Vec2,
  /// 3‑component vector (inner type stored on TypeName for now).
  Vec3,
  /// 4‑component vector (inner type stored on TypeName for now).
  Vec4,
  /// Custom type (for future class support).
  Custom(String),
}

/// Physical unit annotation.
#[derive(Debug, Clone)]
pub struct Unit {
  /// The raw unit string as written in source (e.g., "kg*m^2/s^2").
  pub raw: String,
  /// Location of the unit annotation.
  pub location: SourceLocation,
}

/// Function parameter.
#[derive(Debug, Clone)]
pub struct Parameter {
  /// Whether this parameter is passed by copy.
  pub is_copy: bool,
  /// The parameter name with its source location.
  pub name: Spanned<String>,
  /// Type annotation for this parameter.
  pub type_annotation: TypeName,
  /// Optional physical unit.
  pub unit: Option<Unit>,
  /// Location spanning the entire parameter declaration.
  pub location: SourceLocation,
}

/// Return type of a function.
#[derive(Debug, Clone)]
pub struct ReturnType {
  /// The kind of return type.
  pub kind: ReturnTypeKind,
  /// Location of the return type specification.
  pub location: SourceLocation,
}

/// Return type variants.
#[derive(Debug, Clone)]
pub enum ReturnTypeKind {
  /// Function returns nothing (void).
  Nothing,
  /// Function returns a value of the specified type.
  Type(TypeName),
}

/// A condition in if/loop statements.
///
/// Supports optional negation (not).
#[derive(Debug, Clone)]
pub struct Condition {
  /// The condition expression.
  pub expr: Expr,
  /// Whether the condition is negated (not expr).
  pub negated: bool,
  /// Location of the condition.
  pub location: SourceLocation,
}

/// A range for loop iteration: `variable as Type from expr to expr [step expr]`
#[derive(Debug, Clone)]
pub struct Range {
  /// The loop variable with its source location.
  pub variable: Spanned<String>,
  /// The type of the loop variable.
  pub var_type: TypeName,
  /// Starting expression.
  pub from: Expr,
  /// Ending expression (inclusive).
  pub to: Expr,
  /// Optional step expression (default step depends on from/to).
  pub step: Option<Expr>,
  /// Location spanning the entire range.
  pub location: SourceLocation,
}

use crate::types::NormalizedUnit;

/// Expression unit (result of unit analysis for expressions).
///
/// This represents the physical unit of an expression as computed during
/// semantic analysis. Unlike `Unit` which is a syntactic element, `ExprUnit`
/// is a semantic property derived from unit propagation rules.
///
/// Units are stored in normalized form for correct comparison of semantically
/// equivalent units like `kg*m^2/s^2` and `<kg>*<m/s>^2`.
#[derive(Debug, Clone)]
pub enum ExprUnit {
  /// No physical unit (pure number, bool, string, etc.)
  Unitless,
  /// A known physical unit in normalized form
  Unit(NormalizedUnit),
  /// Unit is unknown due to missing info or prior error
  Unknown,
}

impl PartialEq for ExprUnit {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (ExprUnit::Unitless, ExprUnit::Unitless) => true,
      (ExprUnit::Unknown, ExprUnit::Unknown) => true,
      (ExprUnit::Unit(a), ExprUnit::Unit(b)) => a == b,
      _ => false,
    }
  }
}

impl Eq for ExprUnit {}

impl ExprUnit {
  /// Create an ExprUnit from an optional Unit.
  pub fn from_option(unit: &Option<Unit>) -> Self {
    match unit {
      Some(u) => ExprUnit::from_string(&u.raw),
      None => ExprUnit::Unitless,
    }
  }

  /// Create an ExprUnit from a unit string.
  pub fn from_string(s: &str) -> Self {
    let normalized = NormalizedUnit::parse(s);
    if normalized.is_unitless() {
      ExprUnit::Unitless
    } else {
      ExprUnit::Unit(normalized)
    }
  }

  /// Check if this unit is without unit.
  pub fn is_unitless(&self) -> bool {
    matches!(self, ExprUnit::Unitless)
  }

  /// Check if this unit is unknown.
  pub fn is_unknown(&self) -> bool {
    matches!(self, ExprUnit::Unknown)
  }

  /// Check if two units are compatible (same unit or one is unknown).
  pub fn same_unit(u1: &ExprUnit, u2: &ExprUnit) -> bool {
    match (u1, u2) {
      (ExprUnit::Unknown, _) | (_, ExprUnit::Unknown) => true,
      (ExprUnit::Unitless, ExprUnit::Unitless) => true,
      (ExprUnit::Unit(a), ExprUnit::Unit(b)) => a == b,
      _ => false,
    }
  }

  /// Combine two units through multiplication.
  pub fn combine_mul(u1: &ExprUnit, u2: &ExprUnit) -> ExprUnit {
    match (u1, u2) {
      (ExprUnit::Unknown, _) | (_, ExprUnit::Unknown) => ExprUnit::Unknown,
      (ExprUnit::Unitless, u) | (u, ExprUnit::Unitless) => u.clone(),
      (ExprUnit::Unit(a), ExprUnit::Unit(b)) => ExprUnit::Unit(a.multiply(b)),
    }
  }

  /// Combine two units through division.
  pub fn combine_div(u1: &ExprUnit, u2: &ExprUnit) -> ExprUnit {
    match (u1, u2) {
      (ExprUnit::Unknown, _) | (_, ExprUnit::Unknown) => ExprUnit::Unknown,
      (u, ExprUnit::Unitless) => u.clone(),
      (ExprUnit::Unitless, ExprUnit::Unit(b)) => {
        ExprUnit::Unit(NormalizedUnit::unitless().divide(b))
      }
      (ExprUnit::Unit(a), ExprUnit::Unit(b)) => ExprUnit::Unit(a.divide(b)),
    }
  }

  /// Apply an exponent to a unit.
  pub fn power(u: &ExprUnit, exp: i64) -> ExprUnit {
    match u {
      ExprUnit::Unknown => ExprUnit::Unknown,
      ExprUnit::Unitless => ExprUnit::Unitless,
      ExprUnit::Unit(nu) => {
        let result = nu.power(exp);
        if result.is_unitless() {
          ExprUnit::Unitless
        } else {
          ExprUnit::Unit(result)
        }
      }
    }
  }

  /// Get a display string for error messages.
  pub fn display(&self) -> String {
    match self {
      ExprUnit::Unitless => "unitless".to_string(),
      ExprUnit::Unknown => "unknown".to_string(),
      ExprUnit::Unit(nu) => format!("<{}>", nu),
    }
  }
}

/// Expression types.
#[derive(Debug, Clone)]
pub enum Expr {
  /// Binary operation: `a + b`
  Binary(BinaryExpr),
  /// Unary operation: `-a`, `not a`
  Unary(UnaryExpr),
  /// Type conversion: `x as T`
  Conversion(ConversionExpr),
  /// Identifier reference
  Identifier(IdentifierExpr),
  /// Integer literal
  IntLiteral(IntLiteral),
  /// Unsigned integer literal
  UintLiteral(UintLiteral),
  /// Float literal
  FloatLiteral(FloatLiteral),
  /// Hex literal
  HexLiteral(HexLiteral),
  /// Character literal
  CharLiteral(CharLiteral),
  /// Boolean literal
  BoolLiteral(BoolLiteral),
  /// String literal (with embedded value support)
  StringLiteral(StringLiteral),
  /// Array literal
  ArrayLiteral(ArrayLiteral),
  /// Function call as expression
  FnCallExpr(FnCall),
  /// Member access: `obj.field`
  MemberAccess(MemberAccess),
  /// Array indexing: `arr[i]`
  ArrayIndex(ArrayIndex),
  /// Compiler constant: `#main`, `#source_file`, etc.
  CompilerConst(CompilerConst),
  /// Grouped expression (parenthesized)
  Grouped(Box<Expr>),
  /// Optional presence check: `expr has value`
  HasValue(Box<Expr>),
  /// Optional absence check: `expr has no value`
  HasNoValue(Box<Expr>),
  /// Nothing literal: `nothing`
  NothingExpr,
  /// Error stack check: `errors has messages`
  HasErrors,
  /// Pop last error: `last error`
  LastError,
  /// Optional propagation: `expr?` — returns nothing if absent
  TryPropagate(Box<Expr>),
  /// Heap allocation: `allocate(size)` — returns a pointer
  Allocate(AllocateExpr),
}

/// Allocate expression: `allocate(size_expr)`
#[derive(Debug, Clone)]
pub struct AllocateExpr {
  /// The size expression (must evaluate to u64)
  pub size: Box<Expr>,
  /// Location spanning the entire expression
  pub location: SourceLocation,
}

/// Binary expression.
#[derive(Debug, Clone)]
pub struct BinaryExpr {
  /// Left operand.
  pub left: Box<Expr>,
  /// The binary operator.
  pub operator: BinaryOp,
  /// Right operand.
  pub right: Box<Expr>,
  /// Location of the operator (for precise error reporting).
  pub location: SourceLocation,
}

/// Binary operators.
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BinaryOp {
  // Logical
  Or,
  And,
  Xor,
  // Bit operations
  BitAnd,
  BitOr,
  BitXor,
  // Comparison
  Eq,
  NotEq,
  Lt,
  Gt,
  LtEq,
  GtEq,
  // Arithmetic
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Pow,
  // Bitwise shifts
  UnsignedLeftShift,
  UnsignedRightShift,
  SignedLeftShift,
  SignedRightShift,
  // Other
  Append,
  // Vector operations
  Dot,
  Cross,
}

/// Unary expression.
#[derive(Debug, Clone)]
pub struct UnaryExpr {
  /// The unary operator.
  pub operator: UnaryOp,
  /// The operand.
  pub operand: Box<Expr>,
  /// Location of the operator.
  pub location: SourceLocation,
}

/// Unary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
  /// Negation: `-x`
  Neg,
  /// Logical not: `not x`
  Not,
  /// Bitwise invert: `invert x`
  Invert,
  /// Type query: `type of x`
  TypeOf,
  /// Size query: `size of x`
  SizeOf,
  /// Unit query: `unit of x`
  UnitOf,
  /// Pointer creation: `pointer to x`
  PointerTo,
  /// Dereference: `value at x`
  ValueAt,
  /// Unsafe bit reinterpretation: `unsafe cast value at x`
  UnsafeCast,
  /// Optional extraction: `value of x`
  ValueOf,
}

/// Type conversion expression: `x as T`
#[derive(Debug, Clone)]
pub struct ConversionExpr {
  /// The expression to convert.
  pub operand: Box<Expr>,
  /// The target type.
  pub target_type: TypeName,
  /// Location of the `as` keyword.
  pub location: SourceLocation,
}

/// Identifier expression - can be a bare name (`sqrt`) or a path name using `->` (`math -> sqrt`).
#[derive(Debug, Clone)]
pub struct IdentifierExpr {
  /// The identifier path (e.g., ["math", "sin"] for "math -> sin").
  pub path: Vec<String>,
  /// Location of the identifier.
  pub location: SourceLocation,
}

impl IdentifierExpr {
  /// Get the bare name (last part of the path). For `math -> sqrt`, returns `"sqrt"`.
  pub fn name(&self) -> &str {
    self.path.last().map(|s| s.as_str()).unwrap_or("")
  }

  /// Get the full qualified name as "module -> module -> name".
  pub fn full_path(&self) -> String {
    self.path.join(" -> ")
  }

  /// Returns true if this identifier uses a path (contains `->`), e.g. `math -> sqrt`.
  /// Returns false for bare names like `sqrt` with no path separator.
  pub fn is_qualified(&self) -> bool {
    self.path.len() > 1
  }
}

/// Integer literal.
#[derive(Debug, Clone)]
pub struct IntLiteral {
  /// The integer value.
  pub value: i64,
  /// Optional physical unit.
  pub unit: Option<Unit>,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// Unsigned integer literal.
#[derive(Debug, Clone)]
pub struct UintLiteral {
  /// The unsigned integer value.
  pub value: u64,
  /// Optional physical unit.
  pub unit: Option<Unit>,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// Float literal.
#[derive(Debug, Clone)]
pub struct FloatLiteral {
  /// The floating-point value.
  pub value: f64,
  /// Optional physical unit.
  pub unit: Option<Unit>,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// Hex literal.
#[derive(Debug, Clone)]
pub struct HexLiteral {
  /// The hex string (including 0x prefix).
  pub value: String,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// Character literal.
#[derive(Debug, Clone)]
pub struct CharLiteral {
  /// The character value.
  pub value: char,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// Boolean literal.
#[derive(Debug, Clone)]
pub struct BoolLiteral {
  /// The boolean value.
  pub value: bool,
  /// Location of the literal.
  pub location: SourceLocation,
}

/// String literal with embedding support.
#[derive(Debug, Clone)]
pub struct StringLiteral {
  /// The parts of the string (text and embedded values).
  pub parts: Vec<StringPart>,
  /// Location of the entire string literal.
  pub location: SourceLocation,
}

/// Part of a string (text or embedding).
#[derive(Debug, Clone)]
pub enum StringPart {
  /// A text segment with its source location.
  Text(Spanned<String>),
  /// An embedded expression: `{expr}`.
  EmbeddedValue(Box<Expr>),
}

/// Array literal.
#[derive(Debug, Clone)]
pub struct ArrayLiteral {
  /// The array elements.
  pub elements: Vec<Expr>,
  /// If Some, this is a fill literal: [fill with value]
  /// The count is always inferred from the type annotation.
  pub fill: Option<Box<Expr>>,
  /// Location of the array literal.
  pub location: SourceLocation,
}

/// Member access expression.
#[derive(Debug, Clone)]
pub struct MemberAccess {
  /// The object being accessed.
  pub object: Box<Expr>,
  /// The member name with its source location.
  pub member: Spanned<String>,
  /// Location of the entire member access.
  pub location: SourceLocation,
}

/// Array index expression.
#[derive(Debug, Clone)]
pub struct ArrayIndex {
  /// The array being indexed.
  pub array: Box<Expr>,
  /// The index expressions.
  pub indices: Vec<Expr>,
  /// Location of the entire indexing expression.
  pub location: SourceLocation,
}

/// Compiler constants.
#[derive(Debug, Clone)]
pub struct AssertStmt {
  pub condition: Expr,
  pub location: SourceLocation,
  pub comments: AttachedComments,
}

#[derive(Debug, Clone)]
pub struct CompilerConst {
  /// The kind of compiler constant.
  pub kind: CompilerConstKind,
  /// Location of the compiler constant.
  pub location: SourceLocation,
}

/// Compiler constant variants.
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerConstKind {
  /// `#main` - true if this is the main module
  Main,
  /// `#source_file` - the source file name
  SourceFile,
  /// `#source_line` - the current line number
  SourceLine,
  /// `#compile_time` - the compilation timestamp
  CompileTime,
  /// `#compiler_version` - the compiler version
  CompilerVersion,
  /// `__function__` - the current function name
  Function,
  /// `#posix` - true if running on POSIX systems
  Posix,
  /// `#windows` - true if running on Windows
  Windows,
  /// `#debug` - true in debug mode (default), false with --release
  Debug,
  /// `#mode` - the execution mode as a string: "run" or "test"
  Mode,
}

// ==================== UNIT TESTS ====================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_spanned_new() {
    let loc = SourceLocation {
      line: 1,
      col: 5,
      start_pos: 4,
      end_pos: 10,
      source_file: "test.lale".to_string(),
    };
    let spanned = Spanned::new("hello".to_string(), loc.clone());

    assert_eq!(spanned.node, "hello");
    assert_eq!(spanned.span.line, 1);
    assert_eq!(spanned.span.col, 5);
    assert_eq!(spanned.span.start_pos, 4);
    assert_eq!(spanned.span.end_pos, 10);
  }

  #[test]
  fn test_spanned_value_and_location() {
    let loc = SourceLocation::dummy();
    let spanned = Spanned::new(42, loc);

    assert_eq!(*spanned.value(), 42);
    assert_eq!(spanned.location().line, 0);
  }

  #[test]
  fn test_spanned_map() {
    let loc = SourceLocation {
      line: 5,
      col: 10,
      start_pos: 50,
      end_pos: 55,
      source_file: "test.lale".to_string(),
    };
    let spanned_str = Spanned::new("42".to_string(), loc);
    let spanned_int = spanned_str.map(|s| s.parse::<i32>().unwrap());

    assert_eq!(spanned_int.node, 42);
    assert_eq!(spanned_int.span.line, 5);
    assert_eq!(spanned_int.span.col, 10);
  }

  #[test]
  fn test_spanned_equality_ignores_location() {
    let loc1 = SourceLocation {
      line: 1,
      col: 1,
      start_pos: 0,
      end_pos: 5,
      source_file: "test1.lale".to_string(),
    };
    let loc2 = SourceLocation {
      line: 10,
      col: 20,
      start_pos: 100,
      end_pos: 105,
      source_file: "test2.lale".to_string(),
    };

    let spanned1 = Spanned::new("same".to_string(), loc1);
    let spanned2 = Spanned::new("same".to_string(), loc2);
    let spanned3 = Spanned::new("different".to_string(), SourceLocation::dummy());

    assert_eq!(spanned1, spanned2);
    assert_ne!(spanned1, spanned3);
  }

  #[test]
  fn test_source_location_dummy() {
    let loc = SourceLocation::dummy();
    assert_eq!(loc.line, 0);
    assert_eq!(loc.col, 0);
    assert_eq!(loc.start_pos, 0);
    assert_eq!(loc.end_pos, 0);
  }

  #[test]
  fn test_spanned_with_vec() {
    let loc = SourceLocation::dummy();
    let spanned_vec = Spanned::new(vec!["a".to_string(), "b".to_string()], loc);

    assert_eq!(spanned_vec.node.len(), 2);
    assert_eq!(spanned_vec.node[0], "a");
    assert_eq!(spanned_vec.node[1], "b");
  }

  #[test]
  fn test_spanned_clone() {
    let loc = SourceLocation {
      line: 3,
      col: 7,
      start_pos: 30,
      end_pos: 40,
      source_file: "test.lale".to_string(),
    };
    let original = Spanned::new("cloned".to_string(), loc);
    let cloned = original.clone();

    assert_eq!(original.node, cloned.node);
    assert_eq!(original.span.line, cloned.span.line);
  }
}
