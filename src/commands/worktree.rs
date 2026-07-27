use anyhow::{bail, Context, Result};
use console::Style;
use std::path::{Path, PathBuf};

use crate::config::{self, Project, Workspace, WorkspaceProject};
use crate::git;
use crate::i18n::t;
use crate::ui;
use crate::workspace;

/// Resolve the current workspace (from cwd or via prompt).
fn current_workspace() -> Result<Workspace> {
    workspace::get_or_select_workspace()
}

/// Find the registered project definition for a workspace project by name.
fn find_project<'a>(projects_file: &'a config::ProjectsFile, name: &str) -> Option<&'a Project> {
    projects_file.projects.iter().find(|p| p.name == name)
}

/// Derive the workspace directory that holds each project's worktree.
/// Worktree paths are `<ws_dir>/<project>`, so the parent is the ws_dir. Falls
/// back to `<workpath>/<safe(name)>` when the workspace has no projects yet.
fn workspace_dir(ws: &Workspace) -> Result<PathBuf> {
    if let Some(first) = ws.projects.first() {
        return Path::new(&first.worktree_path)
            .parent()
            .map(PathBuf::from)
            .context("Cannot determine workspace directory from worktree path");
    }
    let global = config::load_global_config()?;
    let base = config::resolve_workpath(&global.workpath)?;
    Ok(base.join(config::safe_dir_name(&ws.name)))
}

/// Mutate the stored workspace record in place and persist it.
fn update_workspace<F>(workspace_name: &str, mutate: F) -> Result<()>
where
    F: FnOnce(&mut Workspace),
{
    let mut file = config::load_workspaces()?;
    let ws = file
        .workspaces
        .iter_mut()
        .find(|ws| ws.name == workspace_name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;
    mutate(ws);
    config::save_workspaces(&file)
}

/// Best-effort go.work refresh after the project set changes.
fn maybe_sync_go_work(workspace_name: &str) {
    let global = match config::load_global_config() {
        Ok(g) => g,
        Err(_) => return,
    };
    if !global.auto_go_work {
        return;
    }
    let (ws, matched) = match resolve_workspace_by_name(workspace_name) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Err(e) = crate::commands::gowork::sync_workspace(&ws, &matched) {
        ui::warn(&format!("Failed to sync go.work: {}", e));
    }
}

/// Reload a workspace by name together with its matched project definitions.
fn resolve_workspace_by_name(
    workspace_name: &str,
) -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let file = config::load_workspaces()?;
    let ws = file
        .workspaces
        .into_iter()
        .find(|ws| ws.name == workspace_name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;
    let projects_file = config::load_projects()?;
    let matched = ws
        .projects
        .iter()
        .filter_map(|wp| {
            find_project(&projects_file, &wp.name).map(|p| (wp.clone(), p.clone()))
        })
        .collect();
    Ok((ws, matched))
}

pub fn list() -> Result<()> {
    let ws = current_workspace()?;
    if ws.projects.is_empty() {
        ui::info(&t("wt_no_projects_in_workspace").replace("{}", &ws.name));
        return Ok(());
    }

    let bold = Style::new().bold();
    let dim = Style::new().dim();
    let green = Style::new().green();
    let yellow = Style::new().yellow();

    for wp in &ws.projects {
        let wt_path = Path::new(&wp.worktree_path);
        let branch = match git::current_branch(wt_path) {
            Ok(b) => b,
            Err(_) => t("wt_detached"),
        };
        let state = match git::is_clean(wt_path) {
            Ok(true) => green.apply_to(t("wt_clean")).to_string(),
            Ok(false) => {
                let count = git::status_short(wt_path)
                    .map(|s| s.lines().count())
                    .unwrap_or(0);
                yellow
                    .apply_to(t("wt_dirty").replace("{}", &count.to_string()))
                    .to_string()
            }
            Err(_) => yellow.apply_to(t("missing")).to_string(),
        };
        println!("{}", bold.apply_to(&wp.name));
        println!("  {}", dim.apply_to(&wp.worktree_path));
        println!("  {} · {}", branch, state);
    }
    Ok(())
}

pub fn add(project: Option<String>) -> Result<()> {
    let ws = current_workspace()?;
    let projects_file = config::load_projects()?;
    if projects_file.projects.is_empty() {
        ui::info(&t("no_projects_registered"));
        return Ok(());
    }

    // Candidates: registered projects not already in the workspace.
    let in_ws: Vec<&str> = ws.projects.iter().map(|p| p.name.as_str()).collect();
    let candidates: Vec<&Project> = projects_file
        .projects
        .iter()
        .filter(|p| !in_ws.contains(&p.name.as_str()))
        .collect();

    let project = match project {
        Some(name) => {
            if in_ws.contains(&name.as_str()) {
                bail!(
                    "{}",
                    t("wt_already_in_ws")
                        .replacen("{}", &name, 1)
                        .replacen("{}", &ws.name, 1)
                );
            }
            find_project(&projects_file, &name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("{}", t("wt_project_unknown").replace("{}", &name)))?
        }
        None => {
            if candidates.is_empty() {
                ui::info(&t("wt_no_addable").replace("{}", &ws.name));
                return Ok(());
            }
            let names: Vec<String> = candidates.iter().map(|p| p.name.clone()).collect();
            let idx = ui::select(&t("wt_select_add"), &names)?;
            candidates[idx].clone()
        }
    };

    let ws_dir = workspace_dir(&ws)?;
    std::fs::create_dir_all(&ws_dir)?;
    let repo_dir = Path::new(&project.path);
    let wt_path = ws_dir.join(&project.name);
    let branch = ws.branch.clone();

    // Drop stale worktree registrations first so a hand-deleted directory
    // can't keep the branch marked as checked out.
    let _ = git::worktree_prune(repo_dir);

    // Create the worktree, reusing the branch if it already exists locally.
    let _ = git::fetch(repo_dir);
    if git::branch_exists(repo_dir, &branch)? {
        if let Ok(Some(holder)) = git::worktree_for_branch(repo_dir, &branch) {
            bail!(
                "{}: {}",
                project.name,
                t("branch_checked_out_elsewhere")
                    .replacen("{}", &branch, 1)
                    .replacen("{}", &holder, 1)
            );
        }
        git::worktree_add_existing(repo_dir, &wt_path, &branch)
            .with_context(|| format!("Failed to add worktree for '{}'", project.name))?;
    } else {
        let start_point = git::resolve_remote_start_point(repo_dir, &project.branches.main)?;
        git::worktree_add(repo_dir, &wt_path, &branch, &start_point)
            .with_context(|| format!("Failed to add worktree for '{}'", project.name))?;
    }

    if config::load_global_config()?.auto_upstream {
        if let Err(e) = git::ensure_upstream_config(repo_dir, &branch) {
            ui::warn(&format!("{}: {}", project.name, e));
        }
    }

    // Record in workspaces.toml; roll back the git worktree if persistence fails.
    let new_wp = WorkspaceProject {
        name: project.name.clone(),
        worktree_path: wt_path.to_string_lossy().to_string(),
    };
    if let Err(e) = update_workspace(&ws.name, |w| w.projects.push(new_wp)) {
        match git::worktree_remove(repo_dir, &wt_path) {
            Ok(()) => {
                ui::error(&t("wt_add_rollback").replace("{}", &project.name));
            }
            Err(_) => {
                ui::error(
                    &t("wt_add_rollback_failed")
                        .replacen("{}", &project.name, 1)
                        .replacen("{}", &wt_path.to_string_lossy(), 1),
                );
            }
        }
        return Err(e);
    }

    ui::success(
        &t("wt_added")
            .replacen("{}", &project.name, 1)
            .replacen("{}", &wt_path.to_string_lossy(), 1)
            .replacen("{}", &branch, 1),
    );
    maybe_sync_go_work(&ws.name);
    Ok(())
}

pub fn remove(project: Option<String>, force: bool) -> Result<()> {
    let ws = current_workspace()?;
    if ws.projects.is_empty() {
        ui::info(&t("wt_no_projects_in_workspace").replace("{}", &ws.name));
        return Ok(());
    }

    let target = match project {
        Some(name) => ws
            .projects
            .iter()
            .find(|p| p.name == name)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{}",
                    t("wt_not_in_ws")
                        .replacen("{}", &name, 1)
                        .replacen("{}", &ws.name, 1)
                )
            })?,
        None => {
            let names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();
            let idx = ui::select(&t("wt_select_remove"), &names)?;
            ws.projects[idx].clone()
        }
    };

    if ws.projects.len() == 1 {
        bail!("{}", t("wt_last_project").replace("{}", &target.name));
    }

    let projects_file = config::load_projects()?;
    let project = find_project(&projects_file, &target.name).ok_or_else(|| {
        anyhow::anyhow!("{}", t("wt_project_unknown").replace("{}", &target.name))
    })?;
    let repo_dir = Path::new(&project.path);
    let wt_path = Path::new(&target.worktree_path);

    // Safety: refuse to discard uncommitted work unless --force.
    if !force {
        match git::is_clean(wt_path) {
            Ok(false) => bail!("{}", t("wt_dirty_block").replace("{}", &target.name)),
            Ok(true) => {}
            Err(_) => {} // worktree dir missing/broken; let git worktree remove decide
        }
        let prompt = t("wt_remove_confirm")
            .replacen("{}", &target.name, 1)
            .replacen("{}", &target.worktree_path, 1);
        if !ui::confirm(&prompt, false)? {
            ui::info(&t("cancelled"));
            return Ok(());
        }
    }

    // Git is the source of truth: remove the worktree first, then metadata.
    if force {
        git::worktree_remove(repo_dir, wt_path)?;
    } else {
        git::worktree_remove_checked(repo_dir, wt_path)?;
    }

    if let Err(e) = update_workspace(&ws.name, |w| {
        w.projects.retain(|p| p.name != target.name);
    }) {
        ui::error(
            &t("wt_remove_meta_failed")
                .replacen("{}", &target.name, 1)
                .replacen("{}", &e.to_string(), 1),
        );
        return Err(e);
    }

    ui::success(&t("wt_removed").replace("{}", &target.name));
    maybe_sync_go_work(&ws.name);
    Ok(())
}

pub fn prune() -> Result<()> {
    run_repo_maintenance(git::worktree_prune, "wt_pruned")
}

pub fn repair() -> Result<()> {
    run_repo_maintenance(git::worktree_repair, "wt_repaired")
}

/// Run a repo-level maintenance op against each project's main repository.
fn run_repo_maintenance<F>(op: F, success_key: &str) -> Result<()>
where
    F: Fn(&Path) -> Result<()>,
{
    let ws = current_workspace()?;
    let projects_file = config::load_projects()?;
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for wp in &ws.projects {
        let Some(project) = find_project(&projects_file, &wp.name) else {
            ui::warn(&t("wt_project_unknown").replace("{}", &wp.name));
            failed += 1;
            continue;
        };
        let repo_dir = Path::new(&project.path);
        match op(repo_dir) {
            Ok(()) => {
                ui::success(&t(success_key).replace("{}", &wp.name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, ProjectsFile};
    use std::collections::BTreeMap;

    fn ws_with_projects(paths: &[(&str, &str)]) -> Workspace {
        Workspace {
            name: "feature-x".to_string(),
            branch: "feature/x".to_string(),
            created_at: "2026-05-30".to_string(),
            projects: paths
                .iter()
                .map(|(name, path)| WorkspaceProject {
                    name: (*name).to_string(),
                    worktree_path: (*path).to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_workspace_dir_inferred_from_worktree_parent() {
        let ws = ws_with_projects(&[
            ("api", "/work/feature-x/api"),
            ("web", "/work/feature-x/web"),
        ]);
        let dir = workspace_dir(&ws).unwrap();
        assert_eq!(dir, PathBuf::from("/work/feature-x"));
    }

    #[test]
    fn test_find_project_matches_by_name() {
        let pf = ProjectsFile {
            groups: Vec::new(),
            projects: vec![Project {
                name: "api".to_string(),
                path: "/repos/api".to_string(),
                group: String::new(),
                order: 0,
                tags: Vec::new(),
                branch_aliases: BTreeMap::new(),
                branches: BranchConfig {
                    main: "main".to_string(),
                    aliases: BTreeMap::new(),
                },
            }],
        };

        assert_eq!(find_project(&pf, "api").map(|p| p.path.as_str()), Some("/repos/api"));
        assert!(find_project(&pf, "missing").is_none());
    }
}
