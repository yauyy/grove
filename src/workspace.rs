use anyhow::{Context, Result};
use std::fs;
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

/// Merge agents.md files from multiple projects into a single output file.
/// Each project's content is preceded by a `# project-name` header and separated by `---`.
/// If no projects have agents_md set, no file is created.
pub fn merge_agents_md(projects: &[Project], output_path: &Path) -> Result<bool> {
    let mut sections: Vec<String> = Vec::new();

    for project in projects {
        if let Some(ref agents_path) = project.agents_md {
            let path = Path::new(agents_path);
            if path.exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                sections.push(format!("# {}\n\n{}", project.name, content));
            }
        }
    }

    if sections.is_empty() {
        return Ok(false);
    }

    let merged = sections.join("\n\n---\n\n");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, merged)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(true)
}

/// Compute the intersection of environment branch names across the given projects.
/// Returns environment names (test, staging, prod) that ALL specified projects have.
pub fn common_environments(
    projects_file: &ProjectsFile,
    project_names: &[String],
) -> Vec<String> {
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
pub fn get_env_branch<'a>(project: &'a Project, env_name: &str) -> Option<&'a String> {
    match env_name {
        "test" => project.branches.test.as_ref(),
        "staging" => project.branches.staging.as_ref(),
        "prod" => project.branches.prod.as_ref(),
        _ => None,
    }
}

/// Resolve the worktree path for a workspace project.
#[allow(dead_code)]
pub fn resolve_worktree_path(workpath: &str, workspace_name: &str, project_name: &str) -> Result<PathBuf> {
    let base = config::resolve_workpath(workpath)?;
    Ok(base.join(workspace_name).join(project_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project, ProjectsFile};
    use tempfile::TempDir;

    fn make_project(name: &str, agents_md: Option<&str>, branches: BranchConfig) -> Project {
        Project {
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            group: String::new(),
            order: 0,
            agents_md: agents_md.map(|s| s.to_string()),
            branches,
        }
    }

    fn default_branches() -> BranchConfig {
        BranchConfig {
            main: "main".to_string(),
            test: None,
            staging: None,
            prod: None,
        }
    }

    #[test]
    fn test_merge_agents_md_single() {
        let tmp = TempDir::new().unwrap();
        let agents_path = tmp.path().join("agents1.md");
        fs::write(&agents_path, "Agent instructions for project A").unwrap();

        let projects = vec![make_project(
            "project-a",
            Some(agents_path.to_str().unwrap()),
            default_branches(),
        )];

        let output = tmp.path().join("merged.md");
        let created = merge_agents_md(&projects, &output).unwrap();
        assert!(created);

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("# project-a"));
        assert!(content.contains("Agent instructions for project A"));
    }

    #[test]
    fn test_merge_agents_md_multiple() {
        let tmp = TempDir::new().unwrap();

        let agents1 = tmp.path().join("agents1.md");
        fs::write(&agents1, "Instructions for A").unwrap();

        let agents2 = tmp.path().join("agents2.md");
        fs::write(&agents2, "Instructions for B").unwrap();

        let projects = vec![
            make_project(
                "project-a",
                Some(agents1.to_str().unwrap()),
                default_branches(),
            ),
            make_project(
                "project-b",
                Some(agents2.to_str().unwrap()),
                default_branches(),
            ),
        ];

        let output = tmp.path().join("merged.md");
        let created = merge_agents_md(&projects, &output).unwrap();
        assert!(created);

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("# project-a"));
        assert!(content.contains("# project-b"));
        assert!(content.contains("---"));
        assert!(content.contains("Instructions for A"));
        assert!(content.contains("Instructions for B"));
    }

    #[test]
    fn test_merge_agents_md_no_agents() {
        let tmp = TempDir::new().unwrap();
        let projects = vec![make_project("project-a", None, default_branches())];

        let output = tmp.path().join("merged.md");
        let created = merge_agents_md(&projects, &output).unwrap();
        assert!(!created);
        assert!(!output.exists());
    }

    #[test]
    fn test_common_environments_all_have_test() {
        let pf = ProjectsFile {
            groups: vec![],
            projects: vec![
                make_project(
                    "a",
                    None,
                    BranchConfig {
                        main: "main".to_string(),
                        test: Some("test".to_string()),
                        staging: None,
                        prod: None,
                    },
                ),
                make_project(
                    "b",
                    None,
                    BranchConfig {
                        main: "main".to_string(),
                        test: Some("develop".to_string()),
                        staging: None,
                        prod: None,
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
                    None,
                    BranchConfig {
                        main: "main".to_string(),
                        test: Some("test".to_string()),
                        staging: None,
                        prod: None,
                    },
                ),
                make_project("b", None, default_branches()),
            ],
        };

        let envs = common_environments(&pf, &["a".to_string(), "b".to_string()]);
        assert!(envs.is_empty());
    }
}
