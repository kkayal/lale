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

- The browser build reads source from stdin and writes to stdout/stderr. Lale
  programs that read/write **files** or `import` **modules** from disk are not
  supported here, because no filesystem is mounted into the WASI sandbox.
- The first run downloads ~a few MB (the compiler + SQLite). Subsequent runs
  are instant since the module stays loaded in memory.
- The page must be served over **HTTP** (e.g. `python3 -m http.server` or
  `npx serve`). Opening `index.html` directly from `file://` will not work,
  because the browser blocks `fetch()` on `file://` URLs.
