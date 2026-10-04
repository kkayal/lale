# Lale Compiler — Security Audit

**Audit date**: 2026-09-22
**Compiler version**: 0.1.0 (per `Cargo.toml`)
**Status**: Initial audit — completed with limitations (see "Limitations")

This document records a security review of the Lale compiler and its reference
interpreter. It is a living document: re-audit at each release boundary, and
update the "Audit date" and findings when the code changes.

## 1. Scope and method

The audit covers:

- The compiler and interpreter in `src/` (parsing, semantic analysis, IR
  generation, and the runtime interpreter).
- The language server in `lale-lsp/`.
- The project's build and dependency configuration (`Cargo.toml`,
  `Cargo.lock`, `Makefile.toml`).

It does **not** cover the browser playground's JavaScript (`demo/`), the editor
extensions' TypeScript, or third-party grammar tooling, beyond what the compiler
links at build time.

The review combined two techniques:

1. **Manual static analysis** — reading the security-relevant paths: the
   interpreter's memory model, the foreign-function-interface (FFI) boundary,
   file I/O, and every `unsafe` block in Rust source.
2. **Automated tooling** — `cargo clippy --workspace -- -D warnings`, the
   project's own silent-error scanner (`cargo run --bin lale-validate`), and
   `cargo audit` (dependency vulnerabilities).

## 2. Threat model

Lale is a **systems language with explicit safety**, and its reference
implementation is an **interpreter that runs Lale programs with the user's own
privileges** — the same trust model as running a Python or Ruby script. The
compiler is **not a sandbox**: a Lale program can read and write files, allocate
memory, and — through the language's explicit-`unsafe` constructs (`pointer to`,
`value at`, `unsafe bitcast`, `unsafe decl`) — perform raw memory access.

The security boundary is therefore the same one the language draws for its
users: **safe code is checked by the compiler; `unsafe` code is the
programmer's responsibility.** This audit focuses on whether the _compiler and
interpreter themselves_ uphold that boundary and fail safely — not on whether an
arbitrary Lale program can do dangerous things (it can, by design).

Attack surfaces considered:

| Surface                      | Exposure                         | Notes                                                     |
| ---------------------------- | -------------------------------- | --------------------------------------------------------- |
| Compiler CLI (`lale`)        | Local; runs user-supplied source | Parses and interprets untrusted source files              |
| Language server (`lale-lsp`) | Local; stdio transport only      | Input comes from the editor (the user); no network socket |
| Browser playground (`demo/`) | Client-side WASI                 | Runs entirely in the browser; no server                   |
| FFI boundary                 | Local                            | Bridges simulated memory to libc                          |

## 3. Summary

| Severity | Count | Result                  |
| -------- | ----- | ----------------------- |
| Critical | 0     | —                       |
| High     | 0     | —                       |
| Medium   | 2     | §4.2–4.3                |
| Low      | 3     | §4.4–4.6                |
| Info     | —     | Positive controls in §5 |

No critical or high-severity issues were found. The dependency vulnerability
scan (§4.1) completed with **no known vulnerabilities**; the remaining findings
are two Medium and three Low issues described below.

## 4. Findings

### 4.1 Dependency vulnerabilities — none found (resolved)

**Location**: project dependencies (`Cargo.toml`, `Cargo.lock`).

`cargo audit` was run against the RustSec advisory database (1261 advisories
loaded) and scanned all 164 crate dependencies in `Cargo.lock`. It reported
**no known vulnerabilities**, exiting with status 0:

```text
Loaded 1261 security advisories
Scanning Cargo.lock for vulnerabilities (164 crate dependencies)
```

**Impact**: None — no known-vulnerable dependencies are present at the time of
writing.

**Recommendation**: Keep `cargo audit` wired into `cargo make security-check`
and CI so this is re-checked on every change. Any future advisory is
release-blocking.

### 4.2 Unbounded allocation from program-controlled sizes — Medium

**Location**: `src/interpreter.rs`, `call_extern`, the `read` handler
(approximately line 1263).

The `read` extern allocates a buffer whose size comes directly from the Lale
program:

```rust
let count = get_int_arg(args, 2, values, 0) as usize;
...
let mut buf = vec![0u8; count];
```

A Lale program can request an arbitrarily large `count` (for example,
`read(fd, buffer, 9223372036854775807)`), forcing a correspondingly large
allocation. The same pattern appears in `mem_read_bytes`, which loops
`0..count` times for the `write` extern.

**Impact**: Denial of service — memory exhaustion or a long-running loop from a
hostile or buggy program.

**Recommendation**: Bound the buffer size (for example, read in chunks, or
reject sizes above a documented limit), or document the interpreter as
untrusted-input-unsafe. This matters most if Lale programs are ever executed
with untrusted input.

### 4.3 `v3_store` / `v3_load` raw-bytes bridge is incomplete — Medium

**Location**: `src/interpreter.rs`, `v3_store` (≈ line 876) and `v3_load`
(≈ line 911).

These helpers serialize and deserialize `Value`s to and from raw memory using
`unsafe` pointer reads and writes keyed by IR type. The project's own roadmap
flags them as incomplete:

> `v3_store`/`v3_load` raw‑bytes bridge — helpers written but integration
> blocked — needs a metadata system that distinguishes raw‑byte addresses from
> `Value`‑enum addresses.

The risk is **type confusion**: if a raw-byte address is passed where a `Value`
enum is expected (or vice versa), bytes are reinterpreted as the wrong type,
which can corrupt memory or produce an invalid `Value`.

**Impact**: Memory unsafety is possible, but only behind the language's
`unsafe bitcast` / `value at` constructs, which already require the programmer
to opt in.

**Recommendation**: Complete the metadata system that tracks address kinds
(raw bytes vs. `Value` enum), or add a runtime tag check before interpreting an
address. Track progress in `doc/TODO.md` item 23.

### 4.4 Silent error swallowing in the FFI `read` handler — Low

**Location**: `src/interpreter.rs`, `call_extern`, `read` handler.

Reads store each byte with the error discarded:

```rust
let _ = mem.store(buf_base + buf_off + i as i64, Value::Int(b as i64));
```

If the destination region is smaller than the requested count, the failing
stores are silently dropped rather than reported.

**Impact**: Data is silently lost; no error is surfaced to the program or user.

**Recommendation**: Propagate the store result (return an error, or at minimum
log it), consistent with the project's "no silent error acceptance" policy.

### 4.5 Out-of-bounds reads silently truncate — Low

**Location**: `src/interpreter.rs`, `mem_read_bytes` (≈ line 833).

`mem_read_bytes` reads `count` bytes with `filter_map` over `read_byte`, so an
out-of-bounds read simply produces fewer bytes than requested:

```rust
(0..count)
  .filter_map(|i| mem.read_byte(base, offset + i as i64))
  .collect()
```

**Impact**: A `write(fd, ptr, count)` where `count` exceeds the buffer writes a
silently truncated result, with no indication that data was lost.

**Recommendation**: Detect the truncation (compare the resulting length to
`count`) and surface an error or warning.

### 4.6 No runtime bounds checking on pointer arithmetic — Low (by design)

**Location**: `src/ir_gen.rs` (pointer arithmetic lowering) and the interpreter's
`Add`/`Sub` handlers.

Pointer arithmetic (`ptr + n`, `ptr - n`, `ptr1 - ptr2`) is not bounds-checked at
runtime. This is a documented, intentional design decision — the safety model
states "no runtime bounds checking; the user is responsible for validity" — and
is consistent with the language's explicit-safety philosophy.

**Impact**: None for correct programs; out-of-bounds pointer arithmetic is
undefined behavior for the program, exactly as documented.

**Recommendation**: Keep as-is, but ensure the behavior is clearly documented
(it is, in `doc/ARCHITECTURE.md` §4.5.5 and the pointer-arithmetic section of
`doc/lale.md`).

## 5. Security controls already in place

The project already applies several strong controls, which this audit verified:

- **Clippy as a hard gate** — `cargo clippy --workspace -- -D warnings` passes
  clean. The crates deny `clippy::unwrap_used` and `clippy::expect_used` in
  non-test code, forcing explicit error handling instead of panics.
- **Silent-error scanner** — `cargo run --bin lale-validate` reports
  **0 HIGH, 11 MEDIUM, 157 LOW** silent-fallback patterns. The HIGH count (the
  class most likely to hide a security-relevant fallback) is zero, and the
  MEDIUM/LOW counts are tracked so they must not increase.
- **Compile-time numeric checking** — integer overflow/underflow on constant
  operands is rejected at compile time, and non-constant arithmetic uses checked
  operations (`CheckedAdd`/`CheckedMul`) that trap rather than wrap.
- **Array bounds checks** — array indexing lowers to a bounds-check instruction
  that reports out-of-bounds access instead of reading adjacent memory.
- **Memory-region overlap detection** — the interpreter warns when a store
  matches multiple overlapping regions rather than silently using the first.
- **Concentrated, documented `unsafe`** — the `unsafe` Rust is confined almost
  entirely to `src/interpreter.rs` and falls into two justified categories:
  (a) the FFI boundary (copying strings to/from the real heap for the single
  libc call, `puts`), and (b) the raw-bytes bridge in §4.3. The interpreter is
  single-threaded, so the `unsafe impl Send` on the allocation log is sound.

## 6. Limitations

This audit has known gaps that should be closed in future passes:

- **No fuzzing** — the parser and interpreter have not been fuzzed with
  malformed or hostile input.
- **No formal verification** — the `unsafe` blocks were reviewed by inspection,
  not with tools like Miri or a sanitizer.
- **Language server not deeply reviewed** — `lale-lsp` is stdio-only and
  therefore low-risk, but its request handlers were not exhaustively checked for
  panic-on-malformed-input.

## 7. Recommended follow-ups

1. Bound the FFI `read` buffer size (§4.2).
2. Finish the `v3_store`/`v3_load` address-kind metadata (§4.3, TODO item 23).
3. Replace the silent error swallowing in §4.4 and the silent truncation in §4.5.
4. Run the interpreter under Miri or an address sanitizer to validate the
   `unsafe` code, and add parser fuzzing.
