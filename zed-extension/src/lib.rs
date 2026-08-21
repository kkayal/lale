use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct LaleExtension;

impl zed::Extension for LaleExtension {
  fn new() -> Self {
    Self
  }

  fn language_server_command(
    &mut self,
    _language_server_id: &LanguageServerId,
    _worktree: &Worktree,
  ) -> Result<zed::Command> {
    Ok(zed::Command {
      command: "lale-lsp".to_string(),
      args: vec![],
      env: vec![],
    })
  }
}

zed::register_extension!(LaleExtension);
