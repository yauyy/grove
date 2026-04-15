use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::config;
use crate::git;
use crate::i18n::t;
use crate::ui;
use crate::workspace;

/// Workspace rename: rename the workspace itself, with optional branch rename.
pub fn run() -> Result<()> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;
    let mut workspaces_file = config::load_workspaces()?;
    let global = config::load_global_config()?;

    let ws_idx = workspaces_file
        .workspaces
        .iter()
        .position(|w| w.name == ws.name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", ws.name))?;

    let old_name = ws.name.clone();

    // 1. Prompt for new workspace name
    let new_name = ui::input(&t("new_workspace_name"), "")?;
    if new_name.trim().is_empty() {
        bail!("Workspace name cannot be empty");
    }
    if new_name == old_name {
        ui::info(&t("no_changes"));
        return Ok(());
    }
    if workspaces_file.workspaces.iter().any(|w| w.name == new_name) {
        bail!("{}", t("workspace_name_exists").replace("{}", &new_name));
    }

    // 2. Rename workspace directory on disk
    let workpath = config::resolve_workpath(&global.workpath)?;
    let old_ws_dir = workpath.join(config::safe_dir_name(&old_name));
    let new_ws_dir = workpath.join(config::safe_dir_name(&new_name));

    if old_ws_dir.exists() {
        fs::rename(&old_ws_dir, &new_ws_dir)?;
    }

    // 3. Repair git worktree links for each project
    for wp in &ws.projects {
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            let repo_dir = Path::new(&project.path);
            if let Err(e) = git::worktree_repair(repo_dir) {
                ui::warn(&format!("{}: worktree repair failed: {}", wp.name, e));
            }
        }
    }

    // 4. Update workspace record: name + worktree paths
    workspaces_file.workspaces[ws_idx].name = new_name.clone();
    for wp in &mut workspaces_file.workspaces[ws_idx].projects {
        let old_wt = Path::new(&wp.worktree_path);
        // Replace old workspace dir with new workspace dir in path
        if let Ok(relative) = old_wt.strip_prefix(&old_ws_dir) {
            let new_wt = new_ws_dir.join(relative);
            wp.worktree_path = new_wt.to_string_lossy().to_string();
        }
    }

    // 5. Ask if user wants to rename the branch too
    if ui::confirm(&t("rename_branch_too"), true)? {
        let old_branch = ws.branch.clone();
        let new_branch = if global.git_prefix.is_empty() {
            new_name.clone()
        } else {
            format!("{}{}", global.git_prefix, new_name)
        };

        let mut branch_succeeded = 0usize;
        let mut branch_failed = 0usize;

        for wp in &workspaces_file.workspaces[ws_idx].projects {
            let wt_path = Path::new(&wp.worktree_path);
            match git::branch_rename(wt_path, &old_branch, &new_branch) {
                Ok(()) => {
                    ui::success(&format!("{}: {} -> {}", wp.name, old_branch, new_branch));
                    branch_succeeded += 1;
                }
                Err(e) => {
                    ui::warn(&t("ws_rename_branch_failed")
                        .replacen("{}", &wp.name, 1)
                        .replacen("{}", &e.to_string(), 1));
                    branch_failed += 1;
                }
            }
        }

        if branch_succeeded > 0 {
            workspaces_file.workspaces[ws_idx].branch = new_branch;
        }

        ui::batch_summary(branch_succeeded, branch_failed);
    }

    // 6. Save
    config::save_workspaces(&workspaces_file)?;

    ui::success(&t("ws_rename_success")
        .replacen("{}", &old_name, 1)
        .replacen("{}", &new_name, 1));

    Ok(())
}

/// Branch rename across all projects in a workspace (git operation).
pub fn grename() -> Result<()> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;
    let mut workspaces_file = config::load_workspaces()?;

    let ws_idx = workspaces_file
        .workspaces
        .iter()
        .position(|w| w.name == ws.name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", ws.name))?;

    let old_branch = ws.branch.clone();

    // Prompt for new branch name (no prefix auto-applied)
    let new_branch = ui::input(&t("new_branch_name"), "")?;
    if new_branch.trim().is_empty() {
        bail!("Branch name cannot be empty");
    }
    if new_branch == old_branch {
        ui::info(&t("no_changes"));
        return Ok(());
    }

    // Pre-check: verify new branch doesn't already exist in any project
    for wp in &ws.projects {
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            let repo_dir = Path::new(&project.path);
            if git::branch_exists(repo_dir, &new_branch)? {
                bail!(
                    "{}",
                    t("branch_already_exists")
                        .replacen("{}", &new_branch, 1)
                        .replacen("{}", &wp.name, 1)
                );
            }
        }
    }

    // Confirm
    let confirm_msg = t("rename_confirm")
        .replacen("{}", &ws.projects.len().to_string(), 1)
        .replacen("{}", &old_branch, 1)
        .replacen("{}", &new_branch, 1);
    if !ui::confirm(&confirm_msg, true)? {
        ui::info(&t("cancelled"));
        return Ok(());
    }

    // Rename branch in each project worktree
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for wp in &ws.projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::branch_rename(wt_path, &old_branch, &new_branch) {
            Ok(()) => {
                ui::success(&format!("{}: {} -> {}", wp.name, old_branch, new_branch));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                failed += 1;
            }
        }
    }

    // Update workspace record
    if succeeded > 0 {
        workspaces_file.workspaces[ws_idx].branch = new_branch.clone();
        config::save_workspaces(&workspaces_file)?;
    }

    println!();
    ui::header(
        &t("workspace_branch_renamed")
            .replacen("{}", &ws.name, 1)
            .replacen("{}", &new_branch, 1),
    );
    ui::batch_summary(succeeded, failed);

    Ok(())
}
