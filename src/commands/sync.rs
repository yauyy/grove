use anyhow::Result;
use std::path::Path;

use crate::config;
use crate::git;
use crate::ui;
use crate::workspace;

pub fn run() -> Result<()> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for wp in &ws.projects {
        let wt_path = Path::new(&wp.worktree_path);

        // Look up the project's main branch from the registry
        let main_branch = projects_file
            .projects
            .iter()
            .find(|p| p.name == wp.name)
            .map(|p| p.branches.main.clone())
            .unwrap_or_else(|| "main".to_string());

        let remote_main = format!("origin/{}", main_branch);

        let result = (|| -> Result<()> {
            git::fetch(wt_path)?;
            git::merge(wt_path, &remote_main)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                ui::success(&format!("{}: synced with {}", wp.name, remote_main));
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
