use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{self, Project, Workspace, WorkspaceProject};
use crate::ui;
use crate::workspace;

pub fn run() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    sync_workspace(&ws, &projects)
}

pub fn sync_workspace(ws: &Workspace, projects: &[(WorkspaceProject, Project)]) -> Result<()> {
    let ws_dir = infer_workspace_dir(ws)?;

    let go_projects: Vec<(WorkspaceProject, Project)> = projects
        .iter()
        .filter(|(wp, project)| project_is_go_project(Path::new(&wp.worktree_path), project))
        .cloned()
        .collect();

    if go_projects.is_empty() {
        remove_go_work_files(&ws_dir)?;
        ui::info("No Go projects with go.mod found in this workspace");
        return Ok(());
    }

    if go_projects.len() < 2 {
        remove_go_work_files(&ws_dir)?;
        ui::info("Only one Go project in this workspace; skipping go.work");
        return Ok(());
    }

    let project_paths: Vec<PathBuf> = go_projects
        .iter()
        .map(|(wp, _)| PathBuf::from(&wp.worktree_path))
        .collect();
    rebuild_go_work(&ws_dir, &project_paths)?;

    ui::success(&format!(
        "go.work updated for workspace '{}' ({} projects)",
        ws.name,
        go_projects.len()
    ));
    Ok(())
}

/// Derive the workspace directory from stored worktree paths.
/// Each worktree_path is `<ws_dir>/<project_name>`, so the parent is the actual ws_dir.
fn infer_workspace_dir(ws: &Workspace) -> Result<PathBuf> {
    let first = ws.projects.first().context("Workspace has no projects")?;
    let wt = Path::new(&first.worktree_path);
    wt.parent()
        .map(PathBuf::from)
        .context("Cannot determine workspace directory from worktree path")
}

fn get_workspace_context() -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;

    let mut matched = Vec::new();
    for wp in &ws.projects {
        if let Some(proj) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            matched.push((wp.clone(), proj.clone()));
        }
    }

    Ok((ws, matched))
}

pub fn project_has_go_mod(path: &Path) -> bool {
    path.join("go.mod").is_file()
}

pub fn project_is_go_project(worktree_path: &Path, _project: &Project) -> bool {
    project_has_go_mod(worktree_path)
}

fn rebuild_go_work(workspace_dir: &Path, project_paths: &[PathBuf]) -> Result<()> {
    let tmp_go_work = workspace_dir.join(".grove-go.work.tmp");
    let tmp_go_work_sum = workspace_dir.join(".grove-go.work.tmp.sum");
    remove_file_if_exists(&tmp_go_work)?;
    remove_file_if_exists(&tmp_go_work_sum)?;

    let result = (|| -> Result<()> {
        let final_go_work = workspace_dir.join("go.work");
        let final_go_work_sum = workspace_dir.join("go.work.sum");

        let init_args = build_go_work_init_args(workspace_dir, project_paths);
        let init_arg_refs: Vec<&str> = init_args.iter().map(String::as_str).collect();
        run_go_with_workspace_file(workspace_dir, &init_arg_refs, Some(&tmp_go_work))?;
        run_go_with_workspace_file(workspace_dir, &["work", "sync"], Some(&tmp_go_work))?;

        fs::rename(&tmp_go_work, &final_go_work).with_context(|| {
            format!(
                "Failed to replace {} with {}",
                final_go_work.display(),
                tmp_go_work.display()
            )
        })?;

        if tmp_go_work_sum.exists() {
            fs::rename(&tmp_go_work_sum, &final_go_work_sum).with_context(|| {
                format!(
                    "Failed to replace {} with {}",
                    final_go_work_sum.display(),
                    tmp_go_work_sum.display()
                )
            })?;
        } else if final_go_work_sum.exists() {
            fs::remove_file(&final_go_work_sum)
                .with_context(|| format!("Failed to remove {}", final_go_work_sum.display()))?;
        }

        Ok(())
    })();

    let _ = fs::remove_file(&tmp_go_work);
    let _ = fs::remove_file(&tmp_go_work_sum);
    result
}

fn build_go_work_init_args(workspace_dir: &Path, project_paths: &[PathBuf]) -> Vec<String> {
    let mut args = vec!["work".to_string(), "init".to_string()];
    args.extend(
        project_paths
            .iter()
            .map(|path| go_work_use_path(workspace_dir, path)),
    );
    args
}

fn go_work_use_path(workspace_dir: &Path, project_path: &Path) -> String {
    if let Ok(relative) = project_path.strip_prefix(workspace_dir) {
        return format!("./{}", relative.to_string_lossy().replace('\\', "/"));
    }

    let canonical_ws = fs::canonicalize(workspace_dir).ok();
    let canonical_proj = fs::canonicalize(project_path).ok();
    if let (Some(ws), Some(proj)) = (canonical_ws, canonical_proj) {
        if let Ok(relative) = proj.strip_prefix(&ws) {
            return format!("./{}", relative.to_string_lossy().replace('\\', "/"));
        }
    }

    project_path.to_string_lossy().replace('\\', "/")
}

fn remove_go_work_files(workspace_dir: &Path) -> Result<()> {
    for file_name in ["go.work", "go.work.sum"] {
        let path = workspace_dir.join(file_name);
        remove_file_if_exists(&path)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn run_go_with_workspace_file(dir: &Path, args: &[&str], gowork: Option<&Path>) -> Result<()> {
    let mut command = Command::new("go");
    command.current_dir(dir).args(args);
    if let Some(gowork) = gowork {
        command.env("GOWORK", gowork);
    }

    let output = command
        .output()
        .with_context(|| format!("Failed to run go {:?} in {}", args, dir.display()))?;

    if !output.status.success() {
        bail!(
            "go {:?} failed in {}: {}",
            args,
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_project_has_go_mod() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!project_has_go_mod(tmp.path()));

        fs::write(tmp.path().join("go.mod"), "module example.com/app\n").unwrap();
        assert!(project_has_go_mod(tmp.path()));
    }

    #[test]
    fn test_project_is_go_project_when_tagged_go() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("go.mod"), "module example.com/app\n").unwrap();
        let project = Project {
            name: "api".to_string(),
            path: tmp.path().to_string_lossy().to_string(),
            group: String::new(),
            order: 0,
            tags: vec!["go".to_string()],
            agents_md: None,
            branch_aliases: std::collections::BTreeMap::new(),
            branches: config::BranchConfig {
                main: "main".to_string(),
                aliases: std::collections::BTreeMap::new(),
            },
        };

        assert!(project_is_go_project(tmp.path(), &project));
    }

    #[test]
    fn test_project_is_go_project_requires_worktree_go_mod_even_when_tagged() {
        let tmp = tempfile::tempdir().unwrap();
        let project = Project {
            name: "api".to_string(),
            path: tmp.path().to_string_lossy().to_string(),
            group: String::new(),
            order: 0,
            tags: vec!["go".to_string()],
            agents_md: None,
            branch_aliases: std::collections::BTreeMap::new(),
            branches: config::BranchConfig {
                main: "main".to_string(),
                aliases: std::collections::BTreeMap::new(),
            },
        };

        assert!(!project_is_go_project(tmp.path(), &project));
    }

    #[test]
    fn test_project_is_go_project_when_go_mod_exists() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("go.mod"), "module example.com/app\n").unwrap();
        let project = Project {
            name: "api".to_string(),
            path: tmp.path().to_string_lossy().to_string(),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            agents_md: None,
            branch_aliases: std::collections::BTreeMap::new(),
            branches: config::BranchConfig {
                main: "main".to_string(),
                aliases: std::collections::BTreeMap::new(),
            },
        };

        assert!(project_is_go_project(tmp.path(), &project));
    }

    #[test]
    #[cfg(unix)]
    fn test_go_work_use_path_resolves_symlinked_workspace_dir() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let real_ws = tmp.path().join("real_ws");
        fs::create_dir_all(real_ws.join("common")).unwrap();
        fs::create_dir_all(real_ws.join("client")).unwrap();

        let link_ws = tmp.path().join("link_ws");
        symlink(&real_ws, &link_ws).unwrap();

        let path = go_work_use_path(&link_ws, &real_ws.join("common"));
        assert_eq!(path, "./common");
    }

    #[test]
    fn test_build_go_work_init_args_uses_workspace_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace_dir = tmp.path();
        let common = workspace_dir.join("common");
        let client = workspace_dir.join("table_server_client");

        let args = build_go_work_init_args(workspace_dir, &[common, client]);

        assert_eq!(
            args,
            vec![
                "work".to_string(),
                "init".to_string(),
                "./common".to_string(),
                "./table_server_client".to_string(),
            ]
        );
    }
}
