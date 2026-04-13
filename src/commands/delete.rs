use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::config;
use crate::git;
use crate::i18n::t;
use crate::ui;

pub fn run() -> Result<()> {
    // 1. Load config files
    let mut workspaces_file = config::load_workspaces()?;
    let projects_file = config::load_projects()?;
    let global = config::load_global_config()?;

    // 2. Check we have workspaces
    if workspaces_file.workspaces.is_empty() {
        ui::info(&t("no_workspaces"));
        return Ok(());
    }

    // 3. Select workspace to delete
    let ws_names: Vec<String> = workspaces_file
        .workspaces
        .iter()
        .map(|ws| ws.name.clone())
        .collect();
    let ws_idx = ui::select(&t("select_workspace_delete"), &ws_names)?;
    let ws = &workspaces_file.workspaces[ws_idx];

    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(&ws.name);

    // 4. Check each project for uncommitted changes
    let mut dirty_projects: Vec<String> = Vec::new();
    for wp in &ws.projects {
        let wt_path = Path::new(&wp.worktree_path);
        if wt_path.exists() {
            if let Ok(false) = git::is_clean(wt_path) {
                dirty_projects.push(wp.name.clone());
            }
        }
    }

    // 5. If dirty, confirm with user
    if !dirty_projects.is_empty() {
        ui::warn(&format!(
            "The following projects have uncommitted changes: {}",
            dirty_projects.join(", ")
        ));
        if !ui::confirm(&t("delete_with_changes"), false)? {
            ui::info("Aborted.");
            return Ok(());
        }
    }

    // 6. For each project: worktree remove, branch delete
    let branch = ws.branch.clone();
    for wp in &ws.projects {
        // Find the project's repo dir from projects_file
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            let repo_dir = Path::new(&project.path);
            let wt_path = Path::new(&wp.worktree_path);

            // Remove worktree
            if wt_path.exists() {
                match git::worktree_remove(repo_dir, wt_path) {
                    Ok(()) => {}
                    Err(e) => ui::warn(&format!(
                        "Failed to remove worktree for '{}': {}",
                        wp.name, e
                    )),
                }
            }

            // Delete branch (ignore errors - branch may already be gone)
            let _ = git::branch_delete(repo_dir, &branch);
        }
    }

    // 7. Remove workspace directory
    if ws_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&ws_dir) {
            ui::warn(&format!(
                "Failed to remove workspace directory: {}",
                e
            ));
        }
    }

    // 8. Remove from workspaces.toml
    let ws_name = workspaces_file.workspaces[ws_idx].name.clone();
    workspaces_file.workspaces.remove(ws_idx);
    config::save_workspaces(&workspaces_file)?;

    // 9. Print success
    ui::success(&t("workspace_deleted").replace("{}", &ws_name));
    Ok(())
}
