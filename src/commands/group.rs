use anyhow::{bail, Result};

use crate::config::{self, Group};
use crate::i18n::t;
use crate::ui;

/// Add a new group.
pub fn add(name: &str) -> Result<()> {
    let mut pf = config::load_projects()?;

    // Check for duplicate
    if pf.groups.iter().any(|g| g.name == name) {
        bail!("{}", t("group_exists").replace("{}", name));
    }

    let order = pf
        .groups
        .iter()
        .map(|g| g.order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    pf.groups.push(Group {
        name: name.to_string(),
        order,
    });

    config::save_projects(&pf)?;
    ui::success(&t("group_created").replace("{}", name));
    Ok(())
}

/// Remove a group (moves its projects to ungrouped).
pub fn remove() -> Result<()> {
    let mut pf = config::load_projects()?;

    if pf.groups.is_empty() {
        ui::info(&t("no_groups"));
        return Ok(());
    }

    let group_names: Vec<String> = pf.groups.iter().map(|g| g.name.clone()).collect();
    let idx = ui::select(&t("select_group_remove"), &group_names)?;
    let group_name = group_names[idx].clone();

    // Move projects in this group to ungrouped
    let moved_count = pf
        .projects
        .iter_mut()
        .filter(|p| p.group == group_name)
        .map(|p| {
            p.group = String::new();
        })
        .count();

    // Remove the group
    pf.groups.retain(|g| g.name != group_name);

    config::save_projects(&pf)?;

    if moved_count > 0 {
        ui::info(&t("projects_become_ungrouped").replace("{}", &moved_count.to_string()));
    }
    ui::success(&t("group_removed").replace("{}", &group_name));
    Ok(())
}

/// List all groups with project counts.
pub fn list() -> Result<()> {
    let pf = config::load_projects()?;

    if pf.groups.is_empty() && pf.projects.is_empty() {
        println!("No groups or projects defined.");
        return Ok(());
    }

    let mut groups = pf.groups.clone();
    groups.sort_by_key(|g| g.order);

    for group in &groups {
        let count = pf.projects.iter().filter(|p| p.group == group.name).count();
        println!("  {} ({} project{})", group.name, count, if count == 1 { "" } else { "s" });
    }

    let ungrouped_count = pf.projects.iter().filter(|p| p.group.is_empty()).count();
    if ungrouped_count > 0 {
        let dim = console::Style::new().dim();
        println!(
            "  {} ({} project{})",
            dim.apply_to(t("ungrouped")),
            ungrouped_count,
            if ungrouped_count == 1 { "" } else { "s" }
        );
    }

    if pf.groups.is_empty() && ungrouped_count > 0 {
        println!("  No groups defined. All {} project(s) are ungrouped.", ungrouped_count);
    }

    Ok(())
}

/// Reorder groups by selecting a group and a new position.
pub fn reorder() -> Result<()> {
    let mut pf = config::load_projects()?;

    if pf.groups.len() < 2 {
        ui::info("Need at least 2 groups to reorder.");
        return Ok(());
    }

    // Sort groups by current order for display
    pf.groups.sort_by_key(|g| g.order);

    let group_names: Vec<String> = pf.groups.iter().map(|g| g.name.clone()).collect();
    let idx = ui::select(&t("move_which_group"), &group_names)?;

    // Build position options
    let positions: Vec<String> = (1..=pf.groups.len())
        .map(|i| {
            if i - 1 == idx {
                format!("{} (current)", i)
            } else {
                format!("{}", i)
            }
        })
        .collect();

    let new_pos = ui::select(&t("move_to_position"), &positions)?;

    if new_pos == idx {
        ui::info("Position unchanged.");
        return Ok(());
    }

    // Move the group in the vec
    let group = pf.groups.remove(idx);
    pf.groups.insert(new_pos, group);

    // Reassign order values
    for (i, g) in pf.groups.iter_mut().enumerate() {
        g.order = i as u32;
    }

    config::save_projects(&pf)?;
    ui::success(&t("groups_reordered"));
    Ok(())
}
