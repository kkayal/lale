#!/bin/bash
# build.sh — Build the Lale Zed extension and install it.
# Run from zed-extension/ directory.
set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      cat << 'EOF'
Usage: ./build.sh [--dry-run] [--help]

Builds the Lale Zed extension to WebAssembly and installs it into Zed.

Options:
  --dry-run  Print the detected install directory and exit without building.
  --help     Show this message.

Environment:
  LALE_ZED_INSTALL_DIR  Override the Zed extension install directory.
EOF
      exit 0
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    *)
      echo "error: unknown argument '$arg' (run './build.sh --help')" >&2
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Where to install the extension. The default is Zed's per-OS extension
# directory; override it explicitly when your setup differs:
#   LALE_ZED_INSTALL_DIR=/path/to/zed/extensions/installed/lale ./build.sh
if [ -n "${LALE_ZED_INSTALL_DIR:-}" ]; then
  INSTALL_DIR="$LALE_ZED_INSTALL_DIR"
else
  case "$(uname -s)" in
    Darwin)
      INSTALL_DIR="$HOME/Library/Application Support/Zed/extensions/installed/lale"
      ;;
    Linux)
      INSTALL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zed/extensions/installed/lale"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      INSTALL_DIR="${LOCALAPPDATA:-$HOME/AppData/Local}/Zed/extensions/installed/lale"
      ;;
    *)
      echo "error: unknown operating system '$(uname -s)'; set LALE_ZED_INSTALL_DIR to your Zed extensions/installed directory" >&2
      exit 1
      ;;
  esac
fi

if [ "$DRY_RUN" = 1 ]; then
  echo "Install directory: $INSTALL_DIR"
  echo "(dry run — nothing was built or installed)"
  exit 0
fi

GRAMMAR_SRC="$SCRIPT_DIR/grammars/lale"
GRAMMAR_INSTALL="$INSTALL_DIR/grammars/lale"

echo "=== Lale Zed Extension Builder ==="

# ---- Step 1: Generate tree-sitter parser ----
echo "● Generating tree-sitter parser..."
cd "$GRAMMAR_SRC"
if [ ! -d node_modules ]; then
    echo "  Installing tree-sitter dependencies..."
    npm install
fi
npx tree-sitter generate 2>&1

# ---- Step 2: Compile tree-sitter grammar to WASM ----
echo "● Compiling grammar to WASM..."
npx tree-sitter build --wasm 2>&1

# ---- Step 3: Build Rust LSP adapter to WASM ----
echo "● Building LSP adapter (Rust → WASM)..."
cd "$SCRIPT_DIR"
cargo build --target wasm32-wasip2 --release 2>&1

# ---- Step 4: Install ----
echo "● Installing to $INSTALL_DIR ..."
rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR/grammars" "$INSTALL_DIR/languages/lale" "$INSTALL_DIR/src"

# Grammar WASM for syntax highlighting
cp "$GRAMMAR_SRC/tree-sitter-lale.wasm" "$INSTALL_DIR/grammars/lale.wasm"

# Grammar git repo (Zed needs this to compile/validate the grammar)
mkdir -p "$GRAMMAR_INSTALL"
cp "$GRAMMAR_SRC/grammar.js" "$GRAMMAR_INSTALL/"
cp "$GRAMMAR_SRC/package.json" "$GRAMMAR_INSTALL/"
cd "$GRAMMAR_INSTALL"
if [ ! -d .git ]; then
    git init -q
fi
git add -A
GIT_COMMITTER_DATE="2024-01-01 00:00:00" git commit -m "Update" --date="2024-01-01 00:00:00" --allow-empty -q 2>&1 || true
REV=$(git rev-parse HEAD)

# Language config
cp "$SCRIPT_DIR/languages/lale/config.toml" "$INSTALL_DIR/languages/lale/"
cp "$SCRIPT_DIR/languages/lale/highlights.scm" "$INSTALL_DIR/languages/lale/"

# LSP adapter WASM (pre-compiled, Zed loads this instead of compiling from source)
cp "$SCRIPT_DIR/target/wasm32-wasip2/release/lale_zed.wasm" "$INSTALL_DIR/extension.wasm"

# Rust source (kept for reference but Zed uses the pre-compiled extension.wasm)
cp "$SCRIPT_DIR/Cargo.toml" "$INSTALL_DIR/"
cp "$SCRIPT_DIR/src/lib.rs" "$INSTALL_DIR/src/"

# Extension manifest
cat > "$INSTALL_DIR/extension.toml" << ENDTOML
id = "lale"
name = "Lale Language"
version = "0.1.0"
schema_version = 1
languages = ["languages/lale"]

[lib]
kind = "Rust"
version = "0.7.0"

[grammars.lale]
repository = "file://${INSTALL_DIR// /%20}/grammars/lale"
rev = "$REV"

[language_servers.lale-lsp]
name = "Lale Language Server"
language = "Lale"
ENDTOML

echo ""
echo "=== Done. Restart Zed to reload. ==="
echo ""
echo "LSP binary path is configured in .zed/settings.json as a worktree-relative path."
echo "No global settings.json changes are needed."
