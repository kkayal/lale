# Silent Errors — Deep Audit

- **Version:** 0.1.0
- **Audit date:** 2026-10-04
- **Scope:** `src/` — patterns not caught by the `lale-validate` text-scanner.
- **Complements:** `cargo run --bin lale-validate` (0 HIGH / 11 MEDIUM / 157 LOW).

This audit targets the _semantic_ fallbacks that a text-scanner misses: silent
default type fallbacks, `unwrap_or_default()`/`Default::default()` that hide
missing data, DB error discards that silently drop symbols/types, and default
`match` arms that keep going instead of crashing. Findings are classified by
the consequence of the silent fallthrough, not merely by the surface pattern
(several of the `let _ = …` / `.ok()` sites are also counted by the scanner,
but their _semantic_ severity is what matters here).

## Summary

| Severity | Count |
| -------- | ----- |
| HIGH     | 0     |
| MEDIUM   | 0     |
| LOW      | 1     |

## LOW

### `src/semantic_analysis/sqlite_symbol_management.rs` — `define_symbol` discards the real DB error detail

```rust
if let Err(_e) = stmt.execute(… ) { return Err("… is already defined …"); }
```

- **Why it silently falls through:** The underlying SQLite error (`_e`) is
  dropped and replaced with a generic "already defined" message. The error is
  still _reported_ (so it is not a silent fallback), but the true cause (a
  constraint violation, disk error, malformed value) is hidden, which can
  misdirect the user.

## Intentional crashes (`todo!` / `unimplemented!` / `unreachable!`)

These are LOUD by design and are listed only to show remaining incomplete work,
**not** as silent errors. Total: **44** (0 `todo!`, 8 `unimplemented!`, 36
`unreachable!`). Line numbers are approximate and may drift as the source moves.

### `unimplemented!` — 8 (all in `src/ast/visitor.rs`, default trait methods)

- `src/ast/visitor.rs:216` — `visit_move_on` — visitor default not implemented.
- `src/ast/visitor.rs:221` — `visit_missing_code` — visitor default not implemented.
- `src/ast/visitor.rs:227` — `visit_release` — must return `T`.
- `src/ast/visitor.rs:232` — `visit_on_exit` — must return `T`.
- `src/ast/visitor.rs:243` — `visit_test_suite` — must return `T`.
- `src/ast/visitor.rs:250` — `visit_test_case` — must return `T`.
- `src/ast/visitor.rs:256` — `visit_allocate` — must return `T`.
- `src/ast/visitor.rs:271` — `visit_match` — must return `T`.

### `unreachable!` — 36

`src/interpreter.rs` (13):

- `:141` — `alloc_log` — alignment 8 is always valid.
- `:163` — `dealloc_log` — alignment 8 is always valid.
- `:2380` — `checked_signed_arith` — non-signed-integer type.
- `:2409` — `checked_unsigned_arith` — non-unsigned-integer type.
- `:2427` — `checked_unsigned_arith` — unsigned negation invalid.
- `:2469`, `:2473` — `checked_binary` — expected signed integer operand.
- `:2479`, `:2483` — `checked_binary` — expected unsigned integer operand.
- `:2488` — `checked_binary` — non-integer type.
- `:2496`, `:2501` — `checked_neg_value` — expected integer operand / non-signed type.
- `:4121` — `CallVoid` with non-external function.

`src/ir/parser.rs` (6):

- `:387` — `parse_type` — unknown vec type.
- `:804`, `:908` — `parse_value_instruction` — unknown instruction tag.
- `:1178` — `binary_instruction` — unknown op.
- `:1185` — `unary_instruction` — unknown op.
- `:1203` — `conversion_instruction` — unknown op.

`src/ir_gen.rs` (6):

- `:887` — `eval_const` — unknown boolean operator.
- `:1588` — `load_stdlib_into_module` — `StdlibLevel::None` unexpected.
- `:5077` — `generate_unary_expr` — `#count of` on non-array (should be rejected).
- `:5252` — `generate_unary_op` — type-query ops should be handled elsewhere.
- `:6229` — `generate_fn_call_expr` — unknown call arm.
- `:7882` — `value_to_text_ptr` — unknown vec dim.

`src/semantic_analysis/analyzer.rs` (8):

- `:1542` — `validate_literal_range` — unknown unsigned type.
- `:2014` — `expr_type_with_context` — unknown inner arm.
- `:7576`, `:7595`, `:7601`, `:7615`, `:7639`, `:7657` — `process_statement` — unevaluable `#if`/`#when`/`#match`/`#switch` condition/value reaching IR generation.

`src/semantic_analysis/const_eval.rs` (3):

- `:352`, `:362`, `:374` — `fold_unsigned_binary` — only `Div`/`Rem` (or `Add`/`Sub`/`Mul`) reach these arms.

## `.unwrap()` / `.expect()` outside tests

- **Library crate** (`src/lib.rs`) denies `clippy::unwrap_used` and
  `clippy::expect_used` in non-test code via `#![cfg_attr(not(test), deny(…))]`.
  Every non-test `.unwrap()`/`.expect()` in the library is covered by a scoped
  `#[allow(clippy::unwrap_used)]`/`#[allow(clippy::expect_used)]` with a
  documented reason, so none violate the deny:
  - `src/interpreter.rs` — `log.lock().expect("mutex poisoned")` (thread-local
    mutex poisoning), inline-allowed at each site.
  - `src/ir/builder.rs`, `src/ir/function.rs` — "Invariant: …" `.expect()` on
    builder/function invariants.
  - `src/ir_gen.rs:5973` — `.unwrap()` guarded by `len() == 1`, with
    `#[allow(clippy::unwrap_used)]`.
  - `src/semantic_analysis/sqlite_symbol_management.rs` —
    `.prepare_cached(…).expect("Failed to prepare …")` and
    `.execute(…).expect("…")`, each method carrying
    `#[allow(clippy::expect_used)]` ("panic indicates corruption").
- **Binary crate** `src/main.rs` is a _separate_ crate not covered by the
  library's `#![deny]`, and contains two `.expect()` calls with no individual
  `#[allow]`:
  - `src/main.rs:538` — `compilation_order().expect("No cycles (already checked)")`.
  - `src/main.rs:613` — `graph.get(module_id).expect("Module exists")`.

  Both crash loudly with a documented reason, so they are not silent fallbacks —
  but they sit outside the crate-wide deny scope, which is worth noting.

## Notes

- Previous HIGH/MEDIUM findings (silent type-default fallbacks, DB error
  discards that dropped symbols/types, and uninitialized-data fallbacks) have
  been resolved: each now either propagates the error, logs a diagnostic via
  `eprintln!`, crashes via `ice!`/`unwrap_or_panic`, or is documented in a code
  comment as an intentional best-effort discard. The only remaining finding is
  the `define_symbol` error-detail discard listed above.
- `src/semantic_analysis/analyzer.rs` (`try_extract_constant_int`) and
  `src/unit_analysis/mod.rs` (`try_extract_int_value`) return `None` via
  `.ok()` on a failed hex parse. These are defensible: the hex literal is
  grammar-validated before this point, so the parse cannot fail in practice, and
  the `Option` return is the documented contract.
- The `if let Err(e) = self.symbols.define_function(…) { self.add_error(…) }`
  sites throughout `src/semantic_analysis/analyzer.rs` were checked and all
  correctly call `add_error` before continuing — no silent early return found in
  the symbol/function/variable registration paths.
- No `todo!` macros remain in `src/`; remaining incomplete work is expressed as
  `unimplemented!` (visitor trait defaults) and `unreachable!` (defensive
  invariants).
