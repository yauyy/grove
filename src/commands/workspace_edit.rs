use anyhow::{bail, Result};
use std::path::Path;

use crate::commands::create::build_grouped_project_list;
use crate::config::{self, WorkspaceProject};
use crate::git;
use crate::i18n::t;
use crate::ui;
use crate::workspace;

pub fn run(name: Option<String>) -> Result<()> {
    // 1. Load config files
    let projects_file = config::load_projects()?;
    let mut workspaces_file = config::load_workspaces()?;
    let global = config::load_global_config()?;

    // 2. Check we have workspaces
    if workspaces_file.workspaces.is_empty() {
        ui::info(&t("no_workspaces_edit"));
        return Ok(());
    }

    // 3. Select workspace
    let ws_idx = match name {
        Some(ref n) => {
            workspaces_file
                .workspaces
                .iter()
                .position(|ws| ws.name == *n)
                .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", n))?
        }
        None => {
            let ws_names: Vec<String> = workspaces_file
                .workspaces
                .iter()
                .map(|ws| ws.name.clone())
                .collect();
            ui::select(&t("select_workspace_edit"), &ws_names)?
        }
    };

    // 4. Build grouped project list
    let (display_items, index_map) = build_grouped_project_list(&projects_file);

    // 5. Pre-check projects already in the workspace
    let ws = &workspaces_file.workspaces[ws_idx];
    let current_project_names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();

    let defaults: Vec<bool> = index_map
        .iter()
        .map(|&proj_idx| {
            let proj_name = &projects_file.projects[proj_idx].name;
            current_project_names.contains(proj_name)
        })
        .collect();

    // 6. Show multi_select with defaults
    let selected = ui::multi_select(&t("edit_projects"), &display_items, &defaults)?;

    // 7. Compute additions and removals
    let new_project_names: Vec<String> = selected
        .iter()
        .map(|&sel_idx| {
            let proj_idx = index_map[sel_idx];
            projects_file.projects[proj_idx].name.clone()
        })
        .collect();

    let additions: Vec<String> = new_project_names
        .iter()
        .filter(|n| !current_project_names.contains(n))
        .cloned()
        .collect();

    let removals: Vec<String> = current_project_names
        .iter()
        .filter(|n| !new_project_names.contains(n))
        .cloned()
        .collect();

    // 8. If no changes, print info and return
    if additions.is_empty() && removals.is_empty() {
        ui::info(&t("no_changes"));
        return Ok(());
    }

    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws = &workspaces_file.workspaces[ws_idx];
    let ws_dir = workpath.join(config::safe_dir_name(&ws.name));
    let branch = ws.branch.clone();

    // 9. For removals: check uncommitted changes
    for removal in &removals {
        let wt_path = ws_dir.join(removal);
        if wt_path.exists() {
            if let Ok(false) = git::is_clean(&wt_path) {
                bail!(
                    "{}",
                    t("uncommitted_changes").replace("{}", removal)
                );
            }
        }
    }

    // 10. Ask whether to delete local branch for removed projects
    let delete_branch = if !removals.is_empty() {
        ui::confirm(&t("delete_local_branch").replace("{}", &branch), false)?
    } else {
        false
    };

    // 11. Process removals
    for removal in &removals {
        // Find the project's repo dir
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == *removal) {
            let repo_dir = Path::new(&project.path);
            let wt_path = ws_dir.join(removal);
            if wt_path.exists() {
                match git::worktree_remove(repo_dir, &wt_path) {
                    Ok(()) => ui::success(&format!("Removed worktree for '{}'", removal)),
                    Err(_) => {
                        // Worktree not recognized by git, clean up manually
                        let _ = std::fs::remove_dir_all(&wt_path);
                        let _ = git::worktree_prune(repo_dir);
                        ui::success(&format!("Cleaned up worktree for '{}'", removal));
                    }
                }
            }
            if delete_branch {
                let _ = git::branch_delete(repo_dir, &branch);
            }
        }
    }

    // 12. Process additions
    let mut added_count = 0;
    let mut add_failed = 0;
    let mut new_ws_projects: Vec<WorkspaceProject> = Vec::new();

    for addition in &additions {
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == *addition) {
            let repo_dir = Path::new(&project.path);
            let wt_path = ws_dir.join(addition);

            // git fetch (best effort)
            let _ = git::fetch(repo_dir);

            // If branch already exists, checkout it; otherwise create new branch
            let result = if git::branch_exists(repo_dir, &branch).unwrap_or(false) {
                git::worktree_add_existing(repo_dir, &wt_path, &branch)
            } else {
                let start_point = git::resolve_remote_start_point(repo_dir, &project.branches.main);
                git::worktree_add(repo_dir, &wt_path, &branch, &start_point)
            };
            match result {
                Ok(()) => {
                    new_ws_projects.push(WorkspaceProject {
                        name: project.name.clone(),
                        worktree_path: wt_path.to_string_lossy().to_string(),
                    });
                    added_count += 1;
                    ui::success(&format!("Added worktree for '{}'", addition));
                }
                Err(e) => {
                    ui::error(&format!("Failed to add worktree for '{}': {}", addition, e));
                    add_failed += 1;
                }
            }
        }
    }

    // 12. Update workspace record
    let ws = &mut workspaces_file.workspaces[ws_idx];

    // Remove removed projects
    ws.projects.retain(|p| !removals.contains(&p.name));

    // Add new projects
    ws.projects.extend(new_ws_projects);

    config::save_workspaces(&workspaces_file)?;

    // 13. Regenerate AGENTS.md
    let ws = &workspaces_file.workspaces[ws_idx];
    let ws_project_names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();
    let agents_projects: Vec<config::Project> = projects_file
        .projects
        .iter()
        .filter(|p| ws_project_names.contains(&p.name))
        .cloned()
        .collect();

    let agents_path = ws_dir.join("AGENTS.md");
    if agents_projects.is_empty() {
        // Remove AGENTS.md if no projects left
        let _ = std::fs::remove_file(&agents_path);
    } else {
        workspace::merge_agents_md(&agents_projects, &agents_path)?;
    }

    // 14. Print summary
    println!();
    ui::header(&format!("Workspace '{}' updated", ws.name));
    if !removals.is_empty() {
        println!("  Removed: {}", removals.join(", "));
    }
    if added_count > 0 || add_failed > 0 {
        println!("  Added: {} succeeded, {} failed", added_count, add_failed);
    }

    Ok(())
}
