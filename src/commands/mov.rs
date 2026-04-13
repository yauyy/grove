use anyhow::Result;

use crate::config;
use crate::ui;

pub fn run(project: Option<String>) -> Result<()> {
    let mut pf = config::load_projects()?;

    if pf.projects.is_empty() {
        ui::info("No projects registered.");
        return Ok(());
    }

    // 1. Find or select project
    let project_idx = match project {
        Some(ref name) => {
            pf.projects
                .iter()
                .position(|p| p.name == *name)
                .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", name))?
        }
        None => {
            let names: Vec<String> = pf
                .projects
                .iter()
                .map(|p| {
                    if p.group.is_empty() {
                        format!("{} (ungrouped)", p.name)
                    } else {
                        format!("{} [{}]", p.name, p.group)
                    }
                })
                .collect();
            ui::select("Select project to move", &names)?
        }
    };

    let project_name = pf.projects[project_idx].name.clone();
    let current_group = pf.projects[project_idx].group.clone();

    // 2. Show target group options
    let mut options: Vec<String> = pf.groups.iter().map(|g| g.name.clone()).collect();
    options.push("Ungrouped".to_string());

    let target_idx = ui::select("Select target group", &options)?;

    let new_group = if target_idx == options.len() - 1 {
        String::new() // Ungrouped
    } else {
        options[target_idx].clone()
    };

    if new_group == current_group {
        ui::info("Project is already in that group.");
        return Ok(());
    }

    // 3. Update project.group and project.order
    let new_order = pf
        .projects
        .iter()
        .filter(|p| p.group == new_group)
        .map(|p| p.order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    pf.projects[project_idx].group = new_group.clone();
    pf.projects[project_idx].order = new_order;

    // 4. Save
    config::save_projects(&pf)?;

    let display_group = if new_group.is_empty() {
        "Ungrouped".to_string()
    } else {
        new_group
    };
    ui::success(&format!("Moved '{}' to '{}'", project_name, display_group));
    Ok(())
}
