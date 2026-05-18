use std::path::Path;

use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset, Local};

use crate::config::{self, GcreateRecord, GcreateRecordsFile, WorkspaceProject};
use crate::i18n::t;
use crate::ui;

pub fn append_record(
    file: &mut GcreateRecordsFile,
    workspace: &str,
    branch: &str,
    input: &str,
    projects: &[WorkspaceProject],
) {
    let created_at: DateTime<FixedOffset> = chrono::Local::now().fixed_offset();
    file.records.push(GcreateRecord {
        id: uuid::Uuid::new_v4().to_string(),
        workspace: workspace.to_string(),
        branch: branch.to_string(),
        input: input.to_string(),
        created_at: created_at.to_rfc3339(),
        projects: projects
            .iter()
            .map(|wp| crate::config::GcreateRecordProject {
                name: wp.name.clone(),
                worktree_path: wp.worktree_path.clone(),
            })
            .collect(),
    });
}

pub fn remove_record_by_id(file: &mut GcreateRecordsFile, id: &str) -> bool {
    if let Some(idx) = file.records.iter().position(|r| r.id == id) {
        file.records.remove(idx);
        true
    } else {
        false
    }
}

pub fn purge_records_for_workspace(file: &mut GcreateRecordsFile, workspace: &str) {
    file.records.retain(|r| r.workspace != workspace);
}

pub fn rename_records_workspace(
    file: &mut GcreateRecordsFile,
    old_name: &str,
    new_name: &str,
    old_ws_dir: &Path,
    new_ws_dir: &Path,
) {
    for record in &mut file.records {
        if record.workspace != old_name {
            continue;
        }
        record.workspace = new_name.to_string();
        for project in &mut record.projects {
            rewrite_worktree_path(&mut project.worktree_path, old_ws_dir, new_ws_dir);
        }
    }
}

fn rewrite_worktree_path(worktree_path: &mut String, old_ws_dir: &Path, new_ws_dir: &Path) {
    let old_wt = Path::new(worktree_path.as_str());
    if let Ok(relative) = old_wt.strip_prefix(old_ws_dir) {
        *worktree_path = new_ws_dir.join(relative).to_string_lossy().to_string();
    }
}

pub fn sort_records_newest_first(records: &mut [GcreateRecord]) {
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordStatus {
    Ok,
    Partial,
    MissingWorkspace,
}

pub fn format_created_display(created_at: &str) -> String {
    DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| created_at.to_string())
}

pub fn select_record_interactive(records: &mut [GcreateRecord], prompt: &str) -> Result<usize> {
    if records.is_empty() {
        bail!("{}", t("no_gcreate_records"));
    }
    sort_records_newest_first(records);
    let labels: Vec<String> = records
        .iter()
        .map(|record| {
            format!(
                "{}  {}  {}",
                record.workspace,
                record.branch,
                format_created_display(&record.created_at)
            )
        })
        .collect();
    ui::select(prompt, &labels)
}

pub fn select_record_for_workspace(workspace: &str) -> Result<GcreateRecord> {
    let file = config::load_gcreate_records()?;
    let mut records: Vec<GcreateRecord> = file
        .records
        .into_iter()
        .filter(|record| record.workspace == workspace)
        .collect();
    if records.is_empty() {
        ui::info(&t("no_gcreate_records_for_workspace"));
        bail!("{}", t("no_gcreate_records_for_workspace"));
    }
    sort_records_newest_first(&mut records);
    let labels: Vec<String> = records
        .iter()
        .map(|record| {
            format!(
                "{}  {}",
                record.branch,
                format_created_display(&record.created_at)
            )
        })
        .collect();
    let idx = ui::select(&t("select_gcreate_record_switch"), &labels)?;
    Ok(records[idx].clone())
}

pub fn compute_record_status(record: &GcreateRecord, workspace_exists: bool) -> RecordStatus {
    if !workspace_exists {
        return RecordStatus::MissingWorkspace;
    }

    let mut existing = 0usize;
    let mut checked = 0usize;
    for project in &record.projects {
        let path = Path::new(&project.worktree_path);
        if !path.exists() {
            continue;
        }
        checked += 1;
        if crate::git::branch_exists(path, &record.branch).unwrap_or(false) {
            existing += 1;
        }
    }

    if checked == 0 || existing < checked {
        RecordStatus::Partial
    } else {
        RecordStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GcreateRecordProject;

    fn sample_record(workspace: &str, branch: &str) -> GcreateRecord {
        GcreateRecord {
            id: uuid::Uuid::new_v4().to_string(),
            workspace: workspace.to_string(),
            branch: branch.to_string(),
            input: branch.to_string(),
            created_at: "2026-05-17T10:00:00+08:00".to_string(),
            projects: vec![GcreateRecordProject {
                name: "api".to_string(),
                worktree_path: "/tmp/ws/api".to_string(),
            }],
        }
    }

    #[test]
    fn test_sort_records_newest_first() {
        let mut records = vec![];
        let mut a = sample_record("ws", "a");
        a.created_at = "2026-05-16T10:00:00+08:00".to_string();
        let mut b = sample_record("ws", "b");
        b.created_at = "2026-05-17T10:00:00+08:00".to_string();
        records = vec![a, b];
        sort_records_newest_first(&mut records);
        assert_eq!(records[0].branch, "b");
    }

    #[test]
    fn test_purge_records_for_workspace() {
        let mut file = GcreateRecordsFile::default();
        file.records.push(sample_record("keep", "x"));
        file.records.push(sample_record("drop", "y"));
        purge_records_for_workspace(&mut file, "drop");
        assert_eq!(file.records.len(), 1);
        assert_eq!(file.records[0].workspace, "keep");
    }

    #[test]
    fn test_rename_records_workspace_updates_paths() {
        let mut file = GcreateRecordsFile::default();
        let mut record = sample_record("old-ws", "feature");
        record.projects[0].worktree_path = "/work/old-ws/api".to_string();
        file.records.push(record);

        rename_records_workspace(
            &mut file,
            "old-ws",
            "new-ws",
            Path::new("/work/old-ws"),
            Path::new("/work/new-ws"),
        );

        assert_eq!(file.records[0].workspace, "new-ws");
        assert_eq!(file.records[0].projects[0].worktree_path, "/work/new-ws/api");
    }

    #[test]
    fn test_select_record_for_workspace_filters_records() {
        let mut file = GcreateRecordsFile::default();
        file.records.push(sample_record("other", "x"));
        file.records.push(sample_record("ws", "feature-a"));
        let matched: Vec<GcreateRecord> = file
            .records
            .into_iter()
            .filter(|record| record.workspace == "ws")
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].branch, "feature-a");
    }

    #[test]
    fn test_rewrite_worktree_path_noop_when_prefix_mismatch() {
        let mut path = "/other/api".to_string();
        rewrite_worktree_path(
            &mut path,
            Path::new("/work/old-ws"),
            Path::new("/work/new-ws"),
        );
        assert_eq!(path, "/other/api");
    }
}
