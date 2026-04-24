use anyhow::Result;
use std::path::Path;

use crate::commands::add;
use crate::config::{self, Project};
use crate::ui;

pub fn run() -> Result<()> {
    let mut projects_file = config::load_projects()?;
    let mut updated = 0usize;

    for project in &mut projects_file.projects {
        let detected = add::detect_project_tags(Path::new(&project.path));
        if merge_detected_tags(project, detected) {
            updated += 1;
        }
    }

    config::save_projects(&projects_file)?;
    ui::success(&format!("Updated tags for {} project(s)", updated));
    Ok(())
}

pub fn merge_detected_tags(project: &mut Project, detected: Vec<String>) -> bool {
    let mut changed = false;
    for tag in detected {
        if !project.tags.iter().any(|existing| existing == &tag) {
            project.tags.push(tag);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_tags(tags: Vec<&str>) -> Project {
        Project {
            name: "api".to_string(),
            path: "/tmp/api".to_string(),
            group: String::new(),
            order: 0,
            tags: tags.into_iter().map(str::to_string).collect(),
            agents_md: None,
            branches: config::BranchConfig {
                main: "main".to_string(),
                test: None,
                staging: None,
                prod: None,
            },
        }
    }

    #[test]
    fn test_merge_detected_tags_adds_missing_tags() {
        let mut project = project_with_tags(vec![]);
        let changed = merge_detected_tags(&mut project, vec!["go".to_string()]);

        assert!(changed);
        assert_eq!(project.tags, vec!["go"]);
    }

    #[test]
    fn test_merge_detected_tags_preserves_existing_tags_without_duplicates() {
        let mut project = project_with_tags(vec!["go", "backend"]);
        let changed = merge_detected_tags(&mut project, vec!["go".to_string()]);

        assert!(!changed);
        assert_eq!(project.tags, vec!["go", "backend"]);
    }
}
