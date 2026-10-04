<img src="lale-logo.jpg" align="right" alt="Logo" width="200">

# Lale Compiler – Architecture

**Website**: [https://lale-lang.dev](https://lale-lang.dev)

This document describes the internal architecture of the Lale compiler for developers who want to understand, maintain, or extend the compiler. It also serves to onboard new contributors.

---

## Table of Contents

1. [Overview and Design Principles](#1-overview-and-design-principles)
2. [Execution Model and Backend Strategy](#2-execution-model-and-backend-strategy)
3. [Developer Setup and Installation](#3-developer-setup-and-installation)
4. [Compilation Pipeline](#4-compilation-pipeline)
   - 4.1 [Parser Generation](#41-parser-generation)
     - [Grammar Structure](#grammar-structure)
     - [Parse Tree Structure](#parse-tree-structure)
   - 4.2 [AST Construction and Pratt Parsing](#42-ast-construction-and-pratt-parsing)
     - [Pratt Parser Configuration](#pratt-parser-configuration)
   - 4.3 [AST Structure and Source Fidelity](#43-ast-structure-and-source-fidelity)
     - [Program Root](#program-root)
     - [Statement Types](#statement-types)
     - [Expression Types](#expression-types)
     - [Source Fidelity Design](#source-fidelity-design)
     - [Comment and Doc Attachment](#comment-and-doc-attachment)
   - 4.4 [Visitor Pattern](#44-visitor-pattern)
     - [`AstVisitor<T>` Trait](#astvisitort-trait)
     - [Implementations](#implementations)
   - 4.5 [Semantic Analysis](#45-semantic-analysis)
     - [4.5.1 Module Organisation](#451-module-organisation)
     - [4.5.2 Symbol Tables](#452-symbol-tables)
     - [4.5.3 Symbol Table Design Decisions](#453-symbol-table-design-decisions)
     - [4.5.4 Scope Management](#454-scope-management)
     - [4.5.5 Memory Safety](#455-memory-safety)
     - [4.5.6 Unit Analysis](#456-unit-analysis)
     - [4.5.7 Unused Symbol Detection](#457-unused-symbol-detection)
     - [4.5.7a Division-by-Zero Detection](#457a-division-by-zero-detection)
     - [4.5.7b Allocate-Release Pairing Check](#457b-allocate-release-pairing-check)
     - [4.5.7c Test Suite Validation](#457c-test-suite-validation)
     - [4.5.7d Definite Assignment Across Branches](#457d-definite-assignment-across-branches)
     - [4.5.8 SQLite‑Backed Symbol Table](#458-sqlitebacked-symbol-table)
     - [4.5.9 Database Infrastructure for Path Names](#459-database-infrastructure-for-path-names)
   - 4.6 [Module System](#46-module-system)
     - [`use` vs. `import`](#use-vs-import)
     - [Module Resolution](#module-resolution)
     - [Exports and Visibility](#exports-and-visibility)
     - [Module Execution Model](#module-execution-model)
     - [Circular Import Prevention](#circular-import-prevention)
     - [Built‑ins (`builtins.lale`)](#builtins-builtinslale)
     - [Standard Library Handling](#standard-library-handling)
     - [Conditional Compilation for Platform‑Specific Modules](#conditional-compilation-for-platformspecific-modules)
     - [Module Import Ambiguity Detection](#module-import-ambiguity-detection)
     - [Stdlib Design Principles](#stdlib-design-principles)
   - 4.7 [Intermediate Representation](#47-intermediate-representation)
     - [Design Goals](#design-goals)
     - [IR Types](#ir-types)
     - [Values (SSA)](#values-ssa)
     - [Module Structure](#module-structure)
     - [Instructions](#instructions)
     - [Stable Serialisation Format](#stable-serialisation-format)
     - [Validation Rules](#validation-rules)
     - [Interpretation Strategy](#interpretation-strategy)
     - [Backend Lowering Overview](#backend-lowering-overview)
     - [4.7.10 IR Semantics and Backend Independence](#4710-ir-semantics-and-backend-independence)
   - 4.8 [Interpreter Backend](#48-interpreter-backend)
     - [Design: Split Representation](#design-split-representation)
     - [Memory Layout: The Lale ABI](#memory-layout-the-lale-abi)
     - [Allocation and Leak Detection](#allocation-and-leak-detection)
     - [Stack Frame Reclamation](#stack-frame-reclamation)
     - [Bounds Checking](#bounds-checking)
     - [FFI](#ffi)
     - [Key Instructions](#key-instructions)
     - [Test Execution](#test-execution)
     - [Write System and String Handling](#write-system-and-string-handling)
     - [Runtime Hooks and FFI Boundary](#runtime-hooks-and-ffi-boundary)
     - [Error Function: Single User‑Replaceable Hook](#error-function-single-userreplaceable-hook)
     - [File I/O Architecture](#file-io-architecture)
     - [Semantic Conversions: Numeric‑to‑Character](#semantic-conversions-numerictocharacter)
   - 4.9 [Future AOT Backends](#49-future-aot-backends)
   - 4.10 [Validation Tool](#410-validation-tool)
     - [Validation Stages](#validation-stages)
5. [Language Design Decisions](#5-language-design-decisions)
   - 5.1 [UTF‑8 Everywhere](#51-utf8-everywhere)
   - 5.2 [Inferred Properties: No `const` or `mutability` Keywords (SSA-Based)](#52-inferred-properties-no-const-or-mutability-keywords-ssa-based)
   - 5.3 [No Type Aliases](#53-no-type-aliases)
   - 5.4 [Volatile Semantics via Import/Export](#54-volatile-semantics-via-importexport)
   - 5.5 [Explicit Type Conversions (Widening Only)](#55-explicit-type-conversions-widening-only)
   - 5.6 [Chained Type Conversions](#56-chained-type-conversions)
   - 5.7 [Pointer Conversions (Unsigned Only)](#57-pointer-conversions-unsigned-only)
   - 5.8 [Numeric Literal Range Validation](#58-numeric-literal-range-validation)
   - 5.9 [1‑Based Array Indexing](#59-1based-array-indexing)
   - 5.10 [Complete Array Initialization (No Partial Init)](#510-complete-array-initialization-no-partial-init)
   - 5.11 [No Exception Handling](#511-no-exception-handling)
   - 5.12 [Runtime Array Bounds Checking](#512-runtime-array-bounds-checking)
   - 5.13 [Unit Inference and No Unit Stripping](#513-unit-inference-and-no-unit-stripping)
   - 5.14 [Raw Pointers Only](#514-raw-pointers-only)
   - 5.14a [The `text` Type](#514a-the-text-type)
   - 5.14b [`text` Compiler-Managed Memory: A Deliberate Design Compromise](#514b-text-compiler-managed-memory-a-deliberate-design-compromise)
   - 5.14c [Fat Text (`ptr` + `bytes`): Why Not Null‑Terminated?](#514c-fat-text-ptr-bytes-fat-pointer-with-ffi-nulltermination)
   - 5.14d [Function Parameter Passing: Copy by Value, `copy`, and `ref`](#514d-function-parameter-passing-copy-by-value-copy-and-ref)
   - 5.14e [The `binary` Type and the `byte` Type](#514e-the-binary-type-and-the-byte-type)
   - 5.15 [Semantic Conversions: Numeric‑to‑Character](#515-semantic-conversions-numerictocharacter)
   - 5.16 [Read as String, Convert Explicitly](#516-read-as-string-convert-explicitly)
   - 5.17 [Optional Types](#517-optional-types)
   - 5.18 [Error Stack](#518-error-stack)
   - 5.19 [Alert Statement](#519-alert-statement)
   - 5.20 [Vector Types and Dot/Cross Product](#520-vector-types-and-dotcross-product)
   - 5.21 [`×` (U+00D7 MULTIPLICATION SIGN) Excluded from Grammar](#521-u00d7-multiplication-sign-excluded-from-grammar)
   - 5.22 [Superscript Digits as Power Operators](#522-superscript-digits-as-power-operators)
   - 5.23 [Field Encapsulation via `private`](#523-field-encapsulation-via-private)
   - 5.24 [`switch` — Value Comparison and Enum Matching](#524-switch-value-comparison-and-enum-matching)
   - 5.25 [Integer Overflow Traps by Default](#525-integer-overflow-traps-by-default)
6. [Error Catalog](#6-error-catalog)
7. [Appendices](#7-appendices)
   - [Adding a New Language Feature](#adding-a-new-language-feature)

---

## 1. Overview and Design Principles

The Lale compiler transforms source code through several stages:

```text
Source Code → Parser → Parse Tree → AST Builder → AST → Semantic Analysis → IR Generation → Interpreter
```

It is implemented in Rust as a _bootstrapping_ language (written in Rust for now), with the long‑term goal of being written in Lale itself. The compiler is built around a single, stable, _serialisable_ (writeable out as text and read back) Intermediate Representation (IR) that decouples the frontend from all backends — today the interpreter, tomorrow ahead‑of‑time compilers.

These stages can be read without prior compiler knowledge: the _parser_ reads source text and checks it against the language's grammar, producing a _parse tree_ that mirrors that grammar. The _AST builder_ turns the parse tree into an _abstract syntax tree (AST)_ — a simplified representation of the program's meaning, with the grammar's punctuation and structure removed. _Semantic analysis_ then checks that the meaning is valid (names exist, types match, units are consistent). The result is translated into an _intermediate representation (IR)_ — a lower-level form, shared by every execution engine, that is easier to analyze and run. The _frontend_ is the part of the compiler that understands Lale source (parsing through IR generation); the _backend_ is the part that runs or compiles the IR.

### Core Design Principles

These principles guide every language and compiler decision. They are elaborated in [Chapter 5 – Language Design Decisions](#5-language-design-decisions).

**1. UTF‑8 Everywhere**
The entire pipeline treats UTF‑8 as a first‑class citizen. The grammar supports comprehensive Unicode ranges (Latin Extended, Greek, Cyrillic, CJK, Indic scripts, Arabic, Hebrew, combining marks, subscript digits). Identifiers accept Unicode letters and diacritics. Code generation preserves UTF‑8 identifiers into binaries. Error messages use mathematical symbols (Δ, α, β) for diagnostics. This reduces transcription errors in scientific computing and enables international collaboration.

**2. Explicit over Implicit**

- **No `const` or `mutability` keywords** – the IR's SSA values capture immutability; the backends’ optimisation passes (mem2reg, DSE) make explicit annotations redundant. (_SSA_ is _static single assignment_, meaning every value is assigned exactly once; _mem2reg_ moves values out of memory into registers, and _DSE_ is _dead-store elimination_, removing writes that are never read.)
- **No type aliases** — the `type` keyword always creates a genuinely new, distinct type. Renaming an existing type (e.g., `alias Meters = f64`) is intentionally unsupported and will never be added.
- **Explicit type conversions (widening only)** – the `as` operator is required; narrowing and incompatible conversions are rejected.
- **No chained conversions** – each conversion must be a separate, explicit step.
- **No implicit imports** – symbols must be explicitly listed; `*` is available but discouraged.
- **No hidden control flow** – exception handling is absent; all errors are explicit.
- **No hidden coercions** – Boolean conditions must be Boolean.

  **One documented exception:** `text` and `binary` are the two types where the compiler manages heap memory automatically. String embedding (`"x = {x}"`), type-to-string conversions, and concatenation all allocate behind the scenes. The compiler tracks every `text` value and frees its data at the correct point — the user never writes `release` for a string. This is a deliberate compromise: these types are part of the grammar and the compiler takes full responsibility. For raw pointer allocations, use `allocate` / `release`. See [§5.14a](#514a-the-text-type).

**3. Memory Safety without GC (garbage collection) or Lifetime Annotations**
A simple, per‑function escape rule replaces lifetimes: “References to local variables must not escape the function.” Pointers to globals are safe because they outlive the current scope; pointers passed as parameters are _assumed_ to outlive the call, but this is not verified across a call boundary. This rule is enforced at compile time.

**4. Physical Unit Safety**
Units of measure are inferred from expressions and can be declared on variables. Unit mismatches are compile‑time errors. Units cannot be silently stripped; a value must be unitless from its origin to lose its unit. When static analysis cannot determine a unit, runtime assertions are emitted — these are active in **all builds**, not just debug.

**5. 1‑Based Array Indexing**
Following Fortran, Julia, MATLAB, R, Mathematica, and many other scientific languages, indexing starts at 1. This aligns with mathematical notation, reducing off‑by‑one errors. C FFI (foreign function interface) wrappers translate indices transparently.

**6. No Exception Handling**
Lale uses explicit error returns, a runtime abort path (write a formatted message to stderr and abort) for unrecoverable errors, and optional types (`T?`, see §5.17) for recoverable errors. (Result types are intentionally excluded — the error stack handles diagnostics as a side channel, see §5.18.) Control flow stays visible and deterministic.

**7. Backend Independence and Self‑Hosting Readiness**
The Lale IR is the canonical executable semantic representation of the language. A single, stable, self‑contained IR (serialisable in a line-oriented text format) is the sole interface between the frontend and any execution engine. The interpreter is the reference implementation; future AOT (ahead-of-time) backends — compiler toolchains such as LLVM, GCC, and Cranelift — consume the exact same IR, eliminating the risk of semantic divergence. This design directly supports the long‑term goal of rewriting the compiler in Lale — a process called _self-hosting_, in which a compiler is written in its own language.

**8. No reserved keywords** — `var`, `fn`, `type`, `if`, `loop`, `return`, and all other statement keywords are valid identifiers. The grammar is a _parsing expression grammar (PEG)_, in which alternatives are tried in a fixed order; this ordered choice handles disambiguation at the grammar level, so `type()` is a function call while `type Point ... end type` is a type definition. This ordered choice disambiguates _statement‑initial_ keywords cleanly, but it does **not** yet hold for the bare alphabetic infix operators (`or`, `and`, `xor`, `dot`, `cross`): those are matched without a word boundary, so an identifier beginning with one is split (`a oranges` parses as `a or anges`). That gap is tracked as open point 15 of the 1.0.0 language-surface clarity review (`doc/roadmap.md` §4.2).

**9. Language over Library** — Features that require compiler awareness for safety guarantees or natural syntax are grammar‑level, not stdlib. Optional types (`T?`, `has value`, `value of`), the error stack (`add error`, `last error`, `alert error messages`), and tiered output (`write`/`log`/`warn`/`alert`/`debug`) are language constructs, not library APIs. This ensures compile‑time enforcement (e.g., future lints for unchecked `T?` values), eliminates import boilerplate, and provides syntax that reads as natural language (`if val has value`) rather than method‑call chains (`if val.is_some()`). The test for inclusion is: _does this feature require the compiler to know about it to provide safety or ergonomics that a library cannot?_ If yes, it belongs in the grammar. Pure data transformations (e.g., JSON parsing) belong in the stdlib.

**10. Language-independent semantic model** — The source language is a _frontend_
concern, not a property of the program. The AST and IR are keyword-free: a `when`
statement is `Stmt::When`, never the string `"when"`. Identifiers are arbitrary
Unicode (§5.1), so native-language names (`geschwindigkeit`, `döngü`) already work
today. English is the initial keyword set; because the grammar is the only
language-specific layer, additional localized frontends could target the same
AST without a second compiler, runtime, stdlib, or semantic model. Source
locations and comments as well as grouping elements (parenthesis) are retained (§4.3),
so a semantic round-trip preserves structure and comments — though not exact original formatting.

---

## 2. Execution Model and Backend Strategy

### Current: Pure‑Rust Block‑based IR Interpreter

```bash
lale run program.lale
```

- **Backend**: A new interpreter that walks the block-based SSA IR directly.
- **Behaviour**: Parses → analyses → generates IR → executes the in-memory IR directly. Serialisation/deserialisation happens only under `--roundtrip` (a verification pass).
- **Portability**: Any platform where Rust compiles. No external toolchain required.
- **Stdlib**: Compiled on‑demand from `stdlib/src/full.lale` (~50‑100ms startup overhead).

This interpreter is developed first and must reach full feature maturity before any AOT backend is added. It serves as the **golden reference** for Lale’s semantics.

### Future: Pluggable AOT Backends

The same serialised IR that the interpreter consumes can be fed to one or more ahead‑of‑time compilers. Because the IR is stable and self‑contained, each backend can be developed independently, in any language, and tested against the interpreter’s output.

| Backend             | API                        | Strengths                                                          | Planned Use             |
| ------------------- | -------------------------- | ------------------------------------------------------------------ | ----------------------- |
| **GCC (libgccjit)** | Stable C API, high‑level   | 45+ architectures, mature optimisations, self‑hosting‑friendly     | Primary AOT backend     |
| **LLVM**            | C API, low‑level           | Industry‑leading peak performance, broad platform support          | High‑performance AOT    |
| **Cranelift**       | Rust API, fast compilation | Memory‑safe, ~100× smaller than LLVM, no undefined behaviour in IR | Fast development builds |

All AOT backends are **future work**. They will be implemented against the stable IR once the interpreter is complete. The interpreter’s behaviour will be the compliance test suite for every backend.

### Multilingual frontends (vision)

Lale is authored in English, but the compiler already separates source-language
syntax from semantics, so the same program could in principle be authored, viewed,
and maintained in different natural-language variants that lower to the same AST:

```text
localized source (en / de / …)
          ↓
per-language grammar (keyword mapping)
          ↓
language-independent AST (comments & source spans, §4.3)
          ↓
semantic analysis → IR → interpreter / AOT (unchanged)
```

Because the AST is keyword-free and identifiers are arbitrary Unicode, a frontend
localizes keywords, error messages, and documentation — **not** identifiers (those
belong to the programmer). Semantic translation is lossless; exact textual
round-tripping (whitespace, original spelling) is not a guarantee without retaining
the original source or token stream (`src/ast/token_stream.rs`).

This is a _vision_, not a current milestone: English is the only implemented
frontend, and no localized grammar is scheduled.

---

## 3. Developer Setup and Installation

### Build and Test

```bash
cargo build --workspace       # compiler + LSP
cargo test --workspace        # full test suite
```

For the complete workflow (clippy, security checks, formatting, validation), see `AGENTS.md`.

### Directory Structure

```text
lale/
├── src/            # Compiler (grammar, AST, semantic analysis, IR, interpreter)
├── lale-lsp/       # Language server workspace member
├── stdlib/         # Lale standard library
├── examples/       # Example Lale programs
├── tests/          # Test suite
├── scripts/        # Build/helper scripts
├── doc/            # Documentation
└── Cargo.toml
```

### Required Environment

- Rust 1.85+
- Cargo (latest)
- No external toolchain is required for interpreter‑only operation.

---

## 4. Compilation Pipeline

```mermaid
graph TD
    SourceCode["Source Code"] --> P1["LaleParser (pest PEG)"]
    P1 --> T1["Parse Tree"]
    T1 --> B1["AST Builder (+ Pratt)"]
    Builtins["Embedded builtins.lale"] --> P2["LaleParser"]
    P2 --> T2["Parse Tree"]
    T2 --> B2["AST Builder"]
    StdlibSrc["stdlib/src/*.lale"] --> P3["LaleParser"]
    P3 --> T3["Parse Tree"]
    T3 --> B3["AST Builder"]
    B1 --> AST["User AST"]
    B2 --> BuiltinsAST["Builtins AST"]
    B3 --> StdlibAST["Stdlib AST"]
    BuiltinsAST --> SA_validate["Semantic Analyzer<br/>(temp manager, validation only)"]
    BuiltinsAST --> SA_main["Semantic Analyzer<br/>(shared SqliteSymbolManager)"]
    StdlibAST --> SA_main
    AST --> SA_main
    SA_main --> IRGen["IR Generator (user code)"]
    SA_main --> StdlibIR["IR Generator (stdlib + builtins)"]
    IRGen --> Merge["Merge stdlib + builtins<br/>IR into user module"]
    StdlibIR --> Merge
    Merge --> SerialisedIR["Serialised IR (line-oriented text)"]
    SerialisedIR --> Interpreter["Interpreter Backend"]
    SerialisedIR -.-> Future["Future AOT Backends<br/>(LLVM, GCC, Cranelift, …)"]
```

The pipeline progresses through independent parsing of three input sources, AST construction, and semantic analysis. Builtins are analyzed **twice**:

1. **Builtins validation** — with a **temporary** `SqliteSymbolManager` that is immediately discarded. This is a fail‑fast validation pass: if `builtins.lale` itself contains errors, compilation stops immediately with a clear message, before any user or stdlib code is touched. Using a temporary manager avoids polluting the shared symbol table with builtins symbols that will be registered again later.
2. **During stdlib IR generation** — `builtins::BUILTINS_SOURCE` is parsed again, merged with stdlib source files, and analyzed with a **fresh internal** `SqliteSymbolManager`. This pass is the one that actually produces IR. It needs its own clean manager because stdlib functions reference builtins symbols and the symbol table must be self‑contained for IR lowering.

The two passes cannot be merged: builtins validation must not taint the shared manager (or stdlib's internal manager), and the internal manager must not share state with user code analysis. Since `builtins.lale` is under 500 lines, the runtime cost of parsing and analyzing it twice is negligible.

The interpreter reads the serialised IR; future AOT backends will do the same.

### 4.1 Parser Generation

The parser is generated from `grammar/lale.pest` at compile time:

```rust
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/lale.pest"]
pub struct LaleParser;
```

This generates:

- A `Rule` enum with one variant per grammar rule.
- Parsing logic implementing the `Parser` trait.

#### Grammar Structure

The grammar uses Pest's PEG syntax with modifiers:

- **Silent rules** (`_{ }`) – match but do not produce parse tree nodes.
- **Atomic rules** (`@{ }`) – parsed as a single token, no inner structure.

Example:

```pest
sd = _{ (os ~ (NEWLINE | ";") ) + ~ os }        // statement delimiter
single_identifier = @{ alpha ~ identifier_continue* }  // atomic — one name segment
qualified_identifier = { single_identifier ~ (os_ln ~ "." ~ os_ln ~ single_identifier)* }
                                                  // path name — segments joined by .
```

The `sd` rule is the statement delimiter used throughout the grammar. Because
`NEWLINE | ";"` are alternatives inside a single rule, a line feed and a semicolon
are **exactly** equivalent: they can be used interchangeably, and repeated (so
blank lines and runs of semicolons are equally fine). This is what lets Lale make
semicolons optional without the ambiguity JavaScript needs automatic-semicolon-
insertion rules to resolve — there is no ambiguity because both tokens mean the
same thing.

#### Parse Tree Structure

The parse tree is a hierarchy of `Pair<Rule>` objects. Example:

```text
Pair<Rule::program>
├── Pair<Rule::var>
│   ├── Pair<Rule::var_symbol>
│   │   ├── Pair<Rule::single_identifier>: "x"
│   │   └── Pair<Rule::type_name>
│   └── Pair<Rule::expression>
│       └── Pair<Rule::float>
└── ...
```

_Note: The name `Pairs` is misleading – it is a generic container of `Pair` nodes, each holding metadata and children._

### 4.2 AST Construction and Pratt Parsing

The `ast/builder.rs` module converts the parse tree into a typed AST. Each grammar rule has a builder function:

```rust
pub fn build_program(pairs: Pairs<Rule>) -> Result<Program, String> { ... }
fn build_var_def(pair: Pair<Rule>) -> Result<VarDefStmt, String> { ... }
fn build_fn_def(pair: Pair<Rule>) -> Result<FnDefStmt, String> { ... }
fn build_expression(pair: Pair<Rule>) -> Result<Expr, String> { ... }
```

#### Pratt Parser Configuration

Expression parsing uses a top‑down operator precedence parser — a technique known as _Pratt parsing_ that assigns each operator a precedence and an associativity:

```rust
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
        .op(Op::infix(Rule::power, Assoc::Right))   // Right‑associative!
        .op(Op::infix(Rule::append, Assoc::Left))
        .op(Op::prefix(Rule::unary))
        .op(Op::postfix(Rule::member_access))
        .op(Op::postfix(Rule::conversion))
});
```

### 4.3 AST Structure and Source Fidelity

The AST is defined in `ast/definitions.rs` as Rust structs and enums. Every node includes a `SourceLocation`:

```rust
pub struct SourceLocation {
    pub line: usize,
    pub col: usize,
    pub start_pos: usize,
    pub end_pos: usize,
    pub source_file: String,
}
```

#### Program Root

```rust
pub struct Program {
    pub statements: Vec<Stmt>,
    pub location: SourceLocation,
    pub global_symbol_table: RefCell<Option<SymbolTable>>,
}
```

There is no separate "AST" struct; the grammar's top‑level rule is `program`.

#### Statement Types

```rust
pub enum Stmt {
    // Definitions
    VarDef(VarDefStmt), TypeDef(TypeDefStmt), EnumDef(EnumDefStmt),
    FnDef(FnDefStmt), FnSignature(FnSignatureStmt), UnsafeDecl(UnsafeDeclStmt),
    Use(UseStmt),
    // Control flow
    If(IfStmt), When(WhenStmt), Match(MatchStmt), Switch(SwitchStmt),
    Loop(Box<LoopStmt>), Return(ReturnStmt), Assert(AssertStmt),
    ExitProgram(ExitProgramStmt), ExitLoop(ExitLoopStmt), Rewind(RewindStmt),
    MoveOn(MoveOnStmt), MissingCode(MissingCodeStmt),
    // Assignments
    Assign(AssignStmt), CompoundAssign(CompoundAssignStmt),
    ValueAtAssign(ValueAtAssignStmt),
    // I/O
    Stdout(StdoutStmt), Stderr(StderrStmt), Stdin(StdinStmt), Debug(DebugStmt),
    Log(LogStmt),
    // Error stack
    AddError(AddErrorStmt), AlertErrors(AlertErrorsStmt), Alert(AlertStmt),
    // Function calls as statements
    FnCall(FnCall),
    // Compile-time
    CtIf(CtIfStmt), CtFail(CtFailStmt), CtWarn(CtWarnStmt),
    CtWhen(CtWhenStmt), CtMatch(CtMatchStmt), CtSwitch(CtSwitchStmt),
    // Comments
    Doc(DocStmt), Comment(CommentStmt),
    // Memory management and deferred statements
    Release(ReleaseStmt), OnExit(OnExitStmt),
    // Test suites (top-level only)
    TestSuite(TestSuiteStmt),
}
```

##### Conditional Matching Statements

```rust
pub struct WhenStmt {
    pub condition: Condition,
    pub body: Vec<Stmt>,
    pub else_branch: Option<Vec<Stmt>>,
    pub location: SourceLocation,
    pub comments: AttachedComments,
}

pub struct MatchStmt {
    pub arms: Vec<MatchArm>,
    pub else_arm: Vec<Stmt>,
    pub location: SourceLocation,
    pub comments: AttachedComments,
}

pub struct MatchArm {
    pub guard: Condition,
    pub body: Vec<Stmt>,
    pub location: SourceLocation,
}
```

##### Value Comparison Statement

```rust
pub struct SwitchStmt {
    pub value: Expr,
    pub cases: Vec<SwitchCase>,
    pub default_case: Option<Vec<Stmt>>,
    pub default_location: Option<SourceLocation>,
    pub location: SourceLocation,
    pub comments: AttachedComments,
}

pub struct SwitchCase {
    pub pattern: SwitchPattern,
    pub body: Vec<Stmt>,
    pub location: SourceLocation,
}

pub enum SwitchPattern {
    Enum { variant_name: Spanned<String>, fields: Vec<SwitchPatternField>, location: SourceLocation },
    Literal { value: Box<Expr>, location: SourceLocation },
}
```

##### Flow Control Placeholders

```rust
pub struct MoveOnStmt {
    pub location: SourceLocation,
    pub comments: AttachedComments,
}

pub struct MissingCodeStmt {
    pub location: SourceLocation,
    pub comments: AttachedComments,
}
```

#### Test Suite Statements

Test suites and test cases are top-level-only statements. Their names are
**identifiers** (not string literals); underscores are allowed. Comment/doc
attachment follows the **same rules as every other statement**: comment lines
directly above a case (no blank line) become that case's `comments.leading`; a
comment separated by a blank line — or one before `end test suite` — is a
standalone `TestSuiteItem` node preserved in source order.

```rust
pub enum TestSuiteItem {
    Case(TestCaseStmt),
    // Suite-level `var`/`fn` declaration, shared by all cases of the suite and
    // invisible outside it. Boxed to keep the enum small (Stmt is large).
    Declaration(Box<Stmt>),
    Comment(CommentStmt),
    Doc(DocStmt),
}

pub struct TestSuiteStmt {
    pub name: String,          // suite name (identifier)
    pub items: Vec<TestSuiteItem>, // ordered: declarations/cases interleaved with standalone comments/docs
    pub location: SourceLocation,
    pub comments: AttachedComments,
}

pub struct TestCaseStmt {
    pub name: String,          // case name (identifier)
    pub body: Vec<Stmt>,
    pub location: SourceLocation,
    pub comments: AttachedComments,
}
```

`TestSuiteStmt::declarations()` iterates just the suite-level `var`/`fn`
statements (`TestSuiteItem::Declaration`), mirroring `cases()`.

Grammar (`lale.pest`) — the `statement_comments` rule collects comment lines
preceding an item as its leading comments and tolerates the indentation of the
item keyword. The suite body is a list of `test_case | var | fn_def | info`
elements (`suite_item`), so any comment separated from the next item by a blank
line (or before `end test suite`) is a standalone `info` node of the suite:

```text
statement_comments = _{ (info ~ NEWLINE)* ~ os }
suite_item = _{ test_case | var | fn_def | info }
test_case  = { statement_comments ~ "test case" ~ single_identifier ~ stmt_block? ~ "end test case" }
test_suite = { statement_comments ~ "test suite" ~ single_identifier ~ (ms_ln ~ suite_item ~ (sd ~ suite_item)*) ~ "end test suite" }
```

Semantic analysis enforces that suite-level declarations appear before the first
test case. Validation and IR lowering are described in §4.5.7c and §4.8.

#### Expression Types

```rust
pub enum Expr {
    Binary(BinaryExpr), Unary(UnaryExpr), Conversion(ConversionExpr),
    Identifier(IdentifierExpr),
    IntLiteral(IntLiteral), UintLiteral(UintLiteral), FloatLiteral(FloatLiteral),
    HexLiteral(HexLiteral), CharLiteral(CharLiteral), BoolLiteral(BoolLiteral),
    StringLiteral(StringLiteral), ArrayLiteral(ArrayLiteral),
    FnCallExpr(FnCall), MemberAccess(MemberAccess), ArrayIndex(ArrayIndex),
    CompilerConst(CompilerConst), Grouped(Box<Expr>),
    // Optional type operators
    HasValue(Box<Expr>), HasNoValue(Box<Expr>),
    // Error stack expressions
    NothingExpr, HasErrors, LastError,
    // Optional propagation and heap allocation
    TryPropagate(Box<Expr>), Allocate(AllocateExpr),
}
```

#### Source Fidelity Design

The AST deliberately preserves syntactic information to enable exact source reconstruction and reformatting.

- **`Comment` / `DocComment`**: All comment text and placement preserved. Statements carry `AttachedComments`.
- **`Grouped(Box<Expr>)`**: Preserves parenthesized expressions. At IR generation, `Grouped` unwraps; no runtime overhead.

Example:

```rust
// Source: var x as i32 = ((a + b))
// AST: Stmt::VarDef(VarDefStmt { value: Expr::Grouped(Box::new( Expr::Grouped(...) )) })
```

A source reconstructor can recreate the exact parentheses or choose a different formatting.

Design trade‑offs:

- ✅ Perfect source reconstruction, future transcription tools.
- ❌ AST slightly larger (non‑semantic nodes present).

#### Comment and Doc Attachment

Lale deliberately keeps **two** comment representations (the "attached vs.
first-class" split):

- **Attached** — `comments.leading` / `comments.trailing` on a statement, for
  comments that are _adjacent_ to that statement.
- **First-class** — `Stmt::Comment` / `Stmt::Doc` (and
  `TestSuiteItem::Comment` / `TestSuiteItem::Doc`), for standalone comments.

The exact rule for what counts as "adjacent" (and therefore attached) is:

- A comment or `///` doc **on the line(s) directly above** a statement (no blank
  line in between) is stored as that statement's `comments.leading`, in source
  order. A comment on its own line _between_ two statements therefore belongs
  to the **following** statement, not the preceding one.
- A comment **on the same line as** the statement (after the statement's
  content) is stored as `comments.trailing`.
- Every other comment is standalone first-class: a comment **separated from the
  next statement by a blank line**, or a comment at the **end** of a block or
  file with no following statement.

Example (between statements):

```text
// Comment 1                      → leading of `foo`
var foo as i32 = 5 // comment 2    → trailing of `foo`
// Comment 3                      → leading of `bar` (not trailing of `foo`)
var bar as i32 = 6 // comment 2    → trailing of `bar`
```

Example (blank-line separation and end-of-file):

```text
// Comment 1                      → standalone (blank line below)

// Comment 2                      → leading of `foo`
var foo as i32 = 5 // comment 3    → trailing of `foo`

// Comment 4                      → standalone (blank line above)

// Comment 5                      → leading of `bar`
var bar as i32 = 6 // comment 6    → trailing of `bar`
```

The leading rule is encoded by the silent `statement_comments` grammar rule
(`(info ~ NEWLINE)* ~ os`) at the start of every statement; the trailing rule by
an optional `(info)?` at the end of each statement; and the standalone rule by
the `info` alternative in the `statement` rule. Test suites use the same rules
per case and keep standalone comments/docs as `TestSuiteItem` nodes between
cases for source fidelity.

### 4.4 Visitor Pattern

Visitors traverse the AST without modifying it, enabling multiple analysis passes.

#### `AstVisitor<T>` Trait

```rust
pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    // Dispatch methods (default-implemented; pattern-match over enum variants)
    fn visit_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_expr(&mut self, expr: &Expr) -> T;
    // Per-node visitors (concrete visitors override the ones they care about)
    fn visit_var_def(&mut self, var_def: &VarDefStmt) -> T;
    fn visit_assign(&mut self, assign: &AssignStmt) -> T;
    fn visit_use(&mut self, use_stmt: &UseStmt) -> T;
    // ... every Stmt/Expr variant has a corresponding visit_* method
}
```

#### Implementations

| Visitor            | Purpose               | Return Type |
| ------------------ | --------------------- | ----------- |
| `AstPrinter`       | Debug pretty‑printing | `()`        |
| `SemanticAnalyzer` | Semantic analysis     | `()`        |

Example: `AstPrinter::visit_var_def` pretty‑prints a definition with indentation.

### 4.5 Semantic Analysis

The `SemanticAnalyzer` performs static analysis to enforce type safety, physical unit constraints, escape prevention (pointer-to-local must not leave function scope), and `allocate`/`release` pairing (see §4.5.7b). It populates symbol tables attached directly to AST nodes (`Program` and `FnDef`) via `RefCell<Option<SymbolTable>>`, for direct AST-node access by tooling (IDE/LSP).

#### 4.5.1 Module Organisation

```text
src/semantic_analysis/
├── mod.rs               # Module exports and public API
├── analyzer.rs          # Core SemanticAnalyzer implementing AstVisitor
├── const_eval.rs        # Typed binary folding for compile-time constant evaluation
├── const_value.rs       # Compile-time constant value and SQLite (de)serialization
├── error_types.rs       # Error types and reporting infrastructure
├── expression_analysis.rs # Expression utilities and pointer detection
├── memory_safety.rs     # Memory safety validation for pointer and cast operations
├── module_resolver.rs   # Module resolution: discovery, DAG, circular-import detection
├── qualified_names.rs   # Qualified-name natural keys for the symbol-table schema
├── sqlite_symbol_management.rs # SQLite-backed symbol table operations
├── type_compatibility.rs # Type compatibility checking
├── type_conversion.rs   # Type conversion validation
└── unit_computer.rs     # Context-aware physical unit computation
```

Physical unit computation is handled by the separate `unit_analysis` module.

#### 4.5.2 Symbol Tables

Symbol types are defined in `ast/definitions.rs` and re‑exported. A unified `Symbol` type stores both variables and functions.

```rust
pub enum Linkage { Internal, Import, Export }
pub enum Visibility { Public, Private }
pub enum StorageClass { Default, ThreadLocal }
pub enum SymbolKind { Variable, Parameter, Function, TypeDef }

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
    /// For pointers: the type this pointer points to (if known).
    pub pointer_to_type: Option<String>,
    /// Module path where this symbol was defined.
    pub module_path: String,
}

type SymbolTable = HashMap<String, Symbol>;
```

##### AST‑Symbol Table Linkage

Global and function‑local symbol tables are linked directly to their AST nodes, populated during semantic analysis:

```rust
pub struct Program {
    pub statements: Vec<Stmt>,
    pub location: SourceLocation,
    pub global_symbol_table: RefCell<Option<SymbolTable>>,
}

pub struct FnDefStmt {
    pub name: Spanned<String>,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Stmt>,
    // ...
    pub symbol_table: RefCell<Option<SymbolTable>>,
}
```

Tooling (e.g. the LSP) reads symbols directly from the AST:

```rust
let globals = program.global_symbol_table.borrow().as_ref().unwrap();
let locals = fn_def.symbol_table.borrow().as_ref().unwrap();
```

##### Interior Mutability with RefCell<Option<>>

`RefCell<Option<>>` allows mutable borrowing of the symbol table through an immutable AST reference. The `Option` starts as `None` and is replaced during analysis.

##### Population During Analysis

After analyzing a scope, the analyzer stores the final symbol table in the corresponding AST node:

```rust
*program.global_symbol_table.borrow_mut() = Some(global_table.clone());
*fn_def.symbol_table.borrow_mut() = Some(fn_table.clone());
```

This architecture decouples analysis from code generation: the IR generator receives a fully analysed AST with pre‑computed symbol tables.

#### 4.5.3 Symbol Table Design Decisions

##### Implemented Fields

| Field            | Purpose                                 | Notes                                                                   |
| ---------------- | --------------------------------------- | ----------------------------------------------------------------------- |
| `linkage`        | Cross‑compilation‑unit visibility       | `Internal` (default), `Import`, `Export`                                |
| `visibility`     | Access control within source            | `Public` or `Private` (controlled via `private` keyword on type fields) |
| `storage_class`  | Memory storage location                 | Always `Default` (single‑threaded; `ThreadLocal` reserved for future)   |
| `is_definition`  | Distinguish declaration from definition | `true` for `var` and `fn`, enables forward declarations                 |
| `is_initialized` | Track initialization state              | `false` for `unsafe decl`, `true` otherwise                             |

##### Fields Not Implemented (and Why)

| Field            | Reason Not Included                                                                 |
| ---------------- | ----------------------------------------------------------------------------------- |
| `mutability`     | The IR's SSA values capture immutability; backend passes (mem2reg, DSE) handle it   |
| `address_taken`  | Only useful for sophisticated register allocation; not needed in the current design |
| `lifetime`       | Avoided by simple variable escape rule (see Memory Safety below)                    |
| `generic_params` | Lale does not yet support generics; will be added with types                        |

##### Type Representation

`data_type` uses `String` rather than a structured `TypeId` enum because:

1. Lale will support user‑defined classes (naturally strings).
2. String comparison is sufficient for current type checking.
3. If performance becomes an issue, strings can be interned into a `HashMap<String, u32>`.

#### 4.5.4 Scope Management

Lale uses a simple two‑level **runtime** scope model: **Global** and **Function**. There are no nested scopes for loops or blocks. A third kind of _storage_ scope — **Suite** — exists for the built-in test framework, but it is **name‑keyed storage in the SQLite symbol table**, not a third runtime scope (see §4.5.7c).

```rust
pub enum VarScope {
    Global,
    Function { scope_key: FnScopeKey },
    /// Suite-level storage shared by all cases of one test suite. The suite
    /// name is stored in `variables.function_qualified_name`, so it is invisible
    /// to non-test code and other suites.
    Suite { suite_name: String },
}
```

##### Scope Restrictions

| Item              | Global Scope | Function Scope | Suite Scope                        |
| ----------------- | ------------ | -------------- | ---------------------------------- |
| Variables (`var`) | ✓            | ✓              | ✓ (suite-level, shared by cases)   |
| Functions (`fn`)  | ✓ Only       | ✗ Forbidden    | ✓ (suite-level, callable by cases) |
| Types (`type`)    | ✓ Only       | ✗ Forbidden    | ✗ Forbidden                        |
| Parameters        | —            | ✓              | ✓ (suite function parameters)      |

Examples of forbidden nesting:

```lale
fn outer() returns nothing
    fn inner() returns nothing  // ERROR: Function inside function
        write "hello"
    end fn
end fn
```

These restrictions simplify analysis and avoid lifetime tracking. Inside a test suite, module-level (global) variables are invisible; a case/suite-function falls back to the suite scope rather than the global scope, keeping the runtime two‑level model intact.

#### 4.5.5 Memory Safety

Lale enforces a compile‑time rule: **references to local variables must not escape the function**. The check is **syntactic** and **per‑function**: it catches a literal `pointer to <local>` returned or assigned directly to a global, and a pointer variable that was assigned `pointer to <local>` and is then returned or stored globally within the same function. It does not track inter‑procedural escape, so a function that passes a pointer to a callee is not itself flagged, even if the called function might later store it globally.

| Allowed                                  | Forbidden                                                            |
| ---------------------------------------- | -------------------------------------------------------------------- |
| `pointer to x` used within same function | `return pointer to localVar`                                         |
| `pointer to x` passed to function call   | Store `pointer to localVar` in global                                |
| `return p` where `p` is a parameter      | `return p` / `global = p` where `p` was set to `pointer to localVar` |

Storing `pointer to localVar` into a struct/type that later escapes is **not** detected.

This works because:

- Pointers to global variables live forever → always safe.
- Pointers passed as parameters are _assumed_ to come from the caller’s scope (longer‑lived) → treated as safe, but this is not verified across a call boundary.
- A pointer variable assigned `pointer to localVar` is tracked within the function.
- Only `pointer to localVar` creates a pointer to short‑lived data → forbidden to escape.

The semantic analyzer detects escaping pointers and reports errors; raw `allocate`/`release` misuse is reported as warnings (see Section 4.5.7b).

##### Why Only Raw Pointers

Lale uses a single `pointer` primitive rather than typed pointers. The full design rationale is given in Section 5.14 (Raw Pointers Only). In brief: the semantic analyzer tracks the pointed‑to type via the `pointer_to_type` field, but the programmer works with an opaque pointer type that carries no type annotation, keeping the language simple while still allowing compile‑time validation of bit widths and semantics at dereference.

#### 4.5.6 Unit Analysis

Every expression has an associated unit:

```rust
pub enum ExprUnit {
    Unitless,
    Unit(String),
    Unknown,
}
```

The `expr_unit()` method computes expression units recursively. For function calls, the return unit is computed by substituting argument units for parameter names and evaluating the return expression. The return expression is located by `UnitAnalyzer::find_return_expr`, which scans the body in source order and recurses into control-flow bodies (`if` / `when` / `match` / `switch` / `loop`), so a `return` guarded by a condition still contributes its unit to the call site. Without this recursion, a guarded `return` would be missed and the call inferred as unitless, producing spurious “Return unit mismatch” errors in callers.

##### Unit Normalization

Units are stored in normalized form using `NormalizedUnit` (defined in `types/mod.rs`) with a `BTreeMap<String, Rational>` representation, where `Rational` is an exact reduced fraction (numerator/denominator). E.g., `kg*m^2/s^2 → {kg:1, m:2, s:-2}`. Fractional unit exponents (from powers like `^(1/2)`, `^(1/3)`) are represented exactly, so `m³ ^ (1/3) → {m:1}` and `m ^ (1/2) → {m:1/2}`.

Division binds the rest of a unit into the denominator (physics convention): `m²/s²⋅K` normalizes to `m²/(s²⋅K)`, not `(m²/s²)⋅K`. Unit annotations are limited to a single `/` at the grammar level. For the rationale and the contrast between unit syntax and expression associativity, see the [User Guide § How Division Works in Units vs. Expressions](lale.md#how-division-works-in-units-vs-expressions).

##### Unit Inference – No Unit Stripping

When assigning a value with a unit to a variable without a declared unit, the unit is **inferred** from the RHS. There is no syntax to strip units from a value. This is a deliberate design decision:

1. Dimensional safety – units cannot be accidentally discarded.
2. Explicit intent – if a unitless value is needed, start with one.
3. Consistency with “no implicit conversions”.

##### Runtime Unit Assertions

When a unit cannot be determined statically (e.g., exponent is a variable), the code generator emits runtime assertions that verify unit compatibility. These assertions are present in **both debug and release builds** – unit safety is a core language guarantee.

#### 4.5.7 Unused Symbol Detection

The analyzer warns about unused variables, functions, parameters, and loop variables. Exported and imported symbols are excluded. Detection uses definition locations to handle scope collisions correctly (e.g., same name in different scopes).

#### 4.5.7a Division-by-Zero Detection

_This is a compiler diagnostic (warning), not a language feature. It is analogous to
unused-symbol warnings — it does not affect the language specification, only the
compiler's ability to detect likely bugs._

The semantic analyzer performs compile-time division-by-zero detection for both `/` (division)
and `%` (modulo) operators, as well as their compound-assignment forms `/=` and `%=`.
A warning is emitted when both operands are integer literals and the divisor is zero, or when
a variable is used as a divisor without a prior non-zero guard.

This is a conservative, sound analysis: the compiler warns when it cannot prove the divisor is
non-zero, and stays silent only when safety is provable.

##### How It Works

Division-by-zero safety is proven by either of two independent checks, tried in order:

1. **Constant-expression proof** (`expr_is_constant_nonzero`): the divisor is a compile-time
   constant that is provably non-zero, _regardless of any surrounding guards_. This is the
   stronger, newer check — it is what proves `4 ⋅ π` non-zero in the balloon example.

2. **Guard matching** (`is_guarded_nonzero` fallback): the divisor structurally matches an
   expression that a surrounding `if` / `when` / `match` guard has already proven non-zero.

Either check proving safety suppresses the warning; only when both fail does the compiler warn.

**Constant-expression proof.** `expr_is_constant_nonzero` folds the divisor to its exact value
with the declared numeric type (via `expr_const_value_typed` + `const_eval`) and checks the result
for non-zeroness. It therefore recognizes:

- a non-zero numeric literal (`4`, `4.0`, `0x4`, …), matched by value;
- a variable whose persisted constant value is a known non-zero constant
  (`lookup_variable_const_value`, subject to the soundness rule below);
- any constant arithmetic expression folded exactly — including `a + b` (so `5 + 5` is non-zero
  and `5 + -5` is zero), `-x`, `a ⋅ b`, `a / b`, …;
- `Grouped`/`Conversion` wrappers, which are transparent and fold through.

When no declared type is available (bare-literal combinations such as `5 ⋅ 5`), a structural
`Neg`/`Mul`/`Dot`/`Div` fallback preserves the previous behaviour.

**The soundness rule — what counts as a constant.** A variable's constant value is persisted in
the SQLite `variables` table as three columns:

- `const_value` (the serialized value, `NULL` when unknown) — the _current_ known value, updated
  on every write;
- `is_shared` (TRUE once the storage could be mutated out of the compiler's sight — `pointer to`,
  `ref`, or `export`/`import`) — sticky;
- `is_reassigned` (TRUE once the variable is assigned with `=` / `+=` etc.) — sticky.

These give rise to **two** derived predicates, both exposed by the `variable_constants` view:

- `is_constant` = `NOT is_shared AND const_value IS NOT NULL` — the value is known _at the current
  point of the analysis walk_. This is flow-sensitive and is what div-zero and underflow detection
  read (`lookup_variable_const_value`).
- `is_effectively_constant` = `NOT is_shared AND NOT is_reassigned AND const_value IS NOT NULL` —
  the variable is a true program-wide constant, so the same value is valid at every use site. This
  is what IR constant propagation reads (`lookup_effectively_constant_value`).

The distinction matters because `const_value` is flow-sensitive: it is written on a `var`
definition or a straight-line assignment and cleared — or the variable is marked shared — the
moment any of the following could let it change:

- a plain assignment (`x = …`) or a compound assignment (`x += …`, `x /= …`) _inside a branch or
  loop body_ (the value is unknown at the join, so it is conservatively cleared rather than
  re-established; it is also marked `is_reassigned`);
- taking its address with `pointer to x` (a pointer can write through the address);
- passing it as a `ref` argument (the callee holds a mutable view of the caller's variable);
- being declared `export` or `import` (shared with another module or with C/FFI, hence mutable
  out of the compiler's sight).

`is_shared` and `is_reassigned` are sticky: once set, no later write can unset them. `arr[i]` and
`p.field` are unwrapped to their root variable (`lvalue_root_name`), so `pointer to arr[i]` or
passing an element by `ref` invalidates the whole aggregate.

A `var` defined in sibling branches of the same construct is a **join**: its value may differ
across branches, so it is conservatively treated as non-constant after the join (even if both
branches happen to assign the same value). Tracking the exact point where a joined variable
"becomes" constant again would require a full flow-sensitive constant-propagation lattice, which
was judged not worth the complexity.

**Type coverage.** All scalar kinds are tracked — `Bool`, `Int`, `Uint`, `Float`, and `Text`.
Aggregate values (structs, enums, arrays, optionals, vectors) are intentionally **not** tracked:
no consumer needs aggregate constants today, and supporting them would require a recursive
`ConstValue`, recursive literal folding, and a JSON-style encoding. This is a scoped, additive
extension for if/when a consumer appears.

The same store also backs the unsigned-subtraction underflow check (§6.1): when both operands of
`u8..u64 − u8..u64` resolve to known constants, their values are compared at compile time instead
of rejecting the subtraction conservatively.

**IR constant propagation.** During IR generation, a reference to an _effectively constant_
module-global is folded to an IR constant (`ConstInt`/`ConstUint`/`ConstFloat`/`ConstBool`) instead
of a `GlobalAddr` + `Load`. This is sound because the variable was never reassigned or shared, so
its value is the same at every use site. Function-local and suite-scoped variables are not folded
(the IR generator's scope resolution differs from the symbol manager's, so those are deferred).

**Typed binary folding — core + analyzer adapter (implemented).** The constant evaluator now
supports _typed_ binary folding, not just leaves. Two layers make this sound:

1. **`src/semantic_analysis/const_eval.rs`** — the shared arithmetic core. It folds a
   binary/unary/comparison operation over a `NumericTypeInfo { bits, is_signed, is_float }`
   descriptor (from `TypeValidator::get_type_category`) to a **three-valued result**:

   | `EvalResult` | Meaning                                                                  |
   | ------------ | ------------------------------------------------------------------------ |
   | `Value`      | a concrete `ConstValue` (folded)                                         |
   | `Unknown`    | an operand is not a known constant, or the op is not a numeric fold      |
   | `Trap`       | the operation traps at runtime (overflow, underflow, div-by-zero, `NaN`) |

   The arithmetic matches the interpreter exactly: integers fold in `i128`/`u128` and are then
   reduced to the declared width — `+ - * neg` trap on overflow when `checked_overflow` is true
   (the default) and wrap two's-complement otherwise; `/ %` **always** wrap (`MIN / -1 == MIN`)
   and trap only on a zero divisor (`/` truncates toward zero, `%` uses the
   Euclidean remainder — always non-negative).
   Floats fold in `f64` (the interpreter stores every float width — including `f16` — as `f64`);
   division by zero traps, and a `NaN`/`Inf` result or a `NaN`/`Inf` comparison traps, per Lale's
   "no silent errors" policy (§5.25). Operands are coerced by **declared type** rather than their stored tag,
   because an unsuffixed literal `5` is persisted as `Int(5)` even when it was later inferred to a
   `u32`/`f64` variable.

2. **`SemanticAnalyzer::expr_const_value_typed(expr, ty_hint)`** — the analyzer adapter (in
   `src/semantic_analysis/analyzer.rs`). It resolves the operand type via
   `expr_type_with_context`, recursively evaluates the operands through itself, and dispatches to
   `const_eval`. For a comparison it folds with the **operand** type (the expression's own result
   type would be `bool`); for a bare literal it honours the caller's `ty_hint`. `Trap` propagates
   from a subexpression, and `Unknown` is returned when any operand is unknown or the operator is
   not a numeric fold (logical/bitwise/shift/pow/vector/string operations).

3. **`IrGenerator::eval_const` + dead-branch elimination** — the IR-generation consumer (in
   `src/ir_gen.rs`). `eval_const` is the IR-side counterpart of `expr_const_value_typed`: it folds
   the same literal/arithmetic/comparison/logical operations, but resolves identifiers through
   `lookup_effectively_constant_value` (a _program-wide_ constant — IR generation has no
   flow-sensitive information) and only for module-globals. `fold_condition` reduces the guard of an
   `if`/`when`/`match`/`loop` to a compile-time boolean: a proven `true`/`false` emits only the
   live branch and drops the dead one's code entirely; `Unknown`/`Trap` emit both branches as
   before. Only the _leading_ condition of each construct is folded today (see below).

4. **`SemanticAnalyzer::fold_compound_assign_const` — compound-assignment folding** (in
   `src/semantic_analysis/analyzer.rs`). When `x += rhs` (also `-=`, `*=`, `/=`, `%=`) has a known
   current value and a known RHS, the analyzer folds the result and writes it back to the
   flow-sensitive `const_value` store instead of clearing it. `is_reassigned` remains sticky (so
   `is_effectively_constant` stays false), but the flow-sensitive `is_constant` stays true, feeding
   the division-by-zero and unsigned-underflow checks with the value _after_ the assignment. An
   overflow/underflow/division-by-zero/`NaN` (`Trap`) clears the constant conservatively.

5. **Precise div-zero / underflow proofs** — `expr_is_constant_nonzero` and
   `check_unsigned_underflow` now fold their operands through `expr_const_value_typed` (items 1–2)
   instead of the old leaf-only proof, so `(a + b) - c` and `a + b` resolve to exact values. The
   remaining conservative paths are only `Unknown`/`Trap` operands and bare-literal combinations
   with no declared type.

**Still to be wired up.** The runtime `NaN` and `Inf` traps are done.
Deeper chain folding — eliminating a later `match` arm once an earlier guard is proven
`true`/`false` — is likewise deferred. Aggregate constants (structs/enums/arrays/optionals/vectors)
are not tracked (see "Type coverage" above).

**Guard matching (fallback).** When the constant proof does not apply, the analyzer falls back
to the structural guard analysis:

1. **Guard extraction**: When the analyzer encounters the guard of a conditional statement
   (`if`, `when`, or a `match` arm), it extracts facts about which expressions are provably
   non-zero using interval arithmetic. For example, `when divisor != 0` yields the fact
   `NotZero(divisor)`. Works for all comparison operators and constants: `x > 5`, `x < -1`,
   `x >= 1`, `x != 0`, `0 != x`, and their negated forms.

2. **Path-sensitive tracking**: Facts are pushed onto a guard stack when entering a guarded
   branch and popped when leaving. Each guarded branch has its own fact set.

3. **Structural matching**: When a division is encountered, the divisor expression is compared
   structurally (ignoring source locations) against all facts on the guard stack. If the exact
   expression tree matches, the division is proven safe. Numeric literals are matched by value,
   so `4` and `4.0` (and `0x4`) denote the same divisor.

##### Limitations

The analysis is intentionally conservative. Safe divisions may still trigger warnings when:

- The divisor is a compound expression (e.g., `a / (b + c)`) not matched by a guard on the same
  expression, and not a constant product/quotient.
- The divisor contains a function call (`a / (R ⋅ T(h))`): the constant proof only reasons about
  literals and tracked variables, so a call can only be proven non-zero by an explicit structural
  guard, never by constant folding.
- The guard uses a mathematically equivalent but structurally different expression (e.g.,
  `sin(x) != 0` does not guard `2*sin(x)` — requires Layer 2 sub-expression reasoning).
- A variable that _could_ be constant is not provably so, because it is `export`ed/`import`ed,
  address-taken, or `ref`-passed (the soundness rule deliberately errs toward warning).

Numeric-literal spelling differences (`4` vs `4.0`) do **not** fall in this category: they are
matched by value, so `4 ⋅ π` guards `4.0 ⋅ π` — and, in fact, `4 ⋅ π` is proven non-zero
outright as a constant product.

The warning message includes a note explaining this limitation.

##### Relationship to Runtime Checks

The `ZeroCheck` IR instruction is emitted before `/`, `%`, `/=` and `%=` operations in both
debug and release builds. If the divisor is zero at runtime, the program aborts. The compile-time
warning is an additional safety layer.

##### Implementation

`src/semantic_analysis/const_value.rs` — the `ConstValue` type (`Bool`, `Int`, `Uint`, `Float`
stored as IEEE-754 bits, `Text`), `is_non_zero`, the numeric coercions (`as_unsigned`, `as_signed`,
`to_f64`), and the SQLite (de)serialization.

`src/semantic_analysis/sqlite_symbol_management.rs` — the `variables` table gains `is_shared`,
`is_reassigned`, and `const_value` columns (with `ALTER TABLE` migrations) plus the
`variable_constants` view; `resolve_variable_key` resolves a name with the same local→global/suite
fallback as `lookup_var_symbol`, and `lookup_variable_const_value` / `update_variable_const_value`
/ `mark_variable_shared` / `mark_variable_reassigned` / `lookup_effectively_constant_value` are the
scope-aware accessors.

`src/semantic_analysis/analyzer.rs` — functions `structurally_equal`,
`numeric_literal_value`, `lvalue_root_name`, `expr_is_constant_nonzero`, `expr_const_value`,
`compare_against_constant`, `extract_facts`, `divisor_equivalent_forms`, `is_guarded_nonzero`,
plus modifications to `visit_if`, `visit_when`, `visit_match`, `visit_binary`,
`visit_compound_assign`, `visit_unary`, `visit_var_def`, `visit_fn_call_stmt`,
`visit_fn_call_expr`, and `visit_assign`. Constant values are written in `visit_var_def` and
`visit_assign`, and invalidated in `visit_assign` (branch writes), `visit_compound_assign`,
`visit_unary` (`pointer to`), and `visit_fn_call_stmt` / `visit_fn_call_expr` (via
`invalidate_ref_passed_vars`); `mark_variable_reassigned` is called on any assignment.

`src/ir_gen.rs` — `emit_const_value` coerces a `ConstValue` to an IR constant of the declared
type, and `try_generate_expr_with_resolved_type` folds an effectively-constant module-global
identifier to that constant instead of a `Load`.

#### 4.5.7b Allocate-Release Pairing Check

_This is a compiler diagnostic (warning), not a language feature._

The semantic analyzer tracks `allocate()` calls in each function scope and verifies that
every allocation has a corresponding `release` or `on exit release` on all code paths.
Warnings are emitted for:

- **Unreleased allocations** — a variable allocated via `allocate()` is never passed to
  `release` or `on exit release` within the same function.
- **Overwritten allocations** — reassigning a pointer variable with `allocate()` without
  first releasing the previous allocation.
- **Double release** — a variable is released more than once (explicitly, or via a
  combination of `release` and `on exit release`).
- **Use-after-release** — a pointer is dereferenced with `value at` after an explicit
  `release` in the same function.

The check follows the same per-function-scope pattern as unused-symbol detection and
unchecked-optional warnings. Tracking state (`allocated_vars_in_scope`, `released_vars`,
`on_exit_released_vars`, `explicitly_released_vars`) is cleared at function exit.

#### 4.5.7c Test Suite Validation

Test suites are validated at module scope (`visit_test_suite` / `visit_test_case`):

- **Placement** — a test suite must be the last top-level statement; only
  comments/docs may follow it.
- **Structure** — a suite requires at least one test case; suites and cases may
  not be nested; suite-level `var`/`fn` declarations must appear before the first
  `test case`.
- **Duplicate names** — duplicate suite names (program-wide), duplicate case
  names within a suite, and colliding suite/case scope keys are rejected.
- **Function-like case scope** — a test case behaves like a function: variables
  defined in a case body are case-local (invisible to other cases, other suites,
  and non-test code). `fn`, `type`, `enum`, and `use` definitions are rejected
  inside a case body.
- **Suite scope** — a suite-level `var` is stored under the suite name in the
  `variables.function_qualified_name` column (`VarScope::Suite { suite_name }`),
  so it is shared by every case and reachable from suite functions but invisible
  to non-test code and other suites. A suite-level `fn` is registered under the
  mangled name `suite::fn` and analyzed with the suite scope as its fallback.
  Both are entered/analyzed by `enter_suite_scope`/`visit_suite_fn_def` before the
  cases.
- **Invisible module globals** — module-level (global) variables are **not**
  visible from a suite or its cases: `is_variable_defined`/`lookup_var_symbol` for
  a suite/case scope never fall back to the global scope. Referencing a global is
  therefore the normal `Undefined variable` / `Assignment to undefined variable`
  error. Module-level **functions** remain callable (they are resolved from the
  global function table when no suite function shadows them).

This restores the two-level runtime scope model: for a test suite the two levels
are **suite scope + case-local**, and for normal code they are **global +
function-local**. Suite scope is implemented as name-keyed SQLite storage, not a
third runtime scope.

#### 4.5.7d Definite Assignment Across Branches

A `var` defined inside a control-flow branch is usable after the construct only
if it is **defined on every path** through the construct with a **consistent type
and unit**. Otherwise, using it afterwards is a compile error (not a silent
zero), because the single function/global scope holds the binding while the
initializer may never have executed.

Applied to:

- **`if`/`else`** — both branches must define the variable (same type and unit).
- **`when`** — one-sided, so a body `var` is never definitely assigned after.
- **`match`** — every arm **and** the `else` arm must define it.
- **`switch`** — every case **and** the `default` (if present) must define it.
- **`loop`** — a body `var` is never definitely assigned after (the body may run
  zero times); the loop **header** variable (injected as a synthetic `VarDefStmt`
  before the loop) remains accessible.

Implementation (`src/semantic_analysis/analyzer.rs`):

- `branch_def_stack` — a stack of `BranchDefs` frames. `visit_var_def` records a
  definition (name → type+unit+location) into the top frame via
  `record_branch_def`.
- `sibling_defined_stack` — one set per open construct. `end_branch` merges a
  branch's defined names into it, so a later sibling branch's same-named `var` is
  a **join** (allowed) rather than a duplicate; `visit_var_def` skips
  re-registration in that case (`is_sibling_join`).
- `join_branch_defs` — intersects the branch definitions; any name not defined in
  every branch (or with a differing type/unit) is added to `branch_only_vars`.
- `visit_identifier` reports a clear error for a `branch_only_vars` name.

IR generation (`src/ir_gen.rs`) mirrors the join: a sibling-branch `var` reuses
the slot allocated in the previous branch (`is_sibling_join` in
`try_generate_var_def`) instead of allocating a second one. For globals the
address is re-derived via `global_addr` in the current block (the original
`global_addr` value may live in a sibling block). `generate_if`/`generate_when`/
`generate_match`/`generate_switch`/`generate_loop` enter/exit a sibling-defined
set so nested constructs scope correctly.

#### 4.5.8 SQLite‑Backed Symbol Table

The symbol table is backed by an **in‑memory SQLite database** (`:memory:`) by default (feature `sqlite-symbols`). SQLite is the single source of truth during semantic analysis and IR generation; the AST-attached `HashMap` tables (§4.5.2) are retained for direct AST-node access by tooling (IDE/LSP).

##### Rationale

- Declarative queries (self‑documenting).
- Universal language (SQL) understood by new contributors.
- Natural fit for module graph → relational tables.
- Future‑ready for IDE integration.

##### Schema (Module-Aware)

```sql
CREATE TABLE modules (
  path TEXT PRIMARY KEY,
  base_dir TEXT NOT NULL,
  is_parsed BOOLEAN NOT NULL DEFAULT 0,
  is_analyzed BOOLEAN NOT NULL DEFAULT 0
);

-- qualified_name = simple_name + "_" + param_types.join("_")
-- (C-style mangling; the return type is NOT part of the key)
CREATE TABLE functions (
  qualified_name TEXT PRIMARY KEY,
  simple_name TEXT NOT NULL,
  return_type TEXT NOT NULL,
  return_unit TEXT,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_export BOOLEAN NOT NULL DEFAULT 0,
  linkage TEXT NOT NULL DEFAULT 'Internal',
  visibility TEXT NOT NULL DEFAULT 'Private',
  storage_class TEXT NOT NULL DEFAULT 'Auto',
  is_definition BOOLEAN NOT NULL DEFAULT 1,
  FOREIGN KEY (module_path) REFERENCES modules(path),
  UNIQUE (module_path, simple_name, return_type)
);

-- function_qualified_name IS NULL for global variables
CREATE TABLE variables (
  function_qualified_name TEXT,
  simple_name TEXT NOT NULL,
  var_type TEXT NOT NULL,
  physical_unit TEXT,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_global BOOLEAN NOT NULL DEFAULT 0,
  is_parameter BOOLEAN NOT NULL DEFAULT 0,
  param_index INTEGER,
  pass_mode TEXT NOT NULL DEFAULT 'copy',
  linkage TEXT NOT NULL DEFAULT 'Internal',
  visibility TEXT NOT NULL DEFAULT 'Private',
  storage_class TEXT NOT NULL DEFAULT 'Auto',
  is_definition BOOLEAN NOT NULL DEFAULT 1,
  is_initialized BOOLEAN NOT NULL DEFAULT 0,
  pointer_to_type TEXT,
  is_shared BOOLEAN NOT NULL DEFAULT FALSE,
  is_reassigned BOOLEAN NOT NULL DEFAULT FALSE,
  const_value TEXT,
  PRIMARY KEY (function_qualified_name, simple_name),
  FOREIGN KEY (function_qualified_name) REFERENCES functions(qualified_name),
  FOREIGN KEY (module_path) REFERENCES modules(path)
);

-- Derived view: which variables are compile-time constants
-- (is_constant vs. is_effectively_constant — see §4.5.7a constant propagation)
CREATE VIEW variable_constants AS
SELECT
  function_qualified_name,
  simple_name,
  is_global,
  is_shared,
  is_reassigned,
  const_value,
  (NOT is_shared AND const_value IS NOT NULL) AS is_constant,
  (NOT is_shared AND NOT is_reassigned AND const_value IS NOT NULL) AS is_effectively_constant
FROM variables;

CREATE TABLE type_defs (
  qualified_name TEXT PRIMARY KEY,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_export BOOLEAN NOT NULL DEFAULT 0,
  FOREIGN KEY (module_path) REFERENCES modules(path)
);

CREATE TABLE type_def_fields (
  type_def_qualified_name TEXT NOT NULL,
  field_index INTEGER NOT NULL,
  field_name TEXT NOT NULL,
  field_type TEXT NOT NULL,
  physical_unit TEXT,
  is_private BOOLEAN NOT NULL DEFAULT 0,
  PRIMARY KEY (type_def_qualified_name, field_name),
  FOREIGN KEY (type_def_qualified_name) REFERENCES type_defs(qualified_name)
);

-- Key-value store for analyzer state (avoids in-memory redundancy)
CREATE TABLE metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Secondary indices (see init_schema in sqlite_symbol_management.rs):
--   functions(simple_name), functions(module_path),
--   variables(function_qualified_name), variables(module_path),
--   variables(simple_name), type_defs(module_path),
--   type_def_fields(type_def_qualified_name)
```

#### 4.5.9 Database Infrastructure for Path Names

Additional types (`QualifiedFunctionName`, `VariableKey`, `QualifiedTypeName`) and a `SymbolTableAdapter` trait have been implemented (240+ tests passing) to support path names in future phases.

### 4.6 Module System

Lale’s module system enforces a **directed acyclic graph** (DAG) of imports – circular imports are forbidden.

#### `use` vs. `import`

| Feature               | `use` statement                         | `import` modifier                                  |
| --------------------- | --------------------------------------- | -------------------------------------------------- |
| **Scope**             | Compile‑time module loading             | Linker‑level external symbols (C FFI)              |
| **Purpose**           | Load symbols from Lale modules          | Declare external C functions and variables         |
| **Syntax**            | `use add, subtract from local.math.lib` | `import var x as u8` / `import fn signature foo()` |
| **Symbol visibility** | Imported symbols stay in importing file | Symbols available at link time only                |
| **Resolution**        | Compiler reads `.lale` files            | Linker resolves symbols from C libraries           |
| **DAG Enforcement**   | Yes – circular `use` rejected           | No – linker handles external symbols               |

##### Imported Function Signatures

enable the standard library to wrap C functions without hidden libc dependencies.

#### Symbol Naming: Mangled Internals vs. Plain-C Externs

Lale has **two symbol-naming spaces**, aligned with the two ABIs
(`doc/ABI_SPECIFICATION.md` §1–§4):

| Axis                 | Naming                         | Overloading                      | Analogy               |
| -------------------- | ------------------------------ | -------------------------------- | --------------------- |
| Internal (Lale→Lale) | mangled (`name + "_" + types`) | intended — see `roadmap.md` §4.2 | C++ default (Itanium) |
| FFI (`import fn`)    | plain C symbol name            | impossible — C has one `sqrt`    | C++ `extern "C"`      |

- **Internal functions** are mangled to keep distinct signatures distinct at link
  time. The current scheme is `qualified_name = simple_name + "_" +
param_types.join("_")` (no return type, no unit). It is an ad-hoc, incomplete
  mangling: two functions differing only by unit (`sqrt(x as f64 in <m>)` vs.
  `sqrt(x as f64 in <s>)`) would collide, and it does not encode module
  qualification.
- **Extern functions** (`import fn`) map directly to the C symbol name and are
  resolved by the linker. C has no overloading, so the FFI boundary is inherently
  single-name: `import fn` is Lale's analogue of C++ `extern "C"`, and overloading
  cannot apply there.

This split is why C++-style overloading does **not** require abandoning C-style
linking — it only requires upgrading the internal mangling to a complete,
unambiguous scheme. See `doc/roadmap.md` §4.2 "Function overloading" for the
current gaps (simple-name lookups, the `functions` UNIQUE constraint, and the
dead arg-type resolver).

#### Module Resolution

A `use` statement has the form `use <symbols> from <origin>.<path>` (or `use all from …` to
import every export). The **origin** selects one of two search roots, then the dotted **path**
maps to a file by the same flat rule — the last segment is the file, the preceding segments
are directories:

| Origin  | Search root                                         | Example                            |
| ------- | --------------------------------------------------- | ---------------------------------- |
| `std`   | the installed standard-library root (`stdlib/src/`) | `std.math` → `math.lale`           |
| `local` | the importing file's directory                      | `local.math.lib` → `math/lib.lale` |

```lale
use add, subtract from local.math.lib  // ./math/lib.lale
use calculate from local.physics.mechanics.force
```

The origin is always explicit — a bare `use math` is a grammar error. `std` and `local` are
**versionless** (no `version` clause); the `hub` origin and versioning are deferred to 2.0.0
(roadmap §4.3). A path must end in a file: a bare directory is rejected with a dedicated
`ModuleError::BareDirectory`. Parent directory access (`..`) is forbidden, there is no
reserved entry file (`lib.lale`/`main.lale`), and all `use` statements must appear at the top
of the file (a warning is emitted otherwise).

#### Exports and Visibility

Only symbols marked `export` are accessible to other modules. To import all exported symbols from a module, omit the import list:

```lale
use all from local.math.lib      // imports all exports from math/lib.lale
use add, subtract from local.math.lib  // imports only add and subtract
```

Exported symbols use the `export` modifier:

```lale
export var gravity_constant as f64 = 9.8
export fn calculate_weight(mass as f64) returns f64 ...
```

Cross-module `use` resolution reads a module's exports directly from the SQLite
symbol database via `SqliteSymbolManager::get_exports(module_path)`, filtered by
`linkage = export` and `module_path`. The `ModuleResolver` does **not** keep a
redundant in-memory copy of exports — SQLite is the single source of truth for
symbols, exports included (§4.5.8).

##### Cross-module IR (done)

Cross-module access to values and functions — including from **user modules** — now
works end-to-end:

1. ✅ `compile_and_execute` generates IR for every non-root user module in dependency
   order (`IrGenerator::generate_and_merge_module`, using the shared SQLite symbol
   manager) and merges each via `copy_module_to_target` before the root module.
2. ✅ `copy_module_to_target` copies `Linkage::Export` globals (not only `Internal`)
   and remaps every `GlobalAddr` a copied function references — `Export` included —
   carrying the constant `initializer` with the `Global` entry (§5.29). The importing
   module references the _same storage_ as the exporting module (see §5.4).
3. ✅ The IR generator resolves imported value names via `lookup_imported_global`
   (SQLite-backed), for reads (`Expr::Identifier`) and writes (`try_generate_assign`); a
   standalone user-module pass creates a placeholder global that `copy_module_to_target`
   remaps by name.

##### Module-qualified access (`math.e`, `math.double(…)`)

Module-qualified access to **values and functions** now works when the module is part
of the compilation (referenced via `use`). Same-named exports across modules are
disambiguated by qualifying IR names with the module file name (`@math.lale.e` vs
`@physics.lale.e`); the analyzer's `visit_member_access` / `expr_type_with_context` /
`visit_fn_call_expr` resolve a module-name object against `get_exports(module_path)`,
and the IR generator emits a reference to the shared global or the mangled function.
Reads (`write math.e`), writes (`math.e = …`), and calls (`math.double(21)`) are all
supported; an unqualified imported name prefers the `Import` entry (so `use e from local.math`
then `write e` resolves to `math.e`, not an unrelated `e`).

Remaining:

1. **Qualified access without any `use`** (`write math.e` where `math` is never imported)
   is not supported — the module must be in the dependency graph. On-demand module
   discovery is a separate feature.
2. **Same-named functions across modules** still collide at the IR level: function
   mangling (`name + "_" + param_types`) does not encode the module, so `math.double` and
   `physics.double` both mangle to `double_i32`. This is the function-overloading gap in
   `roadmap.md` §4.2 item 2.

#### Module Execution Model

Module-level `var` initializers must fold to compile-time constants (§5.29). There is
no per-module initialization block beyond those constant initializers: the value
travels with the `Global` entry (its `initializer` field), so merging modules carries
the value without propagating top-level `Store` instructions. Non-constant setup is
written explicitly as top-level code in the entry point. Tests use the dedicated
`test_suite` / `test_case` keywords.

#### Circular Import Prevention

The compiler builds a dependency graph during semantic analysis:

1. Each import records a dependency edge.
2. A topological order is maintained; an import that would create a back‑edge is rejected.
3. The error message lists the cycle.

No post‑facto cycle detection – cycles are prevented during compilation.

#### Built‑ins (`builtins.lale`)

`src/builtins/builtins.lale` is embedded in the compiler binary via `include_str!` (`src/builtins/mod.rs`). It contains functions the compiler pipeline depends on regardless of whether the standard library is present:

- **Type definitions**: `text` struct (`{ ptr, bytes, chars }`), `binary` struct (`{ ptr, bytes }`)
- **FFI declarations**: `__lale_malloc`, `__lale_free`, `__lale_exit` (user must provide these in embedded/no‑stdlib environments; user-facing Lale keywords are `allocate`/`release`/`exit program`), plus `__lale_timestamp` and `__lale_log_level` (runtime UTC timestamp and log-level threshold)
- **Memory operations**: `__lale_memcpy` (pure Lale)
- **String construction**: `__lale_construct_text_in_place`
- **Type conversion**: `__lale_bool_to_str`, `__lale_char_to_str`, `__lale_u64_to_str`, `__lale_i64_to_str`, `__lale_i32_to_str`. (The `f64`-to-string conversion is implemented in the interpreter — see §4.8 — because shortest round-trip formatting needs 128-bit arithmetic Lale cannot express.)

**`text` type: single source of truth.** The `text` struct is defined only in `builtins.lale`. Builtins are loaded into the IR module before user code is generated (`compile_and_execute` loads `builtins.lale` first), and struct definitions are copied into the target module by `copy_module_to_target`. The generator seeds its type-layout table from the module's struct definitions, so there is no Rust-side pre-registration of `text`. `builtins.lale` is the single source of truth for the `text` type definition, consistent with every other type.

Builtins are **not part of the standard library** — they live in `src/builtins/`, not `stdlib/src/`. They are always available, even with `--stdlib-level=none`. They are validated before any user or stdlib module is analyzed.

#### Standard Library Handling

Stdlib modules (`stdlib/src/*.lale`) are compiled alongside the user program in a merged IR module. The compilation order is:

1. **Builtins validation** — `src/main.rs:compile_and_execute`: validates embedded `builtins.lale` against a **temporary** `SqliteSymbolManager` (not the shared one) to catch errors before the main analysis.
2. **User code semantic analysis** — modules are analyzed in dependency order using the shared manager.
3. **User code IR generation** — the main program's AST is lowered to IR using the shared `SqliteSymbolManager` (has user module symbols).
4. **Stdlib + builtins IR generation** — `load_stdlib_and_dependencies` in `src/ir_gen.rs` combines embedded `builtins::BUILTINS_SOURCE` and stdlib source files into a merged program, runs semantic analysis with its **own internal** `SqliteSymbolManager` (separate from the shared one), then lowers to IR. The result is merged into the user's IR module (function IDs remapped).

   The two IR generation runs are separate because they use **different symbol managers**: the shared one holds user module symbols, while stdlib+builtins needs its own clean manager. Merging them at the AST level would pollute the shared manager with stdlib internals.

- `use all from std.full` in source code does **not** trigger on‑demand compilation — stdlib symbols are already available from the merged module.
- `--stdlib-level=none` disables stdlib loading entirely. Builtins remain available (they are part of the compiler, not the stdlib).
- stdlib loading failure is a **hard error** — execution stops.

#### Conditional Compilation for Platform‑Specific Modules

```lale
#if #posix
    // POSIX‑only code
#end if
#if #windows
    // Windows‑only code
#end if
```

Compiler defines: `#posix`, `#windows`, `not #posix`, `not #windows`.

#### Module Import Ambiguity Detection

Two modules may each `export` a same-named symbol — definitions are stored
independently (module-aware uniqueness in `define_symbol`, keyed by `module_path`),
so two `export var e` in different modules coexist. What is forbidden is _importing_
the same name from two modules without qualification:

```text
Symbol 'add' imported multiple times (from 'math.lale' and 'extra_math.lale'). Use a qualified path.
```

Detection happens at **import time**: `visit_use` → `import_one` records an `Import`
row whose `module_path` is the source module, and a second import of the same name
from a _different_ module is rejected (re-importing from the _same_ module is
idempotent). The source is queried via `SqliteSymbolManager::imported_var_source()`.
A use-site backstop (`visit_identifier` → `lookup_all_var_symbols`) catches bare
names that resolve to imports from multiple modules.

The escape hatch is a qualified path (`math.add`), resolved by module-qualified
access against `get_exports(module_path)` — see the remaining gap in §4.6
"Exports and Visibility".

#### Stdlib Design Principles

1. **Explicit error handling** – optional types (`T?`, `?` operator) for recoverable errors, error stack (`add error`/`last error`) for diagnostics. No hidden control flow — every error path is visible in the code.
2. **No implicit type coercion** – all conversions are explicit via the `as` operator. Numeric types, pointers, and characters require visible casts.
3. **Memory safety** – no buffer overflows, bounds‑checked strings.
4. **Clear functionality** – functions are small, focused, and named for what they do.
5. **No hidden allocations** – allocations are explicit.
6. **Single low-level FFI boundary** – the stdlib builds on a small, stable set of
   primitive `import fn` functions (unbuffered I/O, allocation, libm math,
   exit). Buffered/stream abstractions and everything else are implemented in
   Lale on top of that boundary — not duplicated as POSIX _and_ C stdio.

#### Open Questions

1. **`use` (source) vs `import fn` (FFI)** — whether `import fn` stays a separate, versionless
   mechanism or is unified with the origin syntax.
2. **`std` on-disk layout** — how `std.<path>` maps onto the installed stdlib, reconciled with
   the existing stdlib import mechanism.
3. **Re-export semantics** — whether `use all` pulls re-exported symbols, and whether
   re-export exists at all.

These are tracked in `roadmap.md` §11.

### 4.7 Intermediate Representation

The IR is the single, stable interface between the Lale frontend and any execution engine. It is designed for a pure‑Rust interpreter, future self‑hosting in Lale, and _lowering_ — translating the IR into progressively lower-level forms — to LLVM, GCC (libgccjit), Cranelift, or any other backend.

#### Design Goals

1. **SSA values** – every temporary is defined exactly once (single assignment), enabling simple, efficient interpretation and optimisation.
2. **Backend agnostic** – one IR serves the interpreter and all future backends.
3. **Type safe** – IR types mirror Lale’s type system exactly.
4. **Unit aware** – physical units are preserved and checked (via `AssertUnit`).
5. **Block-based control flow** – a function is represented as a _control-flow graph (CFG)_: a set of _basic blocks_ (straight-line instruction sequences with no internal branches) connected by branches. Every block ends in an explicit `Br`/`CondBr` terminator. This shape maps directly onto LLVM, libgccjit, Cranelift, and C emission, without a _relooper_ (a tool that reconstructs structured loops and conditionals from flat blocks).
6. **Stable serialisation** – a line-oriented text format allows the frontend to
   emit a self-contained IR file that any backend (in any language) can consume.

#### IR Types

Mirrors Lale's type system exactly. There is no separate `Str` built‑in; `text` is a named struct (type) as in the source language.

```rust
pub enum IrType {
    Void,
    Bool,
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    Byte,                            // Raw octet (not a number)
    F16, F32, F64,
    Char,
    Ptr(Box<IrType>),                 // Typed pointer with target type
    Array { element: Box<IrType>, size: u64 },
    Struct { name: String },           // Struct name; field info stored in module
    Vec2(Box<IrType>), Vec3(Box<IrType>), Vec4(Box<IrType>),
    Optional(Box<IrType>),
}
```

Pointers are **typed** in the IR (`Ptr(Box<IrType>)`, rendered `ptr<T>`), so backends and `unsafe bitcast` validation can inspect the pointee. When the pointee is unknown or irrelevant, `IrType::raw_ptr()` produces an opaque `Ptr(Void)` rendered as `ptr`. This keeps the language's raw-pointer surface while retaining IR-level type information for validation.

#### Values (SSA)

Every intermediate result is an SSA value, identified by a function-local
`ValueId`. Type and unit metadata are recorded in the function's value table:

```rust
pub struct ValueInfo {
    pub ty: IrType,
    pub unit: Option<NormalizedUnit>,
    pub name: Option<String>,   // debug/source-variable name
}

pub struct Function {
    // ...
    pub values: HashMap<ValueId, ValueInfo>,
}
```

This is SSA at the register level, not full SSA form: the IR has no φ-nodes (a
φ-node merges the different values a variable can hold where control-flow paths
join). Mutable and cross-branch values live in memory instead, via
`Alloca`/`Load`/`Store`, and are promoted back into φ-nodes by a backend's
`mem2reg` pass when lowering.

#### Module Structure

```rust
pub struct Module {
    pub name: String,
    pub structs: Vec<StructDef>,
    pub extern_funcs: Vec<ExternFunc>,
    pub globals: Vec<Global>,
    pub functions: Vec<Function>,
}

pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, IrType)>,
    pub field_units: Vec<Option<String>>,
}

pub struct Global {
    pub id: GlobalId,
    pub name: String,
    pub ty: IrType,
    pub unit: Option<NormalizedUnit>,
    pub initializer: Option<Constant>,
    pub linkage: Linkage,         // Internal | Export | Import
}

pub struct Function {
    pub id: FuncId,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: IrType,
    pub return_unit: Option<NormalizedUnit>,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
    pub linkage: Linkage,
    pub values: HashMap<ValueId, ValueInfo>,
}

pub struct BasicBlock {
    pub id: BlockId,
    pub name: String,
    pub instructions: Vec<Instruction>,
}
```

##### Global Linkage and Module Boundaries

#### Instructions

Instructions are grouped into categories. Every instruction produces zero or one
`ValueId`. There are **no implicit conversions**; the IR generator must emit
explicit conversion instructions where needed.

##### Arithmetic, vectors, and bitwise

- Arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Pow`
- Checked integer arithmetic: `CheckedAdd`, `CheckedSub`, `CheckedMul`, `CheckedNeg`
- Pointer difference: `PtrDiff`
- Vector ops: `Cross`, `Dot`, `BuildVec2`, `BuildVec3`, `BuildVec4`, `ExtractVecElement`
- Bitwise: `BitAnd`, `BitOr`, `BitXor`, `BitNot`, `Shl`, `Shr`, `UShr`

##### Comparison & Logical

`Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`  
`And`, `Or`, `Xor`, `Not`

##### Memory

- `Alloca(type)` – allocate stack space, returns `Ptr`. The IR generator hoists
  allocas to the entry block so loop-body allocations do not grow the stack
  repeatedly; the initializer `Store` remains at the source location.
- `Load(dst, ptr, type)` – load value from pointer.
- `Store(value, ptr, type)` – store value to pointer.
- `GetElementPtr` – computed pointer into an array/aggregate.
- `GetFieldPtr` – pointer to a field by precomputed byte offset.
- `StructFieldPtr` – pointer to a named field, resolved from the struct definition.
- `PtrToInt`, `IntToPtr`
- `BoundsCheck(index, length)` – trap via the runtime-error path (stderr + abort) on violation.
- `ZeroCheck(operand)` – trap via the runtime-error path on zero (div/mod guard).

Constants: `ConstInt`, `ConstUint`, `ConstFloat`, `ConstBool`, `ConstString`,
`ConstNull`.

##### Control Flow (block-based CFG)

A function is a list of basic blocks. Every block ends in exactly one terminator:

```rust
pub enum Instruction {
    // terminators
    Br { target: BlockId },
    CondBr { cond: ValueId, then_block: BlockId, else_block: BlockId },
    Ret { val: ValueId },
    RetVoid,
    // ...
}
```

- There are **no structured `Loop`/`Break`/`Continue`/`IfElse` nodes** and no
  `Phi`/`Select` nodes.
- Loops and conditionals are lowered to blocks + `CondBr`/`Br` by the IR
  generator.
- Values that must remain live across a merge are kept in allocas and loaded
  after the merge; the IR does not carry `phi` nodes.

##### Handling Values That Cross Branches

Because the IR has no `phi` or `Select`, branch-local results that must be used
after a join are written to an alloca before the branch and loaded after the
join. This is a lowering detail owned by the IR generator; the interpreter and
backends simply execute the resulting `Store`/`Load` instructions.

##### Calls

- `Call(dst, func, args)` – returns `ValueId`.
- `CallVoid(func, args)` – no return value.
- Both carry optional source locations (`at "file" line col` in serialized form)
  for stack traces.

##### Type Conversions

All explicit Lale conversions are encoded:
`Trunc`, `SExt`, `ZExt`, `FpToSi`, `FpToUi`, `SiToFp`, `UiToFp`, `FpTrunc`,
`FpExt`, `Bitcast`, `PtrToInt`, `IntToPtr`.

##### Strings, structs, optionals, and the error stack

- `Concat(lhs, rhs)` — string concatenation.
- `TextCopy(src)` — deep-copy a `text`'s data without freeing the source.
- `DeepCopy(src)` — recursively deep-copy an aggregate value (`text`, struct, enum, optional, vector).
- `BuildStruct`, `ExtractField`, `InsertField`, `StructFieldPtr`, `GetFieldPtr`.
- `GlobalAddr(global)` — address of a global variable.
- `Some`, `None`, `UnwrapOptional`.
- `PushError`, `PopError`, `ErrorCount`, `DrainErrors`.
- `TestBegin(suite, case)`, `TestFail(file, line, column, expected, found)`, `TestEnd` — test-suite
  reporting: `TestBegin` sets the current suite/case context, `TestFail` records a
  failure, `TestEnd` prints `Pass: suite / case` unless a failure was recorded. A
  failure forces a non-zero exit code. `TestFail`'s optional `expected`/`found`
  operands (present for `assert a == b`) are formatted into the message as
  `expected <b>, found <a>`; they serialize as `%N`, or `-` when absent.

##### Unit & Safety

- `AssertUnit(value, expected_unit)` – runtime unit check.
- `BoundsCheck(index, length)` – bounds check; writes a message to stderr and aborts on failure.
- `ZeroCheck(operand)` – zero-division guard; writes a message to stderr and aborts on failure.

#### Stable Serialisation Format

The canonical serialized form is **line-oriented** (LLVM-like) and matches
`src/ir/printer.rs`. It is self-contained: it includes struct definitions,
external declarations, globals, and function bodies. Comments start with `;` and
run to end of line; they are ignored by parsers.

Example:

```text
; ir-version: 1.0
; Module: example

%text = type { ptr: Ptr, bytes: I64, chars: I64 }

declare @puts(Ptr) -> I32

@answer: I64 = 42

func @main(%0 n: I64) -> I64 {
entry:
    %1 = alloca I64
    store %0, %1, I64
    %2 = load I64 %1
    %3 = const I64 0
    %4 = gt %2, %3
    condbr %4, %then, %else
then:
    %5 = const I64 1
    br %merge
else:
    %6 = const I64 2
    br %merge
merge:
    %7 = add %5, %6
    ret %7
}
```

Grammar (informal):

```text
module        := version_line module_header struct_section? extern_section? global_section? function*
version_line  := "; ir-version: " major "." minor NEWLINE
module_header := "; Module: " name NEWLINE
struct_section:= ("; Struct definitions" NEWLINE)? struct_def*
struct_def    := "%" name " = type { " field (", " field)* " }"
field         := name ": " ir_type
extern_section:= ("; External functions" NEWLINE)? extern_def*
extern_def    := "declare @" name "(" params ")" "->" ir_type
global_section:= ("; Global variables" NEWLINE)? global_def*
global_def    := linkage? "@" name ":" ir_type (" = " constant)? (" ; unit: <" unit ">")?
function      := linkage? "func @" name "(" params ")" "->" ir_type unit? "{" block* "}"
block         := name ":" instruction*
instruction   := value-producing-instruction | terminator | effect-instruction
```

Key points:

- The first line is `; ir-version: <major>.<minor>`. A parser must reject an
  unsupported **major** version; a **minor** version bump is a backward-compatible
  addition.
- `ir_type` is the `IrType` `Display` form, e.g. `i64`, `ptr`, `ptr<i32>`,
  `[10 x i32]`, `%Point`, `f64?`, `vec3<f64>`.
- SSA values are written `%N`; block references are written `%block-name`.
- Strings are quoted and use Rust `escape_default`.
- Trap instructions (`checked_add`/`checked_sub`/`checked_mul`/`checked_neg`,
  `boundscheck`, `zerocheck`, `unwrapoptional`) carry a bracketed render spec,
  e.g. `["ERROR at … (", %i, ", ", %l, ")"]`. A quoted string is a literal; a
  `%N` token is an SSA operand rendered as a decimal integer at trap time. The
  spec is color-free; presentation (color, newline) is applied at the output
  boundary, never in the IR.
- Units appear inside `<...>` and contain no whitespace (`⋅`, `/`, superscripts).
- Every instruction emits enough operands for lossless reconstruction, including
  `Store`'s type, `ExtractVecElement`'s inner type, and call source locations.
- A binary counterpart can be added later for performance.

#### Validation Rules

The IR must satisfy these invariants:

1. **SSA** – each `ValueId` is defined exactly once within its function.
2. **Type consistency** – operand types match instruction requirements.
3. **Well-formed CFG** – every basic block ends in exactly one terminator
   (`Br`, `CondBr`, `Ret`, or `RetVoid`), and every `Br`/`CondBr` target exists.
4. **No dangling references** – every referenced `ValueId` and `BlockId` exists.
5. **Function signatures** – call arguments match parameter counts and types.
6. **Unit assertions** are placed before any operation that requires a specific unit.

These properties are enforced by the IR verifier (`src/ir/verifier.rs`), exercised on every compile and in round-trip tests.

#### Interpretation Strategy

The interpreter maintains:

- A `HashMap<ValueId, Value>` for SSA values.
- The current basic block and a control-flow stack; `Br`/`CondBr` select the next
  block and `Ret`/`RetVoid` return from the function.
- A `MemoryManager` for heap and stack memory, plus the error stack and runtime
  hook state.

Execution walks the block CFG. The interpreter is the reference implementation of
Lale’s semantics; all other backends must produce identical observable behaviour.

#### Backend Lowering Overview

- **To LLVM**: map basic blocks directly; `CondBr` becomes a conditional branch;
  `Alloca`/`Load`/`Store` map naturally. Allocas used for cross-branch values
  are promoted back into SSA registers and φ-nodes by LLVM’s `mem2reg` pass
  during optimization.
- **To libgccjit**: map blocks directly; `CondBr` becomes a conditional block
  end; calls map to libgccjit calls.
- **To future backends** (e.g., Cranelift, C source emission): the block CFG is
  consumed directly; no relooper is needed.

Because the IR is self-contained and stable, new backends can be developed
entirely in parallel with the frontend, using the serialized format as the
interface.

#### 4.7.10 IR Semantics and Backend Independence

To guarantee that backend independence is real and not just syntactic, the IR defines its own deterministic semantics. No backend is allowed to introduce behaviour that contradicts the interpreter. Specifically:

- **No LLVM poison values** (a _poison value_ is a special value LLVM uses to represent an operation whose result is undefined). Every value is a concrete result of a defined operation.
- **No LLVM undefined behaviour (UB).** _Undefined behaviour_ means the language does not define what the program should do — anything could happen. Integer arithmetic traps on overflow by default: the IR emits `CheckedAdd`/`CheckedSub`/`CheckedMul`/`CheckedNeg` instructions that abort (write a message to stderr and abort the process) when a result does not fit its declared width. Under `--unchecked-overflow` the compiler emits the wrapping `Add`/`Sub`/`Mul`/`Neg` variants instead, whose semantics are two’s complement wrapping — the standard way signed integers are represented in binary — (like Rust’s `wrapping_add`, `wrapping_mul`, etc.). The choice is made at IR generation time, so every backend consumes identical IR in both modes. Shift by an amount greater than or equal to the bit width produces zero (or traps — we choose zero for consistency with the interpreter). Division by zero and remainder by zero always trap via the runtime-error path (stderr + abort). Floating‑point operations follow IEEE 754 (the standard that defines how floating-point numbers behave); the interpreter uses Rust’s `f32`/`f64` semantics, which are deterministic. A floating‑point result of `NaN` or `±Inf` traps (aborts) rather than propagating: `NaN` is IEEE 754’s silent‑error vector — every comparison with it returns false and it silently corrupts results — so Lale treats it as an error, mirroring the integer‑overflow trap. `±Inf` (overflow to infinity) traps for the same reason: it is a silent loss of precision.
- **No LLVM memory attributes.** `Alloca`, `Load`, and `Store` have simple, classic semantics. There are no `noalias`, `readonly`, or `dereferenceable` annotations. The memory model (the rules for how reads and writes to memory behave) is defined solely by the interpreter's behaviour: loads and stores operate on real host heap memory (via `libc::malloc`) using the Lale _application binary interface (ABI)_ layout — the low-level convention for how data is laid out in memory and how code exchanges values — described in §4.8.
- **`StructFieldPtr` instruction.** Instead of relying on LLVM’s `getelementptr` with indices, the IR provides `StructFieldPtr(struct_ptr, field_name)`. The byte offset is computed from the struct’s definition stored in the module. This completely hides backend‑specific struct layout. Backends can lower this using their own layout calculation, ensuring consistent behaviour regardless of ABI details.
- **Future optimisation passes** must be proven correct against the IR’s own semantics, not LLVM’s. Any transformation that would be valid under LLVM’s UB rules but invalid under our deterministic rules must be rejected.

These rules ensure that the interpreter is the single, golden reference. Any backend that matches its output for all inputs is correct. There is no “it’s UB so anything can happen” escape hatch.

### 4.8 Interpreter Backend

The interpreter is the **primary and currently the only actively developed backend**. It consumes the in-memory IR directly and executes it in pure Rust; serialisation/deserialisation happens only under `--roundtrip` (a verification pass). The interpreter and all future AOT backends share the same memory layout, ensuring bit‑identical behaviour across backends.

#### Design: Split Representation

The interpreter uses a **split representation** to balance performance and fidelity:

- **SSA registers** (`HashMap<ValueId, Value>`): tagged `Value` enums for zero‑cost arithmetic, comparisons, and logic. No serialisation overhead for the 95%+ of instructions that don't touch memory.
- **The heap**: raw `[u8]` bytes allocated via `libc::malloc`, matching the AOT memory layout bit‑for‑bit. A `Value::Pointer(usize)` holds a real host address.
- **The bridge**: `MemoryManager` serves as a typed‑value cache with regions at every real address. `Store` and `Load` write and read `Value` enums to lazily‑created regions. String data is stored to both real heap (raw bytes for zero‑copy FFI) and MemoryManager (`Value::Uint` per byte for internal readers). No dual‑mode reads, no `unsafe` fallbacks. The raw‑byte serialisation helpers (`v3_store`/`v3_load` via `to_le_bytes`/`from_le_bytes`) are written but deferred — blocked by the need for a metadata system that distinguishes raw‑byte addresses from `Value`‑enum addresses.

#### Memory Layout: The Lale ABI

The authoritative layout is `doc/ABI_SPECIFICATION.md`. Lale uses a canonical,
private **AAPCS64-based internal ABI**: little-endian, 8-byte pointers, 16-byte
stack alignment, and AAPCS64 struct/optional packing. The FFI boundary uses the
host platform C ABI.

| Type                  | Layout                                    |
| --------------------- | ----------------------------------------- |
| `bool`                | 1 byte (0 = false, 1 = true)              |
| `i8` / `u8`           | 1 byte                                    |
| `i16` / `u16` / `f16` | 2 bytes, 2-byte aligned                   |
| `i32` / `u32` / `f32` | 4 bytes, 4-byte aligned                   |
| `i64` / `u64` / `f64` | 8 bytes, 8-byte aligned                   |
| `char`                | 4 bytes, 4-byte aligned                   |
| `pointer`             | 8 bytes, 8-byte aligned                   |
| `f128` (future)       | 16 bytes, 16-byte aligned                 |
| `T?` (optional)       | `{ is_present: bool, value: T }`          |
| Struct                | AAPCS64 field layout                      |
| Array `T[n]`          | `n * sizeof(T)` contiguous bytes          |
| `vec2`/`vec3`/`vec4`  | `2`/`3`/`4` × `sizeof(T)`, aligned to `T` |

#### Allocation and Leak Detection

A thin `AllocLog` wrapper around `libc::malloc`/`free`:

- `allocate N` → calls `malloc(N)`, logs `(address, size, file, line)`
- `release ptr` → calls `free(ptr)`, removes from log
- Program exit: remaining entries in `AllocLog` are reported as leaks with source location

#### Stack Frame Reclamation

The compiler manages every interpreter heap allocation for a stack slot as a
**frame-reclamation** model, mirroring how an AOT backend reclaims a stack frame
on return:

- **Alloca hoisting.** `IrBuilder::alloca` / `alloca_named` emit every `Alloca`
  into the function's entry block (`emit_to_entry`), regardless of where the
  variable is declared. A loop-body `var` is therefore allocated exactly once
  per call, never once per iteration.

- **Shared epilogue.** Each function gets a single epilogue block. Every
  `return` stores its value into a dedicated return slot and branches to the
  epilogue; an implicit fall-through at the end of the body does the same.

- **Frame reclamation.** The epilogue runs the collected `on exit` statements,
  then `emit_auto_free` frees the whole frame in one place. It walks the entry
  block (`collect_function_allocas`) to enumerate every alloca — named and
  temporary — and frees each slot, skipping only allocas whose ownership was
  transferred (consumed `text` returns, the return slot itself).

This replaces the earlier per-scope / per-return auto-free, which freed only the
allocas emitted so far and therefore leaked variables declared after an early
`return`. The interpreter and future AOT backends now share one mental model:
"one frame, allocated on entry, reclaimed on exit."

#### Bounds Checking

**Debug mode**: every `Load`/`Store` validates the address against the `AllocLog`. Out‑of‑bounds → the runtime-error path (write to stderr and abort) with source location. Array indexing validates `1 ≤ index ≤ length`.

**Release mode**: raw dereference, no checks. Out‑of‑bounds → OS segfault. Matches AOT behaviour exactly. Consistent with existing Lale patterns (`missing code`, div‑by‑zero warnings).

#### FFI

The FFI boundary uses the **host platform C ABI** (not the internal AAPCS64 ABI).
The `call_extern` dispatcher keeps thin match arms for typed value extraction and
marshal Lale-only types (`text`, `T?`) to C-compatible forms at the boundary.

#### Key Instructions

| Instruction                                   | Behaviour                                                                                                                             |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `Alloca`                                      | `AllocLog` allocation (real heap via `libc::malloc`)                                                                                  |
| `Store(ptr, Value::I32(n))`                   | Write `Value` enum to `MemoryManager` region at `ptr` (also writes raw bytes via `v3_store` for primitives when the bridge is active) |
| `Load(ptr, I32)`                              | Read `Value` enum from `MemoryManager` region at `ptr`                                                                                |
| `GetFieldPtr(base, struct_name, byte_offset)` | `ptr + byte_offset` from canonical layout                                                                                             |
| `GetElementPtr(arr_ptr, indices…)`            | `ptr + (index − 1) * sizeof(element)` (1‑based)                                                                                       |
| `BuildStruct`                                 | Packs SSA values into `Value::Struct`; serialisation to bytes happens in `Store`                                                      |

#### Test Execution

`#mode` is a compile-time constant (`"run"` in `lale run`, `"test"` in `lale test`).
Top-level statements are wrapped in mode-guard blocks (`begin_mode_guard` /
`end_mode_guard` in IR generation): run-mode statements execute only when
`#mode == "run"`, test suites only when `#mode == "test"`. Both branches remain in
the IR — the interpreter evaluates the guard at runtime, and a future AOT backend
dead-code-eliminates the untaken branch.

Each test case body is bracketed by `TestBegin` / `TestEnd`. A failing `assert`
inside a case emits `TestFail` and execution continues with the next case. Test-case
variables are stack locals of the synthetic `main` frame; each case gets its own
IR-generation scope (so same-named variables in different cases do not collide),
and each case's string data is freed on case exit.

Suite-level `var`/`fn` are emitted **before** the cases, inside the same
`#mode == "test"` guard. Suite variables become mangled IR globals (`suite::name`)
so they are shared across cases and reachable from suite functions without
colliding with module globals; in the IR-generation scope map they are registered
under their simple name in a pushed suite scope. Suite functions become mangled IR
functions (`suite::fn`). Function calls from cases/suite functions resolve the
suite-qualified name first, matching the analyzer's registration. Suite-level
`text` globals are tracked separately in `IrGenerator::suite_globals` so
`emit_global_str_auto_free` frees their string data at program exit (the suite
scope map is popped before the epilogue).

`lale test --filter <pattern>` restricts which suites/cases are emitted at
IR-generation time (`IrGenerator::test_filter`). The flag is repeatable
(`--filter A --filter B`) and a case runs if ANY pattern matches. A pattern
containing `/` (the same separator as the `Pass: suite / case` output, e.g.
`Math/Subtraction`) matches a substring of the full `suite/case` path; any
other pattern matches a substring of the suite name or the case name. Semantic
analysis still validates every suite and case — only code generation skips
non-matching cases — so a filtered run cannot hide compile errors.

In test mode the interpreter forces a non-zero exit when the run leaks heap
memory (`AllocLog::report_leaks` returns the leak count; `execute_module` exits 1
if any test ran and leaked). `lale run` prints the same `HEAP LEAK` report but
leaves the exit code untouched, which makes `lale test` usable as a leak detector
for standard-library development.

#### Write System and String Handling

```text
Lale has a built-in diagnostic model. It treats program results, diagnostics,
and developer introspection as different things. Program results go to stdout.
Diagnostics go to stderr. Developer inspection is debug-only. Errors can be
accumulated and surfaced explicitly. Logging verbosity is controlled by the
user, not hard-coded by the program.
```

The `write` and `write_inline` statements use automatic type‑to‑string conversion. The `text` type is a struct `{ ptr: pointer, bytes: u64, chars: u64 }` (fat pointer, UTF‑8). The integer, boolean, and character conversions are **generated as IR functions** in every module (from `builtins.lale`) — no external C code required. The float conversion is a single interpreter extern (`__lale_f64_to_str_f64`), because shortest round-trip formatting needs 128-bit arithmetic that Lale cannot express:

| Function                   | Input                       | Output                  |
| -------------------------- | --------------------------- | ----------------------- |
| `@__lale_i64_to_str(i64)`  | Signed integer              | `{ ptr, bytes, chars }` |
| `@__lale_u64_to_str(u64)`  | Unsigned integer            | `{ ptr, bytes, chars }` |
| `@__lale_f64_to_str(f64)`  | Float (shortest round-trip) | `{ ptr, bytes, chars }` |
| `@__lale_bool_to_str(i1)`  | Boolean                     | `{ ptr, bytes, chars }` |
| `@__lale_char_to_str(i32)` | Unicode codepoint           | `{ ptr, bytes, chars }` |

Float formatting uses shortest round-trip (the Ryu algorithm, via the `ryu`
crate, in `src/interpreter.rs` `call_extern`):
plain decimal for magnitudes from `1e-4` (inclusive) to `1e16` (exclusive),
scientific notation outside that range, and trailing zeros stripped. An AOT
backend must produce the identical string — by linking the `ryu` crate or an
equivalent — see the `TODO(AOT)` note on that handler.

##### Statement Code Generation

- `write_inline x`: converts `x` to string if needed, then calls `write(1, ptr, byte_len)` (single syscall).
- `write x`: same, but appends `\n` and calls `write(1, ptr, byte_len + 1)`.

Only `write` has an inline form. `warn`, `alert`, `log`, and `debug` are leveled diagnostics and always emit a complete line (level prefix + timestamp + message); they have no inline variant.

##### Compile‑Time Optimisations

| Expression       | Optimisation                                   |
| ---------------- | ---------------------------------------------- |
| `write "Hello"`  | Length = 6 (5 + newline) embedded as constant  |
| `write 'A'`      | Length known (1–4 bytes UTF‑8 + newline)       |
| `write true`     | Length = 5 ("true\n") constant                 |
| `write false`    | Length = 6 ("false\n") constant                |
| `write int_var`  | Length computed by conversion function         |
| `write text_var` | Length from `text_var.bytes` (O(1), no strlen) |

#### Runtime Hooks and FFI Boundary

The authoritative extern set is `doc/ABI_SPECIFICATION.md`. The interpreter
dispatches through a single `call_extern`; an AOT backend links against libc or
the user-provided implementations.

The boundary is a **single low-level set** — unbuffered I/O, allocation, libm
math, and exit. The buffered C stdio family (`fopen`/`fread`/`fwrite`/`fclose`/
`fseek`) is **not** part of the boundary; buffered/stream abstractions are built
in Lale on top of `open`/`read`/`write`/`close`.

Stable C-runtime externs:

- `write`, `read`, `open`, `close`, `lseek`, `malloc`, `free`, `pow`, `puts`,
  `strtod`, `strtol`, `strtoul`

Stable `__lale_*` hooks:

- `__lale_exit(i32) -> void`
- `__lale_malloc_u64(u64) -> ptr`
- `__lale_free_pointer(ptr) -> void`
- `__lale_read_line() -> text`
- `__lale_timestamp() -> text` — runtime UTC timestamp for `log`/`warn`/`alert`/`debug` lines
- `__lale_log_level() -> i32` — runtime log-level threshold (`0`=off, `1`=alert, `2`=warn, `3`=log)

`__lale_error` is **not** an extern; it is the internal runtime-error path used
for bounds violations, division by zero, absent-optional unwrap, and
checked-overflow traps.

#### Error Function: Single User‑Replaceable Hook

All runtime errors (array bounds violations, division by zero, unwrap of absent optional, checked-overflow traps) emit through the single `write` extern used for all output — there is no dedicated `__lale_error` function:

```lale
import fn signature write(fd as i32, buf as pointer, count as i64) returns i64
```

| Mode                 | Implementation                                                                                              |
| -------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Default (stdlib)** | The interpreter formats the message and writes it to stderr via `write(2, …)`.                              |
| **No-std mode**      | The user overrides `write` (e.g., halt CPU, reset system, log to flash), so the error path is redirectable. |

The message is pre-formatted before the call. For example, an array bounds violation produces:

```text
ERROR at main.lale:42:15: array index out of bounds (index=15, length=10)
```

The formatting (source location, error type, diagnostic values) is done at the call site — the hook receives a complete, ready-to-output string. The caller is responsible for terminating the program (typically via `__lale_exit`).

#### File I/O Architecture

File I/O is implemented in Lale stdlib modules (`file_io_posix.lale`, `file_io_windows.lale`) rather than in the interpreter. File handles are represented as `i64` opaque identifiers because they are never dereferenced, they fit all platform representations (32‑bit fds and 64‑bit pointers), and they signal an opaque handle clearly.

The FFI boundary exposes a **single low-level unbuffered I/O set** — `open`,
`read`, `write`, `close` (and `lseek` where the platform provides it). There is
**no** separate buffered C stdio boundary (`fopen`/`fread`/`fwrite`/`fclose`/
`fseek`); buffered/stream abstractions are built **in Lale** on top of the
unbuffered boundary.

##### Platform-Specific Implementation

- **Unix/Linux/macOS** (`file_io_posix.lale`): Uses the unbuffered POSIX syscalls (`open`, `read`, `write`, `close`, `lseek`).
- **Windows** (`file_io_windows.lale`): Uses the unbuffered C-runtime low-level I/O (`_open`, `_read`, `_write`, `_close`, `_lseeki64`) mapped to the same boundary.

Both export an identical public API: `openFile`, `readFile`, `writeFile`, `closeFile`, `seekFile`. Buffering is shared Lale code above the boundary.

#### Semantic Conversions: Numeric‑to‑Character

The type system distinguishes between narrowing conversions (forbidden) and semantic conversions (allowed). Converting a numeric code to a `char` (e.g., `48 as u64 → '0'`) is a semantic category change and does not lose information. This enables stdlib digit‑to‑character conversion without language extensions. The interpreter implements this directly.

### 4.9 Future AOT Backends

Once the interpreter is mature and the IR is stable, AOT backends will be added. They all consume the exact same serialised IR.

| Backend             | API                        | Strengths                                                          | Effort to Integrate |
| ------------------- | -------------------------- | ------------------------------------------------------------------ | ------------------- |
| **GCC (libgccjit)** | Stable C API, high‑level   | 45+ architectures, mature optimisations, self‑hosting‑friendly     | Moderate            |
| **LLVM**            | C API, low‑level           | Industry‑leading peak performance, broad platform support          | Higher              |
| **Cranelift**       | Rust API, fast compilation | Memory‑safe, ~100× smaller than LLVM, no undefined behaviour in IR | Moderate            |

The interpreter serves as the golden reference for testing: any program must produce identical output under the interpreter and any AOT backend. Because the IR is stable and self‑contained, each backend can be developed independently, even in a different language.

### 4.10 Validation Tool

The `lale-validate` binary checks completeness across all three compiler stages by scanning source code for handler functions and cross-referencing them against the constructs they handle. It also detects orphaned handlers (code for constructs that no longer exist).

```bash
cargo run --bin lale-validate                 # All three stages
cargo run --bin lale-validate -- --stage ast  # Grammar → AST only
cargo run --bin lale-validate -- --stage ir   # AST → IR only
cargo run --bin lale-validate -- --stage interpreter  # IR → Execution only
cargo run --bin lale-validate -- --output json          # JSON output
cargo run --bin lale-validate -- --output markdown      # Markdown table
cargo run --bin lale-validate -- --missing-only         # Show only gaps
cargo run --bin lale-validate -- --show-orphans         # Show orphan warnings
```

#### Validation Stages

| Stage          | Source                   | Checks                                                               | How it works                                                                                                       |
| -------------- | ------------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Grammar → AST  | `src/grammar/lale.pest`  | Every grammar rule has a handler in `builder.rs` or `expr_parser.rs` | Scans `Rule::Name` references, marks internal sub-rules (prefixed `_` or only referenced from silent/atomic rules) |
| AST → IR       | `src/ast/definitions.rs` | Every `Stmt`/`Expr` variant has a `generate_*` method in `ir_gen.rs` | Scans `Stmt::Name` and `Expr::Name` references, maps variant names to struct names                                 |
| IR → Execution | `src/ir/instructions.rs` | Every `Instruction` variant has a match arm in `interpreter.rs`      | Scans `Instruction::Name` references                                                                               |

##### Category Detection

Instruction categories (Arithmetic, Memory, Optional, etc.) are parsed from `// ========== Name ==========` section comments in `instructions.rs` — no separate hardcoded map to maintain. New categories appear automatically when section comments are added.

##### Coverage Report

```text
Grammar → AST Coverage: 100.0%
  AST → IR Coverage: 100.0%
  IR → Execution Coverage: 100.0%
```

Orphaned handlers (code for removed constructs) are flagged as `⚠ ORPHAN` with a note identifying the stale code.

##### Source

`src/bin/validate.rs`

---

## 5. Language Design Decisions

This chapter consolidates the detailed rationales for every language design choice in Lale. A summary of these principles appears in [Chapter 1 – Overview and Design Principles](#1-overview-and-design-principles).

### 5.1 UTF‑8 Everywhere

#### Decision

UTF‑8 is a first‑class citizen throughout the entire compiler pipeline.

#### Rationale

- **Parser:** PEG grammar handles comprehensive Unicode character ranges including Latin Extended, Greek & Coptic, Cyrillic, CJK (20,000+ ideographs plus Hiragana, Katakana, Hangul), Indic scripts (Devanagari, Bengali, Tamil, etc.), Middle Eastern (Arabic, Hebrew, Syriac, Thaana), Southeast Asian (Thai, Lao, Tibetan, Myanmar), Georgian, Armenian, Roman numerals, and combining marks (Latin combining diacriticals and extended ranges).
- **Identifiers:** Support Unicode letters, subscript digits (₀–₉), and combining diacritical marks. Subscript digits (e.g., `x₁`, `y₂`) enable mathematical notation directly in source code.
- **Code Generation:** UTF‑8 identifiers are preserved through to object files and binaries.
- **Error Messages:** Compiler output uses Unicode for mathematical symbols (Δ, α, β, etc.) when displaying diagnostics.

This comprehensive Unicode support enables scientific computing code to naturally express mathematical concepts across multiple languages and scripts, reducing transcription errors when implementing algorithms from research papers.

#### Maintenance note

The identifier alphabet is a hand-curated `letter_ranges` table (`src/grammar/lale.pest`) with per-script ranges and manual exclusion of digits, number signs, punctuation, and combining marks. It is a correctness surface: a range that accidentally admits a non-letter becomes a latent parser bug, so any change to the ranges should be reviewed against the Unicode block boundaries for each script.

### 5.2 Inferred Properties: No `const` or `mutability` Keywords (SSA-Based)

#### Decision

No explicit `const` keyword for constant declarations, and no `mutability` keyword or mutability annotations.

#### Rationale

- **SSA encodes immutability:** Variables assigned once are inherently immutable; reassignments create new SSA values.
- **Dead store elimination:** Backends perform automatic dead store elimination (DSE) and constant propagation without needing programmer annotations.
- **No redundant keywords:** Since SSA already captures whether a value is immutable or mutable, explicit `const` or `mutability` keywords are redundant.

By avoiding these annotations, the language is simpler without sacrificing optimization quality. Programmers focus on correctness and logic; the IR and backends handle the rest.

### 5.3 No Type Aliases

#### Decision

Lale has no mechanism for creating type aliases. The `type` keyword always defines a genuinely new, distinct type — never a simple rename of an existing one. Type aliasing is intentionally excluded from the language design and will not be added in the future.

#### Rationale

- **Renaming** an existing type (`alias Meters = f64`) — which adds confusion by creating multiple interchangeable names for the same thing
- **Creating** a new type (`type Meters … end type`) — which has its own identity, fields, and semantics

Requiring every type to be a new definition ensures types carry meaning beyond mere syntax sugar, promoting clearer code organization and preventing the "which name is the real one?" problem that plagues codebases with unchecked aliasing.

#### Why This Will Never Change

Type aliases offer convenience at the cost of clarity. They create a parallel naming system — "Meters is really f64, but only sometimes" — that forces readers to maintain a mental mapping table. Lale already provides two better alternatives for every use case an alias might serve:

- **If you need a named type with identity:** define it with `type`. The new type has its own fields, its own symbol table entry, and its own semantics — it cannot be silently interchanged with other types.
- **If you only need to annotate a value's physical dimension:** use units. `var height as f64 in <m> = 1.80` attaches meaning without pretending there's a new type.

Both alternatives are more expressive than a bare rename and leave no ambiguity about intent. Adding aliases would introduce a third mechanism that is less powerful than either, while undermining the clarity the first two provide.

### 5.4 Volatile Semantics via Import/Export

#### Decision

No `volatile` keyword. Volatility is implicit in `import`/`export` declarations.

#### Rationale

- Variables marked `export` are visible across compilation boundaries, implying they may be modified outside normal control flow.
- Variables marked `import` come from external sources with no guarantees about timing.
- This design groups related concepts (visibility and volatility) together without additional keywords.

#### Shared by reference (read-write)

An imported variable is the _same storage_ as the exported variable it names —
not a copy. The importing module may read and write it, and writes are visible
to the exporting module (C's `extern int foo`, not `extern const int foo`).

### 5.5 Explicit Type Conversions (Widening Only)

#### Decision

Lale requires explicit type conversions using the `as` operator, but only **widening conversions** are allowed. Narrowing conversions and incompatible type conversions are rejected at compile time.

#### Allowed Conversions

- Widening integers: `i32 as i64`, `u8 as u32`
- Widening floats: `f32 as f64`
- Integer to float: `i32 as f64` (widening in precision)

#### Rejected Conversions

- Narrowing integers: `i64 as i32` ❌
- Narrowing floats: `f64 as f32` ❌
- Float to integer: `f64 as i32` ❌ (precision loss)
- String ↔ Numeric: `text as i32` ❌
- Bool ↔ Numeric: `bool as i32` ❌

#### Units Are Preserved

through type conversions. Converting `i32 in <m>` to `f64` produces `f64 in <m>`.

#### Rationale

1. **No silent precision loss:** Narrowing conversions can lose data silently. By rejecting them, the programmer must reconsider the data types in their design.
2. **Type safety:** Incompatible conversions (string/bool ↔ numeric) represent logical errors, not type mismatches.
3. **Unit preservation:** Physical dimensions are intrinsic to the value, not the type. Converting a distance from `i32` to `f64` doesn't change the fact that it's measured in meters.
4. **Condition Type Safety:** Boolean conditions must be explicitly boolean. Lale rejects non‑boolean expressions in conditions, preventing a class of errors common in C‑like languages where any non‑zero value is "truthy." This rule applies to both runtime `if`/`loop when` and compile‑time `#if` conditions.

```lale
var x as i32 = 42
var y as f64 = x as f64    // OK: widening (i32 → f64)
// var z as i32 = y as i32 // ERROR: narrowing (f64 → i32) not allowed

var distance as i32 in <m> = 100
var d as f64 = distance as f64    // OK: d has type f64 and unit <m>

if x > 0                    // OK: comparison produces boolean
    write "positive"
end if
```

This design philosophy aligns with Lale’s goal of safety and explicitness in scientific computing.

### 5.6 Chained Type Conversions

#### Decision

Conversions chain naturally as a left-to-right pipeline. Each `as` step is validated independently — `x as u32 as f64` is two separate widening checks.

#### Rationale

```lale
var x as i32 = 42
var result as f64 = x as u32 as f64    // i32 → u32 → f64, each step widening only
```

For long chains or debuggability, intermediate variables add clarity:

```lale
var x as i32 = 42
var as_u32 as u32 = x as u32
var result as f64 = as_u32 as f64
```

### 5.7 Pointer Conversions (Unsigned Only)

#### Decision

Only unsigned integer types (`u8`, `u16`, `u32`, `u64`) can be converted to `pointer`. Signed integers are explicitly rejected.

#### Rationale

1. **Bitwise Safety:** Unsigned integers map directly to memory addresses without sign‑extension issues.
2. **Semantic Clarity:** Pointers represent addresses, which are naturally unsigned values.
3. **Type Safety:** Restricting to unsigned prevents accidental negative address construction.

#### Allowed

```lale
var addr as u64 = 0x7FFF0000
var ptr as pointer = addr as pointer    // OK: unsigned
```

#### Rejected

```lale
var x as i32 = 42
var p as pointer = x as pointer         // ERROR: signed integer
```

### 5.8 Numeric Literal Range Validation

#### Decision

Numeric literals are validated at compile time against their target type's range. Values that don't fit are rejected with a clear error message.

#### Rationale

Numeric literals are known at compile‑time, enabling precise range checking. This allows natural patterns like `var x as u8 = 255` without forcing users to annotate every literal.

#### Example

```lale
var a as u8 = 255 as u8    // OK: within range [0..255]
var b as u8 = 256 as u8    // ERROR: 256 > 255
var c as i8 = -128 as i8   // OK: within range [-128..127]
var d as i8 = 128 as i8    // ERROR: 128 > 127
```

#### Variable Narrowing Still Rejected

The compiler cannot verify variable values at compile time, so narrowing is rejected:

#### No Default Numeric Type

Lale has no default integer or float type — unlike C (`int`), Rust (`i32`), or Python (`int`). A bare literal like `42` has no type until context provides one (via `as`, a type annotation, or a function signature). Optional values are declared with the `?` modifier (e.g., `var v as i64? = nothing`) and concrete values are auto-wrapped (e.g., `var v as i64? = 42 as i64`).

```lale
var x as i32 = 100
var y as u8 = x as u8      // ERROR: narrowing conversion (value unknown at compile time)
```

### 5.9 1‑Based Array Indexing

#### Decision

Lale uses 1‑based array indexing. The first element of an array is accessed via `arr[1]`, not `arr[0]`.

#### Rationale

1. **Mathematical Convention:** Mathematics and scientific literature count sequences starting from 1. Lale aligns with how scientists naturally express problems, reducing cognitive translation.
2. **Proven in Mathematical and Scientific Computing:** 1‑based indexing has been the standard in the languages most widely used for numerical work:
   - **Fortran** (since 1957) — the foundational language of scientific computing; arrays default to 1‑based indexing because it maps directly onto the conventions of linear algebra and matrix mathematics.
   - **Julia** — a modern high‑performance language that deliberately chose 1‑based indexing to align with the expectations of scientists and engineers migrating from Fortran, MATLAB, and R.
   - **MATLAB** — the dominant platform for numerical computing in academia and industry; its entire matrix‑oriented design assumes 1‑based indexing.
   - **R** — the standard language for statistical computing; uses 1‑based indexing to match the conventions of mathematical statistics.
   - **Mathematica / Wolfram Language** — the leading symbolic mathematics system; 1‑based throughout.
   - **ALGOL 68, APL, AWK, COBOL, Lua, Pascal, Smalltalk** — a broad spectrum of languages spanning scientific, business, and systems programming that have successfully used 1‑based indexing without sacrificing performance or interoperability.
   - The fact that such a diverse set of languages — from high‑performance compiled languages (Fortran, Julia) to interactive environments (MATLAB, R, Mathematica) — all converged on 1‑based indexing is strong evidence that it is the natural choice for a language targeting scientific and mathematical users.
3. **C Interoperability via Wrapper Layer:** Following Julia's model:
   - Standard Lale code is 1‑based: `arr[1]` is the first element.
   - FFI boundary automatically adjusts indices when calling C functions.
   - Wrapper functions handle index translation transparently.
   - Low‑level code can use `unsafe` blocks to access 0‑based memory directly.
   - Result: Scientific code is intuitive; C library interop is managed by the wrapper layer.
4. **Error Prevention in Science:** Off‑by‑one errors in numerical computation are costly. 1‑based indexing aligns with mathematical formulas, reducing transcription errors when implementing algorithms from papers.

#### Example

```lale
// Natural Lale: 1-based indexing
var measurements as f64[10] = [1.5, 2.3, 3.1, ...]
write measurements[1]  // First measurement (not "zeroth")

// For loops follow natural counting
loop var i as i32 from 1 to 10
    write measurements[i]
end loop
```

#### Impact

Requires FFI wrapper functions for C library bindings, but eliminates off‑by‑one errors in mathematical code and aligns Lale with the scientific computing ecosystem.

### 5.10 Complete Array Initialization (No Partial Init)

#### Decision

Arrays must be completely initialized. Partial initialization is not allowed. Only two safe patterns are supported:

1. **Complete initialization with explicit elements:** All elements must be provided.
2. **Uniform fill with `fill` keyword:** All elements filled with the same value.

#### Rationale

1. **Safety:** Partial initialization (e.g., `i32[10] = [1, 2, 3]`) leaves elements uninitialized with garbage values, creating subtle bugs.
2. **Consistency with Unsafe Declaration:** Lale requires `unsafe` keyword for uninitialized primitives (`unsafe decl x as i32`). Array partial init would violate this safety principle.
3. **Ergonomic Alternative:** The `[fill with value]` syntax provides clean syntax for the common case of uniform initialization without compromising safety.

#### Grammar

```pest
a_literal = {
    "[" ~ os_ln ~ "]" |
    "[" ~ os_ln ~ "fill" ~ ms_ln ~ "with" ~ ms_ln ~ expression ~ os_ln ~ "]" |
    "[" ~ os_ln ~ expression ~ (os_ln ~ "," ~ os_ln ~ expression)* ~ os_ln ~ "]"
}
```

#### Examples

```lale
// ✅ Complete initialization - all 5 elements specified
var arr as i32[5] = [10, 20, 30, 40, 50]

// ✅ Uniform fill - all 1000 elements initialized to 0
var buffer as i32[1000] = [fill with 0]

// ✅ Unsafe declaration - programmer responsible for initialization
unsafe decl temp as i32[256]

// ❌ Partial initialization - ERROR
var bad as i32[10] = [1, 2, 3]

// ❌ Incompatible element type
var bad2 as i32[10] = [fill with "x"]
```

#### Type Checking for Fill Syntax

The validation enforces:

1. Element type must be compatible with array base type.
2. Numeric literal flexibility applies: `u32` literals can initialize `i32[n]`.
3. Array dimensions come from the type annotation, not the literal.

#### Implementation

- Parser detects fill pattern: "fill with" keyword followed by single expression.
- Type cannot be inferred from fill literal alone; must use type annotation.
- Semantic validation in `validate_array_initialization()` enforces type compatibility.

#### Impact

No arbitrary uninitialized array elements; programs either fully initialize or explicitly use `unsafe`.

### 5.11 No Exception Handling

#### Decision

Lale intentionally does not include exception handling (`try`/`catch` or similar mechanisms).

#### Rationale

1. **Explicit Control Flow:** Lale is designed around explicit, traceable control flow. Exceptions create invisible paths through the code—a function call could jump to a handler anywhere in the call stack, making reasoning about program behavior difficult.
2. **Scientific Computing Requirements:** Lale targets safety‑critical scientific and engineering code where deterministic behavior is paramount. Exceptions violate this principle by introducing non‑local control transfers that are hard to predict and reason about.
3. **Alignment with Language Philosophy:** Lale's core design is based on:
   - No hidden coercions (all type conversions explicit)
   - No implicit behavior (all operations visible in code)
   - Two simple scopes for predictable variable lifetimes
   - Explicit type annotations and units for clarity
     Exceptions contradict this: they represent hidden, implicit control flow.
4. **Error Handling Alternatives:** Lale uses explicit error handling mechanisms:
   - **Unrecoverable errors** (array bounds violations, assertion failures): Abort with clear error messages through the runtime-error path, which emits via the `write` hook (customizable for embedded systems).
   - **Recoverable errors**: Optional types (`T?`) with `has value`/`has no value`/`value of` for fallible operations. See §5.17.
   - **Error propagation**: The `?` operator (`expr?`) propagates `nothing` upward — if the expression is absent, the enclosing function returns `nothing`. No hidden control flow; every propagation point is visible in source.
   - **Diagnostics**: The built‑in error stack (`add error`, `last error`, `alert error messages`) collects diagnostic messages without complicating function signatures. See §5.18.
5. **Precedent from Systems Languages:** Rust proved that you don't need exceptions. Explicit error handling (`Result` types and the `?` operator) is:
   - More composable and functional
   - Easier to reason about statically
   - Better for performance (no stack unwinding overhead)
   - Clearer in code review and maintenance
   - Supports embedded systems with custom error handlers

#### Example of Explicit Error Handling

```lale
// Array bounds check: writes a message to stderr and aborts if index is out of bounds
var arr as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
write arr[5]    // OK: in bounds
// write arr[11]  // ERROR: writes a message to stderr and aborts

// Optional types for recoverable errors
fn safe_divide(a as f64, b as f64) returns f64?
    if b == 0.0
        return nothing
    end if
    return a / b
end fn

// The ? operator propagates nothing upward
fn compute(a as f64, b as f64) returns f64?
    var result as f64 = safe_divide(a, b)?   // if absent, returns nothing to caller
    return result * 2.0
end fn
```

#### Implementation

All error paths go through the runtime-error path (see Section 4.8), which emits via the `write` hook — customizable for embedded systems. Runtime errors are fatal, and compiler guarantees prevent the worst cases (bounds violations, type mismatches, unit errors). No unwinding or cleanup required—the OS reclaims resources when the process exits.

### 5.12 Runtime Array Bounds Checking

#### Decision

Bounds checking follows the same model as all memory safety checks: **debug mode only**. Release builds use raw dereference — out‑of‑bounds access produces an OS segfault, matching future AOT behaviour exactly.

#### Bounds Check Logic

Since Lale uses 1‑based indexing, valid indices are in the range `[1, length]` (inclusive). The runtime verifies: `if !(index >= 1 && index <= length) { call write a message to stderr and abort }`.

#### Checked Dimensions

Multi‑dimensional arrays are checked per‑dimension. For `matrix as i32[5][5]`, accessing `matrix[i][j]` checks both `i ∈ [1, 5]` and `j ∈ [1, 5]`.

#### Error Message Format

```text
ERROR at <filename>:<line>:<column>: array index out of bounds (index=<value>, length=<size>)
```

The error includes:

- **Filename:** Source filename (basename only, e.g., `program.lale`).
- **Location:** Line and column (1‑based).
- **Details:** The attempted index value and array size.

#### Implementation

- Array indexing emits a dedicated `BoundsCheck(index, length)` IR instruction (see §4.7); the interpreter writes a message to stderr and aborts when `index >= length`.
- Compile‑time bounds checking for literal indices remains unchanged (static analysis).
- Release builds skip all checks — performance is identical to AOT-produced code.

#### Customization

The `write` hook (see Section 4.8) receives the formatted message and can be overridden for embedded systems (e.g., log to UART instead of stderr).

### 5.13 Unit Inference and No Unit Stripping

#### Decision

When assigning a value with a unit to a variable without a declared unit, the unit is **inferred** from the right‑hand side. There is no syntax to strip units from a value.

```lale
var distance as f64 in <m> = 100
var x as f64 = distance    // x has unit <m> (inferred from RHS)
```

#### Rationale

1. **Dimensional safety:** Units cannot be accidentally discarded.
2. **Explicit intent:** If you need a unitless value, you must start with one.
3. **Consistency:** Follows the "no implicit conversions" philosophy. This is the same inference rule applied to units that types use in variable definitions: when the LHS doesn't declare one, the RHS provides it. The sole asymmetry is that a bare numeric literal is ambiguous for types (no default type to infer) but unambiguously unitless for units.

#### Implementation in `visit_var_def()`

```rust
let symbol_unit = match (&def.unit, &rhs_unit) {
    (Some(u), _) => Some(u.raw.clone()),               // LHS declares unit → use it
    (None, ExprUnit::Unit(u)) => Some(u.to_string()),   // LHS no unit, RHS has unit → INFER
    _ => None,                                         // Both unitless → no unit
};
```

#### Implications for Unsafe Bitcast

Since units cannot be stripped, `unsafe bitcast` can only be used with values that are unitless from their origin. This prevents bit reinterpretation of dimensional values, which would be physically meaningless.

### 5.14 Raw Pointers Only

#### Decision

Lale uses a single `pointer` primitive type for **data** pointers rather than typed
pointers like `pointer to T`.

**Function pointers are a planned exception (roadmap, not yet implemented).** The
roadmap (`doc/roadmap.md` §2) proposes a structural `fn (...)` type — not the raw `pointer`
type — because an indirect `call` needs an _input_ contract (the parameter types) that an
opaque data pointer cannot carry. A function pointer would be a typed _code_ pointer,
distinct from the raw _data_ `pointer`: it would not unify with `pointer` and would not be
dereferenceable via `unsafe value at`. This exception is proposed to apply only to function
pointers; data pointers remain raw.

#### Rationale

1. **All Pointers are the Same Size:** Pointers are fixed at 8 bytes on all platforms (System V AMD64 canonical ABI). The pointed‑to type does not affect storage.
2. **Type is Determined at Dereference, Not Declaration:** `var p as pointer = pointer to bar` declares an opaque pointer; `var x as u32 = unsafe value at p` provides the type at the point of use.
3. **Simpler Type System:** Typed pointers require either structural typing (track `pointer to T`) or lifetime tracking (Rust‑style references). Lale avoids both by treating pointers as opaque addresses. The escape-prevention rules provide lifetime safety without type complexity.
4. **Explicit Unsafe Behavior:** Raw pointers make the dangerous operation (bit reinterpretation) explicit:

   | Operation                | Syntax                           | Effect                                      |
   | ------------------------ | -------------------------------- | ------------------------------------------- |
   | Value conversion (safe)  | `x as u32`                       | Converts value: `42.0` → `42`               |
   | Bit reinterpret (unsafe) | `pointer to` + `unsafe value at` | Reinterprets bits: `42.0f32` → `0x42280000` |

   The multi‑step pointer path creates friction proportional to the danger. Users naturally fall into the safe `as` conversion path.

5. **FFI Compatibility:** Raw pointers map directly to C's `void*`, simplifying foreign function interface calls without type conversion boilerplate.

#### Compile-Time Validation

Although the grammar has only raw pointers, the semantic analyzer tracks the underlying type of each pointer via the `pointer_to_type` field in the symbol table. The `unsafe bitcast` operator makes bit reinterpretation explicit while allowing compile‑time verification of bit‑width compatibility.

#### Per-Function Escape Rule

The escape rule (Section 4.5.5) is enforced on a per‑function, syntactic basis. It catches a literal `pointer to <local>` returned or assigned directly to a global, and a pointer variable that was assigned `pointer to <local>` and is then returned or stored globally within the same function. The compiler does not perform inter‑procedural escape analysis, so a function that passes a pointer to a callee is not itself flagged, even if the callee might later store it globally; storing a pointer into a struct/type that later escapes is likewise not detected.

#### Trade-offs of Raw Pointers vs. Typed Pointers

- ✅ Simpler type system and implementation
- ✅ Explicit unsafe operations (via `unsafe bitcast`)
- ✅ Easy FFI interop
- ✅ Compile‑time type checking on pointer dereference (via `unsafe bitcast`), because the compiler tracks pointed‑to types in the symbol table for validation

#### Heap Management

Lale provides `allocate(size)` and `release ptr` as language keywords for explicit heap management of raw pointers. The `on exit` construct defers a statement (typically `release`) to run at every function exit point — before every `return` and at end-of-function, in registration order. Together, these three primitives give the programmer full control over dynamic memory without a garbage collector. See [§5.14a](#514a-the-text-type) for leak detection.

### 5.14a The `text` Type

Lale's built‑in string type is a **fat pointer**: a struct `{ ptr: pointer, bytes: u64, chars: u64 }` that carries both a data address and a byte count. It is one of two compiler-managed heap types (the other is `binary`) — the user never writes `release` for a string.

#### Definition

The `text` type is defined in `builtins.lale` (the single source of truth):

```lale
type text
    ptr as pointer
    bytes as u64 in <bytes>
    chars as u64 in <chars>
end type
```

There is no Rust-side pre-registration of `text`. In the production pipeline, builtins are loaded first — the Lale definition registers the struct in the IR module — and struct definitions are copied into the target module during module merging.

#### What Makes `text` Special

`text` is one of two types that use heap memory (the other is `binary`). All other types—primitives (`i32`, `f64`), composites (`type Point … end type`), arrays—live entirely on the stack or in registers. `text` and `binary` allocate and free heap memory via `allocate`/`release`, and the compiler handles all of it silently:

| Concern                 | How the compiler handles it                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Memory allocation**   | String embedding (`"x = {x}"`), type‑to‑string conversions (`u64_to_str`), and concatenation (`~`) all allocate heap memory via `allocate` behind the scenes. The user never writes `allocate` for a string.                                                                                                                                                                                                                                                                            |
| **Memory deallocation** | Named variables (`var s as text = ...`) are auto‑freed at scope exit. Inline values (`write "{x}"`) are freed after the I/O completes. Concatenated intermediate strings are freed by the `Concat` handler. A temporary string passed straight to a function argument (`writeFile(fd, "{x}")`) is neither named nor an output statement, so it is not tracked and leaks — bind it to a variable first. See [§5.14b](#514b-text-compiler-managed-memory-a-deliberate-design-compromise). |
| **Leak detection**      | The interpreter's `AllocLog` wrapper tracks every `allocate`/`release` call with source location (file + line). At program exit, debug builds report each unfreed allocation with its byte count and allocation site. A call‑stack resolver (`resolve_alloc_source()`) walks the interpreter's call frames to attribute the leak to the user's code, skipping stdlib/builtins frames. Every test run is a leak test.                                                                    |
| **Null termination**    | Every `text` is null‑terminated: `ptr[bytes]` is always `\0`. The `ptr` field can be passed directly to C functions expecting `const char*`. A bare `char*` from C is not a `text`; use `__lale_construct_text_in_place(ptr, bytes)` to wrap it.                                                                                                                                                                                                                                        |

**Why a fat pointer?** See [§5.14c](#514c-fat-text-ptr-bytes-fat-pointer-with-ffi-nulltermination) for the full rationale: safety (bounds checking via known length), UTF‑8 correctness (null bytes in multi‑byte sequences), O(1) length, and FFI compatibility without sacrificing safety.

**Why compiler‑managed memory?** See [§5.14b](#514b-text-compiler-managed-memory-a-deliberate-design-compromise) for the design rationale: the language creates the allocation, so the language frees it. Requiring `on exit release(msg.ptr)` for every string would leak compiler implementation details into user code. This is a deliberate, documented compromise — simplicity of use wins over strict explicitness for this one foundational type.

**Interaction with `on exit`.** The `on exit` construct defers a statement to run at every function exit point, making it the natural tool for pairing `allocate` with `release`:

```lale
var buf as pointer = allocate(1024 as u64)
on exit release buf
```

The deferred statement runs once in the function epilogue — which every return path branches through — in registration order. For `text`, the compiler handles everything silently — `on exit release` is for raw pointers, not strings. The two mechanisms serve different problems and do not conflict.

### 5.14b `text` Compiler-Managed Memory: A Deliberate Design Compromise

#### Decision

`text` and `binary` are the two types where the compiler manages heap memory automatically. The user never writes `release` for a string — the compiler tracks every `text` value and inserts deallocation at the correct point. This is an intentional, documented compromise: the `text` type is part of the grammar and the language definition, and the compiler takes full responsibility for its correct memory management.

#### Why This Breaks "Explicit Over Implicit"

Lale's core principle is that code should read like the problem domain. A scientist writing `write "x = {x}"` is expressing output, not memory management. Requiring `on exit release(msg.ptr)` for every string would leak compiler implementation details into user code — the opposite of "easy to understand and easy to maintain."

The `text` type is foundational: string embedding, type-to-string conversions, and concatenation all heap-allocate behind the scenes. The user didn't write `malloc`, so they shouldn't write `free`. The compiler created the allocation — the compiler cleans it up.

#### What the Compiler Does

1. **Named local variables** (`var s as text = ...`): the frame-reclamation epilogue frees every stack slot at function exit. For `text`, the epilogue first frees the string data (via the `.ptr` field) before freeing the slot itself. By-value `text` parameters are deep-copied (`TextCopy`) on entry and freed like locals; only `ref` parameters' data and returned/transferred allocas are skipped (they are caller-owned or transfer ownership). See §4.8 Stack Frame Reclamation.

2. **Inline string values** (`write "{x}"`, `warn "..."`, `debug expr`): the IR generator emits `release` after the write completes. `free()` is idempotent, so double‑free (from auto‑free on named variables) is harmless.

3. **String concatenation** (`"x={x}"`): the `Concat` handler frees its operand data after creating the result. To prevent a live variable from being corrupted, the IR generator first emits a `TextCopy` for embedded `text` values and struct/enum `text` fields — `Concat` then frees the fresh copy, not the shared original.

4. **Builtins type converters** (`u64_to_str`, `f64_to_str`, etc.): rewritten to allocate an exact-size buffer and return `text(buf, len)` directly — no double allocation, no intermediate leak. The caller's auto-free or inline-free handles cleanup.

**Not tracked — a temporary string passed to a function argument.** A string built by embedding, concatenation, or a type converter is freed only when it reaches one of the paths above: a named variable (case 1) or an output statement (case 2). A temporary handed straight to an ordinary function argument — for example `writeFile(fd, "{x}")` — is tracked by neither path, and the callee does not take ownership, so it leaks one allocation per call. Bind it to a variable first (`var line as text = "{x}"` then `writeFile(fd, line)`), or emit it with an output statement, to get automatic cleanup.

#### Leak Detection

The interpreter's `AllocLog` tracks every `allocate`/`release` and verifies matching pairs at program exit. Each allocation records its source file and line; a call‑stack resolver walks interpreter frames to attribute leaks to user code (skipping stdlib/builtins frames). Every test run is a leak test.

#### Design Principles Preserved

- **Transparency:** The `text` struct is defined in `builtins.lale` — the user can open the file and see `type text { ptr: pointer, bytes: u64, chars: u64 }`. The memory management is transparent at the language level: strings just work.

- **Single source of truth:** `builtins.lale` is the authoritative definition of `text`. There is no Rust-side pre-registration. In the production pipeline, builtins are loaded first — the Lale definition registers the struct in the IR module.

- **Separation of concerns:** Builtins (`type text`, FFI, conversions) are embedded in the compiler. The standard library (`full.lale`) depends on builtins but not vice versa. User code depends on both. The pipeline loads builtins → stdlib → user code in that order.

#### Interaction with `on exit`

`on exit` is the tool for user-initiated `allocate`/`release` pairs on raw pointers. The deferred statement runs once in the function epilogue, which every return path branches through. The two mechanisms serve different problems and do not conflict.

### 5.14c Fat Text (`ptr` + `bytes`): Fat Pointer with FFI Null‑Termination

#### Decision

The `text` type is a struct `{ ptr: pointer, bytes: u64, chars: u64 }` — a "fat pointer" that carries a data address, a byte count, and a code point count. Lale deliberately avoids C‑style length‑by‑scanning strings.

#### Rationale

1. **Safety.** Null‑terminated strings are the single largest source of buffer overflows in C. Every `strcpy`, `strcat`, and `sprintf` relies on the programmer to compute the correct buffer size. Lale's fat strings enable the compiler to bounds‑check every string operation—the loop `for i in 0..s.bytes` knows exactly where to stop, and array‑like access `s[i]` uses the known length.

2. **Binary safety.** A length‑prefixed string can contain the actual Null character (U+0000) without truncation. C's null‑terminated strings treat `0x00` as the end marker—any content after it is invisible to `strlen` and `printf`. A `text` with an explicit length treats `0x00` as just another byte. This prevents an entire class of bugs and security vulnerabilities where malicious input hides data behind an embedded null. (Note: UTF‑8 multi‑byte sequences never contain `0x00` by design—the high bit of every trailing byte is always `1`. This is about intentionally storing the Null character in a string, not about encodings accidentally producing nulls.)

3. **O(1) length.** `strlen()` traverses the entire string to find the null terminator. Lale's `s.bytes` is a direct field access—cheap enough to use in loop conditions and bounds checks without overhead.

4. **FFI compatibility without sacrificing safety.** The `ptr` field maps directly to C's `char*` for syscalls and C library calls (`write(fd, s.ptr, s.bytes)`). The `bytes` field stays on the Lale side for bounds checking. The two worlds coexist: C gets the raw pointer it expects; Lale keeps the length it needs.

5. **Modern precedent.** Rust (`&str` = pointer + length), Go (string header), Swift, and Zig all use fat strings. C is the outlier—not the standard. A new language targeting safety and correctness should follow the modern consensus.

#### Null-Termination Guarantee

Every Lale `text` is null‑terminated: the byte immediately following the data (at offset `bytes` from `ptr`) is always `\0`. This means `s.ptr` can be passed directly to C functions expecting `const char*` — for example, `write(fd, s.ptr, s.bytes)` or `fopen(s.ptr, "r")`. The `bytes` field remains authoritative for Lale code (never scanned for the terminator).

This is a one‑way bridge: `s.ptr` is C‑compatible, but a bare `char*` from C is not a Lale `text`. Lale always needs the length. The `__lale_construct_text_in_place` function wraps a known pointer+length into a `text` for FFI interop.

#### What It Costs

A `text` is 24 bytes on 64‑bit platforms (8 for `ptr`, 8 for `bytes`, 8 for `chars`) versus 8 bytes for a bare `char*`. Passing strings by value copies 24 bytes. This is a negligible cost for the safety and expressiveness gained, and remains compatible with register‑based calling conventions.

### 5.14d Function Parameter Passing: Copy by Value, `copy`, and `ref`

#### Decision

Function arguments are **copied by value** unless the parameter is declared
`ref`. `copy` is an explicit by-value acknowledgement that silences the
aggregate-copy warning. The previous rule — _everything by reference unless you
write `copy`_ — was replaced for Lale 1.0.0.

The change resolves the tension between string slicing (a **copy**) and
parameter passing (previously a **reference**): both now have value semantics —
a slice is a copy, and a parameter is a copy. `ref` is written once, at the
parameter declaration, never at the call site.

#### The Three Modes

| Modifier | Meaning                                                        | Compiler warning         |
| -------- | -------------------------------------------------------------- | ------------------------ |
| _(none)_ | copy (by value)                                                | only for aggregate types |
| `ref`    | mutable reference — the callee may change the caller's value   | none                     |
| `copy`   | explicit copy — acknowledges the copy and silences the warning | none                     |

```lale
fn f(x as i32)              // copy (primitive) — no warning
fn g(data as Point)         // copy (struct)  — warns
fn g(copy data as Point)    // copy (struct)  — no warning
fn h(ref data as Point)     // reference      — no warning
```

`copy` still means "copy": its role changed from _opt into copying_ (the old
by-reference default) to _explicitly acknowledge the copy_. Code that wrote
`copy` keeps its meaning.

#### What "copy" means, by type

| Type                                          | Copy semantics                                                                                   |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `i8…i64`, `u8…u64`, `f16…f64`, `bool`, `char` | bitwise copy of the value (O(1))                                                                 |
| `pointer`                                     | shallow copy of the 8-byte address — see the pointer exception below                             |
| `text`                                        | deep copy — allocate a fresh buffer, copy the UTF-8 bytes, new `{ptr, bytes, chars}`; auto-freed |
| array                                         | deep copy — allocate and copy every element (recursively for nested arrays)                      |
| struct                                        | deep copy — recursively copy every field                                                         |
| `vec2`/`vec3`/`vec4`                          | copy every element — bitwise for primitive elements (O(1)); deep for `text`/struct elements      |
| enum                                          | copy the discriminant and payload (deep-copy aggregate payload)                                  |
| `T?` (optional)                               | copy the inner `T` (deep if aggregate); `nothing` copies to `nothing`                            |

#### The `pointer` exception

A `pointer` is copied **shallowly** — only the 8-byte address — and the reason is
deeper than cost: **the compiler does not know what a pointer points at.**

A raw `pointer` is opaque. It might point at an `i32`, a whole struct, or a `text`.
At an FFI boundary Lale has no type information at all. Because the compiler
cannot know the size, shape, or ownership of the pointed-to data, it _cannot_
deep-copy it. The only honest thing to do is copy the address itself and leave the
data untouched.

`text` is the opposite case, and the contrast is the point. A `text` is a
fully-known, compiler-managed value (see §5.14b): Lale knows its representation
(`{ptr, bytes, chars}`), owns its heap buffer, and frees it at scope exit. Precisely
_because_ the compiler knows everything about a `text`, it can deep-copy it — and
manage the copy's memory. A `pointer` offers none of that knowledge, so it gets
none of that treatment.

So copying a pointer is cheap _because_ it is shallow — an address is just 8
bytes. The low cost is a consequence of the shallow copy, not the justification
for it.

A copied `pointer` still points at the **same** data, so a function can reach
through it and change the contents at that address — which is often the whole
point of passing a pointer:

```lale
fn write_byte(dest as pointer)
    // `dest` is a copy of the address, but it points to the same memory, so
    // this still changes the caller's data:
    unsafe value at dest = 42 as u8
end fn
```

For the same reason the compiler does **not** warn about pointer parameters —
there is no deep copy to warn about. The remaining distinction, for completeness:

- `pointer` (copy) — the function can change the **contents at** the address, but
  not _which address_ the caller's variable holds.
- `ref pointer` — the function can also reassign the caller's pointer variable to
  a different address.

#### Warning (definition-site)

When an **aggregate** parameter (`text`, array, struct, or enum) is declared with
no modifier, the compiler emits one warning **at the function definition** — not
at every call site:

```text
warning: parameter 'data' of type 'Point' is copied by value at every call.

  This performs a deep copy of the whole value (O(n) in its size), which may be
  slow for large arrays, structs, or strings.

  If this function only reads the parameter, pass it by reference instead:
      fn f(ref data as Point)

  Be aware: a `ref` parameter is mutable — writing to it inside the function
  changes the caller's value after the function returns.

  If you deliberately want a copy (so the function cannot affect the caller's
  data), write `copy` to make the intent explicit and silence this warning:
      fn f(copy data as Point)
```

`pointer` is intentionally excluded (see the pointer exception above). Vectors
of primitives (`vec3 of f64`) are fixed-size and cheap, so they are also not
warned; a vector with aggregate elements (`vec2 of text`) behaves like a struct
and is warned.

#### Migration from the old rule

- The default flipped from **by-reference** to **by-value**.
- `ref` is a new keyword.
- Code that relied on the old implicit by-reference mutation must add `ref`;
  code that already wrote `copy` is unaffected.

#### Deferred to 2.0.0 — read-only reference

A read-only reference ("cheap **and** immutable") is deliberately deferred to
2.0.0. The definition-site warning above advises `ref` for read-only
performance, but a `ref` parameter is always mutable — there is no way to
promise read-only access. Rather than add a `const ref` keyword (rejected in
§5.2: constness is inferred from SSA, not declared), a future read-only
reference is **compiler-inferred**: a `ref` parameter that is never written is
treated as read-only, for optimization and possibly a lint. Enforcement is a
separate open question; adding it later is additive and non-breaking. See
`doc/roadmap.md` §4.3 item 8 for the open questions.

### 5.14e The `binary` Type and the `byte` Type

`binary` is the raw-bytes companion to `text`. Its layout is
`{ ptr: pointer, bytes: u64 }` (16 bytes) — the same as `text` but **without** a
code-point counter and **without** a null terminator. It is compiler-managed and
owned exactly like `text`: freed at scope exit, deep-copied on by-value parameters,
and tracked by the leak detector. The stdlib `readBytes`/`writeBytes` functions
produce and consume `binary` buffers for binary I/O.

`byte` is a distinct raw-octet type (`IrType::Byte`), deliberately not an alias of
`u8` and not a number. It has no arithmetic or ordering operators — only bitwise
operations and equality — and converts to/from integers explicitly (`b as u8` /
`u8 as byte`). Byte literals are two-digit uppercase hex (`0x50`), and `write`/
embedding render a `byte` as `0xNN` via the `__lale_byte_to_str` builtin.

### 5.15 Semantic Conversions: Numeric‑to‑Character

#### Decision

The type system distinguishes between **narrowing conversions** (forbidden) and **semantic conversions** (allowed). Converting a numeric code to a `char` is a semantic conversion.

#### Rationale

- **Narrowing Conversion** (❌ Forbidden): Converting within the same semantic category where information is lost (e.g., `u64` → `u8`).
- **Semantic Conversion** (✅ Allowed): Converting between different semantic categories without losing information (e.g., `48 as u64 → '0'` as `char`).

The distinction: Narrowing loses data _within the same category_. Semantic conversion changes _between categories entirely_.

#### Implementation

```rust
(TypeCategory::Numeric(_), TypeCategory::Char) => {
    // Semantic conversion: Numeric → Char
    // Allowed because it's a semantic category change, not data loss.
    // Valid Unicode range: 0x00000000 to 0x10FFFF
}
```

#### Standard Library Application

The `__lale_u64_to_str()` function uses this for digit‑to‑character conversion:

```lale
var char_code as u64 = 48 as u64 + digit  // ASCII code for '0'-'9'
var char_val as char = char_code as char  // ✓ Semantic conversion
```

#### Design Philosophy

This principle maintains safety (information loss is caught) while enabling expressiveness (category changes are allowed). It solved the problem of converting digits to characters without language extensions or IR fallback.

#### Future Extensions

This pattern can extend to other semantic conversions (e.g., `u8 → char`, `u32 → char` with range check up to `0x10FFFF`, `i32 → char` with validation).

### 5.16 Read as String, Convert Explicitly

#### Decision

The `read` statement always reads into a `text` variable, and the type is explicit in the syntax (`read value as text`). Type conversion is the programmer's responsibility, not the language's.

#### Rationale

```lale
// Declares `input` as `text` — the type is explicit, no `var` needed (like loop variables)
read input as text
// Invalid input is caught when the programmer parses it:
// var val as f64? = parse_float(input)
```

- `read` always targets `text` — the `as text` is enforced by the grammar and cannot be any other type, so `read x as i32` is a syntax error; the variable is still auto-declared by the statement
- An explicit `var` before `read` for the same variable is a compile error
- The interpreter's `__lale_read_line()` built‑in returns a `text` struct directly
- Parse functions (`parse_float`, `parse_int`, `parse_uint`) return optional types (`f64?`, `i64?`, `u64?`) — see §5.17
- This design avoids implicit type coercion, silent default values (e.g., returning `0` on parse failure), and keeps the language surface small

### 5.17 Optional Types

#### Decision

Lale provides optional types (`T?`) for fallible operations. Unlike Rust's `Option<T>` (a standard‑library enum), Lale's optionals are language‑level: the `?` type modifier is grammar, and optional semantics are built into `return` and condition checks.

#### Syntax

```lale
fn parse_float(input as text) returns f64?
    if input.len == 0
        return nothing    // → absent optional
    end if
    return strtod(input.ptr, 0 as pointer)  // → present optional
end fn

// Check presence with natural‑language operators
var result as f64? = parse_float(input)
if result has value
    m = value of result   // safe unwrap
else
    write "invalid input"
    exit program 1
end if

if result has no value
    exit program 1
end if
```

- `T?` is a type modifier recognised by the grammar: `f64?`, `i64?`, `text?`
- `return nothing` produces an absent optional in a `T?` function; harmless in `returns nothing` functions
- `return expr` produces a present optional when the return type is `T?`
- `has value` / `has no value` are postfix operators returning `bool` — they work in `if` conditions naturally
- `value of expr` unwraps a present optional; unwrapping an absent optional is a runtime error (the interpreter writes a message to stderr and aborts). Like Rust's `unwrap()` on `None`, this is a programmer error — the contract is that the caller must check `has value` first.
- No `some()` or `none` keywords appear in user code — producing and consuming optionals uses existing language constructs
- **The `?` operator** (`expr?`) propagates `nothing` upward: if `expr` is absent, the enclosing function returns `nothing`. At the top level, it prints an error and exits. Only valid in functions returning `T?` or at the top level. In debug mode, propagation prints full diagnostics; in release mode, a brief message.

#### Implementation

Struct‑based representation in IR (`{is_present: bool, value: T}`). The `UnwrapOptional` IR instruction checks `is_present` at runtime and aborts (writes a message to stderr) if absent. The `?` operator is expanded to a conditional branch + `return nothing` (or `exit` at top level) at IR generation time, using block-level control flow.

---

### 5.18 Error Stack

#### Decision

Lale provides a built‑in, global error stack (`Vec<String>`) for diagnostic messages. It is decoupled from the return‑type system: functions return `T?` for presence/absence; diagnostics flow through a side channel.

#### Rationale

- **Control flow** → handled by `T?` (`has value`/`has no value`/`value of`). The caller branches on presence.
- **Diagnostics** → handled by the error stack (`add error`/`last error`/`alert error messages`). The caller collects and surfaces messages.

No other mainstream language decouples these concerns. Rust, Go, Java all couple error types to function signatures. Lale's design keeps signatures clean (no error type parameter) while enabling multi‑error accumulation in validation workflows.

#### API

| Operation              | Result | Empty‑stack behavior                   |
| ---------------------- | ------ | -------------------------------------- |
| `add error expr`       | (void) | Pushes message                         |
| `errors has messages`  | `bool` | Returns `false`                        |
| `last error`           | `text` | Returns `"Nothing"`                    |
| `alert error messages` | (void) | Drains to stderr (`Alert` + timestamp) |

All operations are total — no runtime aborts. The word `"Nothing"` is the universal sentinel for absence.

#### Design Trade-offs

- **Global mutable state** — messages interleave across calls. Mitigated by documentation prescribing self‑identifying messages (include function name and input value).
- **No structured error types** — messages are plain strings. Accepted as a deliberate simplicity trade‑off for Lale's target domain (scientific computing, scripting).
- **Reading is consuming** — `last error` pops; `alert error messages` drains. No stale messages accumulate.

#### IR Instructions

`PushError`, `PopError`, `ErrorCount`, `DrainErrors`. The interpreter maintains a `Vec<String>` on the execution state.

---

### 5.19 Alert Statement

#### Decision

Lale provides `log`, `warn`, and `alert` as leveled output statements distinct
from `write` (stdout) and `debug` (introspection).

| Statement | Destination | Level token    | Color   |
| --------- | ----------- | -------------- | ------- |
| `write`   | stdout      | (none)         | Default |
| `log`     | stderr      | `Log`          | Default |
| `warn`    | stderr      | `Warn`         | Yellow  |
| `alert`   | stderr      | `Alert`        | Red     |
| `debug`   | stderr      | `DEBUG expr =` | Cyan    |

Each `log`, `warn`, and `alert` emits one tab-separated line
`<Level>\t<timestamp>\t<message>` (plus file/line/function in debug builds).
`alert` signals an error condition without aborting. For termination, pair with
`exit program`.

#### Grammar

`alert` is a keyword statement consumed by `alert_stmt`. PEG ordered choice
handles disambiguation with the error-stack operation `alert error messages`.

---

### 5.20 Vector Types and Dot/Cross Product

#### Decision

Lale supports primitive vector types (`vec2 of T`, `vec3 of T`, `vec4 of T`) with built‑in dot product (`⋅` or `dot`) and cross product (`⨯` or `cross`) operators. Semantic rules validate correct usage at compile time.

#### Rationale

1. **Unique selling point** — No language combines built‑in vector types, first‑class physical units, and dot/cross product operators. Lale is the first.
2. **Physical units on vectors** — The existing unit analysis extends naturally: `force ⋅ displacement` produces a scalar with multiplied units, `r ⨯ F` produces a vector with the same unit multiplication.
3. **Compile‑time safety** — The type system prevents invalid operations:

| Operation | Signature                           | Compile check                                                    |
| --------- | ----------------------------------- | ---------------------------------------------------------------- |
| u ⋅ v`    | `vecN of T ⋅ vecN of T → T`         | Both must be same N and inner type T. Valid for all N.           |
| `s ⋅ t`   | `T ⋅ T → T` (scalar dot)            | Both scalars of same type T. Equivalent to `s * t`.              |
| `u ⨯ v`   | `vec3 of T ⨯ vec3 of T → vec3 of T` | Both must be `vec3 of T`. Rejected for `vec2`/`vec4`/non‑vector. |
| `s * v`   | `T * vecN of T → vecN of T`         | Scalar–vector multiplication uses `*`, not `⋅`.                  |

1. **Scalar-to-vector assignment rejected** — `var foo as vec3 of f64 = 500` is a compile error. No implicit splatting. Use `vec3(500, 500, 500)` or `[500, 500, 500]`.
2. **No operator conflicts** — `⋅` is used inside angle brackets for unit syntax (`<kg⋅m/s²>`) and `⨯` is used in expressions for cross product. The PEG parser distinguishes productions naturally by enclosing context. `⨯` is not valid in unit syntax (cross product of units is meaningless).
3. **Consistent with Lale's style** — Lale uses English keywords for operators (`bitwise and`). `dot`/`cross` keywords are available alongside `⋅`/`⨯` Unicode operators.

#### Important Distinction

The dedicated cross-product symbol `⨯` (VECTOR OR CROSS PRODUCT, U+2A2F) is valid only in expressions (cross product on vec3). It is **not** valid in unit syntax — cross product of scalar units is meaningless. The generic multiplication sign `×` (MULTIPLICATION SIGN, U+00D7) is **completely excluded from the Lale grammar**. For unit multiplication, use `⋅` (U+22C5 DOT OPERATOR) or `*` (ASCII asterisk).

Similarly, the `*` operator is rejected when both operands are vectors (there is no "plain multiplication" between vectors; use `dot` or `cross`).

#### Dot Glyph Clarification

The middle dot `·` (U+00B7) is a generic punctuation character, not a mathematical operator. It is excluded from both expressions and unit syntax, following the same principle as `×` exclusion. Use `⋅` (U+22C5 DOT OPERATOR) for all dot‑product and unit‑multiplication notation.

#### Grammar Design

```pest
vector_kind = { "vec2" | "vec3" | "vec4" }
vector_type = { vector_kind ~ ms_ln ~ "of" ~ ms_ln ~ type_name }

type_name = {
  vector_type |   // "vec3 of f64" — must precede identifier (no reserved keywords)
  ("[" ~ os ~ ... ~ "]" ~ optional_marker?) |
  ((u8 | i8 | ... | single_identifier) ~ arr_index? ~ optional_marker?)
}
```

The `vector_type` rule is recursive via `type_name`, enabling `vec3 of vec3 of f32` (a 3×3 matrix). The `of` keyword separates _storage_ composition from _physical_ dimensions (`in <unit>`):

```lale
var position as vec3 of f64 in <m> = vec3(1.0, 2.0, 3.0)
//           ^^^^^^^^^^^ type        ^^^^^ unit
//           "of" = what it's made of (storage)
//                                "in" = what it represents (physics)
```

**2D cross product — rejected**: `⨯` on `vec2 of T` is a compile error. Rationale: correctness over convenience.

#### Implementation

✅ Implemented (August 2026): grammar (`vector_kind`), type system (`IrType::Vec2/Vec3/Vec4`), Pratt infix operators (`dot`/`⋅`, `cross`/`⨯`), semantic and unit validation, IR instructions (`Dot`, `Cross`, `BuildVec*`, `ExtractVecElement`), and the interpreter.

Remaining: nested vectors / matrices — the grammar accepts `vecN of vecM of T`, but `TypeName.inner_type` stores only one nesting level.

### 5.21 `×` (U+00D7 MULTIPLICATION SIGN) Excluded from Grammar

#### Decision

The character `×` (U+00D7 MULTIPLICATION SIGN) is completely excluded from the Lale grammar — it is not a valid operator in expressions and not a valid multiplication separator in unit angle‑bracket syntax.

#### Rationale

**`·` (U+00B7 MIDDLE DOT) is also excluded** — it is a generic punctuation character, not a mathematical operator. Use `⋅` (U+22C5 DOT OPERATOR) for dot product and unit multiplication.

**`⨯` is expression‑only** — cross product of scalar units is illogical, so `⨯` is not in `mul_sign`. Units multiply with `⋅` or `*`.

#### Unit Multiplication Alternatives

`<kg⋅m/s²>`, `<kg*m/s²>` — both valid.

#### Unit Division Symbols

`div_sign` accepts `/` (solidus), `÷` (U+00F7 division sign), `⁄` (U+2044 fraction slash), and `∕` (U+2215 division slash). The `⅟` (U+215F fraction numerator one) is explicitly excluded — it is a numeric fraction form, not a division operator.

### 5.22 Superscript Digits as Power Operators

#### Decision

Superscript digits (`²`, `³`, `⁻¹`, `⁺²`, etc.) are parsed as postfix power operators in both expressions and unit angle‑bracket syntax.

#### Rationale

Superscript digits are not valid identifier characters in Lale (only subscript digits form identifiers). Therefore `v²` cannot be parsed as a variable name — it can only mean `v ^ 2`. This is consistent with mathematical notation where superscripts denote exponentiation, eliminating the need for a separate `^` operator when writing expressions that mirror formulas directly.

#### Consistency with Lale's Unicode Philosophy

Lale embraces Unicode where it makes code clearer and closer to mathematical notation. Subscript digits extend identifiers (`x₁` is a variable named `x₁`); superscript digits extend operators (`v²` means `v` raised to the power 2). Both serve different semantic roles but share the same goal: code that reads like the formulas it implements.

#### Implementation

- **Grammar**: The `superscript_power` PEG rule matches only superscript digits (`⁰¹²³⁴⁵⁶⁷⁸⁹`) and optional sign (`⁺⁻`). Other Unicode superscript characters (parentheses `⁽⁾`, letters `ᵃᵇ`, etc.) are **not** valid — they produce parse errors.
- **AST**: `build_postfix` in the Pratt parser handles superscript tokens as postfix operators and converts them to `BinaryOp::Pow` with the base expression as left operand and the exponent as right operand.
- **Unit parser**: `parse_term()` in `NormalizedUnit` strips trailing superscript digits from unit names and applies them as exponents.

#### Examples

```lale
// Expressions
var E as f64 = 0.5 * m * v²        // v² → v ^ 2
var reciprocal as f64 = x⁻¹         // x⁻¹ → x ^ (-1)
var big as f64 = x²³                // x²³ → x ^ 23

// Unit syntax
<kg⋅m²/s²>    // same as <kg⋅m^2/s^2>
<m⁻²>          // same as <m^-2>
```

### 5.23 Field Encapsulation via `private`

#### Decision

Type fields can be marked `private`, restricting read and write access to within the defining module. Fields without `private` remain public.

#### Rationale

Without encapsulation, any code can mutate internal fields of core types (e.g., `text.bytes`). This makes it impossible to guarantee invariants — a malicious or buggy module can set `byte_len = 9999999` and pass the corrupted string to C-FFI, causing an out-of-bounds read. The `private` keyword gives library authors an escape hatch: expose a safe public API while keeping raw pointers, lengths, and internal state protected.

#### Design Is Intentionally Minimal

Only `private` exists — no `protected`, `internal`, `readonly`, or other modifiers. This follows Rust's philosophy: `private` is the default for encapsulation, `public` is the default for data. The module boundary (not the type boundary) is the trust unit — any code within the same module can access private fields freely.

#### Grammar

```pest
type_field = { (kw_private ~ ms_ln)? ~ single_identifier ~ ms_ln ~ kw_as ~ ms_ln ~ type_name ~ (ms_ln ~ kw_in ~ ms_ln ~ unit)? ~ (info)?}
```

#### Error Message Format

```text
Cannot access private field 'key' of type 'Secure' from outside its defining module 'types'
```

#### Implementation

The `TypeDefInfo` struct carries both `is_private` (per field) and `module_path` (per type). `visit_member_access` and `visit_assign` compare the analyzer's `current_module_path` against the type's `module_path`. If they differ and the field is private, an error is emitted.

---

### 5.24 `switch` — Value Comparison and Enum Matching

The `switch` statement compares a value against literal cases or enum variant patterns and executes the matching arm. Unlike C `switch`, there is no fallthrough.

```lale
switch shape
    case Circle(r): write "radius: {r}"
    case Rectangle(w, h):
        write "{w} \u00d7 {h}"
    case Point: write "point"
end switch
```

For open types (strings, integers, floats, chars) where not all values can be enumerated, a `default` arm is required:

```lale
switch code
    case 200: write "OK"
    case 404: write "Not Found"
    default: write "Other"
end switch
```

#### Exhaustiveness Rules

- **Enum matching:** All variants must be covered explicitly. If every variant has a `case` arm, no `default` is needed — the match is provably exhaustive.
- **Open values (strings, integers, floats, chars):** A `default` arm is mandatory since the value space is unbounded and cannot be enumerated.
- **`bool` matching:** `bool` is closed — it has exactly two values (`true` and `false`). Covering both is exhaustive, so no `default` is needed; adding one is unreachable.
- A `default` alongside full enum coverage (or full `bool` coverage) is a compile error (unreachable code).

#### Relationship to `match`/`when`

- `switch` compares one value against multiple cases — it replaces the general `else if` chain when dispatching on a single scrutinee.
- `match` / `when` handles ordered-condition matching where each arm has its own guard expression — the true replacement for `else if` chains.

#### Design Decisions

- **No new scope.** Pattern-bound variables are function-scoped, preserving the two-scope memory safety model.
- **Same name, same alloca.** Variables with the same name across arms share one stack allocation. The IR emits a single `alloca` regardless of how many arms bind it.
- **Union allocation for type-inconsistent names.** When `a` is `f64` in one arm and `i32` in another, the alloca uses `max(size, alignment)` of the types. The symbol table stores the union as `"f64|i32"`.
- **Type-consistent names ARE accessible after `end switch`.** If `a` has the same type in every arm that binds it, it can be used after the switch — same semantics as loop variables. Otherwise, the compiler rejects uses after `end switch`.
- **Symbol table impact:** One row per variable name in `symbols`. The `data_type` column holds either a simple type (`"f64"`) or a pipe-separated union (`"f64|i32"`). The analyzer maintains a `current_branch_bindings: HashMap<String, String>` for per-arm type context.
- **IR lowering:** Each arm compiles to a chain of basic blocks joined by `CondBr`/`Br` (the IR has no structured `IfElse` node — see §4.7). The enum's discriminant field is compared against each variant's ordinal. Pattern field bindings become `ExtractField` instructions from the variant struct. No IR changes are needed.
- **Memory safety:** Pattern-bound variables are function-locals. Taking `pointer to a` inside an arm is caught by the existing escape checker — no new analysis required.

### 5.25 Integer Overflow Traps by Default

#### Decision

Integer `+`, `-`, `*`, and unary `-` trap on overflow/underflow by default in **both** debug and release builds. The only way to get wrapping arithmetic is the explicit `--unchecked-overflow` compiler flag.

#### Rationale

Wrapping silently turns `127 as i8 + 1` into `-128` — a wrong number that propagates through a scientific computation without any signal. Trapping makes the failure loud and local, matching Lale’s existing treatment of division by zero and array-out-of-bounds. The flag is the explicit escape hatch for performance-sensitive numeric code that deliberately accepts wrap-around.

#### Implementation

- The IR carries both checked (`CheckedAdd`/`CheckedSub`/`CheckedMul`/`CheckedNeg`) and wrapping (`Add`/`Sub`/`Mul`/`Neg`) arithmetic instructions.
- IR generation selects which variant to emit based on `CompilerOptions::checked_overflow` (default `true`). The decision is therefore baked into the IR before any backend runs, preserving the backend-independence guarantee in §4.7.10.
- The interpreter implements width-aware overflow checks using the instruction’s declared `IrType`, since SSA registers hold `i64`/`u64` values regardless of the declared width.

**Compile-time detection.** In addition to the runtime trap, the semantic analyzer rejects constant integer arithmetic whose operands fold to known constants and whose result provably overflows/underflows. `check_integer_overflow` (`src/semantic_analysis/analyzer.rs`) folds the expression through the typed constant evaluator (`expr_const_value_typed` → `const_eval`) and reports `EvalResult::Trap` as a compile error, so `127 as i8 + 1 as i8` is rejected at compile time instead of aborting at runtime. Non-constant operands remain the responsibility of the runtime trap, and `--unchecked-overflow` disables both layers.

**Float counterpart — non-finite traps.** The same “trap loudly, not silently” principle
applies to floating point. `NaN` is IEEE 754’s silent-error vector: it propagates
through arithmetic and makes every comparison false, silently corrupting results;
`±Inf` (overflow to infinity) is likewise a silent loss of precision that propagates
into later arithmetic. So a float operation that produces `NaN` (e.g. `pow(−1, 0.5)`)
or `±Inf` (e.g. `1e308 ⋅ 1e308`), and a float comparison involving either, abort at
runtime via `src/interpreter.rs::reject_non_finite`.

### 5.26 The Constant `π` — The One Built-in Constant

#### Decision

`π` is the only mathematical or physical constant that is part of the language
itself rather than the standard library — for version 1.0.0 it is the one and
only such constant. There is no ASCII alias: `pi` is an ordinary identifier, free
for the user. No other constant is built in: neither mathematical (`e`, `γ`, `φ`,
Catalan's constant) nor physical (the speed of light, Planck's constant, the
gravitational constant, …).

#### Rationale

`π` is the one symbol that is unambiguous. As a _number_ it always means 3.14159…,
while every other candidate's symbol is already claimed several times over in
physics and engineering: `e` (Euler's number vs. elementary charge), `γ`
(Euler–Mascheroni vs. Lorentz factor vs. specific weight vs. shear strain), `φ`
(golden ratio vs. electric potential vs. phase), `G` (Catalan vs. gravitational
constant vs. Gibbs energy), `c` (speed of light vs. specific heat), `h` (Planck
vs. height vs. enthalpy). Reserving a bare symbol as a language constant is only
safe for `π`. Any other constant, if ever wanted, belongs in the standard library
under an unambiguous spelled-out name (`euler_mascheroni`, `golden_ratio`,
`speed_of_light`, …) — never as a single-letter built-in.

#### Value and Type

`π` behaves like the floating-point literal `3.141592653589793`: it is
_context-typed_, inferring `f16`, `f32`, or `f64` from the surrounding expression,
and defaulting to `f64` when no context constrains it (as in `write π` or
`var x = π`). It is dimensionless. This is deliberate: a fixed `f64` `π` would be
unusable in an `f32` or `f16` program, because Lale rejects narrowing conversions
and `π as f32` would itself be a narrowing error. Context typing makes `π` a
first-class citizen of every floating-point width.

#### Implementation

`π` is a _reserved identifier_ detected and handled entirely in semantic analysis;
the grammar and parser are unchanged. Because `π` is already a legal identifier,
the reservation is enforced in the analyzer rather than the grammar. Two steps:

1. **Name resolution.** When the analyzer resolves an `Identifier` whose text is
   `π`, it does not consult the symbol table. It folds the name directly to the
   literal `3.141592653589793` and types it as a context-typed float literal (see
   above), giving the natural syntax `2 ⋅ π ⋅ r` with no grammar or IR change.

2. **Reservation.** Any definition of `π` — `var π`, a parameter named `π`, or a
   function or type named `π` — is rejected with an error, in _any_ scope. This is
   what makes `π` unshadowable: unlike a pre-populated global symbol (which a
   function-local `var π` could shadow), the name itself is refused wherever it is
   used as a definition.

Two alternatives were considered and rejected: a _grammar token_ (reserving `π` in
the PEG touches the identifier rules for no semantic gain) and _symbol-table
pre-population_ (a synthetic global that a function-local `var π` can still
shadow, weakening "part of the language"). See `lale.md` "The Constant π" for the
user-facing description.

### 5.27 Numeric Literal Form — Digits Required on Both Sides of the Decimal Point

#### Decision

A numeric literal must have at least one digit on each side of the decimal
point. `.5` and `5.` are rejected at parse time; the explicit forms are `0.5`
and `5.0`. A literal also requires a leading digit before an exponent: `5e3` is
valid, `.5e3` is not.

#### Rationale

Lale prioritizes explicitness over abbreviation (see §5.5 on explicit
conversions and the "no implicit magic" principle throughout §5). A leading or
trailing dot is implicit shorthand that hides the scale of the number — `.5`
and `5.` are easy to misread, and in scientific code an order-of-magnitude slip
is exactly the failure the language is built to prevent. Requiring `0.5` and
`5.0` costs one keystroke and removes all ambiguity. This is the same instinct
behind rejecting implicit type conversions and requiring a leading `0` in unit
scalars (`0.5 <m>`, not `.5 <m>`).

#### Implementation

No dedicated error message is needed: the restriction falls directly out of the
literal rules (`float`/`decimal_only` require `digit+ ~ "." ~ digit+`, and `int`
carries a `!"."` guard), so the parser rejects the form before analysis. The
user-facing description is in `doc/lale.md` § "Data Types" → "Numeric Literal
Form".

### 5.28 A Single Member-Access Separator — `.` for Fields, Enum Variants, and Module Paths

#### Decision

One separator, `.`, handles every kind of member access: fields of a value
(`p.name`), enum variants (`Shape.Rectangle`), and module paths in `use`
statements (`use all from std.file_io_posix`). There is no separate arrow (`->`) or
double-colon (`::`) operator.

#### Rationale

The language previously split the separator: `.` for fields of a value, `->`
(the grammar's `link` rule) for names in a namespace — module paths and enum
variants. That split encoded a real distinction (a `.` reads a value's runtime
field; `->` navigates a compile-time namespace), but the boundary was
undocumented and subtle, and the compiler's own 1.0.0 "language-surface clarity"
audit flagged it as a blocker. A single operator is simpler to teach and to
remember — decisive for a language whose user guide targets high-school
students. The accepted cost is the loss of the at-a-glance "value vs. namespace"
signal; Lale deliberately does not recover it through a capitalization
convention.

#### Implementation

The grammar's `link` rule was removed and `qualified_identifier` now joins
segments with `.`. In expression position a dotted chain such as `a.b.c` parses
as nested `MemberAccess` nodes; enum-variant access (`Shape.Point`) is recognized
during semantic analysis (the object resolves to an enum type) and IR generation
(`self.enum_types`), rather than by a distinct token.

### 5.29 Global Initializers Are Compile-Time Constants

#### Decision

A module-level `var` must have an initializer that folds to a compile-time
constant (via constant propagation — a literal, or an expression over other
effectively-constant values). A non-constant initializer such as
`var x = read()` is a compile error; that setup is written explicitly as
top-level code or in the entry point.

#### Rationale

Global variables are shared across modules by reference (§5.4), so their
initializers run at module-load time in dependency order. Arbitrary,
side-effecting initializers reintroduce the C/C++ "static initialization order
fiasco": an initializer that reads another global depends on an initialization
order that is invisible at the type level. Restricting initializers to
compile-time constants eliminates the fiasco by construction and keeps Lale
explicit — the same instinct behind §5.5 (explicit conversions) and §5.27
(explicit literals).

#### C interoperability

This restriction does not affect C interop. C globals are declared with
`import var` (an extern symbol with no Lale initializer) and C functions with
`import fn signature`; both are resolved by the linker and are unaffected. A Lale
program may still assign to an `import var` at runtime (a write, not an
initializer).

#### Implementation

A constant initializer is stored directly in the IR global's `initializer` field
(`Global.initializer = Some(Constant)`), so merging modules carries the value
with the global and requires no propagation of top-level `Store` instructions.

---

## 6. Error Catalog

This section catalogs the semantic errors emitted by the analyzer. The analysis passes and design rules behind them are documented in §4 and §5.

### 6.1 Type Compatibility Errors

| Error                             | When                                                              | Example                  |
| --------------------------------- | ----------------------------------------------------------------- | ------------------------ |
| Binary operation type mismatch    | Two operands have incompatible types                              | `"Alice" > 30`           |
| Variable definition type mismatch | Initializer type doesn't match declared type                      | `var x as i32 = "hello"` |
| Assignment type mismatch          | Assigned value type doesn't match variable type                   | `x = "hello"` (x is i32) |
| Condition type mismatch           | Condition expression isn't boolean                                | `if 5 then ... end if`   |
| Unsigned subtraction underflow    | Right operand may be larger than left                             | `5 as u32 - 10 as u32`   |
| Constant integer overflow         | Constant `+`/`-`/`*`/`⋅` on integer operands overflows/underflows | `127 as i8 + 1 as i8`    |

**Allowed:** Exact type matches (`i32 + i32`), power with float base and integer exponent (`f64 ^ i32` for LLVM powi).

For `var` definitions and assignments, a numeric _literal_ or _introspection_ result (`#size of`, `#count of`) infers to any numeric type, and a binary arithmetic result may be assigned to a _wider_ numeric type (e.g. `i64` → `f64`). Narrowing and same-width signed↔unsigned assignments (`u64` → `i64`, `u64` → `u8`) are rejected and require `unsafe bitcast`.

**Rejected:** Cross‑category operations (`"string" + 5`), non‑boolean conditions (`if x` where x is i32).

### 6.2 Unit (Physical Dimension) Errors

| Error                               | When                                         | Example                       |
| ----------------------------------- | -------------------------------------------- | ----------------------------- |
| Variable definition unit mismatch   | Initializer unit doesn't match declared unit | `var d as f64 in <m> = 5 <s>` |
| Assignment unit mismatch            | Assigned value has different unit            | `distance = 10 <s>`           |
| Addition/subtraction unit mismatch  | Adding quantities with different units       | `5 <m> + 3 <s>`               |
| Comparison unit mismatch            | Comparing quantities with different units    | `5 <m> > 3 <s>`               |
| Exponent must be unitless           | Power operation exponent has units           | `5 ^ (2 <m>)`                 |
| Logical operators require unitless  | Operands to logical operators have units     | `true and (5 <m>)`            |
| Bitwise operations require unitless | Operands have units                          | `(5 <m>) bitwise and 3`       |
| Condition must be unitless          | Condition expression has units               | `if 5 <m> > 3 <m>`            |

### 6.3 Scope and Reference Errors

| Error                   | When                            | Example   |
| ----------------------- | ------------------------------- | --------- |
| Undefined variable      | Using a variable never defined  | `write x` |
| Assignment to undefined | Assigning to undefined variable | `x = 5`   |

### 6.4 Memory Safety Errors

| Error                              | When                                                        | Example                          |
| ---------------------------------- | ----------------------------------------------------------- | -------------------------------- |
| Escaping pointer to local          | Returning pointer to local variable                         | `return pointer to x`            |
| Global assignment of local pointer | Storing local pointer in global                             | `global_ptr = pointer to local`  |
| Pointer variable escape            | Returning/storing a pointer variable set to a local pointer | `var p = pointer to x; return p` |
| Private field cross-module access  | Accessing a `private` field from another module             | `s.key` (key is private)         |

### 6.5 Function Errors

| Error                   | When                                       | Example                                 |
| ----------------------- | ------------------------------------------ | --------------------------------------- |
| Parameter unit mismatch | Argument unit differs from parameter       | `getVelocity(5 <m>)` (expects `<s>`)    |
| Return unit mismatch    | Return value unit differs from declaration | `return 5 <m>` (function returns `<s>`) |

### 6.6 Special Cases

- **Power Operation Exception:** `float ^ integer` is allowed (matches LLVM's `llvm.powi` intrinsic): `5.0 ^ 2` (f64 ^ i32), `3.14 as f64 ^ 10 as u32` (f64 ^ u32).
- **Bit Shift Operations:** Require same‑type operands (no implicit conversion): `5 as u8 unsigned left shift 2 as u8` allowed; `5 as u8 unsigned left shift 2 as u32` rejected.

### 6.7 Error Stack and Alert

| Error                           | When                                     | Example                                        |
| ------------------------------- | ---------------------------------------- | ---------------------------------------------- |
| `value of` on non‑optional type | `value of` applied to a type without `?` | `value of 42` (42 is `i64`, not `i64?`)        |
| `value of` on absent optional   | Runtime abort (stderr + abort)           | `value of nothing` — missing `has value` guard |

### 6.8 `switch` Statement Errors

| Error                            | When                                                                           | Example                                                                                              |
| -------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| Switch not exhaustive            | Not all enum variants are covered and no `default`                             | `switch s / case Circle(r): ... / end switch` (missing `Rectangle`, `Point`)                         |
| Unreachable `default`            | All enum variants covered and `default` is also present                        | `switch s / case Circle(r): ... / case Rectangle: ... / case Point: ... / default: ... / end switch` |
| Duplicate `switch` case          | The same enum variant appears in more than one `case` arm                      | `switch s / case Red: ... / case Red: ... / end switch`                                              |
| Missing `default` for open type  | Open non-enum scrutinee (string, integer, float, char) with no `default` arm   | `switch code / case 200: ... / case 404: ... / end switch`                                           |
| Missing `bool` case              | `bool` scrutinee without both `true` and `false` and no `default`              | `switch b / case true: ... / end switch` (missing `false`)                                           |
| Unreachable `default` for `bool` | Both `true` and `false` covered and `default` present                          | `switch b / case true: ... / case false: ... / default: ... / end switch`                            |
| `switch` with only `default`     | No case arms, only `default`                                                   | `switch s / default: ... / end switch`                                                               |
| Conflicting types across arms    | Variable bound with different types in different arms, used after `end switch` | `a` is `f64` in `Square` arm, `i32` in `Rectangle` arm, then `write a` after `end switch`            |
| Variable not bound in all arms   | Variable bound in some arms but not all, used after `end switch`               | `b` bound only in `Rectangle` arm, then `write b` after `end switch`                                 |
| `_` used after `end switch`      | `_` is a discard marker, not a usable variable                                 | `write _` after `end switch`                                                                         |

### 6.9 `match` and Compile-Time Control Flow Errors

| Error                                           | When                                                                                                                                                            | Example                                                          |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Duplicate `match` arm                           | Two `when` arms have structurally identical guard conditions                                                                                                    | `match / when x > 5: ... / when x > 5: ... / end match`          |
| Duplicate `#match` arm                          | Two `#when` arms have identical compile-time conditions                                                                                                         | `#match / #when #debug: ... / #when #debug: ... / #end match`    |
| Duplicate `#switch` case value                  | The same compile-time constant appears in more than one `#case`                                                                                                 | `#switch 5 / #case 5: ... / #case 5: ... / #end switch`          |
| Compile-time condition must be boolean          | A `#if`/`#when`/`#match` condition is not a boolean expression                                                                                                  | `#if 5`                                                          |
| Compile-time condition not evaluable            | A `#if`/`#when`/`#match` condition references a runtime value                                                                                                   | `#if some_runtime_bool`                                          |
| Compile-time `#switch` value not evaluable      | The `#switch` scrutinee is not a compile-time constant                                                                                                          | `#switch some_runtime_var`                                       |
| Compile-time `#switch` case value not evaluable | A `#case` value is not a compile-time constant                                                                                                                  | `#switch 5 / #case some_var`                                     |
| Variable not definitely assigned                | A `var` defined in only some branches of `if`/`when`/`match`/`switch`/`loop` is used afterwards without being defined on every path with the same type and unit | `if c / var yyy as i8 = 7 / else / move on / end if / debug yyy` |

### 6.10 Runtime Control Flow Errors

| Error                   | When                                                  | Example                                           |
| ----------------------- | ----------------------------------------------------- | ------------------------------------------------- |
| `else if` not supported | A runtime `if` statement contains an `else if` clause | `if a > b then ... else if c > d then ... end if` |

**Parse-then-reject by design.** The grammar deliberately accepts `else if` so the
semantic analyzer can emit a clear, actionable error instead of a cryptic parse
error or a silent nested-`if` reinterpretation (which would reintroduce the
dangling-`else` ambiguity):

> `else if` is not supported. Use `match / when / else` or `switch / case / default` instead for multi-way branching, which is never ambiguous.

The AST retains `else_if_branches` solely so the analyzer can flag it. This mirrors
the `when` + `else` handling documented in rule 29.

---

## 7. Appendices

### Adding a New Language Feature

When extending the Lale language, follow these steps to ensure full compiler coverage:

1. **Grammar:** Add rules to `src/grammar/lale.pest`.
2. **AST:** Define new types in `src/ast/definitions.rs`.
3. **Builder:** Implement parse‑tree‑to‑AST conversion in `src/ast/builder.rs`.
4. **Visitor:** Update the `AstVisitor` trait in `src/ast/visitor.rs` and all visitor implementations.
5. **Printer:** Update `src/printers/ast.rs` for debugging output.
6. **Semantic Analyzer:** Add any necessary validation in `src/semantic_analysis/`.
7. **IR Generation:** Emit appropriate IR instructions in the IR builder. Ensure the new instructions comply with the deterministic semantics defined in Section 4.7.10 (no LLVM‑style UB, poison, or attributes).
8. **Serialisation:** Ensure the new instructions are round‑tripped correctly by the line-oriented IR text serialiser and deserialiser.
9. **Interpreter:** Handle the new IR instructions in the interpreter (`src/interpreter.rs`).
10. **Future backends:** When adding an AOT backend later, lower the new instructions there as well.
11. **Tests:** Add tests to the `tests/` directory.
12. **Validate:** Run `cargo run --bin lale-validate` to confirm coverage across all compiler phases.
