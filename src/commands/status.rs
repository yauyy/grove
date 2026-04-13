use anyhow::Result;
use console::Style;
use std::path::Path;

use crate::config;
use crate::git;
use crate::i18n::t;

pub fn run() -> Result<()> {
    // 1. Load workspaces
    let workspaces_file = config::load_workspaces()?;

    // 2. If empty, print message
    if workspaces_file.workspaces.is_empty() {
        println!("{}", t("no_workspaces_status"));
        return Ok(());
    }

    let bold = Style::new().bold();
    let dim = Style::new().dim();
    let green = Style::new().green();
    let yellow = Style::new().yellow();

    // 3. For each workspace
    for ws in &workspaces_file.workspaces {
        println!(
            "{} {} {}",
            bold.apply_to(&ws.name),
            dim.apply_to(format!("({})", ws.branch)),
            dim.apply_to(&ws.created_at),
        );

        // 4. For each project in workspace
        for wp in &ws.projects {
            let wt_path = Path::new(&wp.worktree_path);

            let status_str = if !wt_path.exists() {
                t("missing")
            } else {
                match git::status_short(wt_path) {
                    Ok(output) => {
                        if output.is_empty() {
                            format!("{}", green.apply_to(t("clean")))
                        } else {
                            let change_count = output.lines().count();
                            format!(
                                "{}",
                                yellow.apply_to(t("changes").replace("{}", &change_count.to_string()))
                            )
                        }
                    }
                    Err(_) => "error".to_string(),
                }
            };

            println!("  {} {}", wp.name, status_str);
        }
        println!();
    }

    Ok(())
}
