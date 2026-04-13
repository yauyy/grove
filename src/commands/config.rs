use anyhow::Result;
use std::fs;

use crate::config as cfg;
use crate::ui;

pub fn set(key: &str, value: &str) -> Result<()> {
    if key != "workpath" {
        anyhow::bail!("Unknown config key: '{}'. Valid keys: workpath", key);
    }

    let mut config = cfg::load_global_config()?;
    config.workpath = value.to_string();

    // Auto-create the directory if it doesn't exist
    let resolved = cfg::resolve_workpath(value)?;
    if !resolved.exists() {
        fs::create_dir_all(&resolved)?;
        ui::info(&format!("Created directory: {}", resolved.display()));
    }

    cfg::save_global_config(&config)?;
    ui::success(&format!("workpath = {}", value));
    ui::info("Note: changing workpath is forward-only. Existing workspaces remain at their original location.");

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
    Ok(())
}
