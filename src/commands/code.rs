use anyhow::{bail, Result};
use std::process::Command;

use crate::config;
use crate::i18n::t;
use crate::ui;
use crate::workspace;

pub fn run(name: Option<String>) -> Result<()> {
    let global = config::load_global_config()?;
    let ws_file = config::load_workspaces()?;

    if ws_file.workspaces.is_empty() {
        bail!("{}", t("no_workspaces_found"));
    }

    let ws = if let Some(ref n) = name {
        ws_file
            .workspaces
            .iter()
            .find(|w| w.name == *n)
            .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found.", n))?
            .clone()
    } else {
        workspace::get_or_select_workspace()?
    };

    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(config::safe_dir_name(&ws.name));

    if !ws_dir.exists() {
        bail!("Workspace directory does not exist: {}", ws_dir.display());
    }

    ui::info(&t("opening_editor").replace("{}", &ws.name));

    let status = Command::new("code")
        .arg(&ws_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            ui::success(&format!("code {}", ws_dir.display()));
        }
        Ok(_) => {
            ui::warn("VS Code exited with non-zero status");
        }
        Err(_) => {
            // Fallback: try 'open' on macOS
            if cfg!(target_os = "macos") {
                Command::new("open").arg(&ws_dir).status()?;
            } else if cfg!(target_os = "windows") {
                Command::new("explorer").arg(&ws_dir).status()?;
            } else {
                bail!("'code' command not found. Install VS Code and add it to PATH.");
            }
        }
    }

    Ok(())
}
