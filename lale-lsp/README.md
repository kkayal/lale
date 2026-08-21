# Lale Language Server (LSP)

A [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) implementation for the **Lale programming language**.

## Features

| Feature              | Description                                                                           |
| -------------------- | ------------------------------------------------------------------------------------- |
| **Diagnostics**      | Real-time error and warning reporting via the compiler's parser and semantic analyzer |
| **Completion**       | Keyword, type, compiler-constant, and snippet completions                             |
| **Hover**            | Type information and documentation on hover                                           |
| **Go to Definition** | Jump to variable, function, and type definitions                                      |
| **Document Symbols** | Outline view showing functions, types, and variables                                  |
| **Semantic Tokens**  | Syntax-highlighting improvements (keywords, types, functions, etc.)                   |

## Architecture

The server reuses the Lale compiler library (`lale` crate) directly — no separate parser or type checker:

1. **Parse** — `pest` grammar → parse tree
2. **AST** — `ast::builder::build_program()` → semantic AST
3. **Analyze** — `semantic_analysis::analyze_ast()` → errors + warnings
4. **Report** — Errors mapped to LSP `Diagnostic` messages with precise source locations

## Building everything

```bash
# 1. Build the LSP server binary
cd lale-lsp
cargo build --release

# 2. Build & install the Zed extension (grammar + LSP adapter)
cd ../zed-extension
./build.sh
```

The Zed extension is installed to `~/Library/Application Support/Zed/extensions/installed/lale/`.

## Editor Integration

### Zed

The project ships with `.zed/settings.json` that configures the LSP binary path
using `${ZED_WORKTREE_ROOT}` — it works out of the box on any machine.

The LSP binary is built alongside the compiler via `cargo build --workspace`
from the project root (the `lale-lsp` crate is a workspace member). No separate
build step is needed.

**After code changes** to the compiler, run `cargo build --workspace` and
restart the LSP in Zed (`lsp: restart`).

### VS Code

1. Build the LSP binary: `cd lale-lsp && cargo build --release`
2. Ensure `lale-lsp` is on your `PATH`, or set `lale.lsp.path` in VS Code settings
3. The `vs-code-extensions/lale` extension auto-launches the server

```jsonc
// .vscode/settings.json
{
  "lale.lsp.path": "/path/to/lale-lsp"
}
```

### Other Editors (Neovim, Helix, Emacs, etc.)

Any editor with LSP support can use this server. Configure it to launch `lale-lsp` with stdio transport for `.lale` and `.a` files.

## Project Layout

```
lale/
├── lale-lsp/                    # LSP server binary (Rust)
│   ├── src/main.rs              # Entry point
│   └── src/server.rs            # LSP backend (diagnostics, completion, hover, ...)
│
├── zed-extension/               # Zed extension source
│   ├── build.sh                 # One-command build & install
│   ├── extension.toml           # Extension manifest
│   ├── Cargo.toml               # Rust LSP adapter crate
│   ├── src/lib.rs               # Tells Zed how to launch lale-lsp
│   ├── languages/lale/
│   │   ├── config.toml          # Language config (file types, brackets, ...)
│   │   └── highlights.scm       # Syntax highlighting rules
│   └── grammars/lale/
│       └── grammar.js           # Tree-sitter grammar source
│
└── vs-code-extensions/lale/     # VS Code extension
    ├── package.json             # Extension manifest + LSP client config
    ├── src/extension.ts         # LSP client (launches lale-lsp)
    └── syntaxes/                # TextMate grammar for syntax highlighting
```

## Dependencies

- **`tower-lsp`** — LSP protocol framework for Rust
- **`lale`** — The Lale compiler library (path dependency, reuses parser + analyzer)
- **`pest`** — PEG parser (shared with the compiler)
- **`dashmap`** — Concurrent document cache
