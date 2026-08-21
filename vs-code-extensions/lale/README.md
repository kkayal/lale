# Lale Language Support for VS Code

Syntax highlighting and LSP features for the Lale programming language.

## Build and Install

```bash
cd vs-code-extensions/lale
npm install
npm run compile
```

Then install into VS Code:

**macOS / Linux:**
```bash
cp -r . ~/.vscode/extensions/lale/
```

**Windows (PowerShell):**
```powershell
Copy-Item -Recurse . $env:USERPROFILE\.vscode\extensions\lale\
```

Restart VS Code. Open any `.lale` file to activate.

### Configure the LSP binary path

If `lale-lsp` is not on your `PATH`:

```json
{
  "lale.lsp.path": "/path/to/target/debug/lale-lsp"
}
```

## Features

- Syntax highlighting (TextMate grammar)
- Diagnostics (errors and warnings via LSP)
- Completions for keywords and snippets
- Hover information
- Semantic tokens

## Extension Settings

| Setting | Default | Description |
|---|---|---|
| `lale.lsp.path` | `"lale-lsp"` | Path to the LSP server binary |
