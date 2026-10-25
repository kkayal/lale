---
status: in-progress
created: 2026-10-25
scope: language | compiler | stdlib | docs
---

# Implementation Plan — Platform Constants, Uptime Arms, File I/O Facade

This file is the working plan and progress log for one change set. It is not a
design record: the decisions live in `doc/proposals/LDP-0002-builtin-uptime.md`
and the review summaries. This file records **order, status, and evidence**.

## Goal

Make platform selection honest and complete:

1. Compile-time conditions work everywhere they are documented to work —
   including around `use` statements and inside function bodies.
2. Each platform constant is an independent predicate; none is defined as the
   negation of another.
3. `__lale_read_uptime()` has a real implementation per platform family, so AOT
   and interpreted behaviour agree.
4. File I/O has one public module, with WASI as its own platform module rather
   than borrowing `#posix`.

## Decisions in force

| Topic            | Decision                                                                                                                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Constants        | `#windows` = `cfg(target_family = "windows")`; `#linux` = linux \|\| android; `#darwin` = `cfg(target_vendor = "apple")`; `#wasi` = `cfg(target_os = "wasi")`; `#posix` = `cfg(unix)` |
| No derivations   | Each constant computed from its own target attribute; the `!windows` fallback is removed                                                                                              |
| Uptime selection | Top-level `#match` over `#linux` / `#darwin` / `#windows` / `#wasi`, `#else: #fail`                                                                                                   |
| Darwin clock     | `mach_continuous_time` + `mach_timebase_info`; both are documented non-C boundary entries                                                                                             |
| WASI clock       | `clock_time_get(CLOCK_MONOTONIC)`; reduced conformance                                                                                                                                |
| Time conversion  | Exact split (`q = t/d`, `r = t%d`, `ns = q·n + (r·n)/d`) in both backends; abort on overflow                                                                                          |
| Failure policy   | Check the platform error code; write one line to fd 2; exit 1                                                                                                                         |
| File I/O         | Platform modules export prefixed names; `std.file_io` is the single public module with thin per-platform wrappers                                                                     |
| WASI file I/O    | New `file_io_wasi.lale`, independent of `#posix`                                                                                                                                      |
| BSD              | Dropped; unported targets hit `#else: #fail`                                                                                                                                          |

## Phases

### Phase 0 — Baseline

- [x] Baseline: `cargo test --workspace` passed at session start; `cargo clippy --workspace -- -D warnings` clean
- [ ] Baseline `lale-validate` silent-error counts (deferred to Phase 7; counts will be compared then)

### Phase 1 — Conditional imports (#5)

Observed defect: a `use` inside a **taken** `#if` branch failed with
`Symbol 'X' is not exported from module ...`; the same `use` in an **untaken**
branch was ignored. Cause: dependency discovery ran before conditions were
evaluated and scanned only top-level `use` statements.

- [x] Identify the ordering: `ModuleResolver::extract_use_statements`
- [x] Scan compile-time branches during dependency discovery
      (`collect_use_statements` in `module_resolver.rs`)
- [x] Regression test: guarded import in a taken branch resolves
      (`tests/compile_time_imports_tests.rs`)
- [x] Regression test: guarded import in an untaken branch is ignored

Note: a module named in an untaken branch must still exist on disk. Acceptable
because platform modules ship together; documented in the resolver comment.

### Phase 2 — Conditional signature extraction (#3)

- [x] `extract_fn_signatures_from_statements` evaluates `#if` conditions and
      recurses only into the taken arm (all else-if/else handled)
- [x] `#when` and `#match` handled the same way
- [ ] `#switch` still scans every case (rare for platform selection; open)
- [x] Regression test: two arms defining the same name register one signature
      (the `#else` arm's `i32` must not overwrite the taken `u64 in <ns>`)

### Phase 3 — Platform constants

- [x] Grammar: `#linux`, `#darwin`, `#wasi` in `compiler_const`
- [x] `CompilerConstKind`: `Linux`, `Darwin`, `Wasi` added; `Posix` redefined
- [x] `expr_parser`: recognise the three names
- [x] `ir_gen`: compute each from its own `cfg`, `!windows` fallback removed
- [x] Analyzer: `Platform` struct replaces `(is_posix, is_windows)`; all four
      match sites use it (`SemanticAnalyzer`, `OwnedAnalyzer`, string forms)
- [x] Printer names
- [x] LSP completion list
- [x] `doc/lale.md` compiler-constants table
- [x] `ARCHITECTURE.md` §4.6 constant list
- [x] Verified on this host: `#darwin` true, `#linux` / `#wasi` false, `#posix` true

### Phase 4 — Uptime arms

- [x] `core.lale`: one top-level `#match` replaces the two `#if` blocks
- [x] `#linux`: `clock_gettime(CLOCK_BOOTTIME)`, checked, abort on failure
- [x] `#darwin`: `mach_continuous_time` + `mach_timebase_info`, checked,
      exact-split conversion
- [x] `#windows`: `QueryInterruptTimePrecise`, checked multiply
- [x] `#wasi`: `clock_time_get`, checked, reduced conformance
- [x] `#else`: `#fail`
- [x] Interpreter: `mach_continuous_time`, `mach_timebase_info`,
      `clock_time_get` handlers; WASI branch removed from `read_clock`
- [x] Tests: `uptime_tests` 3/3 pass; `examples/complex.lale` logs with uptime

**Known divergence found:** the interpreter reconstructs a struct loaded from
memory on 8-byte `Value` slots, so it reads the second `u32` field of
`mach_timebase_info_data_t` at offset 8, not at the C ABI offset 4. The handler
writes both offsets, with a `TODO` to make the interpreter honour declared field
sizes. AOT will use the C layout. This affects any C struct with 32-bit fields.

**Observation:** this host reports `numer = 125, denom = 3`, not 1/1 — the
exact-split conversion is required, not merely defensive.

**WASI arm bug (found by the browser demo):** the `#wasi` wrapper declared
`var nanos as u64 = 0` and returned it from a function declared
`u64 in <ns>`, so it failed with "Return unit mismatch" the first time the arm
was ever analysed. Platform-gated arms are not semantically checked on other
hosts — this is why the demo matters as a test — and it is fixed
(`var nanos as u64 in <ns>`).

### Phase 5 — File I/O facade

- [x] Rename exports in `file_io_posix.lale` (`posix_*`) and `file_io_windows.lale` (`win_*`)
- [x] Add `file_io_wasi.lale` (`wasi_*`), gated `#if #wasi`
- [x] Add `file_io.lale`: public API, one thin wrapper per platform under `#if`
- [x] `full.lale`: import `std.file_io` instead of the three platform modules
- [x] `examples/complex.lale`: import from `std.file_io`
- [x] Docs: `stdlib.md`, `lale.md`, `ARCHITECTURE.md` (file I/O architecture)
- [x] Tests: file I/O round-trip through the facade (`tests/file_io_facade_tests.rs`)

### Phase 6 — In-body compile-time blocks (#1)

- [ ] Make `process_statement` recurse through `FnDef`, `If`, `When`, `Loop`,
      `Match`, `Switch`, `Unsafe`
- [x] Unevaluable/nested condition in an unsupported position → compile error, not
      a silent drop (IR generation now rejects leftover `#if`/`#when`/`#match`/`#switch`)
- [ ] Regression tests: `#if` in a function body; `#match` inside a loop body

### Phase 7 — Documentation and validation

- [x] `ARCHITECTURE.md` §4.6 and `doc/lale.md` constants
- [x] `AGENTS.md`: generalise the handler rule to "native system-call interface
      on Darwin and WASI"
- [x] `LDP-0002`: status note for the implemented arms; O5 closed
- [x] `ARCHITECTURE.md` §4.8: WASI alongside the Mach note
- [x] `ABI_SPECIFICATION.md` §4: `clock_time_get` note
- [x] `cargo clippy --workspace -- -D warnings` — clean
- [x] `cargo test --workspace` — all suites green
- [x] `cargo run --bin lale-validate` — **0 HIGH, 11 MEDIUM, 157 LOW (168 total)**
- [ ] Markdown: `prettier --check` and `markdownlint-cli2`

## Progress log

| Date       | Phase | Note                                                                              |
| ---------- | ----- | --------------------------------------------------------------------------------- |
| 2026-10-25 | 0     | Plan created; clippy baseline clean                                               |
| 2026-10-25 | 1     | Dependency discovery now scans compile-time branches; 3 tests added               |
| 2026-10-25 | 2     | Signature extraction honours `#if`/`#when`/`#match`; 1 test added                 |
| 2026-10-25 | 3     | Constants added end to end; verified on this host                                 |
| 2026-10-25 | 4     | `#match` arms implemented; interpreter handlers added; uptime tests green         |
| 2026-10-25 | 5     | Facade + WASI module + renames; round-trip test green; internal tests updated     |
| 2026-10-25 | 6     | Loud failure for nested compile-time blocks; recursive transform still open       |
| 2026-10-25 | 7 | Handler rule, LDP-0002, ARCHITECTURE §4.8, ABI §4 updated; clippy and tests green |
| 2026-10-26 | 5 | Demo fix: `STDLIB_FILES` in `demo/src/main.js` now seeds `file_io.lale` and `file_io_wasi.lale`; demo rebuilt |
