# grm / gbranch 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增 `grove grm [branch]`（多选 gcreate 记录批量删分支 / 按精确分支名删除）与 `grove gbranch`（当前工作区分支智能聚合展示），删除语义与 `glist --rm` 对齐。

**Architecture:** 抽取 `branch_delete` 共享模块供 `glist --rm` 与 `grm` 复用；`gbranch` 的聚合逻辑为纯函数便于单测；`grm` 无参走 `MultiSelect` + 逐条删除 + `batch_summary`；`grm <branch>` 走当前工作区项目列表 + 元数据按 workspace+branch 批量清理。

**Tech Stack:** Rust 2021、`clap`、`dialoguer`（MultiSelect）、现有 `git` / `config` / `gcreate_records` / `ui` 模块。

**Spec:** `docs/superpowers/specs/2026-05-19-grm-design.md`

---

## File Map

| 文件 | 职责 |
|------|------|
| `src/commands/branch_delete.rs`（新） | 跨项目删本地分支 + 删后元数据联动 helper |
| `src/commands/gbranch.rs`（新） | 读取各项目分支、聚合输出 |
| `src/commands/grm.rs`（新） | 无参多选删除；有参按名删除 |
| `src/commands/workspace_context.rs`（新） | 共享 `get_workspace_context()` |
| `src/commands/glist.rs` | `run_rm` 改为调用 `branch_delete` |
| `src/gcreate_records.rs` | 多选标签、`remove_records_by_workspace_branch`、`status_label` |
| `src/commands/mod.rs` | 注册新模块 |
| `src/main.rs` | `Grm` / `Gbranch` 子命令与 dispatch |
| `src/i18n.rs` | 多选确认、无选中提示等文案 |
| `README.md` | 命令表与示例 |
| `tests/cli_test.rs` | CLI 存在性测试 |

实现过程中除非用户明确要求，否则不要提交 commit。

---

## Task 1: gbranch 聚合纯函数

**Files:**
- Create: `src/commands/gbranch.rs`
- Modify: `src/commands/mod.rs`

- [ ] **Step 1: 编写失败的单元测试**

创建 `src/commands/gbranch.rs`，先只放测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_all_same_single_line() {
        let entries = vec![
            ("api".to_string(), BranchDisplay::Name("feature/login".into())),
            ("web".to_string(), BranchDisplay::Name("feature/login".into())),
        ];
        let lines = format_branch_lines(&entries);
        assert_eq!(lines, vec!["feature/login"]);
    }

    #[test]
    fn test_format_different_per_project() {
        let entries = vec![
            ("api".to_string(), BranchDisplay::Name("feature/login".into())),
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
```

在 `src/commands/mod.rs` 添加 `pub mod gbranch;`。

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test commands::gbranch::tests
```

Expected: FAIL（`BranchDisplay` / `format_branch_lines` 未定义）

- [ ] **Step 3: 实现纯函数**

在 `src/commands/gbranch.rs` 添加：

```rust
use anyhow::Result;
use std::path::Path;

use crate::config::{Project, WorkspaceProject};
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
```

- [ ] **Step 4: 运行测试通过**

```bash
cargo test commands::gbranch::tests
```

Expected: PASS（5 tests）

---

## Task 2: workspace_context 共享模块

**Files:**
- Create: `src/commands/workspace_context.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/gbranch.rs`（确认 `run()` 可编译）

- [ ] **Step 1: 创建共享模块**

创建 `src/commands/workspace_context.rs`：

```rust
use anyhow::Result;

use crate::config::{self, Project, Workspace, WorkspaceProject};
use crate::workspace;

pub fn get_workspace_context() -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;
    let mut matched = Vec::new();
    let mut missing = Vec::new();

    for wp in &ws.projects {
        if let Some(proj) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            matched.push((wp.clone(), proj.clone()));
        } else {
            missing.push(wp.name.clone());
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Workspace '{}' references missing project(s): {}",
            ws.name,
            missing.join(", ")
        );
    }

    Ok((ws, matched))
}
```

在 `src/commands/mod.rs` 添加 `pub mod workspace_context;`。

- [ ] **Step 2: 编译检查**

```bash
cargo build
```

Expected: 编译通过（`gbranch` 尚未挂到 main，仅模块编译）

---

## Task 3: branch_delete 共享模块

**Files:**
- Create: `src/commands/branch_delete.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/glist.rs`

- [ ] **Step 1: 编写失败的单元测试**

在 `src/commands/branch_delete.rs` 先写 `project_main_branch` 测试（纯逻辑，不依赖 git）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project, ProjectsFile};

    fn sample_projects() -> ProjectsFile {
        ProjectsFile {
            groups: vec![],
            projects: vec![Project {
                name: "api".to_string(),
                path: "/tmp/api".to_string(),
                group: "g".to_string(),
                order: 0,
                tags: vec![],
                branch_aliases: Default::default(),
                agents_md: None,
                branches: BranchConfig {
                    main: "master".to_string(),
                    aliases: Default::default(),
                },
            }],
        }
    }

    #[test]
    fn test_project_main_branch_found() {
        let pf = sample_projects();
        assert_eq!(project_main_branch(&pf, "api"), "master");
    }

    #[test]
    fn test_project_main_branch_fallback() {
        let pf = sample_projects();
        assert_eq!(project_main_branch(&pf, "missing"), "main");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test commands::branch_delete::tests
```

Expected: FAIL

- [ ] **Step 3: 实现 branch_delete 核心**

```rust
use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::{self, GcreateRecord, ProjectsFile, Workspace};
use crate::git;
use crate::i18n::t;
use crate::ui;

#[derive(Debug, Clone)]
pub struct DeleteProjectItem {
    pub name: String,
    pub worktree_path: String,
}

#[derive(Debug, Default)]
pub struct DeleteBranchOutcome {
    pub hard_errors: Vec<String>,
    pub operable: usize,
}

pub fn project_main_branch(projects_file: &ProjectsFile, project_name: &str) -> String {
    projects_file
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .map(|p| p.branches.main.clone())
        .unwrap_or_else(|| "main".to_string())
}

pub fn delete_branch_across_projects(
    items: &[DeleteProjectItem],
    branch: &str,
    projects_file: &ProjectsFile,
) -> DeleteBranchOutcome {
    let mut outcome = DeleteBranchOutcome::default();

    for item in items {
        let path = Path::new(&item.worktree_path);
        if !path.exists() {
            ui::info(&format!(
                "{}: worktree path does not exist (skipped): {}",
                item.name, item.worktree_path
            ));
            continue;
        }

        outcome.operable += 1;

        if let Ok(false) = git::is_clean(path) {
            outcome
                .hard_errors
                .push(format!("{}: working tree has uncommitted changes", item.name));
            continue;
        }

        match git::branch_exists(path, branch) {
            Ok(false) => {
                ui::info(&format!(
                    "{}: branch '{}' does not exist (skipped)",
                    item.name, branch
                ));
            }
            Ok(true) => {
                if let Ok(current) = git::current_branch(path) {
                    if current == branch {
                        let main_branch = project_main_branch(projects_file, &item.name);
                        if let Err(e) = git::checkout(path, &main_branch) {
                            outcome.hard_errors.push(format!(
                                "{}: failed to switch to main before delete: {}",
                                item.name, e
                            ));
                            continue;
                        }
                    }
                }
                if let Err(e) = git::branch_delete(path, branch) {
                    outcome.hard_errors.push(format!("{}: {}", item.name, e));
                } else {
                    ui::success(&format!("{}: deleted branch '{}'", item.name, branch));
                }
            }
            Err(e) => outcome.hard_errors.push(format!("{}: {}", item.name, e)),
        }
    }

    outcome
}

pub fn items_from_record(record: &GcreateRecord) -> Vec<DeleteProjectItem> {
    record
        .projects
        .iter()
        .map(|p| DeleteProjectItem {
            name: p.name.clone(),
            worktree_path: p.worktree_path.clone(),
        })
        .collect()
}

pub fn items_from_workspace(projects: &[(config::WorkspaceProject, config::Project)]) -> Vec<DeleteProjectItem> {
    projects
        .iter()
        .map(|(wp, _)| DeleteProjectItem {
            name: wp.name.clone(),
            worktree_path: wp.worktree_path.clone(),
        })
        .collect()
}

pub fn update_workspace_branch_if_matches(
    workspace_name: &str,
    deleted_branch: &str,
    fallback_main: &str,
) -> Result<()> {
    let workspaces = config::load_workspaces()?;
    if !workspaces
        .workspaces
        .iter()
        .any(|ws| ws.name == workspace_name && ws.branch == deleted_branch)
    {
        return Ok(());
    }
    let mut workspaces_file = config::load_workspaces()?;
    if let Some(ws_mut) = workspaces_file
        .workspaces
        .iter_mut()
        .find(|w| w.name == workspace_name)
    {
        ws_mut.branch = fallback_main.to_string();
        config::save_workspaces(&workspaces_file)?;
    }
    Ok(())
}

pub fn finalize_record_delete(
    record: &GcreateRecord,
    projects_file: &ProjectsFile,
    records_file: &mut config::GcreateRecordsFile,
) -> Result<()> {
    let fallback_main = record
        .projects
        .first()
        .map(|p| project_main_branch(projects_file, &p.name))
        .unwrap_or_else(|| "main".to_string());
    crate::gcreate_records::remove_record_by_id(records_file, &record.id);
    update_workspace_branch_if_matches(&record.workspace, &record.branch, &fallback_main)?;
    Ok(())
}
```

- [ ] **Step 4: 重构 glist.rs 的 run_rm**

将 `run_rm` 中 L87–178 的删除循环替换为：

```rust
let items = branch_delete::items_from_record(&record);
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
    // 保留原有二次 Confirm + remove_record_by_id 逻辑
}

branch_delete::finalize_record_delete(&record, &projects_file, &mut records_file)?;
config::save_gcreate_records(&records_file)?;
ui::success(&t("gcreate_record_deleted"));
```

删除 `glist.rs` 内私有 `project_main_branch`，改为 `use crate::commands::branch_delete`.

- [ ] **Step 5: 运行测试**

```bash
cargo test commands::branch_delete::tests
cargo test gcreate_records::tests
cargo build
```

Expected: PASS + 编译通过

---

## Task 4: gcreate_records 扩展

**Files:**
- Modify: `src/gcreate_records.rs`
- Modify: `src/i18n.rs`

- [ ] **Step 1: 编写失败测试**

在 `src/gcreate_records.rs` 的 `tests` 模块追加：

```rust
#[test]
fn test_remove_records_by_workspace_branch() {
    let mut file = GcreateRecordsFile::default();
    file.records.push(sample_record("ws-a", "feat-a"));
    file.records.push(sample_record("ws-a", "feat-b"));
    file.records.push(sample_record("ws-b", "feat-a"));
    let removed = remove_records_by_workspace_branch(&mut file, "ws-a", "feat-a");
    assert_eq!(removed, 1);
    assert_eq!(file.records.len(), 2);
    assert!(file.records.iter().all(|r| r.branch != "feat-a" || r.workspace != "ws-a"));
}

#[test]
fn test_status_label_values() {
    assert_eq!(status_label(RecordStatus::Ok), "ok");
    assert_eq!(status_label(RecordStatus::Partial), "partial");
    assert_eq!(status_label(RecordStatus::MissingWorkspace), "missing-ws");
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test gcreate_records::tests::test_remove_records_by_workspace_branch
```

Expected: FAIL

- [ ] **Step 3: 实现 helper**

在 `src/gcreate_records.rs` 添加：

```rust
pub fn status_label(status: RecordStatus) -> &'static str {
    match status {
        RecordStatus::Ok => "ok",
        RecordStatus::Partial => "partial",
        RecordStatus::MissingWorkspace => "missing-ws",
    }
}

pub fn format_record_select_label(record: &GcreateRecord, workspace_exists: bool) -> String {
    format!(
        "{}  {}  {}  {}",
        record.workspace,
        record.branch,
        format_created_display(&record.created_at),
        status_label(compute_record_status(record, workspace_exists))
    )
}

pub fn remove_records_by_workspace_branch(
    file: &mut GcreateRecordsFile,
    workspace: &str,
    branch: &str,
) -> usize {
    let before = file.records.len();
    file.records
        .retain(|r| !(r.workspace == workspace && r.branch == branch));
    before - file.records.len()
}
```

将 `glist.rs` 中私有 `status_label` 改为调用 `gcreate_records::status_label`。

- [ ] **Step 4: 添加 i18n 文案**

在 `src/i18n.rs` 英文/中文 map 各添加：

```rust
// en
m.insert("grm_multi_confirm", "Delete {} selected gcreate record(s)?");
m.insert("grm_nothing_selected", "No records selected.");

// zh
m.insert("grm_multi_confirm", "删除选中的 {} 条 gcreate 记录？");
m.insert("grm_nothing_selected", "未选择任何记录。");
```

- [ ] **Step 5: 运行测试**

```bash
cargo test gcreate_records::tests
```

Expected: PASS

---

## Task 5: grm 命令

**Files:**
- Create: `src/commands/grm.rs`
- Modify: `src/commands/mod.rs`

- [ ] **Step 1: 实现 grm.rs**

```rust
use anyhow::{bail, Result};

use crate::commands::branch_delete::{
    self, delete_branch_across_projects, finalize_record_delete, items_from_record,
    items_from_workspace, project_main_branch, update_workspace_branch_if_matches,
};
use crate::config::{self, GcreateRecord};
use crate::gcreate_records::{
    compute_record_status, format_record_select_label, remove_records_by_workspace_branch,
    sort_records_newest_first,
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

    // 从大到小索引，避免后续如需按索引删记录时错位；此处按 id 处理无需 reorder
    let records: Vec<GcreateRecord> = selected
        .iter()
        .map(|&idx| records_file.records[idx].clone())
        .collect();

    for record in records {
        let items = items_from_record(&record);
        let outcome =
            delete_branch_across_projects(&items, &record.branch, &projects_file);

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

    ui::success(&format!(
        "Deleted branch '{}' in workspace '{}'",
        branch, ws.name
    ));
    Ok(())
}
```

在 `src/commands/mod.rs` 添加 `pub mod grm;`。

- [ ] **Step 2: 编译**

```bash
cargo build
```

Expected: 编译通过（尚未注册到 main）

---

## Task 6: CLI 注册与文档

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/cli_test.rs`
- Modify: `README.md`

- [ ] **Step 1: 注册子命令**

在 `src/main.rs` 的 `Commands` enum 中，`Glist` 之后添加：

```rust
    /// Remove local branches (multi-select gcreate records, or by exact branch name)
    #[command(alias = "grm")]
    Grm {
        /// Exact local branch name to delete in the current workspace
        branch: Option<String>,
    },

    /// Show current branch(es) for all projects in the workspace
    #[command(alias = "gbr")]
    Gbranch,
```

在 `match cli.command` 中添加：

```rust
        Some(Commands::Grm { ref branch }) => commands::grm::run(branch.as_deref()),
        Some(Commands::Gbranch) => commands::gbranch::run(),
```

- [ ] **Step 2: CLI 测试**

在 `tests/cli_test.rs` 追加：

```rust
#[test]
fn test_grm_command_exists() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("grm").assert().success();
}

#[test]
fn test_grm_with_branch_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["grm", "feature-x"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gbranch_command_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("gbranch")
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gbranch_alias_gbr() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("gbr")
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}
```

- [ ] **Step 3: 运行测试**

```bash
cargo test
```

Expected: 全部 PASS

- [ ] **Step 4: 更新 README**

在「批量 Git 操作」命令表追加：

```markdown
| `grove grm [branch]` | `grm [branch]` | 无参：多选 gcreate 记录批量删本地分支；有参：在当前工作区按精确分支名删除 |
| `grove gbranch` | `gbr` | 查看当前工作区各项目分支（相同则单行，不同则逐项目展示） |
```

在示例区追加 `gbr` / `grm` 一行说明。

- [ ] **Step 5: 最终验证**

```bash
cargo test
cargo clippy -- -D warnings
```

Expected: 无 warning、测试全绿

---

## Spec Coverage Checklist

| Spec 要求 | Task |
|-----------|------|
| `grm` 无参 MultiSelect | Task 5 |
| `grm` 无参继续并汇总 | Task 5 `batch_summary` |
| `grm <branch>` 精确分支名 | Task 5 `run_by_branch_name` |
| `grm <branch>` 元数据联动 | Task 5 + Task 3 |
| `grm <branch>` operable==0 二次 Confirm | Task 5 |
| `glist --rm` 复用共享删除 | Task 3 |
| `gbranch` 聚合输出 | Task 1 |
| `gbranch` detached/unknown | Task 1 |
| i18n 文案 | Task 4 |
| README | Task 6 |
| CLI 测试 | Task 6 |
