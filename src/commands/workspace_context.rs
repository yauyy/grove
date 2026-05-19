use anyhow::Result;

use crate::config::{self, Project, Workspace, WorkspaceProject};
use crate::workspace;

pub fn get_workspace_context() -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    for wp in &ws.projects {
        if let Some(proj) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            matched.push((wp.clone(), proj.clone()));
        } else {
            missing.push(wp.name.clone());
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Workspace '{}' references missing project(s): {}",
            ws.name,
            missing.join(", ")
        );
    }

    Ok((ws, matched))
}
