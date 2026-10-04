#!/bin/bash
# Resolve the LSP binary relative to the repository root (this script lives in
# lale-lsp/, and the workspace builds the binary into the root target/ dir).
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/lale-lsp"

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}'

# Just the first 7 lines
TEXT='// selective std import
use openFile, readFile from std.file_io_posix

warn "~~~ BEGIN ~~~"

loop var i as i32 from 1 to 2
  write "döngü i = {i * 2}"
end loop'

TEXT_JSON=$(python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))" <<< "$TEXT")
DIDOPEN="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"file:///tmp/test.lale\",\"languageId\":\"lale\",\"version\":1,\"text\":${TEXT_JSON}}}}"

HOVER='{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///tmp/test.lale"},"position":{"line":4,"character":20}}}'
SHUTDOWN='{"jsonrpc":"2.0","id":3,"method":"shutdown"}'
EXIT='{"jsonrpc":"2.0","method":"exit"}'

{
    printf 'Content-Length: %d\r\n\r\n%s' "${#INIT}" "$INIT"
    sleep 1
    printf 'Content-Length: %d\r\n\r\n%s' "${#DIDOPEN}" "$DIDOPEN"
    sleep 2
    printf 'Content-Length: %d\r\n\r\n%s' "${#HOVER}" "$HOVER"
    sleep 1
    printf 'Content-Length: %d\r\n\r\n%s' "${#SHUTDOWN}" "$SHUTDOWN"
    printf 'Content-Length: %d\r\n\r\n%s' "${#EXIT}" "$EXIT"
} | "$BIN" 2>/tmp/lsp-stderr7.log > /tmp/lsp-stdout7.log

echo "=== STDERR ==="
cat /tmp/lsp-stderr7.log
echo "=== Hover id:2 ==="
grep '"id":2' /tmp/lsp-stdout7.log
echo "=== Diagnostics ==="
grep 'publishDiagnostics\|diagnostics\|"method"' /tmp/lsp-stdout7.log | head -5
