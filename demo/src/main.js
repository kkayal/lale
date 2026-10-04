import { init, WASI } from "@wasmer/wasi";

// --- Node.js `Buffer` polyfill (browser only) -----------------------------
// @wasmer/wasi's init() decodes its embedded wasm from a base64 data URI using
// `Buffer.from(...)`, a Node.js global. Browsers don't provide `Buffer`, so we
// supply the one method it needs: decoding a base64/ascii string into a
// Uint8Array that WebAssembly.compile() can consume. This runs before init()
// is ever called.
if (typeof globalThis.Buffer === "undefined") {
  globalThis.Buffer = class {
    static from(str, encoding) {
      if (encoding === "base64") {
        const bin = atob(String(str));
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        return bytes;
      }
      // "ascii" (and anything else encountered): each char becomes its low byte.
      const s = String(str);
      const bytes = new Uint8Array(s.length);
      for (let i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i) & 0xff;
      return bytes;
    }
  };
}

const WASM_URL = "lale.wasm";

/**
 * Compile and run a Lale program entirely in the browser.
 *
 * The source string is fed to the compiler through WASI stdin, and the
 * program's output is captured from WASI stdout/stderr. No server-side
 * execution happens anywhere — the compiler runs inside this wasm module.
 *
 * @param {string} source Lale source code.
 * @returns {Promise<{ stdout: string, stderr: string, exitCode: number }>}
 */
export async function runLale(source) {
  // Load @wasmer/wasi's runtime (its wasm is embedded in the bundle).
  await init();

  const wasi = new WASI({
    args: ["lale", "run", "-", "--no-color"],
    env: {},
    preopens: {},
  });

  // Feed the program in through stdin, exactly like `lale run -`.
  wasi.setStdinString(source);

  // Fetch and compile the Lale compiler (a WASI module).
  const response = await fetch(WASM_URL);
  if (!response.ok) {
    throw new Error(`Failed to load ${WASM_URL}: HTTP ${response.status}`);
  }
  const bytes = await response.arrayBuffer();
  const module = await WebAssembly.compile(bytes);

  // Instantiate with the WASI imports, then run to completion.
  await wasi.instantiate(module, {});
  const exitCode = wasi.start();

  return {
    stdout: wasi.getStdoutString(),
    stderr: wasi.getStderrString(),
    exitCode,
  };
}

// --- UI wiring -------------------------------------------------------------

const runButton = document.getElementById("run");
const sourceEl = document.getElementById("source");
const outputEl = document.getElementById("output");
const statusEl = document.getElementById("status");

async function run() {
  runButton.disabled = true;
  statusEl.textContent = "compiling…";
  outputEl.value = "";

  try {
    const { stdout, stderr, exitCode } = await runLale(sourceEl.value);
    outputEl.value = stdout + stderr;
    statusEl.textContent = exitCode === 0 ? "ok" : `exit code ${exitCode}`;
  } catch (err) {
    outputEl.value = err && err.message ? err.message : String(err);
    statusEl.textContent = "error";
  } finally {
    runButton.disabled = false;
  }
}

runButton.addEventListener("click", run);

sourceEl.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    event.preventDefault();
    run();
  }
});
