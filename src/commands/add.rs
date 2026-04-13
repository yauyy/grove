use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::config::{self, BranchConfig, Group, Project};
use crate::git;
use crate::ui;

pub fn run(path: &str) -> Result<()> {
    // 1. Resolve and validate the path
    let resolved = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("Path does not exist: {}", path))?;

    if !resolved.is_dir() {
        bail!("Path is not a directory: {}", resolved.display());
    }

    if !git::is_git_repo(&resolved) {
        bail!("Not a git repository: {}", resolved.display());
    }

    let path_str = resolved.to_string_lossy().to_string();

    // 2. Check if already registered
    let mut pf = config::load_projects()?;
    if pf.projects.iter().any(|p| p.path == path_str) {
        bail!("Project already registered: {}", path_str);
    }

    // 3. Auto-detect project name from directory name, let user modify
    let dir_name = resolved
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let name = ui::input("Project name", &dir_name)?;

    // Check for duplicate name
    if pf.projects.iter().any(|p| p.name == name) {
        bail!("A project named '{}' already exists", name);
    }

    // 4. Select group
    let group = select_group(&mut pf)?;

    // 5. Fetch remote (best effort), list remote branches for main branch selection
    ui::info("Fetching remote branches...");
    let _ = git::fetch(&resolved); // best effort

    let remote_branches = git::list_remote_branches(&resolved).unwrap_or_default();
    let clean_branches: Vec<String> = remote_branches
        .iter()
        .map(|b| {
            b.strip_prefix("origin/")
                .unwrap_or(b)
                .to_string()
        })
        .collect();

    let main_branch = if clean_branches.is_empty() {
        // No remote branches, try current branch or default to "main"
        let current = git::current_branch(&resolved).unwrap_or_else(|_| "main".to_string());
        ui::input("Main branch", &current)?
    } else {
        let branch_options: Vec<String> = clean_branches.clone();
        let idx = ui::select("Select main branch", &branch_options)?;
        branch_options[idx].clone()
    };

    // 6. Configure optional environment branches
    let test_branch = select_env_branch("test", &clean_branches)?;
    let staging_branch = select_env_branch("staging", &clean_branches)?;
    let prod_branch = select_env_branch("prod", &clean_branches)?;

    // 7. Optional agents.md configuration
    let agents_md = ui::input_optional("Path to agents.md", "press Enter to skip")?;

    // Validate agents.md path if provided
    if let Some(ref md_path) = agents_md {
        let md_resolved = PathBuf::from(md_path);
        if !md_resolved.exists() {
            ui::warn(&format!("agents.md not found at: {}", md_path));
            if !ui::confirm("Continue anyway?", false)? {
                bail!("Aborted");
            }
        }
    }

    // 8. Calculate order and save
    let order = pf
        .projects
        .iter()
        .filter(|p| p.group == group)
        .map(|p| p.order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    let project = Project {
        name: name.clone(),
        path: path_str,
        group,
        order,
        agents_md,
        branches: BranchConfig {
            main: main_branch,
            test: test_branch,
            staging: staging_branch,
            prod: prod_branch,
        },
    };

    pf.projects.push(project);
    config::save_projects(&pf)?;

    ui::success(&format!("Added project '{}'", name));
    Ok(())
}

/// Prompt user to select a group from existing groups, create a new one, or choose ungrouped.
fn select_group(pf: &mut config::ProjectsFile) -> Result<String> {
    let mut options: Vec<String> = pf.groups.iter().map(|g| g.name.clone()).collect();
    options.push("+ New group".to_string());
    options.push("Ungrouped".to_string());

    let idx = ui::select("Select group", &options)?;

    if idx == options.len() - 1 {
        // Ungrouped
        Ok(String::new())
    } else if idx == options.len() - 2 {
        // New group
        let group_name = ui::input("Group name", "")?;
        if group_name.is_empty() {
            bail!("Group name cannot be empty");
        }
        if pf.groups.iter().any(|g| g.name == group_name) {
            bail!("Group '{}' already exists", group_name);
        }
        let order = pf.groups.iter().map(|g| g.order).max().map(|m| m + 1).unwrap_or(0);
        pf.groups.push(Group {
            name: group_name.clone(),
            order,
        });
        Ok(group_name)
    } else {
        Ok(options[idx].clone())
    }
}

/// Prompt user to select an environment branch or skip.
fn select_env_branch(env_name: &str, branches: &[String]) -> Result<Option<String>> {
    let mut options = vec!["Skip (none)".to_string()];
    options.extend(branches.iter().cloned());

    let prompt = format!("Select {} branch", env_name);
    let idx = ui::select(&prompt, &options)?;

    if idx == 0 {
        Ok(None)
    } else {
        Ok(Some(options[idx].clone()))
    }
}
