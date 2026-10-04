# Contributing to Lale

Thanks for your interest in contributing to Lale! This guide explains how to get started, how to submit changes, and the conventions the project follows.

[Website](https://lale-lang.dev) · [README](README.md) · [User Guide](doc/lale.md) · [Architecture](doc/ARCHITECTURE.md)

---

## Ways to contribute

You don't have to write compiler code to help. Valuable contributions include:

- Reporting bugs and surprising behavior
- Improving or translating documentation and examples
- Adding or refining tests
- Working on the standard library, language server, or development tooling
- Proposing and implementing language features

Early feedback is especially valuable while the language and toolchain are still evolving. A clear bug report or a well-scoped suggestion is a real contribution.

## Getting started

### Prerequisites

- [Rust 1.85+](https://www.rust-lang.org/tools/install)

### Build and test

```bash
git clone https://github.com/kkayal/lale.git
cd lale

# Build everything (compiler + LSP server)
cargo build --workspace

# Type check without building
cargo check --workspace

# Run the test suite
cargo test --workspace
```

Always run `cargo build --workspace` **before** `cargo test --workspace`. `cargo test` enables `#[cfg(test)]`, which can hide dead-code warnings and unresolved imports in non-test code.

To build the optional subprojects (the browser playground and the editor
extensions), see
[Building the optional extras](README.md#building-the-optional-extras) in the
README. Those need tools beyond Rust: WebAssembly targets, `wasi-libc`, `clang`,
and Node.js.

### Run a program

```bash
cargo run -- run examples/complex.lale
```

A release build is faster for repeated use:

```bash
cargo build --release --workspace
./target/release/lale run hello.lale
```

For a guided introduction, see the [Lale User Guide](doc/lale.md).

## Code style

- Follow the existing patterns in the codebase.
- Use `rustfmt` (configured in `rustfmt.toml`).
- Avoid adding new `.unwrap()` or `.expect()` calls in non-test code; prefer `Result`-based error handling in critical paths.

## Linting

All clippy warnings and errors must be resolved before a change is merged:

```bash
cargo clippy --workspace -- -D warnings
```

Denied lints (`clippy::unwrap_used`, `clippy::expect_used`) are crate-wide. Each suppression must be scoped to the function or block where it is intentional and documented — never crate-wide.

## Documentation

Two kinds of rules apply to documentation: **formatting** and **voice**.

### Formatting

Markdown is formatted with `markdownlint-cli2` and `prettier`, in that order:

```bash
markdownlint-cli2 --fix doc/*.md README.md CONTRIBUTING.md
npx prettier --write doc/*.md README.md CONTRIBUTING.md
```

ASCII art and other whitespace-sensitive content must be enclosed in a fenced code block with the `text` language identifier so the formatters leave it alone.

### Documentation philosophy

Lale follows an **educational documentation philosophy**. The guiding principle is:

> **Never use specialist vocabulary as a prerequisite for understanding the explanation of that vocabulary.**

Lale is intended to be approachable to people who are technically capable but are not professional software engineers or compiler experts — an engineer, scientist, technician, or a technically minded student. They may understand units, equations, measurements, and data structures without knowing terms such as _SSA_, _ABI_, _borrow checking_, or _IR lowering_.

| Layer                        | Target                            | Style                                               |
| ---------------------------- | --------------------------------- | --------------------------------------------------- |
| User Guide (`doc/lale.md`)   | High-school student               | Concrete, assumes little prior programming          |
| Code comments + Architecture | Technically capable non-CS reader | Technical terms allowed, but explained on first use |
| Source code itself           | Experienced developer             | Conventional technical notation; do not simplify    |

**Wrong**:

> Lale lowers each function to a CFG of basic blocks.

**Correct**:

> The compiler turns each function into a _control-flow graph (CFG)_ — a set of _basic blocks_ (straight-line instruction sequences with no internal branches) connected by branches. Each block ends in a branch to another block or a return from the function.

The second version is no less technical — it is _more welcoming without sacrificing accuracy_. The goal is not to make the subject less sophisticated, but to make the **path into the subject shorter**.

The rule is deliberately not "every sentence must be readable by a beginner." A serious compiler will always need terms such as SSA, ABI, UTF-8, and calling conventions. The rule is only that such a term is explained when it is first introduced, then used precisely thereafter.

This policy also supports the project's long-term health. A technically ambitious open-source project can become understandable only to its original authors. Writing the architecture so that a curious newcomer can follow it keeps knowledge from concentrating in one person:

> **Accessible language → accessible documentation → accessible implementation → easier contributions.**

The source code itself is **not** simplified — this philosophy applies to the surrounding prose and comments, not to the implementation.

## Submitting changes

1. Open an issue first for anything larger than a small fix, so the approach can be discussed.
2. Keep changes focused and minimal.
3. Add or update tests for the change.
4. Be respectful and constructive in issues and reviews.
5. Run the full build and test sequence before opening the pull request:

   ```bash
   cargo build --workspace && cargo test --workspace
   ```

## For AI coding agents

Agent instructions live in [`AGENTS.md`](AGENTS.md). If you are an AI agent working on this codebase, read that file first — it contains operating rules (git is read-only, "check" means analyze-only, and compiler-internal conventions) that take precedence over this guide.

## License

Lale is distributed under the GPLv3 License. See [`LICENSE.txt`](LICENSE.txt) for details. Contributions are assumed to be made under the same license.
