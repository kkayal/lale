---
number: LDP-0004
title: Test Feedback Optimisation
status: implemented
scope: tooling
milestone: 1.0.0
author: Kağan Kayal
decided-by: Kağan Kayal
created: 2026-11-13
decided: 2026-11-13
implemented: 2026-11-13
supersedes: null
superseded-by: null
docs-touched:
  - AGENTS.md
  - CONTRIBUTING.md
  - doc/ARCHITECTURE.md
---

# Test Feedback Optimisation

## 1. Objective

The test suite is correct but expensive to run repeatedly. Two costs dominate:
building the test corpus after a change to the compiler, and the fixed work every
`lale run` subprocess performs before executing a user program (parsing,
analysing, and lowering the standard library).

Measured baseline on the reference machine (macOS, 12 logical cores, configured
`[profile.dev] opt-level = 1`; cold run excluded, median of warm runs):

| Measurement                                                       | Value                                                                      |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Tests in `-p lale`                                                | 3,379 in 106 binaries — 275 unit, 3,104 integration across 102 crate files |
| Doctests                                                          | 13 (1 ignored), ~1.7 s for the whole `--doc` run                           |
| Warm suite (`cargo nextest run -p lale`)                          | 11.3 s wall, ~2 m 8 s CPU                                                  |
| `lale run`, full stdlib                                           | 0.140 s                                                                    |
| `lale run`, `--stdlib-level core`                                 | 0.091 s                                                                    |
| `lale run`, `--stdlib-level none`                                 | 0.049 s                                                                    |
| Integration test crates to compile and link after a `src/` change | 102                                                                        |
| Compile + link of one integration crate                           | ~0.65–0.68 s                                                               |
| Subprocess invocations of `lale run` in the suite                 | ~600                                                                       |

Three consequences follow:

- About 65 % of every `lale run` (0.091 s of 0.140 s) is standard-library work
  that produces the same result on every invocation with the same sources.
- Cargo compiles each integration test file as its own crate, so a compiler
  change forces 102 compile-and-link steps; each step is cheap, but the fixed
  per-crate overhead dominates.
- Developers either run the whole package suite (11 s) or the whole workspace
  (which additionally rebuilds the LSP), even when validating a single unit.

The objective is to reduce both costs without removing a test, weakening an
assertion, or changing any observable compiler behaviour.

### Why this record exists

LDP-0001 §2 does not require a proposal for a performance improvement that stays
inside an accepted design. This proposal is recorded anyway because two of its
parts cross that line: consolidating test crates changes the layout of the test
corpus, and the standard-library cache introduces a new invalidation contract
inside the compiler. The third part changes the documented workflow, which
LDP-0001 §2 explicitly places under the proposal process.

## 2. Definition

The proposal has three workstreams.

| ID  | Workstream                      | Summary                                                                                                                                                                                                                |
| --- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Test-corpus consolidation       | Replace the 102 top-level integration test crates with at most 10 suite crates, each declaring its former files as modules. Test bodies are unchanged.                                                                 |
| D2  | Standard-library analysis cache | Cache the analysed symbols and lowered IR of the builtins and standard library in a content-addressed file, reused by later `lale run` invocations and validated against a key. The cache is transparent and optional. |
| D3  | Tiered feedback workflow        | Define and document three test tiers (edit loop, pre-commit, pre-push/release) and make the cheapest tier sufficient during development.                                                                               |

The three workstreams are independent: each can be accepted, implemented, or
abandoned on its own. A partial delivery keeps the proposal `accepted`, not
`implemented` (LDP-0001 §3.2).

## 3. Rules

### 3.1 Test-corpus consolidation (D1)

- **R4.1** Integration test code lives only in suite crates:
  `tests/<suite>/main.rs` plus one module file per former test crate. No
  top-level `tests/*.rs` test target remains. Shared helpers may stay under
  `tests/common/`.
- **R4.2** At most 10 suite crates exist. Suites are grouped by domain (for
  example: parser, semantics, control flow, runtime, IR, modules, standard
  library and I/O, tooling and CLI, declarations, regressions). Each suite
  declares 8–15 modules.
- **R4.3** No test function, assertion, fixture, or input is deleted, merged, or
  weakened. The inventory of test functions before and after the migration is
  identical.
- **R4.4** A file stem becomes a module name; a test's fully qualified name
  becomes `<suite>::<module>::<test>`. No test function is renamed.
- **R4.5** Each former test crate maps to exactly one `mod` declaration in
  exactly one suite.
- **R4.6** A test must not terminate the test process (`std::process::exit`,
  `std::process::abort`, or `panic = "abort"`). Ordinary panics remain allowed
  because the harness isolates them; a process abort would take down the
  sibling modules of the suite.
- **R4.7** `tests/common/mod.rs` remains the single shared copy and is included
  with `#[path]` where a suite needs it.
- **R4.8** Suite crates obtain the compiler binary from `CARGO_BIN_EXE_lale` and
  its working directory from `CARGO_MANIFEST_DIR`, exactly as the current test
  crates do. No test constructs those paths itself.

### 3.2 Standard-library analysis cache (D2)

- **R4.9** The cache key is a cryptographic hash over all inputs that can change
  the analysed standard library: the builtins source bytes, the standard-library
  source bytes for the selected level, the compiler crate version, the standard
  library level, and the target operating system. Any difference is a miss.
- **R4.10** The cached content is the lowered IR of the builtins and the selected
  standard-library modules. The semantic analysis performed by the IR loader
  runs on a miss and is skipped on a hit. Because that analysis is internal to
  IR lowering, no separate symbol snapshot is stored; the analyzer's shared
  symbol table is built exactly as before.
- **R4.11** A cache hit is behaviourally invisible: with a cold cache and with a
  warm cache, `--print-ast`, `--print-ir`, `--print-symbols`, program stdout,
  program stderr, and the exit status are identical.
- **R4.12** Any read, parse, key, or validation failure discards the cache and
  re-analyses. A cache problem must never fail a compilation and must never
  produce a diagnostic above debug level.
- **R4.13** Cache writes are atomic (temporary file, then rename). Concurrent
  processes may race, but no process may observe a partially written entry.
- **R4.14** The cache lives in the platform user cache directory
  (`$XDG_CACHE_HOME/lale`, `~/Library/Caches/lale`, or
  `%LOCALAPPDATA%\lale`), overridable with `LALE_CACHE_DIR` and disabled with
  `--no-stdlib-cache` or `LALE_NO_STDLIB_CACHE=1`.
- **R4.15** If the cache directory is missing or not writable, compilation
  proceeds uncached. At most one informational diagnostic is emitted.
- **R4.16** Cached modules are identified by module path, never by transient
  numeric identifiers; loading renumbers them consistently with the current run.
- **R4.17** User modules are never cached. Only builtins and standard-library
  modules enter the cache.
- **R4.18** Cache entries carry a format version. An unknown version is a miss.

### 3.3 Tiered feedback workflow (D3)

- **R4.19** Three tiers are defined:
  - **T0 — edit loop:** `cargo nextest run -p lale <filter>`, or
    `cargo nextest run -p lale --lib` for the unit suite.
  - **T1 — before commit:** `cargo build --workspace`, then
    `cargo nextest run -p lale`, then
    `cargo clippy --workspace -- -D warnings`.
  - **T2 — before push and before release:** `cargo build --workspace`, then
    `cargo nextest run --workspace`, then `cargo test --workspace --doc`, then
    `cargo run --bin lale-validate`.
- **R4.20** T0 must not build `lale-lsp`. T1 must not run doctests. T2 is
  mandatory before a push that changes `src/`.
- **R4.21** `AGENTS.md` and `CONTRIBUTING.md` document the three tiers as the
  canonical commands, replacing every `cargo test --workspace` instruction
  except the doctest command.
- **R4.22** The pre-commit hook runs T1 or a documented subset of it, never T2.
- **R4.23** Silent-error counts are reported for T2, as required by AGENTS.md
  Rule 1.

### 3.4 Measurement

- **R4.24** Timing claims are made on the reference machine after one cold run,
  as the median of five warm runs. Cold-run figures are reported separately.
- **R4.25** Before-and-after comparisons use the same reference machine, the
  same `[profile.dev]` settings, and the same test inventory.

## 4. Implementation boundary

The change touches:

- `tests/` — layout only: files move into suite directories and gain `mod`
  declarations. Test bodies are not edited except where a path attribute is
  required.
- `src/main.rs` — the compilation pipeline gains the cache lookup and store
  around the existing builtins/core/stdlib analysis.
- `src/semantic_analysis/sqlite_symbol_management.rs` — snapshot and restore of
  the analysed standard-library rows.
- The existing IR printer and parser (`src/ir/printer.rs`,
  `src/ir/parser.rs`) — reused for the IR half of the cache. Implementing the
  cache exposed three latent round-trip defects that are fixed as part of D2:
  duplicate block labels collapsed into one block, `\'` was not unescaped, and
  the `; var:` annotation on `alloca` was dropped.
- `src/printers/symbol_table.rs` — the unused-symbol table is now ordered by
  `(module, line, column)` instead of relying on hash iteration order.
- `AGENTS.md`, `CONTRIBUTING.md`, `doc/ARCHITECTURE.md` — workflow tiers, suite
  layout, and the cache contract.

The change must not affect:

- language syntax, semantics, or the type system;
- the IR contract for user modules, or any compiler invariant;
- the ABI or any extern-boundary behaviour;
- diagnostic wording, format, or exit codes;
- the set of tests, their assertions, or their inputs;
- the output of any CLI flag with the cache enabled or disabled.

## 5. Alternatives considered

| Alternative                                              | Rejected because                                                                                                                                                                   |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D1: keep 102 crates (status quo)**                     | Every `src/` change pays 102 compile-and-link steps, and every historical profile or dependency change leaves a full extra artifact set behind.                                    |
| **D1: one mega-crate**                                   | Minimises link steps but makes a one-test edit recompile the entire corpus, and one process abort would take down all 3,000 tests.                                                 |
| **D1: two or four very large suites**                    | Same edit-granularity penalty as the mega-crate, with less benefit than the ten-suite split.                                                                                       |
| **D1: generate the module list from a build script**     | Hides test names from editor and test tooling, adds a build step, and complicates `--test` selection.                                                                              |
| **D1: rely on nextest alone**                            | Nextest already parallelises execution but cannot reduce the number of rustc and linker invocations. Adopted, but insufficient by itself.                                          |
| **D2: pass `--stdlib-level none` in tests**              | Valid only for tests that do not exercise the default configuration; using it broadly would narrow coverage. Kept as an option for individual tests, not as the general mechanism. |
| **D2: rewrite subprocess tests as in-process API tests** | Loses end-to-end coverage of the CLI, stdin handling, exit codes, and stderr formatting. A useful complement, not a replacement.                                                   |
| **D2: ship a precompiled standard-library artifact**     | Requires a versioned binary format and an AOT-like build step; larger scope than the observed problem.                                                                             |
| **D2: cache only the symbols or only the IR**            | Both halves are needed to skip the work; caching one half leaves the other in the hot path.                                                                                        |
| **D2: no cache**                                         | Leaves ~65 % of every `lale run` and ~55 CPU-seconds of the suite unchanged.                                                                                                       |
| **D3: one command for everything**                       | Makes the inner loop pay for the LSP and the full workspace on every edit.                                                                                                         |
| **D3: rely on the pre-push gate only**                   | Feedback arrives too late to guide the edit.                                                                                                                                       |

## 6. Consequences

What becomes easier:

- A compiler change rebuilds at most 10 test binaries instead of 102, and a
  developer can run the affected suite directly.
- A standard-library-heavy `lale run` avoids repeated analysis; the test suite
  stops paying for it roughly 600 times per run.
- The edit loop becomes a targeted nextest filter rather than a workspace build.

What becomes harder:

- Test names gain a suite prefix. Editor bookmarks and copy-pasted filters must
  be updated once; substring filters continue to work.
- Editing one test file recompiles its suite (8–15 modules) rather than a single
  small crate. The suite split keeps this penalty small.
- The cache introduces an invalidation obligation. A stale entry would be a
  correctness incident, so the key, the atomic write, and the fallback path are
  mandatory and must be covered by tests.
- There is one more concept to understand (the cache directory and its
  opt-out), and one more failure mode to diagnose.

What remains impossible or excluded:

- Removing tests or assertions to buy speed.
- Running the suite without the compiler binary (`CARGO_BIN_EXE_lale`).
- Caching user modules or any per-program result.

## 7. Acceptance criteria

| Requirement                                             | Verification                                                                                                                                                                                                                     |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R4.1: no top-level test crates remain                   | `find tests -maxdepth 1 -name '*.rs' \| wc -l` reports 0                                                                                                                                                                         |
| R4.2: at most 10 suites                                 | `find tests -mindepth 2 -maxdepth 2 -name 'main.rs' \| wc -l` reports ≤ 10                                                                                                                                                       |
| R4.3/R4.5: identical inventory, one assignment per file | `cargo nextest list -p lale` before and after yields the same test-function names after removing the newly added suite prefix, with the same count; a script confirms every former file appears in exactly one `mod` declaration |
| R4.6: no process aborts                                 | `rg 'process::exit\|process::abort\|panic *= *"abort"' tests/` returns nothing                                                                                                                                                   |
| R4.11: cache transparency                               | For `examples/complex.lale` and ten representative test inputs, stdout, stderr, exit status, `--print-ast`, `--print-ir`, and `--print-symbols` are byte-identical with a cold and a warm cache                                  |
| R4.12: corruption tolerance                             | Truncating or corrupting a cache file produces a successful compile identical to the cold run                                                                                                                                    |
| R4.9: invalidation                                      | Editing one byte of a standard-library source changes the key and forces re-analysis                                                                                                                                             |
| R4.13: atomicity                                        | Running many compilers concurrently from a cold cache leaves exactly one well-formed entry per key                                                                                                                               |
| Performance: build (D1)                                 | After `touch src/lib.rs`, `cargo build -p lale --tests` links at most 10 test binaries (visible in `cargo build --timings`) and the warm rebuild is at least 25 % faster than the 102-crate baseline (median of 3 runs)          |
| Performance: suite                                      | Warm `cargo nextest run -p lale` wall time is at least 30 % below the 11.3 s baseline                                                                                                                                            |
| Performance: subprocess                                 | Warm `lale run` with the full standard library is at most 1.3× the time of `--stdlib-level none` on the reference machine                                                                                                        |
| R4.19–R4.22: workflow                                   | `AGENTS.md` and `CONTRIBUTING.md` list T0/T1/T2; no stale `cargo test --workspace` instruction remains outside the doctest command; the pre-commit hook implements T1 or a documented subset                                     |
| R4.23: silent errors                                    | `cargo run --bin lale-validate` reports 0 HIGH; MEDIUM and LOW do not increase against the pre-change counts                                                                                                                     |

## 8. Open points

| #   | Open point                                                      | State                                                                                                                                             |
| --- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| O1  | Can the cache be enabled by default?                            | **Resolved** — yes; R4.10–R4.13 make it transparent, and R4.14 provides an opt-out.                                                               |
| O2  | Where does the cache live when the home directory is read-only? | **Resolved** — R4.15: compilation proceeds uncached without failing.                                                                              |
| O3  | How do saved test filters survive the rename?                   | **Resolved** — nextest filters match substrings, so existing filters still select the same tests; release notes record the suite prefixes (R4.4). |
| O4  | Should redundant tests be deleted to reduce the count?          | **Out of scope** — this proposal must not change coverage (R4.3).                                                                                 |
| O5  | Should the LSP adopt the standard-library cache?                | **Out of scope** — the LSP owns its own process lifecycle; a later proposal may adopt the cache.                                                  |
| O6  | Should an AOT backend reuse the cached IR?                      | **Out of scope** — no AOT backend exists in 1.0.0.                                                                                                |
| O7  | Should the tiers run in CI?                                     | **Out of scope** — the repository has no CI workflow; T2 is the pre-push and release gate.                                                        |

## Status notes

- 2026-11-13 — Implemented. D1 delivered ten suites holding 3,386 tests (inventory unchanged); D2 delivered the prelude cache; D3 documented the tiers in `AGENTS.md` and `CONTRIBUTING.md` and updated `Makefile.toml`. Delivery exposed and fixed three IR round-trip defects (duplicate block labels, `\'` unescaping, dropped `; var:` annotations) and made `--print-symbols` deterministic; see §4.
