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
  WASI_INCLUDE  Path to the WASI C library headers (auto-derived from the
                sysroot when not given).
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
#   - A WASI C library (wasi-libc) and a wasm-capable clang
#   - Node.js + npm

# Resolve the repository root (this script lives in demo/).
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO="$ROOT/demo"
DIST="$DEMO/dist"

# --- Configuration (all overridable via environment variables) ---
OS="$(uname -s)"

# ---------------------------------------------------------------------------
# WASI toolchain detection
# ---------------------------------------------------------------------------
#
# Building the compiler for wasm32-wasip1 links SQLite (via libsqlite3-sys),
# which needs a C compiler plus a WASI C library (wasi-libc). Their locations
# differ across operating systems and package managers, so we detect them here
# instead of assuming a single layout.
#
# The reliable marker for "this directory really is a WASI sysroot" is whether
# it contains <stdio.h> somewhere under its include/ tree. Merely checking that
# a directory exists is not enough — some systems have an empty or partial
# /usr/lib/wasm32-wasi that looks like a sysroot but has no headers. Everything
# below therefore reduces to: find a directory that actually holds stdio.h.

# Given a sysroot, print the include directory that contains stdio.h (nothing
# if the sysroot has no such header). Checks the common layouts first, then
# falls back to a bounded search for a header anywhere under include/.
find_include_in_sysroot() {
  local root="$1" sub found
  for sub in include/wasm32-wasip1 include/wasm32-wasi include/wasm32-wasip2 include; do
    if [ -f "$root/$sub/stdio.h" ]; then
      printf '%s\n' "$root/$sub"
      return 0
    fi
  done
  # Some distros nest the headers more deeply than the common layouts above.
  found="$(find "$root" -maxdepth 6 -type f -name stdio.h -path '*/include/*' 2>/dev/null | head -n 1 || true)"
  if [ -n "$found" ]; then
    printf '%s\n' "$(dirname "$found")"
    return 0
  fi
  return 1
}

# Resolve the sysroot and include directory. An explicit WASI_SYSROOT override
# is honoured, but still validated against stdio.h; otherwise known candidates
# are tried, then a bounded search of common roots.
resolve_wasi_sysroot() {
  local inc

  if [ -n "${WASI_SYSROOT:-}" ]; then
    inc="$(find_include_in_sysroot "$WASI_SYSROOT" || true)"
    if [ -n "$inc" ]; then
      WASI_INCLUDE="${WASI_INCLUDE:-$inc}"
      return 0
    fi
    # The user pointed us at a sysroot without headers. Fall through so the
    # final validation reports the mismatch with clear guidance.
    return 1
  fi

  local candidates=()
  case "$OS" in
    Darwin)
      candidates=( \
        /opt/homebrew/opt/wasi-libc/share/wasi-sysroot \
        /usr/local/opt/wasi-libc/share/wasi-sysroot \
      )
      ;;
    *)
      candidates=( \
        /usr/share/wasi-sysroot \
        /usr/lib/wasm32-wasi \
        /usr/wasm32-wasi \
        /opt/wasi-sdk/share/wasi-sysroot \
        "$HOME/.wasi-sdk/share/wasi-sysroot" \
        "$HOME/.nix-profile/share/wasi-sysroot" \
      )
      ;;
  esac

  local candidate
  for candidate in "${candidates[@]}"; do
    inc="$(find_include_in_sysroot "$candidate" || true)"
    if [ -n "$inc" ]; then
      WASI_SYSROOT="$candidate"
      WASI_INCLUDE="$inc"
      return 0
    fi
  done

  # Fallback: search common roots for a sysroot-like directory that really
  # contains stdio.h. Bounded in depth so it stays fast.
  local root d discovered=""
  for root in /usr /opt "$HOME/.nix-profile" "$HOME/.local" "$HOME"; do
    [ -d "$root" ] || continue
    while IFS= read -r d; do
      [ -n "$d" ] || continue
      if [ -n "$(find_include_in_sysroot "$d" || true)" ]; then
        discovered="$d"
        break
      fi
    done < <(find "$root" -maxdepth 6 -type d \( -name 'wasi-sysroot' -o -name 'wasm32-wasi' \) 2>/dev/null || true)
    [ -z "$discovered" ] || break
  done

  if [ -n "$discovered" ]; then
    WASI_SYSROOT="$discovered"
    WASI_INCLUDE="$(find_include_in_sysroot "$discovered" || true)"
    return 0
  fi
  return 1
}

# Resolve a wasm-capable clang. On macOS the bundled clang lacks the
# wasm32-wasip1 wiring, so Homebrew LLVM is required.
resolve_lale_cc() {
  if [ -n "${LALE_CC:-}" ]; then
    return 0
  fi
  local candidate
  case "$OS" in
    Darwin)
      for candidate in /opt/homebrew/opt/llvm/bin/clang /usr/local/opt/llvm/bin/clang; do
        if [ -x "$candidate" ]; then
          LALE_CC="$candidate"
          return 0
        fi
      done
      ;;
    *)
      candidate="$(command -v clang 2>/dev/null || true)"
      if [ -n "$candidate" ] && [ -x "$candidate" ]; then
        LALE_CC="$candidate"
        return 0
      fi
      ;;
  esac
  return 1
}

resolve_wasi_sysroot || true
resolve_lale_cc || true

# Report what was detected and stop without building.
if [ "$DRY_RUN" = 1 ]; then
  echo "Operating system:    $OS"
  echo "WASI sysroot:        ${WASI_SYSROOT:-<not found>}"
  echo "WASI include:        ${WASI_INCLUDE:-<not found>}"
  echo "C compiler (clang):  ${LALE_CC:-<not found>}"
  if [ -n "${WASI_INCLUDE:-}" ] && [ -f "${WASI_INCLUDE:-}/stdio.h" ]; then
    echo "stdio.h found:       yes"
  else
    echo "stdio.h found:       no"
  fi
  echo "(dry run — nothing was built)"
  exit 0
fi

# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------

print_wasi_hint() {
  echo "  macOS (Homebrew):  brew install wasi-libc llvm" >&2
  echo "  Debian / Ubuntu:   sudo apt-get install wasi-libc clang" >&2
  echo "  Fedora:            sudo dnf install wasi-libc clang" >&2
  echo "  or install wasi-sdk: https://github.com/WebAssembly/wasi-sdk" >&2
  echo "  then set WASI_SYSROOT=/path/to/wasi-sysroot and LALE_CC=/path/to/clang" >&2
}

if [ -z "${WASI_SYSROOT:-}" ] || [ -z "${WASI_INCLUDE:-}" ] || [ ! -f "${WASI_INCLUDE:-}/stdio.h" ]; then
  echo "error: could not find a WASI sysroot with its C headers (wasi-libc)." >&2
  echo "  Detected sysroot:  ${WASI_SYSROOT:-<none>}" >&2
  echo "  Detected include:  ${WASI_INCLUDE:-<none>}" >&2
  print_wasi_hint
  exit 1
fi

if [ -z "${LALE_CC:-}" ] || [ ! -x "${LALE_CC:-}" ]; then
  echo "error: no WebAssembly-capable clang found." >&2
  print_wasi_hint
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
