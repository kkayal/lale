#!/bin/bash
# check-gitignore.sh — Verify that every build-required source file is tracked.
#
# The browser playground, Zed extension, and VS Code extension each have build
# scripts that read, copy, or compile a fixed set of source files. If any of
# those files is missing from version control — because it is gitignored, or
# simply never committed — a fresh clone will lack it and the build will fail
# at a late, confusing step (this is exactly what happened with tree-sitter.json).
#
# This check guards against that class of bug: it fails if any file listed
# below is not tracked by Git, and reports whether the cause is a .gitignore
# rule or an uncommitted file.
set -euo pipefail

cd "$(dirname "$0")/.." # repository root

# Not a Git checkout (e.g. a source tarball)? Nothing to verify against.
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "warning: not inside a Git repository; skipping gitignore check" >&2
  exit 0
fi

# Build-required source files. Each of these is consumed by demo/build.sh,
# zed-extension/build.sh, or vs-code-extensions/lale/build.sh and therefore
# must be committed. Build artifacts (node_modules/, target/, *.wasm, dist/)
# are intentionally excluded and must NOT be listed here.
REQUIRED_FILES=(
  # Browser playground
  demo/build.sh
  demo/src/main.js
  demo/src/index.html
  demo/package.json

  # Zed extension
  zed-extension/build.sh
  zed-extension/Cargo.toml
  zed-extension/extension.toml
  zed-extension/src/lib.rs
  zed-extension/grammars/lale/grammar.js
  zed-extension/grammars/lale/package.json
  zed-extension/grammars/lale/tree-sitter.json
  zed-extension/languages/lale/config.toml
  zed-extension/languages/lale/highlights.scm

  # VS Code extension
  vs-code-extensions/lale/build.sh
  vs-code-extensions/lale/package.json
  vs-code-extensions/lale/tsconfig.json
  vs-code-extensions/lale/language-configuration.json
  vs-code-extensions/lale/syntaxes/lale.tmLanguage.json
  vs-code-extensions/lale/src/extension.ts
)

fail=0
for file in "${REQUIRED_FILES[@]}"; do
  if git ls-files --error-unmatch -- "$file" >/dev/null 2>&1; then
    continue
  fi
  if git check-ignore -q -- "$file" 2>/dev/null; then
    echo "error: build-required file is gitignored: $file" >&2
    echo "       remove it from .gitignore so fresh clones get it" >&2
  else
    echo "error: build-required file is not tracked: $file" >&2
    echo "       run: git add $file" >&2
  fi
  fail=1
done

if [ "$fail" = 1 ]; then
  echo >&2
  echo "One or more files the build depends on are missing from version control." >&2
  echo "Commit them so a fresh clone can build successfully." >&2
  exit 1
fi

echo "OK: all build-required files are tracked by Git."
