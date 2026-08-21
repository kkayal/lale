# Lale Compiler - Agent Instructions

This file provides instructions for AI coding agents working on this codebase.

## Git Operations (Prohibited)

Never perform any git operations that change the content or metadata of a file: no staging, no reverting, no committing, no branching. This is a hard requirement. The only allowed git operations are read-only ones such as `git log` and `git diff`.

## Check = Analyze Only

When asked to "check" something (e.g., "check for inconsistencies", "check the build"), this means:

- **Analyze and report findings only**
- **Do not fix or implement directly**
- **Ask for permission** before proceeding with any fixes or implementation

## Decision-Making: Propose, Don't Declare

When you encounter situations where an action (implementation, documentation) is **not viable**, **not feasible**, or **not preferable**, you must:

1. **Explain the reasons** — Document why the approach won't work (technical constraints, effort, trade-offs)
2. **Make a proposal** — Suggest alternatives, or clearly state the implications of each option
3. **Ask for a decision** — Request explicit guidance before proceeding
4. **Do NOT unilaterally declare work as complete, deferred, or abandoned**

### Anti-Pattern

```text
"Symbol deduplication is not viable because it requires 8-16 hours of work.
Phase 3 is therefore complete without this feature."
```

This approach makes assumptions about priorities, removes user agency, and prevents collaborative problem-solving.

### Correct Pattern

```text
"Symbol deduplication would require implementing native Lale conversion functions
(estimated 8-16 hours). This is separate from core tier filtering which is complete.

Options:
1. Defer symbol deduplication to Phase 4, ship core feature now
2. Extend Phase 3 timeline to include this work
3. Find a hybrid approach (partial implementation)

Which direction do you prefer?"
```

This approach provides clear reasoning, presents options with trade-offs, and preserves decision authority with the user.

### When This Applies

- Technical feasibility concerns (too complex, too much effort)
- Architectural questions (wrong design approach)
- Priority conflicts (multiple options with different trade-offs)
- Scope changes (discovered new requirements)

**Key principle**: Decisions about what gets deferred, abandoned, or reprioritized belong to the user, not the agent.

## Build and Test Commands

**The build system is: compiler → tests (stdlib is loaded from source):**

The `lale-lsp` crate is a workspace member — building the workspace keeps the
LSP binary in sync with compiler changes.

```bash
# Build everything (compiler + LSP server)
cargo build --workspace

# Build only the compiler (for quick iteration when not using the LSP)
cargo build

# For release builds (deployment/benchmarking)
cargo build --release

# Type check without building (fastest verification)
cargo check --workspace

# Run clippy BEFORE the full test suite — fixes are mandatory
cargo clippy --workspace -- -D warnings

# Test the interpreter backend
cargo run -- run --print-ast --print-symbols --export-symbols symbols.db examples/complex.lale

# To verify a specific code change, run a specific test
cargo test <test_name>

# When code + clippy pass, run all tests
cargo test --workspace

# ⚠️ Always run `cargo build --workspace` BEFORE `cargo test --workspace`.
# `cargo test` compiles with `#[cfg(test)]` enabled (test profile), which hides
# dead-code warnings and unresolved imports in non-test code. Build and test
# profiles produce different artifacts — skipping `cargo build` means missing
# compilation errors that only surface without `#[cfg(test)]`.
cargo build --workspace && cargo test --workspace

# Run security checks. This runs:
# 1. `cargo clippy --all -- -D warnings` — All clippy warnings must pass
# 2. Unwrap/expect count check — Must not increase beyond baseline
# 3. `cargo audit` — Check for vulnerable dependencies
cargo make security-check

# Compiler validation — checks grammar → AST → IR → Interpreter coverage
cargo run --bin lale-validate
```

## Code Style

- Follow existing patterns in the codebase
- Use `rustfmt` for formatting (configured in `rustfmt.toml`)
- Avoid adding new `.unwrap()` or `.expect()` calls in non-test code
- Prefer `Result`-based error handling in critical paths

## Clippy Policy

**All clippy warnings and errors must be resolved — no exceptions for pre-existing issues.** When clippy reports a warning or denied lint, fix it. This applies to:

- Source code: replace `.unwrap()` / `.expect()` with proper error handling (`?`, `match`, `if let`)
- Test code: add `#[allow(clippy::unwrap_used)]` or `#[allow(clippy::expect_used)]` to individual test functions, or use `expect("reason")` to document why a panic is acceptable
- Denied lints: `#![deny(clippy::unwrap_used)]` and `#![deny(clippy::expect_used)]` are crate-wide — every instance triggers a compile error. Either fix or allow.

**Do not suppress lints crate-wide** (e.g., `#![allow(...)]` at the module level). Each suppression must be scoped to the function or block where the violation is intentional and documented.

## Markdown Documentation Style

When creating or modifying markdown documents, run the following tools in order:

1. **markdownlint-cli2 --fix**: Auto-fix linting violations:

   ```bash
   markdownlint-cli2 --fix doc/*.md README.md
   ```

2. **prettier**: Improve formatting (line wrapping, spacing, list indentation):

   ```bash
   npx prettier --write doc/*.md README.md
   ```

**Special content protection**: When a markdown document contains ASCII art, ASCII graphics, or other content that depends on exact whitespace alignment, enclose it in a fenced code block with the `text` language identifier:

```text
╔══════════════╗
║  ASCII ART   ║
╚══════════════╝
```

This prevents `prettier` and `markdownlint-cli2` from reformatting or flagging the content.

## Symbol Management: Database as Single Source of Truth

The symbol table uses an **SQLite database as the single source of truth**, with `prepare_cached()` for statement reuse. Minimal caching only where unavoidable.

**Rationale**: Prioritize simplicity, reliability, and maintainability over optimization.

- Eliminates unnecessary data redundancy
- Simplifies state management (database is authoritative)
- Easier to debug and verify correctness
- `prepare_cached()` provides efficient statement reuse without extra complexity
- Acceptable performance for compilation workloads

**Trade-off**:

- Function body lookups use in-memory HashMap (unavoidable — `Vec<Stmt>` is an arbitrary AST tree that can't be practically serialized to SQLite). The unit analyzer needs the body to trace return expression units.
- Parameters are persisted in the `variables` table (with `is_parameter = 1`) and reconstructed on demand when the cache misses.
- Type lookups query database (TypeDefInfo is simple and can be reconstructed)
- All other symbol table queries use database with `get_symbols_for_scope()`

**Implementation**:

- `SqliteSymbolManager` stores persistent symbols in SQLite, parameters in HashMap
- `lookup_function()` returns cloned FnInfo from HashMap
- `lookup_type()` queries database and constructs TypeDefInfo
- `get_symbol_table()` queries database for variables and functions, returns complete symbol table

## Numeric Type System: No Implicit Conversions

**Philosophy**: Lale prioritizes safety and explicitness over convenience. All numeric operations require explicit type guidance.

### Type Inference (Context-Aware)

**Allowed**: Literals with explicit type context

```lale
var a as u32 = 5        // ✅ Literal 5 infers u32 (sole operand, clear context)
var c as i32 = -42      // ✅ Literal -42 infers i32 (sole operand, clear context)
var b = 5 as u32        // ✅ b infers u32. RHS Expression has a defined type for all its operands
```

**Rejected**: Literals without explicit context

```lale
var a = 5               // ❌ No type guidance (missing `as` declaration)
var a as u32 = 5 - 10   // ❌ Expression context without explicit casts
```

### Strict Type Matching in Arithmetic

**Rule**: Both operands in binary operations must have the same type. No implicit widening, no default type assumptions.

```lale
var a as i32 = 5
var b as i32 = 10
var c as i32 = a + b    // ✅ Both i32, exact match

var d as i64 = 100
var e as i32 = a + d    // ❌ ERROR: i32 + i64 (type mismatch)
                        // Solution: var e as i64 = (a as i64) + d
```

**Error Message Pattern**:

```text
Type mismatch in binary operation '+':
  Left operand type: i32
  Right operand type: i64
  Solution: Cast operands to the same type using the 'as' operator
```

### Underflow/Overflow Detection (Unsigned Arithmetic)

**Rule**: When subtracting two unsigned integers, if the right operand could be larger than the left, reject with an error.

```lale
var a as u32 = 5
var b as u32 = 10
var c as u32 = a - b    // ❌ ERROR: underflow risk (5 - 10 cannot fit in u32)
                        // Solution: Use signed integers for arithmetic with negatives
                        // var c as i32 = (5 as i32) - (10 as i32)

// Literal subtraction: compiler compares values at compile time
var safe as u32 = 100 - 50    // ✅ OK: 100 > 50, no underflow

// Variable subtraction: always rejected (compiler cannot track values)
var x as u32 = 100
var y as u32 = 50
// var z as u32 = x - y   // ❌ ERROR: even safe values rejected (use i32 instead)
```

**Error Message Pattern**:

```text
Potential underflow in unsigned subtraction:
  Left operand: 5 (u32)
  Right operand: 10 (u32)
  Result would be negative but u32 cannot represent negative values
  Solution: Use signed integers (i32) if you need to subtract larger values from smaller ones
    var c as i32 = (5 as i32) - (10 as i32)
```

**Detection Rules**:

- If both operands are literals: Compare values at compile time
- If either operand is a variable: Reject conservatively with explanation to use signed arithmetic
- Overflow in addition/multiplication: Similar detection for unsigned types approaching type boundaries

### No Default Type Assumptions

Literals must infer their type from context, never from a default integer type.

**Correct behavior**:

```rust
// ✅ Literal 5 directly infers u32 from context
var a as u32 = 5  // Internally: 5 → u32 (no intermediate default type)
```

### Implementation Locations

| Scenario                         | Handler                          | Location                                                          |
| -------------------------------- | -------------------------------- | ----------------------------------------------------------------- |
| Standalone literal in assignment | Semantic analyzer type inference | `src/semantic_analysis/analyzer.rs` — `expr_type()` method        |
| Literal in complex expression    | Semantic analyzer                | `src/semantic_analysis/analyzer.rs` — Binary operation checking   |
| Underflow detection              | Semantic analyzer + IR gen       | `src/semantic_analysis/analyzer.rs` or `src/ir_gen.rs`            |
| Type mismatch in binary ops      | Semantic analyzer                | `src/semantic_analysis/analyzer.rs` — Binary operation validation |

### Checklist for Type System Changes

When modifying numeric type handling:

- [ ] No intermediate default type for literals
- [ ] Literals in assignments infer type from declaration
- [ ] Binary operations reject mismatched types with clear error messages
- [ ] Unsigned subtraction detects underflow and suggests signed arithmetic
- [ ] All error messages suggest explicit `as` operator for resolution
- [ ] Interpreter backend tested for consistency

## Project Structure

- `src/` — Compiler source code
- `tests/` — Test suite
- `examples/` — Example Lale programs
- `doc/` — Documentation
  - `doc/ARCHITECTURE.md` — Compiler internals
  - `doc/lale.md` — Language reference
  - `doc/TODO.md` — Roadmap
  - `doc/security/` — Security audit documentation

## Handling Unknown Return Types in IR Generation

When functions' return types are needed during IR code generation (e.g., for embedded values), they may not yet be loaded in the IR module. Use the semantic analyzer as a fallback.

**Location**: `src/ir_gen.rs` in `generate_fn_call_expr()`

**Implementation**:

1. Try `self.builder.module().function_by_name()` first
2. If not found, query the analyzer: `analyzer.expr_type(&Expr::FnCallExpr(...))`
3. Convert the type string using `self.type_string_to_ir_type()`

Without this pattern, unknown types default to `IrType::I64`, causing wrong conversion functions to be called. The fix is generic — it works for all types the analyzer supports.

## Adding Support for New Types

When adding a new type to the interpreter (beyond Int, Uint, Float, String, Bool):

**1. IR Generation** (automatic via existing fallback)

No changes needed — semantic analyzer lookup handles it.

**2. Interpreter Comparison Operators** (explicit, type-specific)

If comparisons are allowed, add match arms to:

- `Instruction::Eq`, `Instruction::Ne`
- `Instruction::Lt`, `Instruction::Le`, `Instruction::Gt`, `Instruction::Ge`
- Location: `src/interpreter.rs`

Each type gets dedicated match arms to define comparison semantics.

Supported types: Int, Uint, Float, String, Bool (full comparison support), Pointer (by combined address+offset), Null (equality checks).

**3. Interpreter Logical Operations** (if applicable)

- `Instruction::And`, `Instruction::Or`, `Instruction::Xor`, `Instruction::Not`
- Location: `src/interpreter.rs`

**Design principle**: Fail explicitly. If a type doesn't support an operation, return an error rather than defaulting to a numeric conversion.

## Preventing Silent Error Acceptance

**Critical Pattern**: The compiler has two error reporting mechanisms. Always use them when error conditions are detected.

### Pattern 1: Semantic Errors → Use `add_error()`

When validation fails in semantic analysis, report it to the user:

```rust
// ❌ WRONG: Silent return
if fn_call.arguments.len() != fn_info.parameters.len() {
  return;  // User never sees the error!
}

// ✅ CORRECT: Report error
if fn_call.arguments.len() != fn_info.parameters.len() {
  self.add_error("Function argument count mismatch", &location);
  return;
}
```

**Locations that use `add_error()`**:

- `src/semantic_analysis/analyzer.rs` — Type checking, symbol resolution
- `src/semantic_analysis/unit_analyzer.rs` — Unit validation
- `src/semantic_analysis/memory_safety.rs` — Pointer and cast validation

### Pattern 2: Fallback/Inference Failures → Use `eprintln!()` Diagnostic

When code falls back to a default because of missing information, log a diagnostic:

```rust
// ❌ WRONG: Silent fallback
let ptr_type = self
  .get_pointer_target_type()
  .unwrap_or_else(|| IrType::raw_ptr());  // User doesn't know this happened

// ✅ CORRECT: Log diagnostic
let ptr_type = match self.get_pointer_target_type() {
  Some(t) => t,
  None => {
    eprintln!(
      "WARNING: Pointer type lookup failed for '{}'. \
       Using generic raw pointer.",
      var_name
    );
    IrType::raw_ptr()
  }
};
```

**Locations that use diagnostics**:

- `src/ir_gen.rs` — Type inference, literal parsing
- `src/interpreter.rs` — Memory region operations
- `src/semantic_analysis/module_resolver.rs` — Path canonicalization

### Pattern 3: Multiple Validations → Accumulate Errors, Don't Early Return

When checking multiple conditions, collect all errors before returning:

```rust
// ❌ WRONG: Stops after first error
if unit_error {
  self.add_error("Unit problem", location);
  return;  // Never checks bit-width!
}
if bit_width_error {
  self.add_error("Bit-width problem", location);
}

// ✅ CORRECT: Check all conditions
if unit_error {
  self.add_error("Unit problem", location);
}
if bit_width_error {
  self.add_error("Bit-width problem", location);
}
// Both errors reported in one pass
```

**Location**: `src/semantic_analysis/memory_safety.rs`

### Pattern 4: Loop/Region Operations → Check ALL Items, Don't Early Return

When iterating through items, verify all items before deciding:

```rust
// ❌ WRONG: Returns after first match
for region in &self.regions {
  if region.contains(address) {
    region.store(value);
    return;  // Never checks for overlaps!
  }
}

// ✅ CORRECT: Collect all matches, then decide
let matches: Vec<_> = self.regions
  .iter()
  .filter(|r| r.contains(address))
  .collect();

match matches.len() {
  0 => { /* create new region */ }
  1 => { /* normal store */ }
  _ => {
    eprintln!("WARNING: {} overlapping regions", matches.len());
    matches[0].store(value);  // Use first, but warn
  }
}
```

**Location**: `src/interpreter.rs`

### Checklist for Code Review

When reviewing changes that handle errors, verify:

- [ ] No bare `return` statements without calling `add_error()` first (semantic analysis)
- [ ] No `.unwrap_or()` defaults without explaining the fallback via `eprintln!()`
- [ ] No early returns that skip subsequent validation checks
- [ ] No loops that exit on first match without checking all items
- [ ] All error paths have a corresponding test case

### Pattern 5: Incomplete Code → Crash, Don't Silently Continue

When a code path is genuinely incomplete (feature not yet implemented, edge case not
handled), use Rust's standard crash macros. Never silently continue with a default:

```rust
// ❌ WRONG: Silently skip unimplemented features
FuncRef::External(name) => {
  return Ok(());  // Unknown function? Just ignore it!
}

// ❌ WRONG: Silently fall back to a default
_ => src_value  // Unknown type conversion? Just return the original!

// ✅ CORRECT: Crash with a clear message
FuncRef::External(name) => {
  unimplemented!("External function '{}' not yet implemented", name);
}

// ✅ CORRECT: Crash if this should be impossible
_ => unreachable!("Type conversion from {:?} to {:?} should be validated before IR gen", src, dst)
```

**When to use each macro**:

| Macro              | Meaning                                 |
| ------------------ | --------------------------------------- |
| `todo!()`          | Feature planned but not started         |
| `unimplemented!()` | Feature exists but this path isn't done |
| `unreachable!()`   | This path should be impossible          |

**Corollary: No silent `Ok(())` or `return value` in error-adjacent paths.**
If you encounter an unexpected state, crash. The compiler must never produce wrong
output because it silently fell through to a default.

**Tracking**: Use `rg 'todo!|unimplemented!' src/` to count remaining work.
The count must decline over time.

### Checklist for Code Review (updated)

- [ ] No bare `return` statements without calling `add_error()` first (semantic analysis)
- [ ] No `.unwrap_or()` defaults without explaining the fallback via `eprintln!()`
- [ ] No early returns that skip subsequent validation checks
- [ ] No loops that exit on first match without checking all items
- [ ] All error paths have a corresponding test case
- [ ] No silent `Ok(())` in match arms that don't handle all cases — use `todo!()` or `unimplemented!()`
- [ ] No silent fallback to defaults in type conversion / code generation — use `unreachable!()`
- [ ] After completing: run `cargo run --bin lale-validate` and report silent error counts

## Silent Error Tracking

### Rule 1: Report Silent Error Counts After Every Feature/Fix

After completing a feature implementation or bug fix, run the validator and report
the silent error counts. **Do not fix the findings** — just report them so the user
can track progress:

```bash
cargo run --bin lale-validate
```

Report the counts in a compact format:

```text
Silent errors: 0 HIGH, 44 MEDIUM, 236 LOW (280 total)
```

This makes the trend visible without getting sidetracked into fixing pre-existing issues.
The HIGH count must be 0. The MEDIUM and LOW counts must decline or stay flat —
they must never increase as a result of new code.

### Rule 2: Deep Audit Reminder on Version Change

When a session starts, check if the version in `Cargo.toml` differs from the
version recorded in `doc/silent_errors_audit.md`. If they differ, the audit is
stale and needs updating:

```bash
# Read current version
grep '^version' Cargo.toml | head -1
# Compare against audit
head -3 doc/silent_errors_audit.md
```

If the versions don't match, remind the user:

> ⚠️ Cargo.toml says `v0.9.0` but the audit covers `v0.8.6`.
> Time for a deep analysis audit of silent fallbacks.
> Run the full audit or review `doc/silent_errors_audit.md` and update it.

The deep audit catches patterns the text-scanner in `lale-validate` misses:
empty catch blocks, `unwrap_or_default`, `return` without `add_error()`, and
semantic match-arm fall-through. Version bumps are the right cadence for this —
not every commit, but every release boundary.

## IR Generation: Single `try_generate_*` Entry Point

All IR generation methods that produce code for statements/definitions now follow a unified pattern:

- **Canonical implementation**: `try_generate_*` methods return `CompileResult<()>`
- **Panic-wrapper**: `generate_stmt` is a thin wrapper around `try_generate_stmt` using `Self::unwrap_or_panic()`
- **Leaf methods** (e.g., `generate_if`, `generate_loop`, `generate_type_constructor`) have no duplicate — they are called from both paths

**Rationale**: Eliminated ~500 lines of near-duplicate code between `generate_*` and `try_generate_*` variants. This prevents fix asymmetry bugs.

**When adding new statement types**: Always implement `try_generate_*` returning `CompileResult`. Add the dispatch to `try_generate_stmt`. The `generate_stmt` wrapper handles it automatically.

## IR Generation: SQLite Symbol Manager Only

`IrGenerator` must **never** reference `SemanticAnalyzer`. The analyzer's job ends before IR generation begins. All type lookups during code generation use `self.symbol_manager: Option<&SqliteSymbolManager>` — the SQLite database directly.

**Rationale**: The analyzer is a transient visitor object that walks the AST and populates the database. Once semantic analysis is complete, the database is the single source of truth. Storing a reference to the analyzer in `IrGenerator` creates an unnecessary second code path for type queries, leading to duplicated logic and silently-divergent behavior between the `try_generate` and `try_generate_owned` entry points.

**Enforcement**:

- `IrGenerator` fields: only `symbol_manager` and `is_debug` (extracted from the analyzer before it's dropped)
- If a type query needs the analyzer, the query belongs in semantic analysis, not IR generation
- Both entry points (`try_generate` and `try_generate_owned`) set `self.symbol_manager` from the SQLite database — no analyzer path exists

## Variable Scope Detection: `in_user_function` Flag

Global-scope variable definitions (which become IR globals) are detected via the `in_user_function: bool` flag on `IrGenerator`, not by counting scope stack entries.

- `in_user_function = false` → variable is global-scope → creates IR global
- `in_user_function = true` → variable is function-local → creates stack alloca

Set `in_user_function = true` at the start of `try_generate_fn_def`, restore at the end.

## Interpreter: No Silent Zero-Value Fallbacks

Type conversion instructions (`UiToFp`, `FpToSi`, `Trunc`, `SExt`, etc.) now return `Err` when given operands of unexpected types, instead of silently inserting `Value::Int(0)` / `Value::Float(0.0)`. This makes type errors visible rather than producing wrong results.

**When adding new instructions**: Match only expected types. Return `Err` with a descriptive message for unexpected types.

## Interpreter: AOT Consistency Through IR-Only Semantics

All Lale semantics flow exclusively through the IR. The interpreter must never contain business logic that a future AOT backend would need to duplicate. Both backends consume the same IR and must produce identical behavior for any valid Lale program.

### Handler Constraints

The interpreter may only have handlers for C standard library functions (`write`, `read`, `open`, `close`, `malloc`, `free`, `pow`, `puts`, `strtod`, `strtol`, `strtoul`, `__lale_exit`, `__lale_read_line`). These are the FFI boundary — the point where Lale ends and the OS begins.

- **No per-function boilerplate.** All extern handlers are dispatched through a single `call_extern` function with thin match arms (3–10 lines each). Four helper functions (`get_int_arg`, `get_ptr_arg`, `mem_read_bytes`, `mem_write_bytes`) handle the repetitive work of extracting typed arguments and translating between simulated and real memory.
- **The memory model is the constraint.** The interpreter's `Value::Pointer { base, offset }` is an index into `MemoryManager`, not a real host address. This is documented in `doc/ARCHITECTURE.md` §4.8. Every FFI call requires translation: read bytes from `MemoryManager` into a real buffer, then pass to the OS. This translation is unavoidable — but it belongs in centralized helpers, not scattered across handlers.

### Adding a New Extern

1. Declare it in the stdlib as `import fn` (not hardcoded in `add_stdlib_externs()` — that is the deprecated pattern)
2. Add a thin match arm to `call_extern` in `src/interpreter.rs` using the helpers
3. Both interpreter and AOT see the same `import fn` declaration in the IR — the interpreter provides the runtime implementation, AOT links against libc

### Checklist

- [ ] No new `FuncRef::External(name) if name == ...` match arms outside `call_extern`
- [ ] No Lale type construction or business logic in handler code — only FFI translation
- [ ] New extern uses `get_int_arg`/`get_ptr_arg`/`mem_read_bytes`/`mem_write_bytes` helpers
- [ ] Same behavior verified: `cargo run -- examples/complex.lale` produces identical output
- [ ] Memory model implications documented if the new extern introduces a new pattern

## Known Constraints

- Two scopes only: global and function-local (intentional design)
- No implicit type coercion. All conversions explicit (intentional design)
- 1-based array indexing (mathematical convention)

## Pointer Arithmetic

Pointer arithmetic is supported for systems programming with C-compatible operations.

### Design

**Three operations allowed**:

1. **`ptr + u64` → `pointer`**: Move pointer forward
2. **`ptr - u64` → `pointer`**: Move pointer backward
3. **`ptr1 - ptr2` → `i64`**: Signed byte difference (can be negative)

**Three operations rejected** (compile-time errors):

- `u64 + ptr`: Not commutative
- `ptr + ptr`: Nonsensical
- `ptr * u64`, `ptr / u64`, etc.: Invalid operations

### Implementation Locations

**Type System** (`src/unit_analysis/mod.rs`):

- Validates pointer arithmetic operations
- Allows commutative cases only

**Type Inference** (`src/semantic_analysis/analyzer.rs`):

- `ptr + u64` → `pointer`
- `ptr - u64` → `pointer`
- `ptr - ptr` → **`i64`** (signed, for negative results)

**IR Generation** (`src/ir_gen.rs`):

- `ptr ± u64`: Existing `Add`/`Sub` instructions
- `ptr - ptr`: Calls built-in `ptr_difference` function

**Interpreter** (`src/interpreter.rs`):

- `ptr_difference` handler: Converts pointers to `i64`, subtracts, returns `i64`

### Safety Model

No runtime bounds checking. User responsible for validity.

Example of undefined behavior:

```lale
var arr as i32[10] = [...]
var ptr as pointer = pointer to arr[2]
var bad as pointer = ptr - 5    // ❌ UB: goes before array start
var also_bad as pointer = ptr + 20  // ❌ UB: beyond array bounds
```

### References

- User documentation: `doc/lale.md` § Pointer Arithmetic
- Architecture details: `doc/ARCHITECTURE.md` § 4.5.5 Memory Safety
