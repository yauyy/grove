use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::config::{self, ProjectsFile, Workspace, WorkspaceProject};
use crate::git;
use crate::ui;
use crate::workspace;

/// Build a grouped display list of projects for multi-select prompts.
/// Returns (display_items, index_mapping) where index_mapping[i] is the
/// index into projects_file.projects for display_items[i].
pub fn build_grouped_project_list(projects_file: &ProjectsFile) -> (Vec<String>, Vec<usize>) {
    let mut display_items: Vec<String> = Vec::new();
    let mut index_map: Vec<usize> = Vec::new();

    // Sort groups by order
    let mut groups = projects_file.groups.clone();
    groups.sort_by_key(|g| g.order);

    // Grouped projects
    for group in &groups {
        // Collect projects in this group with their original indices
        let mut group_projects: Vec<(usize, &config::Project)> = projects_file
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| p.group == group.name)
            .collect();
        group_projects.sort_by_key(|(_, p)| p.order);

        for (orig_idx, project) in group_projects {
            display_items.push(format!("[{}] {}", group.name, project.name));
            index_map.push(orig_idx);
        }
    }

    // Ungrouped projects
    let mut ungrouped: Vec<(usize, &config::Project)> = projects_file
        .projects
        .iter()
        .enumerate()
        .filter(|(_, p)| p.group.is_empty())
        .collect();
    ungrouped.sort_by_key(|(_, p)| p.order);

    for (orig_idx, project) in ungrouped {
        display_items.push(project.name.clone());
        index_map.push(orig_idx);
    }

    (display_items, index_map)
}

pub fn run(name: Option<String>) -> Result<()> {
    // 1. Load config files
    let projects_file = config::load_projects()?;
    let mut workspaces_file = config::load_workspaces()?;
    let global = config::load_global_config()?;

    // 2. Check we have projects
    if projects_file.projects.is_empty() {
        ui::info("No projects registered. Use `grove add <path>` to add one first.");
        return Ok(());
    }

    // 3. Get workspace name
    let ws_name = match name {
        Some(n) => n,
        None => ui::input("Workspace name", "")?,
    };

    // 4. Validate name
    if ws_name.trim().is_empty() {
        bail!("Workspace name cannot be empty");
    }

    if workspaces_file.workspaces.iter().any(|ws| ws.name == ws_name) {
        bail!("Workspace '{}' already exists", ws_name);
    }

    // 5. Multi-select projects
    let (display_items, index_map) = build_grouped_project_list(&projects_file);
    let defaults: Vec<bool> = vec![false; display_items.len()];
    let selected = ui::multi_select("Select projects", &display_items, &defaults)?;

    // 6. Bail if none selected
    if selected.is_empty() {
        ui::info("No projects selected.");
        return Ok(());
    }

    // 7. Prompt for branch name
    let branch = ui::input("Branch name", &ws_name)?;

    // 8. Create workspace directory
    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(&ws_name);
    fs::create_dir_all(&ws_dir)?;

    // 9. Process each selected project
    let mut succeeded = 0;
    let mut failed = 0;
    let mut ws_projects: Vec<WorkspaceProject> = Vec::new();
    let mut selected_projects: Vec<config::Project> = Vec::new();

    for &sel_idx in &selected {
        let proj_idx = index_map[sel_idx];
        let project = &projects_file.projects[proj_idx];
        let repo_dir = Path::new(&project.path);
        let wt_path = ws_dir.join(&project.name);

        // a. Check if branch already exists
        if git::branch_exists(repo_dir, &branch)? {
            ui::warn(&format!(
                "Branch '{}' already exists in '{}', skipping",
                branch, project.name
            ));
            failed += 1;
            continue;
        }

        // b. git fetch origin (best effort)
        let _ = git::fetch(repo_dir);

        // c. git worktree add
        match git::worktree_add(repo_dir, &wt_path, &branch, &project.branches.main) {
            Ok(()) => {
                ws_projects.push(WorkspaceProject {
                    name: project.name.clone(),
                    worktree_path: wt_path.to_string_lossy().to_string(),
                });
                selected_projects.push(project.clone());
                succeeded += 1;
                ui::success(&format!("Created worktree for '{}'", project.name));
            }
            Err(e) => {
                ui::error(&format!("Failed to create worktree for '{}': {}", project.name, e));
                failed += 1;
            }
        }
    }

    if ws_projects.is_empty() {
        // Clean up empty workspace dir
        let _ = fs::remove_dir(&ws_dir);
        bail!("No worktrees were created successfully");
    }

    // 10. Merge agents.md files
    let agents_path = ws_dir.join("AGENTS.md");
    if workspace::merge_agents_md(&selected_projects, &agents_path)? {
        ui::info("Merged AGENTS.md");
    }

    // 11. Save workspace record
    let created_at = chrono::Local::now().format("%Y-%m-%d").to_string();
    let ws = Workspace {
        name: ws_name.clone(),
        branch: branch.clone(),
        created_at,
        projects: ws_projects,
    };
    workspaces_file.workspaces.push(ws);
    config::save_workspaces(&workspaces_file)?;

    // 12. Print summary
    println!();
    ui::header(&format!("Workspace '{}' created", ws_name));
    ui::batch_summary(succeeded, failed);

    Ok(())
}
