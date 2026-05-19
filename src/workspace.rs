use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{self, Project, ProjectsFile, Workspace};
use crate::i18n::t;
use crate::ui;

/// Detect if the current working directory is inside a workspace.
/// Returns the workspace if found.
pub fn detect_workspace(cwd: &Path) -> Result<Option<Workspace>> {
    let workspaces_file = config::load_workspaces()?;
    let global = config::load_global_config()?;
    let workpath = config::resolve_workpath(&global.workpath)?;

    for ws in &workspaces_file.workspaces {
        let ws_dir = workpath.join(config::safe_dir_name(&ws.name));
        if cwd.starts_with(&ws_dir) {
            return Ok(Some(ws.clone()));
        }
    }
    Ok(None)
}

/// Get the current workspace by detecting from cwd, or prompt the user to select one.
pub fn get_or_select_workspace() -> Result<Workspace> {
    let cwd = std::env::current_dir().context("Could not determine current directory")?;

    // Try to detect workspace from cwd
    if let Some(ws) = detect_workspace(&cwd)? {
        return Ok(ws);
    }

    // Fall back to prompting the user
    let workspaces_file = config::load_workspaces()?;
    if workspaces_file.workspaces.is_empty() {
        anyhow::bail!("{}", t("no_workspaces_found"));
    }

    let names: Vec<String> = workspaces_file
        .workspaces
        .iter()
        .map(|ws| ws.name.clone())
        .collect();
    let idx = ui::select(&t("select_workspace"), &names)?;
    Ok(workspaces_file.workspaces[idx].clone())
}

/// Compute the intersection of environment branch names across the given projects.
/// Returns environment names (test, staging, prod) that ALL specified projects have.
#[allow(dead_code)]
pub fn common_environments(projects_file: &ProjectsFile, project_names: &[String]) -> Vec<String> {
    let matching_projects: Vec<&Project> = projects_file
        .projects
        .iter()
        .filter(|p| project_names.contains(&p.name))
        .collect();

    if matching_projects.is_empty() {
        return Vec::new();
    }

    let env_names = ["test", "staging", "prod"];
    let mut common = Vec::new();

    for env_name in &env_names {
        let all_have_it = matching_projects
            .iter()
            .all(|p| get_env_branch(p, env_name).is_some());
        if all_have_it {
            common.push(env_name.to_string());
        }
    }

    common
}

/// Get the environment branch for a project by environment name.
#[allow(dead_code)]
pub fn get_env_branch<'a>(project: &'a Project, env_name: &str) -> Option<&'a str> {
    project.branches.get(env_name)
}

/// Resolve the worktree path for a workspace project.
#[allow(dead_code)]
pub fn resolve_worktree_path(
    workpath: &str,
    workspace_name: &str,
    project_name: &str,
) -> Result<PathBuf> {
    let base = config::resolve_workpath(workpath)?;
    Ok(base.join(workspace_name).join(project_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project, ProjectsFile};
    use std::collections::BTreeMap;

    fn make_project(name: &str, branches: BranchConfig) -> Project {
        Project {
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            branch_aliases: BTreeMap::new(),
            branches,
        }
    }

    fn default_branches() -> BranchConfig {
        BranchConfig {
            main: "main".to_string(),
            aliases: BTreeMap::new(),
        }
    }

    #[test]
    fn test_common_environments_all_have_test() {
        let pf = ProjectsFile {
            groups: vec![],
            projects: vec![
                make_project(
                    "a",
                    BranchConfig {
                        main: "main".to_string(),
                        aliases: BTreeMap::from([("test".to_string(), "test".to_string())]),
                    },
                ),
                make_project(
                    "b",
                    BranchConfig {
                        main: "main".to_string(),
                        aliases: BTreeMap::from([("test".to_string(), "develop".to_string())]),
                    },
                ),
            ],
        };

        let envs = common_environments(&pf, &["a".to_string(), "b".to_string()]);
        assert_eq!(envs, vec!["test".to_string()]);
    }

    #[test]
    fn test_common_environments_none() {
        let pf = ProjectsFile {
            groups: vec![],
            projects: vec![
                make_project(
                    "a",
                    BranchConfig {
                        main: "main".to_string(),
                        aliases: BTreeMap::from([("test".to_string(), "test".to_string())]),
                    },
                ),
                make_project("b", default_branches()),
            ],
        };

        let envs = common_environments(&pf, &["a".to_string(), "b".to_string()]);
        assert!(envs.is_empty());
    }
}
