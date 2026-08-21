# Lale — Three-Branch Readiness Plan

**Status:** Phase 1 complete — 1.1–1.5 and 1.7 implemented; 1.6 removed
**Date:** 2026-08-15
**Snapshot:** Lale v0.9.1

---

## Goal

Prepare the codebase so development can be split into three parallel branches
without constant cross-branch breakage:

| Branch   | Scope                                                                       |
| -------- | --------------------------------------------------------------------------- |
| `core`   | Language, grammar, semantic analysis, IR, interpreter, LSP, `lale-validate` |
| `stdlib` | Standard library (`stdlib/*.lale`, `builtins.lale`)                         |
| `aot`    | The native-code backend, consuming the IR                                   |

Branch creation itself is a manual (user) step. This plan only makes the
codebase _ready_ for that split.

---

## Why this needs explicit preparation

Parallel branches are only safe when each branch's inputs and outputs are
pinned down. Three contracts sit at the branch boundaries, and all three are
currently too fluid:

1. **IR** (`core` ↔ `aot`) — the AOT branch's entire input.
2. **Runtime ABI / FFI** (`core` ↔ `stdlib`, `core` ↔ `aot`) — memory layout,
   `str` representation, `import fn` signatures, `__lale_*` hooks.
3. **Parity harness** (`core` ↔ `aot`) — the golden-reference guarantee that
   both backends produce identical output.

Without these contracts, every change in `core` breaks `stdlib` and `aot`, and
the branches converge back into one big merge.

---

## Current state (verified against the code)

- `test suite` / `test case` and the `lale test` runner are implemented
  (`TODO.md` high-priority item 7). Suite/case names are identifiers; test
  cases have function-like local scope (case variables invisible outside the
  case); suites have a suite-level scope for shared `var`/`fn` declarations;
  module-level globals are invisible inside suites/cases while module-level
  functions remain callable; duplicate suite/case names are rejected. Per-case
  `Pass:`/`Fail:` reporting is done. Comprehensive unit coverage lives in
  `tests/test_suite_tests.rs`; imported-module suites running first remains a
  follow-up.
- The IR serialize/deserialize round-trip is implemented (`src/ir/parser.rs`),
  wired behind `--roundtrip`, and specified in `doc/IR_SPECIFICATION.md`
  (Phase 2 complete).
- ABI direction is decided and frozen (Phase 3): a canonical, private
  AAPCS64-based internal ABI plus the platform C ABI at the FFI boundary.
  `doc/ABI_SPECIFICATION.md` is v1.0, and the FFI boundary is a single
  low-level set (unbuffered I/O, allocation, libm, exit) — the buffered C
  stdio family was dropped.
- `str` is defined in two places: `builtins.lale` and `Module::new()`.
  `ARCHITECTURE.md` §4.6 documents a planned single-source-of-truth cleanup.
- The stdlib user-facing surface is partly documented in `stdlib.md`.

---

## The Plan

Work the phases in order. Within a phase, items are listed in dependency order.

### Phase 1 — Integrated testing (do first)

This is the foundation: every other contract is verified through tests. The
design follows `TODO.md` high-priority item 7.

#### 1.1 Grammar and AST for `test suite` / `test case`

**Status:** ✅ Implemented (August 2026)

**Done:** Added `kw_test_suite`/`kw_test_case` and the `test_suite`/`test_case`
grammar rules to `lale.pest`; added `TestSuiteStmt`/`TestCaseStmt` and
`Stmt::TestSuite` to `definitions.rs`; added `build_test_suite`/`build_test_case`
and the `build_statement` dispatch; added visitor arms. Verified by
`tests/test_suite_tests.rs` (grammar + AST build).

Suite and case names are **identifiers**, not string literals (`test suite
MatrixOps`, `test case multiplication`); the original string-literal form was
replaced. Grammar edge cases (identifier requirement, underscore names, empty
case bodies, multiple suites) are covered by dedicated tests.

**Goal:** Parse the `test suite` / `test case` structure from TODO.md item 7:

```lale
test suite MatrixOperations
    test case Multiplication
        // assertions
    end test case

    test case Inversion
        // test logic
    end test case
end test suite
```

**Implementation:** Add `test_suite`, `test_case`, `end_test_case`, and
`end_test_suite` rules to `lale.pest`; add `TestSuiteStmt` and `TestCaseStmt` to
`definitions.rs`; add `build_test_suite` / `build_test_case` to the builder and
the corresponding visitor arms.

**Acceptance:** A `test suite` containing one or more `test case` blocks parses
and round-trips through the AST. `test suite`/`test case` are regular (non-`#`)
statements — they remain present in the IR.

#### 1.2 Semantic analysis: placement and structure

**Status:** ✅ Implemented (August 2026)

**Done:** Added `visit_test_suite`/`visit_test_case` overrides to
`SemanticAnalyzer` and the placement check in `visit_program`. Enforces:
test suite at module scope, no nesting, at least one case, and
"test suite must be last". Verified by the semantic cases in
`tests/test_suite_tests.rs`.

Additional validation added in August 2026: duplicate suite names and duplicate
case names within a suite are rejected; test cases behave like functions — each
case gets a function-like scope keyed `suite::case`, its variables are
case-local (invisible to other cases/suites/non-test code), globals remain
readable, and nested `fn`/`type`/`enum`/`use` definitions are rejected.

**Goal:** Enforce the tripartite file structure from TODO.md item 7:
`use` → definitions → `test suite` (last, optional).

**Implementation:** In the analyzer, validate that `test suite` appears after all
other top-level statements, that there is no nesting of suites/cases, and that a
suite has at least one case. Resolve symbols inside cases normally.

**Acceptance:** Valid placement compiles; `test suite` before a `var`/`fn`/
top-level statement, a nested suite, or an empty suite is a compile error.

#### 1.3 `#mode` compile-time constant and IR gating

**Status:** ✅ Implemented (August 2026)

**Done:** Added `CompilerConstKind::Mode` (`#mode`), threaded a `test_mode` flag
through `CompilerOptions` → `OwnedAnalyzer` → `IrGenerator`, and added
`--test` to `run`. Top-level runtime statements are gated behind a runtime
`#mode == "run"` guard and `test suite` behind `#mode == "test"`, so both
branches stay in the IR. Verified by the gating cases in
`tests/test_suite_tests.rs`.

Two mode-guard value-scoping bugs were fixed in August 2026: the main epilogue's
global-str auto-free now re-derives `globaladdr` at the epilogue (the previous
code referenced a `globaladdr` emitted inside a skipped guard block, crashing
with a `Load`/`ExtractField` ICE), and `ExtractField` treats zeroed memory slots
(uninitialized guarded bodies) as "nothing to extract" instead of crashing. Both
have regression tests.

**Goal:** Compile the full file identically in both modes, then gate execution at
runtime via a `#mode` compile-time constant — the IR is the same in `lale run`
and `lale test`.

**Implementation:** Emit `#mode` (run vs test) as a compile-time constant; wrap
top-level user code and test-suite code in `#if mode == "run"` /
`#if mode == "test"` so the untaken branch is still present in the IR. Keep
`use`/`type`/`enum`/`fn`/`var` initializers executing in both modes.

**Acceptance:** `lale run` executes top-level code and skips test suites;
`lale test` skips top-level code and executes test suites; the IR is identical
modulo the `#mode` constant value.

#### 1.4 `lale test` runner and interpreter mode

**Status:** ✅ Implemented (August 2026)

**Done:** Added the `lale test` subcommand and wired test mode through the CLI
(`test_mode = true`). Test suites execute via the `#mode` gating from 1.3.
Verified by `tests/test_suite_tests.rs`.

**Done:** Per-case `Pass:`/`Fail:` output is implemented as part of 1.5.

**Done:** `lale test --filter <pattern>` runs only matching suites/cases. The
flag is repeatable (`--filter A --filter B`); a case runs if any pattern
matches. Patterns containing `/` (same separator as the `Pass: suite / case`
output) match `suite/case`, otherwise the suite name or case name. Filtering
happens at IR generation; semantic analysis still validates all suites/cases.

**Deferred:** "imported-module suites run first" (requires module-merge support
for `Stmt::TestSuite` — flagged as a follow-up).

**Goal:** Add the `lale test` subcommand and interpreter `--mode test`, with the
execution order and output from TODO.md item 7.

**Implementation:** Add `lale test` to the CLI; imported-module suites run first
(in `use` order), then the main file's suites. Output `Pass: <suite> / <case>` or
`Fail: <suite> / <case>: expected <details>, found <details>`; exit 0 on all-pass,
non-zero on any failure. Run each suite under the interpreter's leak detector.

**Acceptance:** `cargo run -- test <file>` reports per-case pass/fail in the
specified order and returns the correct exit code.

#### 1.5 `assert` behaviour in test mode

**Status:** ✅ Implemented (August 2026)

**Done:** Added `TestBegin`/`TestFail`/`TestEnd` IR instructions plus the
interpreter `TestState` thread-local. `Stmt::TestSuite` brackets each case with
`TestBegin`/`TestEnd`; `generate_assert` emits `TestFail` + branch-to-continue
when `in_test_case` is set instead of writing the abort message and calling
`__lale_exit`. `execute_module` returns exit code 1 on any test failure and
`compile_and_execute` now honours that code. Verified by the two per-case
reporting tests in `tests/test_suite_tests.rs`.

Test-case variables are stack locals of the synthetic `main`; each case gets its
own IR-gen scope (same-named variables in different cases do not collide) and
its string data is freed on case exit (no leaks).

**Done:** In test mode a `HEAP LEAK` report forces a non-zero exit, so `lale test`
acts as a leak detector (plain `lale run` still reports but exits 0). Fixed the
pre-existing leak where writing a numeric/bool/char variable leaked its
type-to-string conversion temp string.

**Goal:** Make a failing `assert` inside a `test case` report and continue in
`lale test`, while retaining the existing abort behaviour for standalone asserts
in `lale run`.

**Implementation:** In test mode, an `assert` records the failure (location,
expected/found) and continues to the next case instead of aborting.

**Acceptance:** A suite with one failing and one passing case reports the
failure, still runs the passing case, and exits non-zero.

#### 1.6 Backend abstraction for execution — ❌ Removed (August 2026)

**Reason:** With only one backend, a `RunBackend` trait is an unverifiable
premature abstraction. The real seam should be introduced when the AOT branch
exists and can provide a second implementation. Its eventual purpose is folded
into 1.7.

#### 1.7 Differential/parity harness scaffold

**Status:** ✅ Implemented (August 2026)

**Done:** Added `src/parity.rs` and the `lale parity <source>` subcommand. The
harness runs each registered backend (`lale run`; `lale exec` later) as a
subprocess, captures `(stdout, stderr, exit_code)` via piped stdio, and diffs the
triples. Verified by `tests/parity_tests.rs` (zero diffs with one backend, and
child exit-code propagation).

**Goal:** Run the same program under one or more backends and diff the canonical
output, so the interpreter stays the golden reference for the future AOT backend.

**Implementation:** Add a `lale parity <source>` subcommand that runs each
registered backend as a **subprocess** (`lale run` now; `lale exec` when AOT
lands) and captures `(stdout, stderr, exit_code)` from the child process via
piped stdio. No interpreter redirection or in-process capture is required.
Compile-once-to-IR-text is intentionally deferred to Phase 2, when a serialized
IR exists to feed to each backend.

**Acceptance:** `parity` runs with one backend and reports zero diffs; the
registered-backend list is the future second-backend registration point.

#### 1.8 Remaining test-suite decisions — ✅ Resolved (August 2026)

Phase 2 and everything after it was **gated on resolving these three open
points** surfaced while hardening the built-in test framework. All three are now
resolved and covered by `tests/test_suite_tests.rs`.

1. **`assert` failure messages carry `expected` / `found`.** — ✅ Implemented (August 2026)

   A failing `assert a == b` now reports
   `Fail: <suite> / <case>: expected <b>, found <a> (<file>:<line>:<col>)`.
   Non-equality asserts keep the location-only message. `TestFail` carries
   optional `expected`/`found` SSA operands, serialized in the IR as `%N` (or
   `-` for none), and the interpreter formats them via the shared
   value-to-string helper. Covered by `tests/test_suite_tests.rs` and the IR
   round-trip suite.

2. **Suite-level scope and/or `setup` / `teardown`.** — ✅ Implemented (August 2026)

   A `test suite` now has a **suite-level scope** for `var` and `fn`
   declarations shared by every case in the suite and invisible outside it:

   - Suite-level `var` are suite-scoped storage shared across cases and
     reachable from suite functions, but invisible to non-test code and other
     suites. Module-level globals are **invisible** from suite/case scope (see
     point 3).
   - Suite-level `fn` are suite-qualified (`suite::fn`) so they don't collide
     with global functions; they are callable from cases and from each other,
     and their bodies fall back to the suite scope.
   - Declarations must appear before the first `test case` in a suite (a
     structural error is reported otherwise).
   - **No new keywords.** `setup`/`teardown` are deliberately **not** auto-run
     hooks; they are only a naming convention shown in examples. Each case
     invokes its fixtures explicitly, matching Lale's "everything is explicit"
     spirit.

3. **Test-case isolation from global variables.** — ✅ Implemented (August 2026, fully invisible)

   Module-level (global) variables are **invisible** inside a test suite and its
   cases: reading or writing one is the normal `Undefined variable` / `Assignment
to undefined variable` error, not a special write rejection. Module-level
   **functions** remain callable. Shared mutable state belongs in suite-level
   variables (point 2), which a case may read and write — order significance is
   therefore an explicit suite-author choice, exactly as global state is for
   regular functions.

---

### Phase 2 — IR contract (Option B: cross-language backend)

The AOT branch consumes the IR. Option B means: a **versioned, round-trippable
IR text format plus a deserializer**, so a backend can be written in any
language against a stable text contract.

#### 2.1 Canonicalize the IR text format

**Status:** ✅ Implemented (August 2026)

**Done:** Adopted the existing line-oriented format as canonical. Made
`src/ir/printer.rs` lossless (`Store.ty`, `ExtractVecElement.inner_ty`, and
call source locations are now emitted) and replaced silent lookup fallbacks with
`ice!`. Rewrote `ARCHITECTURE.md` §4.7 to describe the actual block-based CFG and
the line-oriented grammar.

**Goal:** One formally-specified text format that is actually what the printer
emits.

**Implementation:** Adopt the current line-oriented format and update the doc.
Update all `--print-ir` output and any tests that assert on the format.

**Acceptance:** `ARCHITECTURE.md` and the printer describe the same grammar;
`--print-ir` output matches the spec.

#### 2.2 Version the serialized IR

**Status:** ✅ Implemented (August 2026)

**Done:** Added `IR_FORMAT_MAJOR = 1` / `IR_FORMAT_MINOR = 0` to
`src/ir/mod.rs`; `print_module` now emits `; ir-version: 1.0` as its first line.
Documented the major/minor bump policy in `ARCHITECTURE.md` §4.7. Rejection of an
unsupported major version is the parser's responsibility (2.3).

**Goal:** Every serialized IR file carries a version, so backends can reject
unknown versions instead of misparsing.

**Implementation:** Add a version field to the serialized form. Define a bump
policy (major = breaking, minor = additive).

**Acceptance:** A serialized module declares its version; the parser rejects an
unsupported major version with a clear error.

#### 2.3 Implement the IR deserializer/parser

**Status:** ✅ Implemented (August 2026)

**Done:** Added `src/ir/parser.rs` with `parse_module`. It tokenizes the
line-oriented format, validates the `; ir-version:` major, and reconstructs
structs, externs, globals, functions, blocks, values, and every `Instruction`
variant. Added `Module::new_empty` and a basic round-trip unit test. Full
round-trip coverage is enforced in 2.4/2.5.

**Goal:** Parse the canonical text format back into an in-memory `Module`.

**Implementation:** Add a parser for the format chosen in 2.1, covering every
`Instruction`, type, global, struct, and function. Reuse the existing
`IrType`/`Instruction`/`Module` types.

**Acceptance:** `parse(print(module))` reconstructs an equivalent module for the
full existing test suite and stdlib.

#### 2.4 IR round-trip tests

**Status:** ✅ Implemented (August 2026)

**Done:** Added `tests/ir_roundtrip_tests.rs` with idempotence coverage for a
broad instruction set, global/struct reconstruction, and unsupported-version
rejection. The tests are part of `cargo test --workspace`.

**Goal:** Make the round-trip a continuously enforced invariant.

**Implementation:** Add property-style tests: for representative programs,
`serialize → deserialize → serialize` is idempotent, and executing the
deserialized module produces identical output to executing the original.

**Acceptance:** Round-trip tests are part of `cargo test --workspace` and pass
for every supported instruction.

#### 2.5 Wire the round-trip into the execution path

**Status:** ✅ Implemented (August 2026)

**Done:** Added `--roundtrip` to `run` and `test`. When set, `compile_and_execute`
serializes the freshly generated IR, parses it back, and executes the
reconstructed module. Added a CLI integration test. Default-on can be flipped
once the round-trip proves stable across all programs.

**Goal:** Exercise serialize→deserialize on every compile so drift is caught
immediately, not only in dedicated tests.

**Implementation:** In `compile_and_execute`, after `try_generate_owned`, serialize
then deserialize before calling `execute_module` (behind a flag initially, then
always once stable).

**Acceptance:** Every normal `run` executes the deserialized IR, and output is
unchanged from today.

#### 2.6 IR format specification document

**Status:** ✅ Implemented (August 2026)

**Done:** Created `doc/IR_SPECIFICATION.md` with the lexical grammar, module
EBNF, type/constant tables, full instruction reference, versioning policy, and
semantics/determinism rules. `src/ir/mod.rs` already references it.

**Goal:** A stable reference that all three branches can point at.

**Implementation:** Write the formal grammar, versioning policy, and semantics
summary as a dedicated doc, including the §4.7.10 determinism rules.

**Acceptance:** The doc is the single source of truth for the IR format; the
printer/parser tests reference it.

---

### Phase 3 — ABI / FFI contract

**Decision (August 2026):** Option B — a canonical, private **AAPCS64-based
internal ABI** for Lale-to-Lale code, with the **platform C ABI** used at the
FFI boundary (`import fn` / extern calls). Internal determinism is preserved;
native interop is direct. The internal ABI is a fixed reference, not a target
constraint.

The FFI boundary is a **single low-level set** — unbuffered I/O
(`open`/`read`/`write`/`close`), allocation (`malloc`/`free`), libm math (`pow`
et al.), and exit. There is **no POSIX-and-C-stdio duplication**: buffered/stream
abstractions are built **in Lale** on top of that boundary. This drops the stdio
family (`fopen`/`fread`/`fwrite`/`fclose`/`fseek`) from the extern set and
supersedes the historical `fread` `unimplemented!()` stub.

#### 3.1 Freeze and version the runtime ABI

**Status:** ✅ Implemented (August 2026) — docs frozen

**Goal:** Produce an ABI spec whose _internal_ layout follows AAPCS64 (little-
endian, 8-byte pointers, 16-byte alignment, HFA/HVA struct passing, `f128` quad
as the extended float), and whose _FFI boundary_ is the platform C ABI.

**Implementation:** Revise `doc/ABI_SPECIFICATION.md` (currently documents the
conservative interpreter layout) to the AAPCS64 internal ABI + platform-C-ABI
boundary. Add a version and changelog; freeze it.

**Documents that MUST be updated together (enforced):**

- [x] `doc/ABI_SPECIFICATION.md` — authoritative ABI spec
- [x] `doc/ARCHITECTURE.md` §4.8 — memory layout, hooks, FFI; remove the stale
      pre-freeze hook list (`__lale_write_stdout`, `__lale_write_stderr`, …)
- [x] `doc/ARCHITECTURE.md` §4.7.10 and any §5.x sections that reference
      `__lale_error` or memory layout
- [x] `doc/lale.md` — any `import fn` / FFI / hook references
- [x] `doc/stdlib.md` — FFI surface, if it lists extern/hook signatures
- [x] `doc/TODO.md` — the little-endian discrepancy note and any ABI-adjacent items

**Acceptance:** The ABI spec lists every hook and layout rule; changes require a
version bump. The internal/FFI split is explicit, and **every document in the
checklist above is consistent with the frozen ABI** — a missing or inconsistent
document fails 3.1.

#### 3.2 ABI conformance tests

**Status:** ✅ Implemented (August 2026)

**Goal:** Prevent accidental ABI drift.

**Implementation:** Tests that assert the AAPCS64 internal layout (sizes/offsets,
16-byte alignment, `str` as `{ptr,len}`) and that each `__lale_*` hook / FFI
boundary uses the documented platform C ABI signature.

**Acceptance:** Conformance tests run in `cargo test --workspace` and fail on
ABI changes unless the spec/version is also updated.

#### 3.3 `import fn` / extern declaration stability review

**Status:** ✅ Implemented (August 2026) — single low-level boundary documented

**Goal:** Confirm the stdlib's `import fn` surface is a **single low-level
platform-C-ABI** FFI boundary (no POSIX-and-C-stdio duplication), and that the
interpreter's `call_extern` handlers only cover that set.

**Implementation:** Review `add_stdlib_externs` and `call_extern`; document the
allowed extern set and reject/flag anything outside it. The stdio family
(`fopen`/`fread`/`fwrite`/`fclose`/`fseek`) is **out of scope** — buffered I/O is
Lale stdlib code on top of the unbuffered `open`/`read`/`write`/`close` boundary.
This removes the historical `fread` `unimplemented!()` stub from the boundary.

**Acceptance:** The allowed extern set is explicit and tested; no stealth
handlers; the stdio family is documented as non-boundary.

---

### Phase 4 — Coupling cleanup

#### 4.1 `str` single source of truth

**Status:** 🔴 Not started

**Goal:** Eliminate the dual `str` definition (builtins.lale + `Module::new()`).

**Implementation:** Reverse compilation order so builtins type definitions are
processed before user code, then remove the Rust pre-registration, as planned in
`ARCHITECTURE.md` §4.6.

**Acceptance:** `str` is defined only in `builtins.lale`; all tests and stdlib
still compile.

#### 4.2 Module/import interface stability review

**Status:** 🔴 Not started

**Goal:** Freeze the module-resolution and `use`/`import` surface the stdlib
depends on, so the `stdlib` branch does not break on core refactors.

**Implementation:** Document the module-resolution contract and add tests for
the exact behaviours the stdlib relies on (path resolution, export visibility,
circular-import rejection).

**Acceptance:** The module contract is documented and covered by tests.

---

### Phase 5 — Stdlib surface

#### 5.1 Finalize the stdlib target surface

**Status:** 🟡 Partial — user-facing API partly documented in `stdlib.md`

**Goal:** A frozen list of stdlib modules/functions with idiomatic Lale
signatures (`str`, `T?`, units, 1-based indexing). The surface is built
exclusively on the **single low-level FFI boundary** from Phase 3 — buffered/stream
abstractions are Lale code, not additional externs (no POSIX-and-C-stdio
duplication).

**Implementation:** Complete and freeze `stdlib.md`'s API reference; mark the
surface as the 1.0 contract. Encode the single-boundary rule so new functions do
not introduce new `import fn` externs unless they are genuinely low-level OS/FFI
primitives.

**Acceptance:** The surface is complete enough to develop the stdlib branch
independently against it.

#### 5.2 Stdlib conformance test scaffolding

**Status:** 🔴 Not started

**Goal:** Run the stdlib's own `test` blocks (Phase 1) as a gate, independent of
the core branch's Rust tests.

**Implementation:** Add stdlib `.lale` test suites for the public API and wire
them into `lale test`/CI.

**Acceptance:** Stdlib tests run under `lale test` and pass against the frozen
ABI and surface.

---

### Phase 6 — Branch mechanics (process)

#### 6.1 Interface-PR workflow and merge cadence

**Status:** 🔴 Not started

**Goal:** A written process for changes to the three shared contracts (IR, ABI,
parity harness) so they land on `core` first and are consumed by the others.

**Implementation:** Document: interface PRs are small, version-bumped, and
gated by the conformance/round-trip/parity tests before merge.

**Acceptance:** The workflow is written down and agreed.

#### 6.2 Branch README / CONTRIBUTING

**Status:** 🔴 Not started

**Goal:** Each branch knows what it owns and what it must not touch.

**Implementation:** Write a short `CONTRIBUTING`/README covering branch
boundaries, shared contracts, and the required gates (`cargo build --workspace`,
`cargo clippy --workspace -- -D warnings`, `cargo test --workspace`,
`lale-validate`).

**Acceptance:** The document is in place before branches are created.

#### 6.3 Create the three branches (user action)

**Status:** ⏳ Awaiting all prior items

**Goal:** Physically create `core`, `stdlib`, and `aot`.

**Implementation:** User creates the branches from `main` after the Definition of
Done is met. (Not performed by the agent — git writes are outside agent scope.)

**Acceptance:** Three branches exist; each builds and passes its own gates.

---

## Execution order and dependencies

```text
Phase 1 (testing) ──────────────► Phase 2 (IR) ──► Phase 6 (branch)
        │                              ▲
        └──────────► Phase 3 (ABI) ────┘
                              │
                              └──────────► Phase 5 (stdlib surface)

Phase 4 (coupling cleanup) can run in parallel with Phases 2–3.
```

- Phase 1 is a hard prerequisite for everything else. **Phase 2+ was gated on
  resolving the three open test-suite points in 1.8** (assert expected/found,
  suite-level scope, and global-variable isolation); all three are now resolved
  (August 2026).
- Phase 2 (IR) and Phase 3 (ABI) can overlap once the test runner (1.4/1.5)
  and parity harness (1.7) exist.
- Phase 5 depends on Phase 3 (frozen ABI) and Phase 1 (test runner).
- Phase 6 is last.

## Definition of Done

All of the following are true:

1. `lale test` runs in-language test blocks and exits correctly.
2. The parity harness runs the interpreter as the golden reference and accepts a
   second backend.
3. The IR round-trips through a versioned text format; normal execution uses the
   deserialized module; the printer matches the spec.
4. The runtime ABI/FFI surface is versioned and covered by conformance tests.
5. `str` has a single source of truth.
6. The stdlib surface is frozen and testable independently.
7. Branch workflow/README are written, and the three branches can be created
   with clean, independent gates.

## 7. Left overs

Minor, non-blocking cleanups noticed during Phase 3 (ABI/FFI) work. None of
these gate the three-branch split; they are recorded here so they are not lost.

### 7.1 Unused `posix_*` imports in `std.lale`

`stdlib/src/std.lale` still declares `posix_open`, `posix_read`, `posix_write`,
`posix_close`, and `posix_lseek` as `import fn signature`. The actual file-I/O
wrappers in `file_io_posix.lale` import `open`/`read`/`write`/`close`/`lseek`
directly, so the `posix_*` declarations are dead.

**Action:** remove the five `posix_*` imports from `std.lale`.

### 7.2 Redundant `pow` / `write` / `exit` imports in `std.lale`

`std.lale` declares `pow`, `write`, and `exit` via `import fn signature`. `pow`
and `write` are already registered in `Module::add_stdlib_externs()`, so the
`std.lale` declarations are redundant. `exit` is only used by the dead
`__lale_exit` wrapper (see §7.3).

**Action:** remove the redundant `pow`/`write` declarations (and `exit` together
with §7.3).

### 7.3 Dead `__lale_exit` wrapper in `std.lale`

`std.lale` defines `export fn __lale_exit` that wraps the C `exit` function. The
actual `__lale_exit` is an extern registered in `add_stdlib_externs()` and
handled by `call_extern` (via `std::process::exit`), so the Lale wrapper is dead
code.

**Action:** remove `export fn __lale_exit` and its `exit` import from `std.lale`.

### 7.4 Windows low-level externs have no interpreter handlers

`file_io_windows.lale` declares `_open`, `_read`, `_write`, `_close`, and
`_lseeki64`. These have no `call_extern` handlers — but they are only compiled in
under `#if #windows`, which is unreachable on the current POSIX test host. They
become relevant when the interpreter/AOT targets Windows.

**Action:** implement the Windows low-level I/O handlers (or link them in the AOT
backend) when Windows-host support is added.

### 7.5 Test suite / test case implementation was buggy with no unit coverage — ✅ Resolved (August 2026)

All three issues raised here were addressed:

- **Crashes fixed.** `lale test` on ordinary run-mode code crashed with
  `Load from non-pointer value: None` + an `ExtractField` ICE, and with
  `ExtractField: expected struct or string value, got Int(0)` for struct-typed
  locals in skipped mode-guarded blocks. Both were mode-guard value-scoping
  bugs, fixed and covered by regression tests (see 1.3).
- **Function-like local scope.** Test cases now have their own function-like
  scope: variables are case-local (invisible to other cases, other suites, and
  non-test code). Suites have a suite-level scope for shared `var`/`fn`
  declarations; module globals are invisible inside suites/cases while module
  functions remain callable (see 1.8).
- **Identifier names.** Suite/case names are now proper identifiers instead of
  string literals: `test suite MatrixOps`, `test case multiplication` (see 1.1).
- **Comprehensive unit testing.** `tests/test_suite_tests.rs` grew to 54 tests
  covering grammar, semantic validation (placement, nesting, duplicates,
  scope), mode gating, pass/fail reporting, `lale test <file>`, control flow in
  cases, suite-level scope, and crash regressions.
