use anyhow::Result;
use std::path::Path;

use crate::config::{self, GcreateRecord, ProjectsFile};
use crate::git;
use crate::ui;

#[derive(Debug, Clone)]
pub struct DeleteProjectItem {
    pub name: String,
    pub worktree_path: String,
}

#[derive(Debug, Default)]
pub struct DeleteBranchOutcome {
    pub hard_errors: Vec<String>,
    pub operable: usize,
}

pub fn project_main_branch(projects_file: &ProjectsFile, project_name: &str) -> String {
    projects_file
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .map(|p| p.branches.main.clone())
        .unwrap_or_else(|| "main".to_string())
}

pub fn delete_branch_across_projects(
    items: &[DeleteProjectItem],
    branch: &str,
    projects_file: &ProjectsFile,
) -> DeleteBranchOutcome {
    let mut outcome = DeleteBranchOutcome::default();

    for item in items {
        let path = Path::new(&item.worktree_path);
        if !path.exists() {
            ui::info(&format!(
                "{}: worktree path does not exist (skipped): {}",
                item.name, item.worktree_path
            ));
            continue;
        }

        outcome.operable += 1;

        if let Ok(false) = git::is_clean(path) {
            outcome
                .hard_errors
                .push(format!("{}: working tree has uncommitted changes", item.name));
            continue;
        }

        match git::branch_exists(path, branch) {
            Ok(false) => {
                ui::info(&format!(
                    "{}: branch '{}' does not exist (skipped)",
                    item.name, branch
                ));
            }
            Ok(true) => {
                if let Ok(current) = git::current_branch(path) {
                    if current == branch {
                        let main_branch = project_main_branch(projects_file, &item.name);
                        if let Err(e) = git::checkout(path, &main_branch) {
                            outcome.hard_errors.push(format!(
                                "{}: failed to switch to main before delete: {}",
                                item.name, e
                            ));
                            continue;
                        }
                    }
                }
                if let Err(e) = git::branch_delete(path, branch) {
                    outcome.hard_errors.push(format!("{}: {}", item.name, e));
                } else {
                    ui::success(&format!("{}: deleted branch '{}'", item.name, branch));
                }
            }
            Err(e) => outcome.hard_errors.push(format!("{}: {}", item.name, e)),
        }
    }

    outcome
}

pub fn items_from_record(record: &GcreateRecord) -> Vec<DeleteProjectItem> {
    record
        .projects
        .iter()
        .map(|p| DeleteProjectItem {
            name: p.name.clone(),
            worktree_path: p.worktree_path.clone(),
        })
        .collect()
}

pub fn items_from_workspace(
    projects: &[(config::WorkspaceProject, config::Project)],
) -> Vec<DeleteProjectItem> {
    projects
        .iter()
        .map(|(wp, _)| DeleteProjectItem {
            name: wp.name.clone(),
            worktree_path: wp.worktree_path.clone(),
        })
        .collect()
}

pub fn update_workspace_branch_if_matches(
    workspace_name: &str,
    deleted_branch: &str,
    fallback_main: &str,
) -> Result<()> {
    let workspaces = config::load_workspaces()?;
    if !workspaces
        .workspaces
        .iter()
        .any(|ws| ws.name == workspace_name && ws.branch == deleted_branch)
    {
        return Ok(());
    }
    let mut workspaces_file = config::load_workspaces()?;
    if let Some(ws_mut) = workspaces_file
        .workspaces
        .iter_mut()
        .find(|w| w.name == workspace_name)
    {
        ws_mut.branch = fallback_main.to_string();
        config::save_workspaces(&workspaces_file)?;
    }
    Ok(())
}

pub fn finalize_record_delete(
    record: &GcreateRecord,
    projects_file: &ProjectsFile,
    records_file: &mut config::GcreateRecordsFile,
) -> Result<()> {
    let fallback_main = record
        .projects
        .first()
        .map(|p| project_main_branch(projects_file, &p.name))
        .unwrap_or_else(|| "main".to_string());
    crate::gcreate_records::remove_record_by_id(records_file, &record.id);
    update_workspace_branch_if_matches(&record.workspace, &record.branch, &fallback_main)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project, ProjectsFile};

    fn sample_projects() -> ProjectsFile {
        ProjectsFile {
            groups: vec![],
            projects: vec![Project {
                name: "api".to_string(),
                path: "/tmp/api".to_string(),
                group: "g".to_string(),
                order: 0,
                tags: vec![],
                branch_aliases: Default::default(),
                branches: BranchConfig {
                    main: "master".to_string(),
                    aliases: Default::default(),
                },
            }],
        }
    }

    #[test]
    fn test_project_main_branch_found() {
        let pf = sample_projects();
        assert_eq!(project_main_branch(&pf, "api"), "master");
    }

    #[test]
    fn test_project_main_branch_fallback() {
        let pf = sample_projects();
        assert_eq!(project_main_branch(&pf, "missing"), "main");
    }
}
