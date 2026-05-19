use anyhow::{bail, Result};

use crate::commands::branch_delete::{
    delete_branch_across_projects, finalize_record_delete, items_from_record, items_from_workspace,
    update_workspace_branch_if_matches,
};
use crate::config::{self, GcreateRecord};
use crate::gcreate_records::{
    format_record_select_label, remove_records_by_workspace_branch, sort_records_newest_first,
};
use crate::i18n::t;
use crate::ui;

pub fn run(branch: Option<&str>) -> Result<()> {
    match branch {
        None => run_multi_select(),
        Some(name) => run_by_branch_name(name),
    }
}

fn run_multi_select() -> Result<()> {
    let mut records_file = config::load_gcreate_records()?;
    if records_file.records.is_empty() {
        ui::info(&t("no_gcreate_records"));
        return Ok(());
    }

    let workspaces = config::load_workspaces()?;
    sort_records_newest_first(&mut records_file.records);

    let labels: Vec<String> = records_file
        .records
        .iter()
        .map(|record| {
            let workspace_exists = workspaces
                .workspaces
                .iter()
                .any(|ws| ws.name == record.workspace);
            format_record_select_label(record, workspace_exists)
        })
        .collect();

    let defaults = vec![false; labels.len()];
    let selected = ui::multi_select(&t("select_gcreate_record"), &labels, &defaults)?;
    if selected.is_empty() {
        ui::info(&t("grm_nothing_selected"));
        return Ok(());
    }

    if !ui::confirm(
        &t("grm_multi_confirm").replace("{}", &selected.len().to_string()),
        false,
    )? {
        return Ok(());
    }

    let projects_file = config::load_projects()?;
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    let records: Vec<GcreateRecord> = selected
        .iter()
        .map(|&idx| records_file.records[idx].clone())
        .collect();

    for record in records {
        let items = items_from_record(&record);
        let outcome = delete_branch_across_projects(&items, &record.branch, &projects_file);

        if !outcome.hard_errors.is_empty() {
            for message in &outcome.hard_errors {
                ui::error(message);
            }
            failed += 1;
            continue;
        }

        if outcome.operable == 0 {
            let only_record = t("gcreate_delete_record_only");
            if !ui::confirm(&only_record, false)? {
                continue;
            }
        }

        finalize_record_delete(&record, &projects_file, &mut records_file)?;
        succeeded += 1;
    }

    config::save_gcreate_records(&records_file)?;
    ui::batch_summary(succeeded, failed);
    if failed > 0 {
        bail!("grm failed");
    }
    Ok(())
}

fn run_by_branch_name(branch: &str) -> Result<()> {
    let (ws, projects) = super::workspace_context::get_workspace_context()?;
    let projects_file = config::load_projects()?;

    let confirm_msg = t("gcreate_delete_confirm")
        .replacen("{}", branch, 1)
        .replacen("{}", &ws.name, 1)
        .replacen("{}", &projects.len().to_string(), 1);
    if !ui::confirm(&confirm_msg, false)? {
        return Ok(());
    }

    let items = items_from_workspace(&projects);
    let outcome = delete_branch_across_projects(&items, branch, &projects_file);

    if !outcome.hard_errors.is_empty() {
        for message in &outcome.hard_errors {
            ui::error(message);
        }
        bail!("grm failed");
    }

    if outcome.operable == 0 {
        let mut records_file = config::load_gcreate_records()?;
        let matching = records_file
            .records
            .iter()
            .filter(|r| r.workspace == ws.name && r.branch == branch)
            .count();
        if matching > 0 {
            let only_record = t("gcreate_delete_record_only");
            if !ui::confirm(&only_record, false)? {
                return Ok(());
            }
            remove_records_by_workspace_branch(&mut records_file, &ws.name, branch);
            config::save_gcreate_records(&records_file)?;
        }
        return Ok(());
    }

    let fallback_main = projects
        .first()
        .map(|(_, p)| p.branches.main.clone())
        .unwrap_or_else(|| "main".to_string());
    update_workspace_branch_if_matches(&ws.name, branch, &fallback_main)?;

    let mut records_file = config::load_gcreate_records()?;
    remove_records_by_workspace_branch(&mut records_file, &ws.name, branch);
    config::save_gcreate_records(&records_file)?;

    ui::success(
        &t("grm_branch_deleted")
            .replacen("{}", &ws.name, 1)
            .replacen("{}", branch, 1),
    );
    Ok(())
}
