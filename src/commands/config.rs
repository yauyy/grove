use anyhow::{bail, Result};
use std::fs;
use std::process::Command;

use crate::config as cfg;
use crate::i18n::t;
use crate::ui;

pub fn set(key: &str, value: &str) -> Result<()> {
    let mut config = cfg::load_global_config()?;

    match key {
        "workpath" => {
            config.workpath = value.to_string();

            // Auto-create the directory if it doesn't exist
            let resolved = cfg::resolve_workpath(value)?;
            if !resolved.exists() {
                fs::create_dir_all(&resolved)?;
                ui::info(&format!("Created directory: {}", resolved.display()));
            }

            cfg::save_global_config(&config)?;
            ui::success(&format!("workpath = {}", value));
            ui::info(&t("workpath_forward_only"));
        }
        "git-prefix" => {
            config.git_prefix = value.to_string();
            cfg::save_global_config(&config)?;
            ui::success(&format!("git-prefix = {}", value));
        }
        _ => {
            bail!("Unknown config key: '{}'. Valid keys: workpath, git-prefix", key);
        }
    }

    Ok(())
}

pub fn list() -> Result<()> {
    let config = cfg::load_global_config()?;
    let resolved = cfg::resolve_workpath(&config.workpath)?;
    println!(
        "workpath = {} ({})",
        config.workpath,
        resolved.display()
    );
    if !config.git_prefix.is_empty() {
        println!("git-prefix = {}", config.git_prefix);
    }
    Ok(())
}

pub fn edit(file: Option<&str>) -> Result<()> {
    let grove_dir = cfg::grove_dir()?;
    cfg::ensure_dirs()?;

    let filename = match file.unwrap_or("projects") {
        "projects" => "projects.toml",
        "config" => "config.toml",
        "workspaces" => "workspaces.toml",
        other => bail!(
            "Unknown config file: '{}'. Valid options: projects, config, workspaces",
            other
        ),
    };

    let file_path = grove_dir.join(filename);

    // Create file with defaults if it doesn't exist
    if !file_path.exists() {
        match filename {
            "config.toml" => {
                let config = cfg::GlobalConfig::default();
                cfg::save_global_config(&config)?;
            }
            "projects.toml" => {
                let pf = cfg::ProjectsFile::default();
                cfg::save_projects(&pf)?;
            }
            "workspaces.toml" => {
                let wf = cfg::WorkspacesFile::default();
                cfg::save_workspaces(&wf)?;
            }
            _ => {}
        }
    }

    // Find editor
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                // Try common editors
                for cmd in &["vim", "nano", "vi"] {
                    if Command::new("which")
                        .arg(cmd)
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false)
                    {
                        return cmd.to_string();
                    }
                }
                "vi".to_string()
            }
        });

    ui::info(&t("config_edit_opening")
        .replacen("{}", filename, 1)
        .replacen("{}", &editor, 1));

    let status = Command::new(&editor)
        .arg(&file_path)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to open editor '{}': {}", editor, e))?;

    if status.success() {
        ui::success(&t("config_edited").replace("{}", filename));
    } else {
        ui::warn("Editor exited with non-zero status");
    }

    Ok(())
}
