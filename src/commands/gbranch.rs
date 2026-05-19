use anyhow::Result;
use std::path::Path;

use crate::git;
use crate::ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchDisplay {
    Name(String),
    Detached,
    Unknown,
}

pub fn label_for(display: &BranchDisplay) -> String {
    match display {
        BranchDisplay::Name(name) => name.clone(),
        BranchDisplay::Detached => "(detached)".to_string(),
        BranchDisplay::Unknown => "(unknown)".to_string(),
    }
}

pub fn format_branch_lines(entries: &[(String, BranchDisplay)]) -> Vec<String> {
    let labels: Vec<String> = entries.iter().map(|(_, d)| label_for(d)).collect();
    if !labels.is_empty() && labels.iter().all(|l| l == &labels[0]) {
        return vec![labels[0].clone()];
    }
    entries
        .iter()
        .map(|(name, display)| format!("{}: {}", name, label_for(display)))
        .collect()
}

pub fn run() -> Result<()> {
    let (_ws, projects) = super::workspace_context::get_workspace_context()?;
    let mut entries = Vec::new();

    for (wp, _project) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        if !wt_path.exists() {
            ui::error(&format!(
                "{}: worktree path does not exist: {}",
                wp.name, wp.worktree_path
            ));
            entries.push((wp.name.clone(), BranchDisplay::Unknown));
            continue;
        }
        match git::current_branch(wt_path) {
            Ok(branch) => entries.push((wp.name.clone(), BranchDisplay::Name(branch))),
            Err(_) => entries.push((wp.name.clone(), BranchDisplay::Detached)),
        }
    }

    for line in format_branch_lines(&entries) {
        println!("{}", line);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_all_same_single_line() {
        let entries = vec![
            (
                "api".to_string(),
                BranchDisplay::Name("feature/login".into()),
            ),
            (
                "web".to_string(),
                BranchDisplay::Name("feature/login".into()),
            ),
        ];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["feature/login"]);
    }

    #[test]
    fn test_format_different_per_project() {
        let entries = vec![
            (
                "api".to_string(),
                BranchDisplay::Name("feature/login".into()),
            ),
            ("web".to_string(), BranchDisplay::Name("develop".into())),
        ];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["api: feature/login", "web: develop"]);
    }

    #[test]
    fn test_format_detached_breaks_uniformity() {
        let entries = vec![
            ("api".to_string(), BranchDisplay::Name("main".into())),
            ("web".to_string(), BranchDisplay::Detached),
        ];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["api: main", "web: (detached)"]);
    }

    #[test]
    fn test_format_unknown_breaks_uniformity() {
        let entries = vec![
            ("api".to_string(), BranchDisplay::Unknown),
            ("web".to_string(), BranchDisplay::Name("main".into())),
        ];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["api: (unknown)", "web: main"]);
    }

    #[test]
    fn test_format_single_project() {
        let entries = vec![("api".to_string(), BranchDisplay::Name("main".into()))];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["main"]);
    }
}
