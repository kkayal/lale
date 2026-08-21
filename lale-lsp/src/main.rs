//! Lale Language Server — LSP implementation for the Lale programming language.
//!
//! This binary implements the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
//! to provide IDE features (diagnostics, completion, hover, etc.) for editors
//! that support LSP — including VS Code and the Zed editor.
//!
//! ## Usage
//!
//! ```text
//! lale-lsp                 # Start server on stdio (default for editors)
//! lale-lsp --stdio         # Explicit stdio mode
//! ```
//!
//! ## Integration
//!
//! ### VS Code
//! Update the extension's `package.json` to point at the `lale-lsp` binary.
//!
//! ### Zed
//! Add to `~/.zed/settings.json`:
//! ```json
//! {
//!   "lsp": {
//!     "lale": {
//!       "command": ["lale-lsp"]
//!     }
//!   },
//!   "languages": {
//!     "Lale": {
//!       "language_servers": ["lale"]
//!     }
//!   }
//! }
//! ```

mod server;

#[tokio::main]
async fn main() {
  // Initialize logging (stderr to avoid corrupting stdout LSP transport)
  eprintln!("Lale Language Server starting...");

  let stdin = tokio::io::stdin();
  let stdout = tokio::io::stdout();

  let (service, socket) = tower_lsp::LspService::new(server::Backend::new);

  tower_lsp::Server::new(stdin, stdout, socket)
    .serve(service)
    .await;

  eprintln!("Lale Language Server shut down.");
}
