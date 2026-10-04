#!/bin/bash
# build.sh — Build the Lale VS Code extension.
# Compiles the TypeScript LSP client into out/extension.js.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# --- Prerequisite checks (fail fast) ---
if ! command -v node >/dev/null 2>&1; then
  echo "error: 'node' (Node.js) is required to build the VS Code extension." >&2
  echo "  macOS (Homebrew):  brew install node" >&2
  echo "  Debian / Ubuntu:   sudo apt-get install nodejs npm" >&2
  echo "  Fedora:            sudo dnf install nodejs npm" >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "error: 'npm' is required to build the VS Code extension." >&2
  echo "  macOS (Homebrew):  brew install node" >&2
  echo "  Debian / Ubuntu:   sudo apt-get install nodejs npm" >&2
  echo "  Fedora:            sudo dnf install nodejs npm" >&2
  exit 1
fi

cd "$SCRIPT_DIR"
npm install
npm run compile

echo
echo "Built the VS Code extension into: $SCRIPT_DIR/out"
