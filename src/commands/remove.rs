use anyhow::Result;

use crate::config;
use crate::i18n::t;
use crate::ui;

pub fn run() -> Result<()> {
    let mut pf = config::load_projects()?;

    if pf.projects.is_empty() {
        ui::info(&t("no_projects"));
        return Ok(());
    }

    // Build display names with group info
    let display_names: Vec<String> = pf
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

    let idx = ui::select(&t("select_project_remove"), &display_names)?;
    let project_name = pf.projects[idx].name.clone();

    // Check if project is in any workspace
    let wf = config::load_workspaces()?;
    let in_workspaces: Vec<String> = wf
        .workspaces
        .iter()
        .filter(|ws| ws.projects.iter().any(|wp| wp.name == project_name))
        .map(|ws| ws.name.clone())
        .collect();

    if !in_workspaces.is_empty() {
        ui::warn(&t("project_in_workspaces")
            .replacen("{}", &project_name, 1)
            .replacen("{}", &in_workspaces.join(", "), 1));
        if !ui::confirm(&t("remove_anyway"), false)? {
            ui::info(&t("cancelled"));
            return Ok(());
        }
    }

    pf.projects.remove(idx);
    config::save_projects(&pf)?;

    ui::success(&t("project_removed").replace("{}", &project_name));
    Ok(())
}
