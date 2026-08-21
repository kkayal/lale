<img src="lale-logo.jpg" align="right" alt="Logo" width="200">

# Lale Compiler — TODO and Future Work

## Next Priority Features (ranked by impact)

### 🔴 Critical: Security & Grammar

1. **`value at` / `value_at_assign` missing `unsafe` keyword**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Added `kw_unsafe` prefix to `val_at_op` (`src/grammar/lale.pest:259`) and `value_at_assign` (`src/grammar/lale.pest:534-538`). Both now require `unsafe value at` syntax matching the `unsafe_decl` pattern. Updated `builtins.lale`, `stdlib/src/file_io_posix.lale`, and all 10 test files.

2. **`type_field` missing physical unit annotation**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Added `(ms_ln ~ kw_in ~ ms_ln ~ unit)?` suffix to `type_field` in the grammar,
   mirroring `fn_parameter`. The `TypeField` struct gained a `unit: Option<Unit>` field.
   Semantic analysis validates unit matching on field access/assignment, and IR generation
   propagates unit information. Example: `type Measurement / value as f64 in <m> / end type`.

3. **Memory safety escape rule — `return pointer to param` incorrectly rejected** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix

   Added `AND is_parameter = 0` filter to `is_local_variable` query in `sqlite_symbol_management.rs:2154`. Parameters are no longer treated as locals — `return pointer to param` is now correctly allowed, matching the documented safety model.

   #### Tests

   `test_return_pointer_to_parameter_ref_is_ok` (pointer param), `test_return_pointer_to_param_of_i32_is_ok` (i32 param).

4. **Memory safety escape rule — pointer-to-local stored in global never checked** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix

   Removed `#[allow(dead_code)]` from `check_escaping_pointer_to_global` in `analyzer.rs:969`. Wired the call into `visit_assign` after variable validation — now checks every assignment for pointer-to-local values being stored into global variables.

   #### Tests

   `test_assign_pointer_to_local_to_global_is_error` (rejected), `test_assign_pointer_to_param_to_global_is_ok` (allowed — parameters are safe).

5. **No encapsulation for type fields (missing `private` / visibility modifiers)** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix

   Added `private` keyword to `type_field` in grammar (`lale.pest` line 425).
   - Grammar: `type_field` now accepts `(private ~ ms_ln)?` prefix
   - AST: `TypeField.is_private: bool` field added to `definitions.rs`
   - Builder: `build_type_field` parses `Rule::private` and sets `is_private`
   - SQLite: `type_def_fields.is_private BOOLEAN` column added; `TypeDefInfo` extended with `module_path`
   - Semantic analysis: `visit_member_access` and `visit_assign` check module boundary —
     accessing a private field from outside the defining module produces an error:
     `"Cannot access private field 'X' of type 'Y' from outside its defining module 'Z'"`
   - Core types (`str`) registered with `is_private = false` for `ptr` and `len` fields
   - Tests: 4 grammar tests + 3 semantic tests verifying parsing, DB storage, and module-path tracking

### 🟠 High Priority: Features

1. **"No chained type conversions" claim not enforced**

   #### Status

   ✅ Resolved (July 2026)

   #### Resolution

   Removed the ban. Chained conversions are allowed — they follow Lale's "explicit over implicit" principle. Updated `lale.md`, `ARCHITECTURE.md`, and test comments.

2. **Runtime bounds checking missing on array writes**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Added `BoundsCheck` instructions in `compute_array_offset` (`ir_gen.rs`). Now emits bounds checks per dimension for array writes and compound assignment array writes.

3. **Additional stdlib functions**

   #### Status

   🔴 Not started

   - Includes: math functions, string manipulation, parse functions (`parse_float`, `parse_int`,
     `parse_uint`) to complement the `read` statement redesign.

4. **Duplicate `use` import detection for stdlib modules** ✅ Resolved July 2026

   #### Status

   ✅ Resolved

   #### Source

   `src/semantic_analysis/analyzer.rs` — `visit_use()` (line 1357)

   #### Severity

   Medium — silently accepts redundant imports without warning

   #### Problem

   When both `use std` and `use std -> file_io_posix: openFile, readFile` appear in the same file, the semantic analyzer does not warn that the symbols from the second import may already be available through the first.

   #### Fix

   Added `seen_use_std` flag and `seen_std_submodules` set to the analyzer. Warns when `use std -> submod: ...` is used after `use std`, and also warns about earlier selective imports when `use std` appears after them. Both directions covered.

5. **Debugging support**

   #### Status

   🟡 Partial — call stack on runtime errors ✅

   - Source-level debugging, breakpoints, full stack traces.

   #### Done

   When a runtime error occurs (division by zero, bounds check, etc.), the interpreter now prints a Lale-level call stack showing function names in order, most recent first. Implemented via thread-local `CALL_STACK` in `src/interpreter.rs`.

6. **Vector types (`vec2 of T`, `vec3 of T`, `vec4 of T`) with dot/cross product**

   #### Status

   🟡 Partial (grammar, AST, semantic analysis, IR types, interpreter done — see `ARCHITECTURE.md` §5.20)

   #### Remaining

   — nested vectors / matrices: The grammar accepts recursive `vecN of vecM of T` but `TypeName.inner_type` only stores one level of nesting. To support matrices: change `inner_type` to `Option<Box<TypeName>>` (recursive), update type string formatting, update `parse_vec_type()`, and update IR lowering.

7. **Built‑in test framework (`test suite` / `test case`)** — ✅ Implemented August 2026

   #### Status

   ✅ Implemented (August 2026). The interpreter path is complete: grammar, AST,
   semantic validation, IR (`TestBegin`/`TestFail`/`TestEnd`), `#mode` gating,
   the `lale test` subcommand, and per-case pass/fail reporting. Coverage lives
   in `tests/test_suite_tests.rs` (grammar, semantic, integration, crash
   regressions). The AOT path below remains planned.

   ### Additions (August 2026, second pass)
   - `lale test --filter <pattern>` — runs only suites/cases matching any of the
     given (repeatable) patterns. A pattern containing `/` (same separator as
     the `Pass: suite / case` output) matches `suite/case`; otherwise it
     matches a substring of the suite name or the case name. Filtering happens
     at IR generation (semantic analysis still validates everything).
   - Heap leaks fail the run — a `HEAP LEAK` report forces a non-zero exit in
     test mode (plain `lale run` still reports but exits 0).
   - Fixed the `write` of a numeric/bool/char variable leaking its conversion
     temp string: the generated free is now skipped only for str-typed variable
     references, not for every variable reference.
   - Comment/doc attachment is now uniform across all statements via a shared
     `statement_comments` grammar rule (`(info ~ NEWLINE)* ~ os`): directly-
     attached comment lines become `comments.leading`, the trailing `os` handles
     indented nested statements, and blank-line-separated comments stay
     standalone. Standalone comments in a test suite are preserved in source
     order as `TestSuiteItem::Comment`/`Doc` nodes between cases.
   - Suite-level scope. A `test suite` may declare `var` and `fn` before its
     first `test case`. Suite variables are shared by every case and reachable
     from suite functions but invisible to non-test code and other suites; suite
     functions are suite-qualified (`suite::fn`) and callable from cases.
     Module-level (global) variables are **invisible** inside suites/cases, while
     module-level functions remain callable. `setup`/`teardown` are a convention
     only — there are no auto-run fixture hooks.

   #### Design

   Test suites live inline in any `.lale` source file. The compiler enforces a tripartite file structure:

   ```text
   program.lale
   │
   ├─ use ...          (first — required)
   ├─ ...definitions...  (types, enums, functions, variables, logic)
   ├─ test suite ...   (last  — optional)
   ```

   `test suite` must appear after all other top‑level statements. A test suite requires at least one test case. No nested suites or cases.

   `test suite` and `test case` are **not** `#`-prefixed (they are not compile‑time statements). The `#` prefix in Lale means "resolved during compilation — not present in IR output" (e.g., `#if` strips the untaken branch from the IR). Test code IS present in the IR — both branches compile, and the mode flag selects at runtime (or AOT optimize‑time). Changing mode does not require re‑compilation.

   #### Compilation

   Both `lale run` and `lale test` compile the full file (all `use`, `type`, `enum`, `fn`, `var` including initializers). The IR module is identical regardless of mode. The compiler emits a compile‑time constant (`#mode`) that the IR uses to gate execution:

   | Statement type                          | `lale run`                | `lale test`                |
   | --------------------------------------- | ------------------------- | -------------------------- |
   | `use`, `type`, `enum`, `fn`, `var` init | Run normally              | Run normally               |
   | Top‑level `write`, `loop`, `if`, etc.   | Execute                   | Skipped                    |
   | `test suite` / `test case` blocks       | Skipped                   | Execute                    |
   | `assert` inside a test case             | As in `lale run` (aborts) | Reports failure, continues |

   #### AOT backend

   The same approach extends to AOT compilation. The backend emits a mode flag (a compile‑time constant) and wraps both execution paths in a conditional:

   ```c
   // Generated by AOT backend
   int main() {
       if (LALE_MODE == LALE_MODE_TEST) {
           run_test_suites();
       } else {
           run_user_code();
       }
       return 0;
   }
   ```

   `lale build` sets `LALE_MODE = LALE_MODE_RUN`; `lale build --test` sets `LALE_MODE = LALE_MODE_TEST`. The optimizer sees a constant branch and dead‑code‑eliminates the unused path entirely. The resulting binary for `lale build` contains zero test code — not because a linker stripped it, but because the compiler removed the unreachable branch before linking. Same IR, two binaries, no runtime overhead.

   #### Syntax

   Suite and case names are **identifiers** (not string literals); underscores
   are allowed:

   ```lale
   test suite MatrixOperations
       var tolerance as f64 = 1.0e-6

       fn nearly_equal(a as f64, b as f64) returns bool
           return (a - b) < tolerance
       end fn

       test case Multiplication
           var a as f64[2][2] = [[1.0, 2.0], [3.0, 4.0]]
           assert a == expected
       end test case

       test case Inversion
           assert nearly_equal(1.0, 1.0 + tolerance)
       end test case
   end test suite
   ```

   #### Execution
   - `lale test example.lale` — runs only test suites from the named file (plus transitive imports loaded via `use`).
   - Test suites from imported modules execute first (in `use` order), then the main file's suites.
   - Output: `Pass: <suite> / <case>`, or `Fail: <suite> / <case>` plus a
     `(file:line:col)` location. A failing equality `assert a == b` additionally
     reports `expected <b>, found <a>`.
   - Exit code: 0 if all pass, non‑zero if any fail.

   #### Implementation checklist
   - [x] Grammar: `test_suite`, `test_case` rules in `lale.pest` (names are identifiers)
   - [x] AST: `TestSuiteStmt`, `TestCaseStmt` definitions, `build_test_suite` / `build_test_case` in builder
   - [x] Semantic analysis: validate position (must be last), validate no nesting, reject duplicate suite/case names, function-like case-local scope (case variables invisible outside the case), a suite-level scope for shared `var`/`fn` declarations (invisible outside the suite), and module globals invisible inside suites/cases (module functions remain callable)
   - [x] IR generation: compile all definitions; emit `#mode` compile‑time constant; gate test/user code behind mode-guard blocks (`begin_mode_guard` / `end_mode_guard`)
   - [ ] AOT backend: emit mode flag as C preprocessor constant; optimizer dead‑code‑eliminates the unused branch — **planned**
   - [x] CLI: `lale test` subcommand; `lale run` mode skips test blocks
   - [x] CLI: `lale test --filter <pattern>` runs only matching suites/cases
   - [x] Interpreter: heap leaks force a non-zero exit in test mode
   - [ ] CLI: `lale build --test` sets test mode — **planned (AOT)**
   - [x] Interpreter: test mode flag; `assert` in test mode reports + continues
   - [x] Tests: grammar tests, semantic tests (position, nesting, duplicates, scope), integration tests (pass/fail output, mode gating, crash regressions) — `tests/test_suite_tests.rs`
   - [x] `lale.md` and `ARCHITECTURE.md` updated (August 2026)

8. **User-specified numeric formatting**

   #### Status

   🔴 Not started — syntax deferred, design discussion open

   #### Background

   The default float formatting now follows the mainstream convention (shortest round-trip, plain decimal for human magnitudes, scientific notation for extremes — see `__lale_f64_to_str` in `builtins.lale`). Users also need explicit control over precision, padding, and notation for reporting and data export.

   #### Design question

   Lale already uses `{expr}` for string embedding. Candidates for the formatting syntax:

   - `printf`-style suffix: `{E_k:2f}`, `{E_k:e}`
   - Named specifier: `{E_k to 2 places}`, `{E_k as scientific}`
   - A `format` function: `format(E_k, "0.00")`

   The choice must respect Lale's "almost no punctuation" stance and the "built-in, not bolted-on" principle.

   #### Requirements
   - Fixed decimal places, significant digits, scientific/plain notation, padding
   - Works in string embedding and standalone conversion
   - Consistent between interpreter and future AOT backends

9. **`on exit` — keep as single-statement deferral (block-scoped proposal withdrawn)**

   #### Status

   ✅ Decided (August 2026) — no change needed

   #### Why no block-scoped form

   An earlier proposal converted `on exit` to a block-scoped `on exit … end on` closure for consistency with a future `on` event family (`on error`, `on signal`, `on event`). That family has been withdrawn in favour of `event` / `handle` keywords (see Post 1.0.0 Candidates).

   The block form's only justification was consistency with that family. With it gone, converting `on exit` would add closure/capture/scope complexity for no benefit and would work against Lale's "explicit / easy to understand" principle.

   `on exit` stays a minimal single-statement deferral (like Zig's `defer`), expanded at compile time with zero runtime overhead.

   If a multi-statement cleanup becomes necessary before 1.0.0, revisit it as a standalone decision — not as part of an `on …` family.

10. **Derived SI units — aliasing (scale-1) but no auto-shortening**

#### Status

🔴 Not started — design decided (August 2026)

#### Decision (two parts)

1. **Semantic aliasing — YES, scale-1 SI derived units only.** Teach the unit
   normalizer a curated table of named SI derived units and expand each to its
   base-unit decomposition, so the analyzer treats `J`, `kg⋅m²/s²`, and `N⋅m`
   as the same quantity. This is a pure name→base-vector lookup; the
   `NormalizedUnit` representation (`BTreeMap<String, i64>`) does not change.
2. **Display shortening — NO.** Keep the canonical base-unit output
   (`kg⋅m²/s²`), which is deterministic, unambiguous, and lossless. Do not
   auto-rewrite a unit into a shorter name.

#### Aliasing table (scale-1 SI derived units)

| Name             | Expands to       |
| ---------------- | ---------------- |
| `N` (newton)     | `kg⋅m⋅s⁻²`       |
| `J` (joule)      | `kg⋅m²⋅s⁻²`      |
| `W` (watt)       | `kg⋅m²⋅s⁻³`      |
| `Pa` (pascal)    | `kg⋅m⁻¹⋅s⁻²`     |
| `Hz` (hertz)     | `s⁻¹`            |
| `V` (volt)       | `kg⋅m²⋅s⁻³⋅A⁻¹`  |
| `Ω` (ohm)        | `kg⋅m²⋅s⁻³⋅A⁻²`  |
| `C` (coulomb)    | `A⋅s`            |
| `F` (farad)      | `kg⁻¹⋅m⁻²⋅s⁴⋅A²` |
| `S` (siemens)    | `kg⁻¹⋅m⁻²⋅s³⋅A²` |
| `Wb` (weber)     | `kg⋅m²⋅s⁻²⋅A⁻¹`  |
| `T` (tesla)      | `kg⋅s⁻²⋅A⁻¹`     |
| `H` (henry)      | `kg⋅m²⋅s⁻²⋅A⁻²`  |
| `lm` (lumen)     | `cd`             |
| `lx` (lux)       | `cd⋅m⁻²`         |
| `Bq` (becquerel) | `s⁻¹`            |
| `Gy` (gray)      | `m²⋅s⁻²`         |
| `Sv` (sievert)   | `m²⋅s⁻²`         |
| `kat` (katal)    | `mol⋅s⁻¹`        |
| `sr` (steradian) | `rad²`           |

Angle is a **distinct dimension**, not unitless. `rad` is kept as an opaque
base-like angle unit (an 8th base dimension beyond the seven SI base units) and
is deliberately **not** aliased to `1`. This way rad and degree cannot be mixed up.
`sr` is the scale-1 square of that dimension (`sr ≡ rad²`).
This preserves strict correctness: an angle can no
longer be silently mixed with a plain number (`sin(5)` is rejected; `sin(5
   <rad>)` is required).

Combinations like `N⋅m`, `W⋅s`, `V⋅A`, and `J/m³` need **no** table entry —
they normalize automatically once their constituent named units are recognized
(`N⋅m` → `kg⋅m²⋅s⁻²` ≡ `J`).

#### Explicitly excluded (require a magnitude/offset-aware model — post 1.0)

`L`, `Å`, `eV`, `bar`, `atm`, `cal`, `min`, `hr`, `°C`, `°F`, `deg`, `grad`,
`arcmin`, `arcsec`, … — these have non-1 scale factors or offsets. The current
`NormalizedUnit` is dimension-only (exponents), so equating them with base
units would be silently wrong (e.g. `L` ≠ `m³`; it is `10⁻³·m³`). Do not alias
them until the unit model carries a magnitude field. The angle units `deg`,
`grad`, `arcmin`, and `arcsec` share the `rad` angle dimension but at a scale
factor (e.g. `deg` = π/180 · `rad`), so they too are deferred.

#### Why no auto-shortening (the ambiguity problem)

A dimension can have several valid names with different physical meaning.
Auto-picking "the shortest" would silently rewrite the programmer's intent:

| Dimension    | Could display as | But they mean different things                                        |
| ------------ | ---------------- | --------------------------------------------------------------------- |
| `s⁻¹`        | `Hz`             | frequency — but also `Bq` (radioactivity), `rad/s` (angular velocity) |
| `kg⋅m²⋅s⁻²`  | `J`              | energy — but also `N⋅m` (torque), `W⋅s`                               |
| `kg⋅m⁻¹⋅s⁻²` | `Pa`             | pressure — but also `N/m²`, `J/m³`                                    |

Canonical base-unit display preserves intent; shortening would be lossy and
implicit. If a short form is ever wanted, make it an explicit formatting
request (e.g. `{#unit of x as J}`) — a separate, later feature.

#### Documentation

- Update `doc/lale.md` § "Unit Normalization" with the alias table, the
  scale-1 vs. magnitude distinction, and the no-auto-shortening rationale
  (including the ambiguity table above).
- Update `doc/ARCHITECTURE.md` § 4.5.6 / § 5.13 to document the derived-unit
  expansion table and the canonical-display decision.

#### Implementation checklist

- [ ] Add a `DERIVED_UNITS` lookup (name → base-vector) in `src/types/mod.rs`
- [ ] Expand recognized named units during `NormalizedUnit::parse`
- [ ] Keep `format_unit` in canonical base-unit form (no shortening)
- [ ] Keep `rad` as a distinct angle dimension (no alias to `1`); alias `sr → rad²`
- [ ] Tests: `J ≡ kg⋅m²/s² ≡ N⋅m`, `W ≡ V⋅A`, `Hz ≡ s⁻¹` (alias), `sr ≡ rad²`,
      and that `L`, `°C`, `eV`, `deg`, `arcmin` are NOT aliased
- [ ] Docs updated as above

11. **Runtime `switch` should support arbitrary value types (currently enum-only)**

#### Status

✅ Implemented (August 2026) — runtime `switch` now dispatches on any comparable
value type (integers, floats, strings, bools, chars), not just enums.

#### Discrepancy (resolved)

`doc/lale.md` § "Switch / Case — Comparing a Value" and
`doc/ARCHITECTURE.md` § 5.24 / § 6.8 describe `switch` comparing a value
against literal cases for open types (strings, integers) with a required
`default`. The grammar now accepts literal patterns, and the semantic analyzer
type- and unit-checks each arm against the scrutinee.

#### Future direction

`switch` shall dispatch on any value type, not just enums. An arm is valid
when its value's type and physical unit match the scrutinee's type and unit.
Duplicate case values are a compile error (dead code), matching the
compile-time `#switch`.

12. **Narrow integer widths are not enforced in the main memory model**

#### Status

🔴 Not started — pre-existing correctness gap

#### Problem

The interpreter's SSA registers hold integers as `Value::Int(i64)` /
`Value::Uint(u64)` regardless of declared width. The main `Store`/`Load`
instructions use the field-based `MemoryManager`, which does not truncate
to `i8`/`i16`/`i32`. Width-aware truncation only exists in the FFI
`v3_store`/`v3_load` path.

Result: under `--unchecked-overflow`, `127 as i8 + 1` prints `128` instead
of wrapping to `-128`. The addition wraps at 64 bits in the register and is
never truncated at the declared width.

#### Fix

Make the main data path width-aware. Either route `Store`/`Load` through
`v3_store`/`v3_load` (raw bytes) or teach `MemoryManager::store`/`load` to
truncate/sign-extend using the value's declared `IrType`. This touches the
core memory representation and should be a dedicated pass.

#### Tests

- `i8`/`i16`/`i32` wrap boundaries under `--unchecked-overflow`.
- Global and function-local narrow variables.
- Struct fields and array elements of narrow integer type.

13. **Unary minus does not bind to variables (only literals / function calls)**

#### Status

🔴 Not started — pre-existing grammar gap

#### Problem

`-a` fails with `Expected operator, found qualified_identifier`. The
grammar's `unary` rule does not include `minus`; negation is only accepted
at the `primary` level for `-literal` and `-fn_call(...)`. Negating a
variable or arbitrary expression is therefore impossible.

#### Fix

Add `minus` to the `unary` prefix rule (or the equivalent Pratt prefix
handling) so `-` binds to any expression with correct precedence, then
confirm the existing `generate_unary_op` / `checked_neg` path is exercised.

#### Tests

- `-x` where `x` is a variable.
- `-(a + b)`.
- Negating `i8::MIN` / `i64::MIN` traps under checked arithmetic.

> **Already covered** by Lale's existing design: implicit signed/unsigned
> mixing is forbidden (strict type matching, no implicit conversions), and
> pointer arithmetic is restricted to `ptr ± u64` and `ptr − ptr` (no
> `ptr * ptr`). No new work is needed for those two review points.

14. **Float NaN/Infinity trapping and exact arithmetic**

#### Status

🔴 Not started — mathematical-safety direction under consideration

#### Problem

IEEE-754 floats allow silent `NaN`/`±Inf` to propagate through long
calculations (`0.0/0.0` → `NaN`, `x/0.0` → `±Inf`), and `NaN == NaN` is
false. This breaks the reflexive property of equality and silently corrupts
downstream results.

#### Fix

Trap on operations that produce `NaN`/`±Inf`, and make float equality total
(or provide an explicit non-reflexive comparison). Consider a future
`Decimal`/`Rational` type for exact financial/rational arithmetic.

#### Tests

- `0.0/0.0`, `1.0/0.0`, and overflow-to-infinity trap.
- `NaN == NaN` is `true` (or an explicit alternative is provided).

15. **Euclidean remainder for negative operands**

#### Status

🔴 Not started — design decision

#### Problem

`%` currently uses Rust's truncating remainder, so `-5 % 3` yields `-2`.
Mathematically (Euclidean division), the remainder should be non-negative:
`-5 % 3 == 1`. Truncating semantics are surprising for number theory and
periodic logic.

#### Fix

Switch `%` to Euclidean remainder (`r >= 0`), document the change, and update
the interpreter `Rem` handler and any stdlib uses that depend on sign.

#### Tests

- `-5 % 3 == 1`, `5 % -3 == 2`, `5 % 3 == 2`, `-5 % -3 == 1`.

16. **Integer power returns integer and traps on negative exponent**

#### Status

🔴 Not started

#### Problem

`^` is already exponentiation (not XOR), but `Pow` always produces an
`f64`, so `2 ^ 3` yields `8.0`. A mathematically oriented language should
return an integer for an integer base and non-negative integer exponent, and
trap on a negative exponent for integer bases (which would otherwise be a
fraction).

#### Fix

Add integer fast-path semantics to `Pow` (or a distinct integer-power path in
IR gen), and reject/trap negative exponents on integer operands.

#### Tests

- `2 ^ 3` has integer type and value `8`.
- `2 ^ -1` traps for integer base.

17. **`abs` of a signed minimum must trap or widen**

#### Status

🔴 Not started — correctness

#### Problem

In two's complement, `abs(i8::MIN)`/`abs(i64::MIN)` cannot be represented in
the same type (max is one less than `-MIN`). Returning a negative value is a
mathematical disaster.

#### Fix

Trap on `abs` of a signed minimum, or return a wider type. Never return a
negative value.

#### Tests

- `abs(i8::MIN)` and `abs(i64::MIN)` trap (or widen correctly).

### 🟡 Medium Priority: Blocking Future Work

1. **Interpreter-only handlers bypass IR — block AOT consistency**

   #### Status

   🟡 In progress — 7 of 9 handlers resolved (see Resolution below). V3 memory migration: complete. All allocations on real heap via `AllocLog`. `MemoryManager` serves as typed‑value cache with regions at every address. No dual‑mode reads, no `unsafe` fallbacks. FFI reads raw bytes directly. `v3_store`/`v3_load` raw‑bytes bridge deferred (requires metadata system).

   #### Source

   `src/interpreter.rs` — `FuncRef::External` handlers in `execute_instruction_with_memory`

   #### Audit date

   August 2026

   #### Resolution (Aug 2026)

   | Handler                           | Status                 | Resolution                                                                                                                                           |
   | --------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
   | `str` constructor                 | ✅ Removed             | Dead code — `generate_fn_call_expr` short-circuits to `BuildStruct` IR instruction                                                                   |
   | `ptr_difference`                  | ✅ Moved to IR         | New `Instruction::PtrDiff` + `IrBuilder::ptr_diff()` replaces `call_named`                                                                           |
   | `__lale_pow_f64_f64`              | ✅ Replaced            | Removed handler; added `pow` extern (thin f64::powf wrapper)                                                                                         |
   | `__lale_write_stdout_pointer_i64` | ✅ Merged into `write` | All 4 I/O handlers removed — calls now go through `write(fd, ptr, len)` extern (fd=1 stdout, fd=2 stderr). `lale_put_str_impl` (~110 lines) deleted. |
   | `__lale_write_stderr_pointer_i64` | ✅ Merged into `write` | (same)                                                                                                                                               |
   | `__lale_error_pointer_i64`        | ✅ Merged into `write` | Error output now calls `write(2, ptr, len)`                                                                                                          |
   | `__lale_alert_pointer_i64`        | ✅ Merged into `write` | Alert output now calls `write(2, ptr, len)`                                                                                                          |

   ##### Remaining

   (3 items):

   | Item                                  | Description                                                                                                                                                                                                                                                                      |
   | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
   | `v3_store`/`v3_load` raw‑bytes bridge | Store/Load serialise `Value` ↔ bytes. Helpers written but integration blocked — needs a metadata system that distinguishes raw‑byte addresses from `Value`‑enum addresses.                                                                                                       |
   | `__lale_read_line`                    | ✅ Deferred (same as strto*). `TODO(AOT)` comment added. Stdin fd=0 support in `read` extern is done. Full stdlib implementation would require byte‑by‑byte reads + dynamic buffer growth — not practical in pure Lale today. When AOT exists, call libc getline/fgets directly. |
   | `strtod` / `strtol` / `strtoul`       | ✅ Deferred. `TODO(AOT)` comment added. Thin FFI wrappers — AOT would call libc directly.                                                                                                                                                                                        |

   #### Already Resolved

   (moved to builtins.lale IR):

   | Handler                                               | Resolution                                                                     |
   | ----------------------------------------------------- | ------------------------------------------------------------------------------ |
   | `__lale_f64_to_str_f64`                               | Removed Aug 2026 — now executes from builtins.lale IR (UnsafeCast→Bitcast fix) |
   | `__lale_i32_to_str` / `i64` / `u64` / `bool` / `char` | Removed July 2026 — moved to builtins.lale                                     |

   #### Platform Abstractions

   (same semantics in interpreter and AOT):
   `puts`, `__lale_exit`, `__lale_malloc_u64`/`malloc`, `__lale_free_pointer`/`free`,
   `open`, `read`, `write`, `close`. These wrap libc/POSIX with identical behavior across backends.
   The interpreter emulates the POSIX interface; AOT links against real libc — same result.

   ##### Will Crash

   (unimplemented):
   void-call catch-all (line 2896).

   `fread` is no longer part of the boundary — the buffered stdio family
   (`fopen`/`fread`/`fwrite`/`fclose`/`fseek`) is dropped by the single low-level
   FFI decision; buffered I/O is built in Lale. See `doc/ABI_SPECIFICATION.md` §4.

   ##### Impact

   The 3 remaining items are deferred until AOT: `v3_store`/`v3_load` (needs metadata system),
   `__lale_read_line` (complex to implement in pure Lale — byte‑by‑byte reads, dynamic buffer
   growth), and `strtod`/`strtol`/`strtoul` (thin FFI wrappers). All have `TODO(AOT)` comments.
   The `read` extern already supports fd=0 (stdin) — enough groundwork for AOT.

   ##### Resolution Order

   No further action for interpreter. When AOT exists: link all three against libc (getline, strto*).

2. **"No default numeric type" — bare literals not rejected**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Added `is_bare_numeric_literal` check in `visit_var_def` (`analyzer.rs`). When `var` has no `as type` annotation and the RHS is a bare numeric literal, emits: _"No type guidance. Use 'var x as type = value' or 'var x = value as type'."_

3. **Unsigned arithmetic underflow detection too aggressive**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Corrected the conflicting doc example in `lale.md` and `AGENTS.md`. Both now show that only literal subtraction is checked at compile time; variable subtraction is always rejected.

4. **Symbol table oddities** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Issues Resolved
   1. `str` now shows as `TypeDef` instead of `Function` with `→ __lale_type_def` sentinel. Added `SymbolKind::TypeDef` variant to properly separate type definitions from functions. Removed the `__lale_type_def` sentinel hack.
   2. Parameters already correctly distinguished from variables (confirmed as pre-existing fix — `is_parameter` column and `SymbolKind::Parameter` are used).
   3. `lookup_type()` now properly queries and returns `start_pos`/`end_pos` from the `type_defs` table instead of hardcoding zeros. Added `start_pos`/`end_pos` columns to the `type_defs` CREATE TABLE.
   4. Schema normalization reviewed — the `variables` table's denormalization (storing globals, locals, and parameters together) is acceptable technical debt for query simplicity.

5. **Interpreter bypasses `__lale_error` extern — not user-replaceable**

   #### Status

   🟡 Partial (stdlib and IR signatures fixed, interpreter still hardcoded)

   #### Remaining

   Refactor `lale_error_and_abort()` to format the message, call `__lale_error_pointer_i64` extern, then abort/exit.

6. **Unit normalization — Unicode symbol gaps in `NormalizedUnit::parse()`** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix

   Added `⁄` (U+2044 FRACTION SLASH) and `∕` (U+2215 DIVISION SLASH) as recognized division operators in `NormalizedUnit::parse()` and `parse_term()`. Removed `⅟` (U+215F) from grammar `div_sign` — it's a fraction numerator, not a division operator. All four division symbols now produce equivalent normalizations.

   #### Tests

   4 new tests (`test_parse_fraction_slash`, `test_parse_division_slash`, `test_parse_fraction_slash_compound`, `test_all_division_symbols_equivalent`). to `parse()` and `parse_term()`.

### 🟡 Control Flow Simplification (from proposal.md) — ✅ Implemented August 2026

Based on `doc/proposal.md` (2026-08-04). All phases implemented. Only tests remain.

#### Phase A — New Statements

- [x] **A1. Grammar**: `when_stmt`, `move_on_stmt`, `missing_code_stmt` rules in `lale.pest`
- [x] **A2. AST Builder**: `build_when`, `build_move_on`, `build_missing_code` in `builder.rs`
- [x] **A3. AST Definitions**: `WhenStmt`, `MoveOnStmt`, `MissingCodeStmt` in `definitions.rs`
- [x] **A4. Semantic Analysis**: `visit_when`, `visit_move_on`, `visit_missing_code` in `analyzer.rs`.
      `missing code` emits compile error in release mode.
- [x] **A5. IR Generation**: `generate_when` (condbr), `MoveOnStmt` (no-op),
      `generate_missing_code_warning` (runtime warning in debug) in `ir_gen.rs`
- [x] **A6. Interpreter**: Uses existing condbr/move-on/missing-code as no-ops.
- [x] **A7. Tests**: `when` with true/false, `move on` in `if`/`else` branches,
      `missing code` debug vs release.

#### Phase B — Rename and Upgrade

- [x] **B1. Grammar**: `match_stmt` with `when` arms + `else`, `switch_stmt` with
      `case` + `default`, `SwitchPattern` enum in `lale.pest`
- [x] **B2. AST Builder**: `build_match`, `build_switch`, `build_switch_case`,
      `build_switch_pattern`, `build_match_arm` in `builder.rs`
- [x] **B3. AST Definitions**: `MatchStmt` (+`MatchArm`), `SwitchStmt` (+`SwitchCase`,
      `SwitchPattern`) in `definitions.rs`. Old `BranchStmt` removed.
- [x] **B4. Semantic Analysis**: `visit_switch` with exhaustiveness checking for
      enums, `visit_match` in `analyzer.rs`
- [x] **B5. IR Generation**: `generate_switch` (discriminant + IfElse chain),
      `generate_match` (ordered condbr chain) in `ir_gen.rs`
- [x] **B6. Tests**: `match` with 3+ `when` arms, `switch` on enum (all variants
      covered), `switch` on string with `default`, exhaustiveness errors.

#### Documentation

- [x] Proposal (`doc/proposal.md`)
- [x] User guide (`doc/lale.md` § Control Flow)
- [x] Architecture (`doc/ARCHITECTURE.md` § 4.3, § 5.24)
- [x] Editor syntax (Zed + VS Code + LSP)

#### Compile-Time Counterparts — ⚠️ Not Yet Implemented

The runtime-only constructs were renamed/added but their compile-time (`#`-prefixed)
counterparts were not. Currently only `#if` / `#else if` / `#else` / `#end if`
exist (`CtIfStmt`). Missing:

- [ ] `#when` — compile-time one-sided guard
- [ ] `#match` / `#when` — compile-time ordered condition chain
- [ ] `#switch` / `#case` — compile-time value dispatch

### 🟡 Programmer Silent Errors — from `doc/Lale Programmer Silent Errors.md`

The following are potential solutions for silent errors a Lale programmer can
write that compile and run without complaint but produce wrong results.
See [Lale Programmer Silent Errors](Lale%20Programmer%20Silent%20Errors.md) for the full analysis.

- [ ] **Consider named arguments for type constructors** — same-typed field swaps could be prevented by allowing `Rect(width = w, height = h)` syntax. This would make order irrelevant when names are provided, while still allowing positional syntax for brevity when order is obvious.
- [ ] **Consider runtime integer overflow detection** — signed overflow and unsigned `+`/`*` overflow currently wrap silently. A debug-mode overflow check (similar to Rust's `overflow-checks = true`) or a checked-arithmetic mode could catch these.
- [ ] **Consider `pointer to` behind `unsafe`** — pointer arithmetic out of bounds is currently a footgun. Making `pointer to` require `unsafe` would be a language-level acknowledgment of the risk, but would also make legitimate pointer usage more verbose.
- [x] **Implement memory management** — ✅ Resolved (August 2026). `allocate`/`release` keywords and `on exit` provide explicit heap control. The compiler manages `str` allocations automatically.

  #### Problem 1: `str` — Language-Created Allocations

  ✅ Resolved (August 2026)

  String embedding (`"x = {x}"`), type-to-string conversions, and concatenation all allocate behind the scenes. The compiler manages `str` allocations automatically — the user never writes `release` for a string.

  - [x] **`str` auto-free at scope exit** — compiler tracks `str`-typed locals and frees heap data at scope exit. Parameters, returned values, and globals are skipped (ownership transfers).
  - [x] **Inline `str` free in output statements** — `write`/`warn`/`debug`/`alert` free string data after output. `heap_dealloc` is idempotent — double-free from auto-free is harmless.
  - [x] **Builtins allocate exact-size buffers** — `u64_to_str`, `i64_to_str`, `char_to_str`, `f64_to_str` return `str(buf, len)` directly without double allocation.
  - [x] **Leak detector with source tracking** — `MemoryManager` tracks `heap_alloc`/`heap_dealloc` and reports leaked allocations with file/line/column at program exit in debug builds. Every test run is a leak test.
  - [x] **Static global string data** — global `str` data is allocated via `alloc_static_string` (static address pool, not `AllocLog`) and lives for the program's lifetime, matching an AOT backend's `.rodata`/`.data`. It is intentionally never reported as a leak.
  - [x] **Enum helper `Ptr(str)` return leaks** — ✅ Fixed. Enum display/name helpers now return `str` by value (not `Ptr(str)`), and nested `str` fields (including enum variant payloads) are reclaimed via value-based recursion in `collect_str_allocas`.

  #### Problem 2: Raw Pointers — User-Initiated Allocations

  ✅ Resolved (August 2026)

  `allocate` and `release` are now first-class language keywords. `on exit` provides Zig-style deferred cleanup at scope exit. The programmer controls all raw pointer allocations — the compiler provides the tools.

  - [x] **`allocate`/`release` keywords** — first-class language constructs for heap memory allocation and deallocation (`var buf as pointer = allocate 1024; release buf`).
  - [x] **`on exit` statement** — `on exit release buf` defers execution to scope exit. Compile-time expansion (like Zig's `defer`), zero runtime overhead.
  - [ ] **Consider compile-time `allocate`/`release` pairing lint** — warn when `allocate` has no visible `release`/`on exit` in the same function.
  - [ ] **Consider arena/region-based allocation** — `arena` blocks for batch-allocate/batch-free patterns. Follow-up optimization.
  - [ ] **Consider resource types with destructors** — `type ... drop`. Only revisit if `on exit` boilerplate or future heap-allocating types justify it.

### 🟢 Low Priority

1. **Numeric-to-char range validation missing**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   Added Unicode range check (0x0..0x10FFFF) in `visit_conversion` for numeric-to-char conversions. Also added `HexLiteral` support to `try_extract_int_value` so hex literals benefit from all range checks.

2. **Write-to-identifier target variable never validated** ✅ Resolved July 2026

   #### Status

   ✅ Resolved

   #### Source

   Grammar/semantic review (July 2026), `src/semantic_analysis/analyzer.rs:2240-2242`

   #### Severity

   Low — parsed but not checked.

   #### Problem

   `write expr to var` is parsed into `StdoutStmt.target` but `visit_stdout` / `visit_stderr` only validated the expression value — they never checked that the target identifier refers to a declared variable of appropriate type.

   #### Fix

   Followed the `read` pattern — the AST builder now injects a synthetic `VarDefStmt` for `write ... to var` / `warn ... to var` targets. The variable is auto-defined as `str` (empty string initial value). Re-definition errors are caught by the existing `visit_var_def` through `register_variable_decl_with_pointer`. No explicit target validation needed in `visit_stdout`/`visit_stderr`. 7 tests in `tests/io_tests.rs`.

3. **LSP server — remaining features**

   #### Status

   🟡 Partial

   #### Source

   `lale-lsp/src/server.rs`

   #### Done

   diagnostics, completion, hover, semantic tokens, document symbols, go-to-definition (same-file), syntax highlighting for Zed and VS Code

   #### Remaining

   keyword list needs an update (test suite, test case)

   #### Go to Definition — Cross-File

Same-file works (AST walk of var/fn/type/enum/loop vars). Cross-file (e.g., jumping to stdlib definitions) needs symbol table integration.

#### Go to References

Not registered — needs full symbol usage tracking across files

#### Rename Symbol

Not registered — depends on references implementation

#### Code Actions

Not registered

- could suggest `as` conversions for type mismatches
- could help entering sub and superscript characters with some sort of escape sequenses

#### Formatting

Not registered — basic indentation and spacing rules

1. **Stdlib files double‑compiled when used as main entry** ✅ Resolved August 2026

   #### Status

   ✅ Resolved

   #### Fix

   `src/main.rs` — `compile_and_execute()` now checks if the source file path is inside `stdlib/src/` via canonicalization. When true, `load_and_add_stdlib` is skipped — the file is compiled as regular user code, not double-compiled through the internal stdlib path.

   #### Tests

   `lale run stdlib/src/core.lale` compiles stdlib once, shows warnings/errors directly.

2. **Little-endian canonical ABI documented but not enforced in interpreter**

   #### Status

   🟢 Not started

   #### Source

   `doc/ABI_SPECIFICATION.md` §2 / §5 and `src/interpreter.rs` `v3_store` / `v3_load`

   #### Severity

   Low — documentation/implementation discrepancy; latent on big-endian hosts only.

   #### Problem

   The ABI documentation says Lale is little-endian canonical, but
   `v3_store`/`v3_load` use host-native `write_unaligned`/`read_unaligned`
   instead of `to_le_bytes`/`from_le_bytes`. On little-endian hosts (x86/ARM)
   this is correct by accident; on a big-endian host it would silently produce
   the host's byte order.

   #### Fix

   Route raw-byte serialization through `to_le_bytes`/`from_le_bytes` (or add
   an explicit canonical-endianness path) once the `v3_store`/`v3_load` bridge
   is integrated. Add an ABI conformance test asserting little-endian byte order.

### 📝 Documentation

1. **Add array-with-units example to user guide**

   #### Status

   ✅ Resolved (July 2026)

   - Added **Arrays with physical units** section to `doc/lale.md` § Array Initialization with `in <unit>` annotation and `[fill with value]` examples.

---

### 📐 Code Quality

1. **Break up oversized source files** ✅ Resolved July 2026 — Navigation tables added

   #### Status

   🟢 Resolved (assessment changed)

   #### Assessment

   These four files are large for structural reasons, not organizational ones:

   - `ir_gen.rs` (4,300 lines): Single `impl IrGenerator` block with 97 methods — cannot be split across Rust modules.
   - `analyzer.rs` (4,100 lines): Single `AstVisitor` impl with 50+ visit methods — the visitor pattern requires one impl per type.
   - `interpreter.rs` (3,700 lines): Giant match statement over 74 `Instruction` variants — this is the correct architecture for a bytecode interpreter (same pattern as Lua's `lvm.c`, CPython's `ceval.c`).
   - `sqlite_symbol_management.rs` (3,300 lines): Single `impl SqliteSymbolManager` with tightly-coupled schema, CRUD, and cache methods.

   #### What Was Done

   Added `//! Quick navigation:` tables-of-contents at the top of each file, listing every major section with line numbers. These serve as the practical alternative to file splitting — they make each large file navigable without the maintenance cost of sub-module extraction.

2. **Fix unconventional lib.rs path** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix

   Moved library root from `src/grammar/mod.rs` to `src/lib.rs`. Replaced all `#[path = "..."]` module attributes with standard `mod` declarations. Updated `Cargo.toml` lib path.

3. **Rename type system modules for clarity**

   #### Status

   ✅ Resolved (July 2026)

   #### Fix

   `type_system.rs` → `type_compatibility.rs`, `type_validator.rs` → `type_conversion.rs`. Updated all imports in `analyzer.rs`, `ir_gen.rs`, `sqlite_symbol_management.rs`, and `mod.rs`.

   #### Source

   Code review (July 2026)

   #### Problem
   - `type_system.rs` sounds like it IS the type system but it is just type compatibility
   - `type_validator.rs` handles conversion validation but the name does not convey this

   #### Proposed fix

   Rename to `type_compatibility.rs` and `type_conversion.rs` to match actual responsibilities.

4. **Improve function-level documentation on interpreter and IR generator** ✅ Resolved July 2026

   #### Status

   🟢 Resolved

   #### Fix
   - `execute_instruction_with_memory`: Fixed stale "Execute a basic block" doc — now correctly describes it as the core instruction dispatch with SSA values, memory manager, and error stack.
   - `execute_module`: Added doc explaining entry point discovery, global initialization, and the immutable-module/mutable-state split.
   - `execute_block_with_return`: Added doc explaining sequential instruction iteration, recursive structured control flow (IfElse, Loop), and the ControlFlow + optional return value return model.
   - `try_generate_stmt`: Added doc describing the dispatcher pattern and error propagation.
   - `try_generate_var_def`: Added strategy summary (init expression → optional conversion → store to alloca/global).
   - `try_generate_assign`: Added strategy summary (RHS → bounds check → address lookup → Store + optional conversion).

5. **Review error reporting pipeline — AST build errors block semantic analysis**

   #### Source

   `src/main.rs:compile_and_execute`, `src/ast/expr_parser.rs:build_number_literal`

   #### Severity

   Medium — affects developer experience but not correctness

   #### Problem

   The compilation pipeline has three sequential error stages: (1) grammar parsing, (2) AST construction, (3) semantic analysis. Grammar errors accumulate (pest continues past errors). Semantic errors accumulate (analyzer collects all). But AST construction fails on the first error and exits immediately, preventing semantic analysis from ever running. For example, a literal-overflow error in an AST node blocks detection of a duplicate-variable error on an earlier line.

   #### Root Cause

   `build_program()` returns `Result<Program, String>` — a single error string, not a collection. The pipeline uses `?` which short-circuits. Unlike grammar errors (which pest continues past) and semantic errors (which the analyzer accumulates in a `Vec`), AST build errors cause an immediate pipeline abort.

   #### Proposed fix

   Refactor `build_program` (and all `build_*` functions) to collect errors into a `Vec<String>` while building as much of the AST as possible. Return a partial `Program` along with errors. Then `compile_and_execute` can report all AST errors AND continue to semantic analysis. This is a significant refactor (~50+ build functions).

   #### Interim Mitigation

   Keep the `build_number_literal` error format (file:line:col) consistent with grammar/semantic error formats for visual consistency. The error is displayed in black via Rust's default `Error` display — not ideal but functional.

6. **Systematic silent error fallbacks — 274+ occurrences across the codebase**

   #### Status

   ✅ Resolved (August 2026)

   #### Source

   Full manual audit in `doc/silent_errors_audit.md`

   #### Resolution

   All 21 HIGH + 57 MEDIUM findings fixed across 11 categories. Remaining 4 validator flags are false positives (CLI output + IO diagnostic). Final state: 0 HIGH, 0 MEDIUM, 0 LOW open.

   #### Details

   See `doc/silent_errors_audit.md` for the complete resolved audit.

7. **Support multiple trailing comments** (`AttachedComments.trailing` `Option` → `Vec`)

   #### Status

   🟡 Scheduled (August 2026) — follow-up to the statement/comment relationship work

   #### Problem

   `AttachedComments.trailing` is a single `Option<AttachedComment>`. That is
   enough for one inline trailing comment per statement, but it cannot represent
   multiple trailing comments. Standalone comments are already preserved as
   first-class nodes; multiple _trailing_ comments are the remaining gap.

   #### Proposed fix

   Change `trailing: Option<AttachedComment>` to `Vec<AttachedComment>`, update
   `extract_comments` to collect all trailing comments in source order, and
   update every `print_attached_comments` / consumer call site.

8. **Decide on one comment representation (attached vs. first-class)**

   #### Status

   🟡 Scheduled (August 2026) — design decision

   #### Problem

   Comments are currently represented two ways: attached metadata
   (`AttachedComments` on statements) and first-class nodes (`Stmt::Comment` /
   `Stmt::Doc`, and `TestSuiteItem::Comment` / `TestSuiteItem::Doc`). The model
   is coherent, but the split is implicit and should be a deliberate, documented
   decision — especially before the standard library grows.

   #### Decision to make
   1. Keep both (attached for adjacent comments, first-class for standalone).
   2. Collapse to a single first-class-node representation.

   #### Proposed fix

   Document the chosen model in `doc/ARCHITECTURE.md` §4.3 and enforce it
   uniformly across the builder, AST, and printer.

## Post 1.0.0 Candidates

These are long-term design directions — explicitly deferred until Lale has a
stable 1.0 grammar and a coherent answer to the open questions below.

### 1. `event` / `handle` — top-level event handlers (not an `on …` family)

Lale deliberately does **not** grow a family of `on …` keywords (`on error`,
`on signal`, `on event`). Two reasons:

- **`on` blurs the meaning of `when`.** `when` answers "is this condition true
  now?", whereas a reaction to something happening is a different, more implicit
  concept. Keeping them separate protects the "everything is explicit" principle.
- **`on …` handlers would want to live inside functions**, which forces
  closure/capture/scope semantics that conflict with Lale's two-scope rule
  (global + function-local).

Instead, event handling uses a dedicated **noun** (the event declaration) and
**verb** (the handler), both **top-level only** — never inside a function. This
avoids the capture/scope complexity entirely.

```lale
event ButtonClicked          // declaration
    button as Button
end event

handle ButtonClicked         // handling
    write "clicked"
end handle
```

`event` declares the shape of an event; `handle` registers a reaction. Because
handlers are top-level, there is no closure over a local scope — they behave like
ordinary void-returning entry points the runtime invokes.

#### Candidate extensions (post 1.0.0)

- `handle <signal>` — OS-signal handler (portability question remains).
- `handle <error>` — runtime-error handler (must reconcile with the error stack
  and optional types).
- `emit <Event>` — the future signal source that triggers `handle` blocks.

#### Open design questions (blockers)

- **Runtime model:** a signal/message source implies a scheduler or OS hook — a
  major runtime addition at odds with the current pure-interpreter + no-std hook
  model.
- **Top-level handler invocation:** how and when top-level `handle` blocks are
  registered and invoked must be defined.
- **Portability/no-std:** POSIX signals differ on Windows and are unavailable in
  no-std.
- **`emit` data:** whether events carry payloads, and how they are typed and
  validated, is unresolved.

The existing `on exit` statement is unchanged and is **not** part of this
vocabulary — it remains a minimal single-statement deferral (see TODO item 9).

### 2. Hardware interrupts — a separate concept, not `handle`

GPIO/timer/UART interrupts on microcontrollers are **not** folded into the
`event` / `handle` model. Software events and hardware interrupts differ enough
that unifying them would overload `handle` with two conflicting contracts:

|                        | `event` / `handle` (software)       | Hardware interrupt                        |
| ---------------------- | ----------------------------------- | ----------------------------------------- |
| **Source**             | Declared in Lale, emitted by Lale   | Hardware peripheral / CPU vector table    |
| **Data**               | Lale-typed payload                  | CPU registers, volatile MMIO, no heap     |
| **Calling convention** | Ordinary void-returning entry point | Special ABI (save/restore, `reti`/`iret`) |
| **Constraints**        | May call functions, allocate, block | Must not allocate, block, or call FFI     |
| **Portability**        | Language-level, portable            | CPU + SDK specific                        |

Mainstream systems languages treat ISRs as a _calling-convention / linkage_
concern (Rust's `#[interrupt]`, Zig's `interrupt` callconv, C's
`__attribute__((interrupt))`), not as an event abstraction.

**Direction:** if Lale targets microcontrollers, introduce a distinct
`interrupt` (or `isr`) declaration, constrained to ISR-safe operations and tied
to the AOT backend + a platform/SDK layer.

```lale
interrupt gpio_pin_5
    // ISR-safe body only
end interrupt
```

**Dependencies:** requires the platform-abstraction work (see "Platform
Abstractions" under Blocking Future Work) and the AOT backend. Blocked until
then.

### 3. Generics / unit polymorphism — candidate only, not promised

Lale has no generic types or functions. A routine must be written for a specific
numeric type and unit, so `sqrt`-style code is duplicated for `f32`, `f64`, and
each unit combination. For a scientific language this is the most visible
expressiveness gap.

**Status:** candidate only. This is **not** a 1.0 commitment, and there is no
promise it will ever ship.

#### Open design questions

- **Syntax.** Angle brackets (`T`) versus Lale's existing `as` / `in` vocabulary.
- **Unit polymorphism.** How a type parameter constrains the unit of its value
  (`f64 in <m>` vs. `f64 in <s>`).
- **Monomorphization vs. runtime dispatch**, and interaction with the SQLite
  symbol table and AOT backend.

### 4. First-class functions — candidate only, not promised

Functions cannot currently be passed as values, stored, or returned; there are
no closures or higher-order functions. This follows from the two-scope model and
the "no hidden control flow" principle.

**Status:** candidate only. This is **not** a 1.0 commitment, and there is no
promise it will ever ship. The `event` / `handle` design above deliberately
avoids closures; a first-class-function feature would have to reconcile with
that decision.

#### Open design questions

- **Function types.** How a function value is typed within `type_name`.
- **Capture semantics.** Whether and how a function value closes over locals,
  given the global + function-local two-scope rule.
- **Interaction** with `on exit`, `event`/`handle`, and C FFI function pointers.

### 5. Refinement types (range constraints) — candidate only, not promised

Refinement types would let a type carry a value-range constraint (e.g.
`i32<0..99>`), moving safety from "it is an integer" to "it is a value that
makes sense for this logic". An out-of-range assignment or arithmetic result
would trap (or be rejected at compile time when statically known).

**Status:** candidate only. This is **not** a 1.0 commitment, and there is no
promise it will ever ship.

#### Open design questions

- **Syntax.** `int<0..99>` vs. Lale's existing `as` / `in` vocabulary.
- **Static vs. runtime checking**, and how a range interacts with overflow
  trapping and unit analysis.
- **Interaction with arrays and indexing** (e.g. a range-typed index).

## Previous Issues (Resolved)

### `?` Operator — Optional Propagation (resolved June 2026)

#### Status

✅ Implemented

The `?` postfix operator propagates `nothing` upward: if the expression is absent, the enclosing
function returns `nothing`. At the top level, it prints an error and exits with code 1.

```lale
fn parse_and_double(input as str) returns f64?
    var val as f64 = parse_float(input)?  // if absent, returns nothing to caller
    return val * 2.0
end fn

// Top-level scripts also work:
var val as f64 = parse_float(input)?
```

- `?` is sugar for: `if absent → return nothing` (or `exit 1` at top level)
- Only valid in functions returning `T?` or at the top level
- Debug mode: full diagnostics via `__lale_error`
- Release mode (`--release`): brief message `error: function 'X' returned nothing`
- Equality is now `==`, so `?` propagation needs no `!"="` negative lookahead

### `#debug` Compile-Time Constant + `--release` CLI Flag (resolved June 2026)

#### Status

✅ Implemented

- `#debug` boolean constant — `true` by default, `false` with `--release`
- Usable in `#if #debug` for conditional compilation of debug-only code
- CLI flag `--release` sets `#debug = false` and enables optimisations
- Documented in CLI options and compiler constants table

### `ZeroCheck` IR Instruction — Div/Mod by Zero (resolved June 2026)

#### Status

✅ Implemented

Division and modulo by zero are now checked at the IR level with full source location info.
Previously these errors had no file/line/col information (`:0:0`). Now they report exact locations:

```text
ERROR at main.lale:42:18: division by zero (index=-1, length=0)
```

- New `ZeroCheck` IR instruction with operand, message, file, line, column
- Emitted by IR gen before `Div`/`Mod` operations
- Handled by both interpreter and future AOT backends
- Complemented by compile-time warning (see §Division-by-Zero Compile-Time Warning below)

### Runtime Hook Signature Cleanup (resolved June 2026)

#### Status

✅ Implemented

All stdlib runtime hooks now return the real POSIX `write()` result (`i64`), not hardcoded `0`:

- `__lale_write_stdout`: `returns i64` — returns `write(1, ptr, length)` directly
- `__lale_write_stderr`: `returns i64` — returns `write(2, ptr, length)` directly
- `__lale_error`: `returns i64` — returns `write(2, ptr, length)` directly
- IR extern return types fixed: `Void` → `I64` for all three
- `__lale_alert` hook added with ANSI red `"Error: "` prefix (same pattern)
- Architecture doc §4.8 updated to match actual 2-parameter signatures

### ANSI Coloring Moved to Standard Library (resolved June 2026)

#### Status

✅ Implemented

ANSI terminal coloring now lives in `stdlib/src/std.lale` as module-level globals, user-customizable:

- `ansi_red` / `ansi_reset` / `alert_prefix` — pre-allocated at compile time, zero runtime allocation
- `__lale_error` wraps messages in ANSI red + reset
- `__lale_alert` prefixes with red `"Error: "` + reset
- Removed hardcoded ANSI from `ir_gen.rs` `generate_alert`
- Interpreter keeps temporary ANSI coloring until refactored to call `__lale_error_pointer_i64` extern

### `T?` Lint Checks — Unguarded Unwrap + Unchecked Optional (resolved June 2026)

#### Status

✅ Implemented

Two new compile-time warnings for optional type safety:

1. **Unguarded `value of`**: warns when `value of x` is used without a visible `has value`/`has no value` check
2. **Unchecked `T?` variable**: warns when a `T?` variable is defined but never checked before leaving scope

- `optional_vars_in_scope` + `checked_optional_vars` tracking in `SemanticAnalyzer`
- `visit_has_value`/`visit_has_no_value` trait methods with default implementations
- Applies to both local variables and function parameters
- 7 tests: guarded/unguarded, checked/unchecked, parameter cases

### Grammar Fixes (resolved June 2026)

#### Status

✅ Implemented

- **Word boundary on inline keywords**: `write_inline`/`warn_inline`/`alert_inline` now use `!identifier_continue` negative lookahead to prevent `write inlinehello` from being parsed as `write_inline` + `hello` (was a PEG ordered-choice issue)
- **Statement ordering**: `error_statement` precedes `runtime_io_statement` so `write error messages` (error stack drain) matches before `write <expr>` (stdout output)
- **`--stdlib` → `--stdlib-level`**: CLI flag renamed to match documentation

### Documentation Cleanup (resolved June 2026)

#### Status

✅ Implemented

- Architecture doc §1: removed stale `future` from optional/result types description
- CLI options: `--no_std_lib` → `--stdlib-level=none` in both docs
- AOT commands (`lale exec`, `lale build`, `lale build-lib`) marked as **Planned (future)** throughout
- `#debug` added to compiler constants table
- `--release` CLI flag documented with examples
- `?` operator documented in Optional Types section and operators table
- File I/O section: "Runtime hooks for the AOT backend backend" → "for all backends (user-replaceable)"

### Interpreter String Type Coercion (resolved May 2026)

#### Status

✅ Resolved

The interpreter now correctly handles `Value::String` → `Value::Pointer` conversion
when extracting fields from `str` structs (see `ExtractField` handler in `src/interpreter.rs`).
File I/O functions (`openFile`, `readFile`, `writeFile`, `closeFile`) work correctly
in the interpreter backend.

### `read` Statement Redesign (resolved June 2026)

#### Status

✅ Resolved

The `read` statement now always reads into a `str` variable, following the Python/C model
where input is a string and type conversion is the programmer's responsibility:

```lale
var input as str = ""
read input
// Conversion: var val as f64 = parse_float(input)  (parse functions coming to stdlib)
```

- `read` requires a `str` target variable (semantic error otherwise)
- `__lale_read_line` interpreter built-in returns a `str` struct directly
- Parse functions (`parse_float`, `parse_int`, `parse_uint`) to be implemented in stdlib
- Semantic analyzer validates target existence and type at compile time

### Optional Types for Recoverable Error Handling

#### Status

✅ Implemented

Optional types (`T?`) are fully implemented using natural‑language constructs:

```lale
fn parse_float(input as str) returns f64?
    if input.len == 0
        return nothing          // → absent optional
    end if
    return strtod(input.ptr, 0 as pointer)  // → present optional
end fn

var val as f64? = parse_float(input)
if val has value
    m = value of val           // safe unwrap
else
    write "invalid input"
end if
```

#### Implemented

- `T?` type modifier in the grammar
- `return nothing` — produces absent optional in `T?` functions, no-op in void functions
- `return expr` — auto-wraps as present optional in `T?` functions
- `has value` / `has no value` — postfix operators returning `bool`
- `value of expr` — unwraps optional; runtime error (`__lale_error`) if absent
- `nothing` expression for initializing optional variables (`var v as f64? = nothing`)
- `UnwrapOptional` IR instruction with runtime safety check
- Struct-based representation: `{is_present: bool, value: T}`
- Updated stdlib: `parse_float`, `parse_int`, `parse_uint` return `T?`
- Old `some()`/`none()` syntax and `Some`/`None` IR instructions removed

### Escape Sequences for Strings (resolved June 2026)

#### Status

✅ Implemented

Support for `\n`, `\t`, `\r`, `\\`, `\"`, `\'` in string literals.
Applied in `build_string_literal` (`expr_parser.rs`) and `extract_string_content` (`builder.rs`).

### `write`/`read`/`warn` — Built-in or Stdlib? (resolved June 2026)

#### Status

✅ Resolved

Clarified that `write`, `warn`, `read`, `debug` are language statements (always available).
Stdlib provides additional functions.

### Architecture Doc: Non-Boolean `#if` Conditions (resolved June 2026)

#### Status

✅ Resolved

Added note in `doc/ARCHITECTURE.md` §5.5 that boolean requirement applies to both runtime and
compile-time conditions.

### Undefined Variable Fallback Eliminated (resolved July 2026)

#### Status

✅ Resolved

The IR generator (`src/ir_gen.rs`) no longer silently produces `const 0` for undefined
identifiers. Replaced with `CompileError` propagation via `try_generate_expr_with_resolved_type`.
The semantic analyzer already caught undefined variables at compile time; this closes the
remaining gap where a bug in semantic analysis could produce silently wrong IR.

### Clippy Warnings Cleaned (resolved July 2026)

#### Status

✅ Resolved

All clippy warnings resolved across the codebase.

### Code Quality Audit — All Critical Issues Resolved (resolved July 2026)

#### Status

✅ Resolved

Critical issues from code quality audit resolved.

### Identifier Grammar Refactoring (resolved July 2026)

#### Status

✅ Resolved

Refactored the identifier grammar:

- Renamed `base_identifier` → `single_identifier` (one name segment, no separators)
- Introduced `link = { "->" }` as the path separator (replaces `module_path_separator`)
- Introduced `qualified_identifier` for multi-segment paths joined by `link`
- Removed the old atomic `identifier` rule that mixed single names and paths
- Variable/function/type names now use `single_identifier` — `var my -> variable` is no longer valid
- Module paths, enum variant access, and expression identifiers use `qualified_identifier`

### Enum Definitions (resolved July 2026)

#### Status

✅ Resolved

Implemented Rust-style algebraic data types:

- `enum Name ... end enum` with variants that can carry typed data
- Variant access via `->` separator: `Shape -> Circle(3.14)` or unqualified: `Circle(3.14)`
- Auto-generated constructor functions for each variant
- Debug output formats enum values as `VariantName(val1, val2)`
- Internal discriminant field for runtime variant identification
- `branch` statement: implemented end-to-end (grammar, AST, builder, visitor, semantic analysis with exhaustiveness checking, IR generation via discriminant-based IfElse chain).

### Assert Statement (resolved July 2026)

#### Status

✅ Resolved

Implemented `assert condition`:

- Runtime assertion with source location in error output
- No-op in release mode (`--release`)
- Condition must be boolean (no implicit coercion)

### Division-by-Zero Compile-Time Warning (resolved July 2026)

#### Status

✅ Implemented

The semantic analyzer now performs path-sensitive compile-time detection of
potential division by zero. Guards (`if divisor != 0`, `if x > 0`, etc.) are
tracked through `if`/`else-if`/`else` branches via a guard stack, and structurally
identical expressions in divisors are matched against guard facts.

- Applies to `/`, `%`, `/=`, and `%=` operators
- Guard extraction uses interval arithmetic — works for any constant with any comparison operator
- Conversion and grouping wrappers are transparent (`x != 0` guards `x as f64`)
- Conservative: warns when safety cannot be proven; stays silent only when provably safe
- ~450 LOC in `src/semantic_analysis/analyzer.rs`
- 45 unit tests + 16 integration tests
- Documented in `ARCHITECTURE.md` §4.5.7a
- Warning message includes note about structural matching limitations

### `debug` Statement Release-Mode Behaviour (resolved July 2026)

#### Status

✅ Bug fix

`debug` was emitting code in release mode. Now matches `assert` behaviour — no-op
when `--release`. Added `if !self.is_debug { return; }` guard in `generate_debug`.

### `debug` ANSI Colour Bleed (resolved July 2026)

#### Status

✅ Bug fix

Composite-type debug output was missing ANSI reset code after the metadata suffix,
causing subsequent output lines to render in cyan. Added `self.a_reset()` to the
suffix format string in the composite debug path (`ir_gen.rs`).

### Conversion Table Documentation (resolved July 2026)

#### Status

✅ Documentation fix

Added missing `Same-width signed↔unsigned` conversion row to `lale.md` conversion
rules table (`i32 as u32`, `u32 as i32`).

### `structurally_equal` Left-Right Swap Bug (resolved July 2026)

#### Status

✅ Bug fix

`structurally_equal` was comparing `a.left` against `b.right` instead of `b.left`.
Caused structurally identical `Binary` expressions with differing left and right
operands (e.g., `x as f64 * 2`) to fail matching. One‑character fix.

### `branch` Statement — Exhaustive Pattern Matching (resolved July 2026) — ⚠️ OBSOLETE

#### Status

✅ Implemented — **Superseded by `switch` proposal** (see 🟡 Control Flow Simplification above).

The original `branch` statement will be renamed to `switch` with a new `SwitchPattern`
in Phase B of the control flow simplification work.

Original implementation details:

- Grammar: `branch <expr> / case Pattern: body / else: body / end branch`
- Semantic analysis: exhaustiveness checking, unreachable `else` detection,
  pattern field validation, variable binding with post-branch access control
- IR generation: discriminant-based `IfElse` chain using `ExtractField`
- LSP: completions, snippets, syntax highlighting
- Tree-sitter: grammar and highlight patterns
- Documentation: `lale.md` §branch, `ARCHITECTURE.md` §5.24
