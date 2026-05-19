use anyhow::{bail, Result};
use std::path::Path;

use crate::commands::branch_delete::{self, finalize_record_delete, items_from_record};
use crate::config;
use crate::gcreate_records::{
    compute_record_status, format_created_display, select_record_interactive,
    sort_records_newest_first, status_label,
};
use crate::git;
use crate::i18n::t;
use crate::ui;

pub fn run(rm: bool, rename: bool) -> Result<()> {
    if rm {
        return run_rm();
    }
    if rename {
        return run_rename();
    }
    run_list()
}

fn run_list() -> Result<()> {
    let records_file = config::load_gcreate_records()?;
    let workspaces = config::load_workspaces()?;

    if records_file.records.is_empty() {
        ui::info(&t("no_gcreate_records"));
        return Ok(());
    }

    let mut records = records_file.records;
    sort_records_newest_first(&mut records);

    println!(
        "{:<14} {:<24} {:<20} STATUS",
        "WORKSPACE", "BRANCH", "CREATED"
    );

    for record in &records {
        let workspace_exists = workspaces
            .workspaces
            .iter()
            .any(|ws| ws.name == record.workspace);
        let status = compute_record_status(record, workspace_exists);
        let workspace_label = if workspace_exists {
            record.workspace.clone()
        } else {
            format!("{} (missing)", record.workspace)
        };
        println!(
            "{:<14} {:<24} {:<20} {}",
            workspace_label,
            record.branch,
            format_created_display(&record.created_at),
            status_label(status)
        );
    }

    Ok(())
}

fn run_rm() -> Result<()> {
    let mut records_file = config::load_gcreate_records()?;
    if records_file.records.is_empty() {
        ui::info(&t("no_gcreate_records"));
        return Ok(());
    }

    let idx =
        select_record_interactive(&mut records_file.records, &t("select_gcreate_record"))?;
    let record = records_file.records[idx].clone();
    let projects_file = config::load_projects()?;

    let confirm_msg = t("gcreate_delete_confirm")
        .replacen("{}", &record.branch, 1)
        .replacen("{}", &record.workspace, 1)
        .replacen("{}", &record.projects.len().to_string(), 1);
    if !ui::confirm(&confirm_msg, false)? {
        return Ok(());
    }

    let items = items_from_record(&record);
    let outcome = branch_delete::delete_branch_across_projects(
        &items,
        &record.branch,
        &projects_file,
    );

    if !outcome.hard_errors.is_empty() {
        for message in &outcome.hard_errors {
            ui::error(message);
        }
        bail!("glist --rm failed");
    }

    if outcome.operable == 0 {
        let only_record = t("gcreate_delete_record_only");
        if !ui::confirm(&only_record, false)? {
            return Ok(());
        }
    }

    finalize_record_delete(&record, &projects_file, &mut records_file)?;
    config::save_gcreate_records(&records_file)?;

    ui::success(&t("gcreate_record_deleted"));
    Ok(())
}

fn run_rename() -> Result<()> {
    let mut records_file = config::load_gcreate_records()?;
    if records_file.records.is_empty() {
        ui::info(&t("no_gcreate_records"));
        return Ok(());
    }

    let idx =
        select_record_interactive(&mut records_file.records, &t("select_gcreate_record"))?;
    let record = records_file.records[idx].clone();
    let workspaces = config::load_workspaces()?;

    if !workspaces
        .workspaces
        .iter()
        .any(|ws| ws.name == record.workspace)
    {
        bail!(
            "{}",
            t("gcreate_rename_workspace_missing").replace("{}", &record.workspace)
        );
    }

    let global = config::load_global_config()?;
    let input = ui::input_with_placeholder(&t("gcreate_rename_prompt"), &record.input)?;
    if input.trim().is_empty() {
        bail!("{}", t("gcreate_rename_empty"));
    }
    let new_branch = config::apply_git_prefix(input.trim(), &global);
    let old_branch = record.branch.clone();

    if new_branch == old_branch {
        ui::info(&t("no_changes"));
        return Ok(());
    }

    let mut failures = Vec::new();
    for project in &record.projects {
        let path = Path::new(&project.worktree_path);
        if !path.exists() {
            failures.push(format!(
                "{}: worktree path does not exist: {}",
                project.name, project.worktree_path
            ));
            continue;
        }
        if let Ok(false) = git::is_clean(path) {
            failures.push(format!(
                "{}: working tree has uncommitted changes",
                project.name
            ));
            continue;
        }
        match git::branch_exists(path, &old_branch) {
            Ok(false) => failures.push(format!(
                "{}: branch '{}' does not exist",
                project.name, old_branch
            )),
            Ok(true) => {}
            Err(e) => failures.push(format!("{}: {}", project.name, e)),
        }
        if git::branch_exists(path, &new_branch).unwrap_or(false) {
            failures.push(format!(
                "{}: branch '{}' already exists",
                project.name, new_branch
            ));
        }
    }

    if !failures.is_empty() {
        for message in &failures {
            ui::error(message);
        }
        bail!("glist --rename precheck failed");
    }

    let mut renamed = Vec::new();
    for project in &record.projects {
        let path = Path::new(&project.worktree_path);
        match git::branch_rename(path, &old_branch, &new_branch) {
            Ok(()) => {
                renamed.push(project.name.clone());
                ui::success(&format!(
                    "{}: renamed {} -> {}",
                    project.name, old_branch, new_branch
                ));
            }
            Err(e) => {
                for name in renamed.iter().rev() {
                    if let Some(project) = record.projects.iter().find(|p| p.name == *name) {
                        let path = Path::new(&project.worktree_path);
                        let _ = git::branch_rename(path, &new_branch, &old_branch);
                    }
                }
                ui::error(&format!("{}: {}", project.name, e));
                bail!("glist --rename failed");
            }
        }
    }

    let record_id = record.id.clone();
    if let Some(record_mut) = records_file
        .records
        .iter_mut()
        .find(|r| r.id == record_id)
    {
        record_mut.branch = new_branch.clone();
        record_mut.input = input.trim().to_string();
    }
    config::save_gcreate_records(&records_file)?;

    let mut workspaces_file = config::load_workspaces()?;
    if let Some(ws) = workspaces_file
        .workspaces
        .iter_mut()
        .find(|ws| ws.name == record.workspace)
    {
        if ws.branch == old_branch {
            ws.branch = new_branch.clone();
            config::save_workspaces(&workspaces_file)?;
        }
    }

    ui::success(&t("gcreate_record_renamed"));
    Ok(())
}
