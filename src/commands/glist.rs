use anyhow::{bail, Result};
use chrono::{DateTime, Local};
use std::path::Path;

use crate::config::{self, GcreateRecord, ProjectsFile};
use crate::gcreate_records::{
    compute_record_status, remove_record_by_id, sort_records_newest_first, RecordStatus,
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
        "{:<14} {:<24} {:<20} {}",
        "WORKSPACE", "BRANCH", "CREATED", "STATUS"
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
            format_created(&record.created_at),
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

    let idx = select_record(&mut records_file.records)?;
    let record = records_file.records[idx].clone();
    let projects_file = config::load_projects()?;
    let workspaces = config::load_workspaces()?;

    let confirm_msg = t("gcreate_delete_confirm")
        .replacen("{}", &record.branch, 1)
        .replacen("{}", &record.workspace, 1)
        .replacen("{}", &record.projects.len().to_string(), 1);
    if !ui::confirm(&confirm_msg, false)? {
        return Ok(());
    }

    let mut hard_errors = Vec::new();
    let mut operable = 0usize;

    for project in &record.projects {
        let path = Path::new(&project.worktree_path);
        if !path.exists() {
            ui::info(&format!(
                "{}: worktree path does not exist (skipped): {}",
                project.name, project.worktree_path
            ));
            continue;
        }

        operable += 1;

        if let Ok(false) = git::is_clean(path) {
            hard_errors.push(format!(
                "{}: working tree has uncommitted changes",
                project.name
            ));
            continue;
        }

        match git::branch_exists(path, &record.branch) {
            Ok(false) => {
                ui::info(&format!(
                    "{}: branch '{}' does not exist (skipped)",
                    project.name, record.branch
                ));
            }
            Ok(true) => {
                if let Ok(current) = git::current_branch(path) {
                    if current == record.branch {
                        let main_branch = project_main_branch(&projects_file, &project.name);
                        if let Err(e) = git::checkout(path, &main_branch) {
                            hard_errors.push(format!(
                                "{}: failed to switch to main before delete: {}",
                                project.name, e
                            ));
                            continue;
                        }
                    }
                }
                if let Err(e) = git::branch_delete(path, &record.branch) {
                    hard_errors.push(format!("{}: {}", project.name, e));
                } else {
                    ui::success(&format!(
                        "{}: deleted branch '{}'",
                        project.name, record.branch
                    ));
                }
            }
            Err(e) => hard_errors.push(format!("{}: {}", project.name, e)),
        }
    }

    if !hard_errors.is_empty() {
        for message in &hard_errors {
            ui::error(message);
        }
        bail!("glist --rm failed");
    }

    if operable == 0 {
        let only_record = t("gcreate_delete_record_only");
        if !ui::confirm(&only_record, false)? {
            return Ok(());
        }
    }

    let record_id = record.id.clone();
    if !remove_record_by_id(&mut records_file, &record_id) {
        bail!("gcreate record not found");
    }
    config::save_gcreate_records(&records_file)?;

    if let Some(ws) = workspaces
        .workspaces
        .iter()
        .find(|ws| ws.name == record.workspace)
    {
        if ws.branch == record.branch {
            let mut workspaces_file = config::load_workspaces()?;
            if let Some(ws_mut) = workspaces_file
                .workspaces
                .iter_mut()
                .find(|w| w.name == record.workspace)
            {
                if let Some(first) = record.projects.first() {
                    ws_mut.branch = project_main_branch(&projects_file, &first.name);
                    config::save_workspaces(&workspaces_file)?;
                }
            }
        }
    }

    ui::success(&t("gcreate_record_deleted"));
    Ok(())
}

fn run_rename() -> Result<()> {
    let mut records_file = config::load_gcreate_records()?;
    if records_file.records.is_empty() {
        ui::info(&t("no_gcreate_records"));
        return Ok(());
    }

    let idx = select_record(&mut records_file.records)?;
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

fn select_record(records: &mut [GcreateRecord]) -> Result<usize> {
    sort_records_newest_first(records);
    let labels: Vec<String> = records
        .iter()
        .map(|r| {
            format!(
                "{}  {}  {}",
                r.workspace,
                r.branch,
                format_created(&r.created_at)
            )
        })
        .collect();
    ui::select(&t("select_gcreate_record"), &labels)
}

fn format_created(created_at: &str) -> String {
    DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| created_at.to_string())
}

fn status_label(status: RecordStatus) -> &'static str {
    match status {
        RecordStatus::Ok => "ok",
        RecordStatus::Partial => "partial",
        RecordStatus::MissingWorkspace => "missing-ws",
    }
}

fn project_main_branch(projects_file: &ProjectsFile, project_name: &str) -> String {
    projects_file
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .map(|p| p.branches.main.clone())
        .unwrap_or_else(|| "main".to_string())
}
