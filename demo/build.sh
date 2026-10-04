#!/usr/bin/env bash
set -euo pipefail

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      cat << 'EOF'
Usage: ./build.sh [--dry-run] [--help]

Builds the Lale browser playground (demo/dist/index.html, lale.wasm, main.js).

Options:
  --dry-run  Print the detected toolchain paths and exit without building.
  --help     Show this message.

Environment (all optional — the script detects sensible defaults):
  WASI_SYSROOT  Path to the WASI C library sysroot.
  WASI_INCLUDE  Path to the WASI C library headers.
  LALE_CC       Path to a WebAssembly-capable clang.
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

# Build the Lale browser playground into demo/dist/:
#   index.html + lale.wasm + main.js
#
# Prerequisites (see demo/README.md):
#   - Rust toolchain with the `wasm32-wasip1` target installed
#   - A WASI C library (Homebrew `wasi-libc`) and a wasm-capable clang
#   - Node.js + npm

# Resolve the repository root (this script lives in demo/).
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO="$ROOT/demo"
DIST="$DEMO/dist"

# --- Configuration (all overridable via environment variables) ---
OS="$(uname -s)"

# Default WASI sysroot and compiler depend on the operating system. Override
# either with an environment variable when the defaults do not match your setup:
#   WASI_SYSROOT=/path/to/wasi-sysroot LALE_CC=/path/to/clang ./build.sh
if [ -z "${WASI_SYSROOT:-}" ]; then
  case "$OS" in
    Darwin)
      for candidate in \
        /opt/homebrew/opt/wasi-libc/share/wasi-sysroot \
        /usr/local/opt/wasi-libc/share/wasi-sysroot; do
        if [ -d "$candidate" ]; then
          WASI_SYSROOT="$candidate"
          break
        fi
      done
      ;;
    *)
      for candidate in \
        /usr/share/wasi-sysroot \
        /usr/lib/wasm32-wasi \
        /opt/wasi-sdk/share/wasi-sysroot; do
        if [ -d "$candidate" ]; then
          WASI_SYSROOT="$candidate"
          break
        fi
      done
      ;;
  esac
fi

# Use a dedicated variable rather than the generic CC, which some shells export
# globally (and may point at a nonexistent binary).
if [ -z "${LALE_CC:-}" ]; then
  case "$OS" in
    Darwin)
      # Homebrew LLVM is needed on macOS: Apple's bundled clang lacks the
      # wasm32-wasip1 target wiring this build relies on.
      for candidate in /opt/homebrew/opt/llvm/bin/clang /usr/local/opt/llvm/bin/clang; do
        if [ -x "$candidate" ]; then
          LALE_CC="$candidate"
          break
        fi
      done
      ;;
    *)
      LALE_CC="$(command -v clang 2>/dev/null || true)"
      ;;
  esac
fi

if [ -n "$WASI_SYSROOT" ]; then
  WASI_INCLUDE="${WASI_INCLUDE:-$WASI_SYSROOT/include/wasm32-wasip1}"
fi

# Report what was detected and stop without building.
if [ "$DRY_RUN" = 1 ]; then
  echo "Operating system:    $OS"
  echo "WASI sysroot:        ${WASI_SYSROOT:-<not found>}"
  echo "WASI include:        ${WASI_INCLUDE:-<not found>}"
  echo "C compiler (clang):  ${LALE_CC:-<not found>}"
  echo "(dry run — nothing was built)"
  exit 0
fi

if [ -z "$WASI_SYSROOT" ]; then
  echo "error: could not find a WASI sysroot (wasi-libc)." >&2
  echo "  macOS:          brew install wasi-libc" >&2
  echo "  Debian/Ubuntu:  sudo apt-get install wasi-libc" >&2
  echo "  or set WASI_SYSROOT=/path/to/wasi-sysroot" >&2
  exit 1
fi

if [ -z "$LALE_CC" ] || [ ! -x "$LALE_CC" ]; then
  echo "error: no WebAssembly-capable clang found." >&2
  echo "  macOS:          brew install llvm" >&2
  echo "  Debian/Ubuntu:  sudo apt-get install clang" >&2
  echo "  or set LALE_CC=/path/to/clang" >&2
  exit 1
fi

cd "$ROOT"

# --- 1. Compile the Lale compiler to a WASI wasm module (release) ---
WASI_SYSROOT="$WASI_SYSROOT" \
  CFLAGS_wasm32_wasip1="-isystem $WASI_INCLUDE" \
  CC="$LALE_CC" \
  cargo build --bin lale --release --target wasm32-wasip1

# --- 2. Copy the wasm into dist/ ---
mkdir -p "$DIST"
cp "$ROOT/target/wasm32-wasip1/release/lale.wasm" "$DIST/lale.wasm"

# Shrink further if Binaryen's wasm-opt is available.
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o "$DIST/lale.wasm" "$DIST/lale.wasm"
fi

# --- 3. Bundle main.js (inlines @wasmer/wasi, which embeds its own wasm) ---
cd "$DEMO"
npm install
npm run bundle

# --- 4. Copy the HTML shell ---
cp "$DEMO/src/index.html" "$DIST/index.html"

echo
echo "Built the Lale playground into: $DIST"
echo "Serve it locally with:  npx --yes serve "$DIST""
