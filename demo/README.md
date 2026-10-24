# Lale Browser Playground

A tiny web page that runs the **full Lale compiler + interpreter** in the
browser. It is compiled to WebAssembly for the WASI target, so the compiler
runs entirely client-side: no user code is ever executed on a server, and the
page can be hosted from any static file server.

The page shows two panels — type Lale source on the left, click **Run** (or
press `Ctrl`/`Cmd` + `Enter`), and the program's output appears on the right.

## How it works

1. The `lale` CLI is compiled to `wasm32-wasip1` (`lale.wasm`).
2. `main.js` loads that module with `@wasmer/wasi`, a WASI runtime that runs in
   the browser.
3. Your source is fed to the compiler through WASI stdin (`lale run -`), and
   the interpreter's output is captured from WASI stdout/stderr.
4. An in-memory filesystem is mounted into the WASI sandbox. It holds the
   standard library and any files the program reads or writes, so `use std.full`
   and file I/O work in the browser.

## Build output

`build.sh` produces exactly three files in `demo/dist/`:

```text
dist/
  index.html   — the page (UI + styles)
  lale.wasm    — the Lale compiler as a WASI module
  main.js      — bundled JS (the WASI runtime is embedded inside)
```

## Prerequisites

- **Rust** with the `wasm32-wasip1` target installed:

  ```sh
  rustup target add wasm32-wasip1
  ```

- **A WASI C library** (wasi-libc) and a wasm-capable clang, because the
  compiler links SQLite (via `rusqlite`/`libsqlite3-sys`):

  | System        | Install command                                     |
  | ------------- | --------------------------------------------------- |
  | macOS         | `brew install wasi-libc llvm`                       |
  | Debian/Ubuntu | `sudo apt-get install wasi-libc clang`              |
  | Fedora        | `sudo dnf install wasi-libc clang`                  |
  | other         | [wasi-sdk](https://github.com/WebAssembly/wasi-sdk) |

  On macOS the bundled Apple clang lacks the `wasm32-wasip1` wiring, so
  Homebrew `llvm` is required there.

- **Node.js + npm** (to bundle the JS):

  | System        | Install command                   |
  | ------------- | --------------------------------- |
  | macOS         | `brew install node`               |
  | Debian/Ubuntu | `sudo apt-get install nodejs npm` |
  | Fedora        | `sudo dnf install nodejs npm`     |
  | other         | [Node.js](https://nodejs.org/)    |

The build script detects the sysroot, header directory, and clang for you. To
see what it found without building, run:

```sh
./demo/build.sh --dry-run
```

If your layout is non-standard, override the detected locations:

```sh
WASI_SYSROOT=/path/to/wasi-sysroot \
WASI_INCLUDE=/path/to/wasi-sysroot/include \
LALE_CC=/path/to/clang \
  ./demo/build.sh
```

`WASI_INCLUDE` is optional — when it is unset the script derives it by locating
`stdio.h` under the sysroot.

## Build & run

```sh
./demo/build.sh            # builds dist/index.html, dist/lale.wasm, dist/main.js
npx --yes serve demo/dist  # serve it, then open the printed URL
```

Alternatively, serve `demo/dist/` with any static file server:

```sh
cd demo/dist && python3 -m http.server 8000
```

## Limitations

- Files live in an **in-memory filesystem**, not the host disk. Lale programs
  can read and write files (and `import` standard-library modules), but the
  filesystem is recreated **fresh for each run**, so files do not persist
  between runs and nothing is ever written to the host machine. The standard
  library is pre-seeded under `/lale`, and a `/sandbox` directory is available
  for scratch writes.
- Two `@wasmer/wasi` limitations shape the above: a filesystem instance cannot
  be reused across runs (it raises a "recursive use of an object" error), and
  JavaScript cannot read a file back after a program has written it (a
  `wasmer_vfs` lock error). File contents are therefore only observable from
  inside the running program.
- `__lale_read_uptime()` uses a **monotonic** clock in the browser (WASI's
  `CLOCK_MONOTONIC`). It does not include host suspend time and is measured from
  the runtime's start rather than a real OS boot — a documented platform
  limitation (see `../doc/lale.md` § Boot-Relative Time). Its granularity is
  millisecond-scale rather than true nanosecond precision, so sub-millisecond
  intervals read as 0.
- The first run downloads ~a few MB (the compiler + SQLite). Subsequent runs
  are instant since the module stays loaded in memory.
- The page must be served over **HTTP** (e.g. `python3 -m http.server` or
  `npx serve`). Opening `index.html` directly from `file://` will not work,
  because the browser blocks `fetch()` on `file://` URLs.
