# gcreate 记录与 glist 管理实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为每次成功的 `gcreate` 持久化记录，并提供 `glist` / `glist --rm` / `glist --rename` 跨工作区查看与管理批量创建的分支。

**Architecture:** 在 `~/.grove/gcreate-records.toml` 存储扁平记录列表；`gcreate` 成功后追加快照；新命令 `glist` 负责列表与交互式删/改名，复用现有 `git::*` 与 `dialoguer` UI；工作区删除/重命名时同步记录。将 `apply_git_prefix` 提取到 `config` 供 `gcreate` 与 `glist --rename` 共用。

**Tech Stack:** Rust 2021、`clap`、`serde`/`toml`、`dialoguer`、`chrono`、`uuid`（v4 记录 id）、现有 `git` 模块。

**Spec:** `docs/superpowers/specs/2026-05-17-gcreate-records-design.md`

---

## File Map

| 文件 | 职责 |
|------|------|
| `Cargo.toml` | 添加 `uuid` 依赖 |
| `src/config/models.rs` | `GcreateRecord`、`GcreateRecordProject`、`GcreateRecordsFile` |
| `src/config/mod.rs` | `load/save_gcreate_records`、`apply_git_prefix` |
| `src/gcreate_records.rs`（新） | 追加/删除/按工作区清理/重命名工作区字段、状态计算、排序 |
| `src/commands/git_ops.rs` | `gcreate` 成功后 `append_record`；删除私有 `apply_git_prefix`，改用 `config::` |
| `src/commands/glist.rs`（新） | `run(rm, rename)`：列表、`--rm`、`--rename` |
| `src/commands/delete.rs` | 工作区删除后 `purge_records_for_workspace` |
| `src/commands/rename.rs` | 工作区重命名后 `rename_records_workspace` |
| `src/commands/mod.rs` | `pub mod glist` |
| `src/main.rs` | `Glist` 子命令、`gli` 别名、dispatch |
| `src/i18n.rs` | 中英文文案 |
| `README.md` | 命令表与示例 |
| `tests/cli_test.rs` | CLI 存在性、flag 互斥 |
| `src/config/models.rs` / `src/gcreate_records.rs` | 单元测试 |

实现过程中除非用户明确要求，否则不要提交 commit。

---

## Task 1: 配置模型与持久化

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config/models.rs`
- Modify: `src/config/mod.rs`
- Test: `src/config/models.rs`

- [ ] **Step 1: 添加依赖**

在 `Cargo.toml` `[dependencies]` 增加：

```toml
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: 编写失败的模型 roundtrip 测试**

在 `src/config/models.rs` 的 `#[cfg(test)] mod tests` 末尾添加：

```rust
#[test]
fn test_gcreate_records_file_roundtrip() {
    let file = GcreateRecordsFile {
        records: vec![GcreateRecord {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            workspace: "feature-x".to_string(),
            branch: "demo1".to_string(),
            input: "demo1".to_string(),
            created_at: "2026-05-17T10:30:00+08:00".to_string(),
            projects: vec![GcreateRecordProject {
                name: "api".to_string(),
                worktree_path: "/tmp/feature-x/api".to_string(),
            }],
        }],
    };

    let toml_str = toml::to_string(&file).unwrap();
    let parsed: GcreateRecordsFile = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.records.len(), 1);
    assert_eq!(parsed.records[0].branch, "demo1");
    assert_eq!(parsed.records[0].projects[0].name, "api");
}
```

- [ ] **Step 3: 运行测试确认失败**

```bash
cargo test config::models::tests::test_gcreate_records_file_roundtrip
```

Expected: FAIL（类型未定义）

- [ ] **Step 4: 实现模型**

在 `src/config/models.rs` 追加（放在 `WorkspacesFile` 之后）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcreateRecordProject {
    pub name: String,
    pub worktree_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcreateRecord {
    pub id: String,
    pub workspace: String,
    pub branch: String,
    pub input: String,
    pub created_at: String,
    pub projects: Vec<GcreateRecordProject>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GcreateRecordsFile {
    #[serde(default)]
    pub records: Vec<GcreateRecord>,
}
```

- [ ] **Step 5: 实现 load/save**

在 `src/config/mod.rs`：

```rust
pub fn load_gcreate_records() -> Result<GcreateRecordsFile> {
    let path = grove_dir()?.join("gcreate-records.toml");
    if !path.exists() {
        return Ok(GcreateRecordsFile::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let file: GcreateRecordsFile =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(file)
}

pub fn save_gcreate_records(file: &GcreateRecordsFile) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("gcreate-records.toml");
    let content = toml::to_string(file).context("Failed to serialize gcreate records")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
```

- [ ] **Step 6: 运行测试通过**

```bash
cargo test config::models::tests::test_gcreate_records_file_roundtrip
```

Expected: PASS

---

## Task 2: 记录辅助模块与 `apply_git_prefix` 提取

**Files:**
- Create: `src/gcreate_records.rs`
- Modify: `src/main.rs`（`mod gcreate_records;`）
- Modify: `src/config/mod.rs`
- Modify: `src/commands/git_ops.rs`
- Test: `src/gcreate_records.rs`

- [ ] **Step 1: 将 `apply_git_prefix` 移到 config**

在 `src/config/mod.rs` 添加（从 `git_ops.rs` 复制逻辑）：

```rust
pub fn apply_git_prefix(input: &str, global: &GlobalConfig) -> String {
    let git_prefix = expand_date_templates(&global.git_prefix);
    if git_prefix.is_empty() || input.starts_with(&git_prefix) {
        input.to_string()
    } else {
        format!("{}{}", git_prefix, input)
    }
}
```

`git_ops.rs` 中删除 `fn apply_git_prefix`，所有调用改为 `config::apply_git_prefix`。

更新 `git_ops` 内现有测试为 `config::apply_git_prefix`（或保留测试在 `config/mod.rs`）。

- [ ] **Step 2: 编写 `gcreate_records` 单元测试**

创建 `src/gcreate_records.rs`，先写测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GcreateRecord, GcreateRecordProject, GcreateRecordsFile};

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
        let mut file = GcreateRecordsFile::default();
        let mut a = sample_record("ws", "a");
        a.created_at = "2026-05-16T10:00:00+08:00".to_string();
        let mut b = sample_record("ws", "b");
        b.created_at = "2026-05-17T10:00:00+08:00".to_string();
        file.records = vec![a, b];
        sort_records_newest_first(&mut file.records);
        assert_eq!(file.records[0].branch, "b");
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
}
```

- [ ] **Step 3: 实现辅助函数**

```rust
use crate::config::{GcreateRecord, GcreateRecordsFile, WorkspaceProject};
use chrono::{DateTime, FixedOffset};

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
            .map(|wp| GcreateRecordProject {
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

pub fn rename_records_workspace(file: &mut GcreateRecordsFile, old: &str, new: &str) {
    for record in &mut file.records {
        if record.workspace == old {
            record.workspace = new.to_string();
        }
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

pub fn compute_record_status(
    record: &GcreateRecord,
    workspace_exists: bool,
) -> RecordStatus {
    use std::path::Path;
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
    if checked == 0 {
        RecordStatus::Partial
    } else if existing == checked {
        RecordStatus::Ok
    } else if existing == 0 {
        RecordStatus::Partial
    } else {
        RecordStatus::Partial
    }
}
```

在 `src/main.rs` 增加 `mod gcreate_records;`。

- [ ] **Step 4: 运行测试**

```bash
cargo test gcreate_records
```

Expected: PASS

---

## Task 3: `gcreate` 成功后写入记录

**Files:**
- Modify: `src/commands/git_ops.rs`
- Test: `src/commands/git_ops.rs`（可选：用 mock 路径较难，以集成逻辑测试为主）

- [ ] **Step 1: 在 `gcreate` 末尾追加记录**

在 `update_workspace_branch` 成功且打印 workspace success **之后**：

```rust
let mut records = config::load_gcreate_records()?;
let workspace_projects: Vec<WorkspaceProject> = projects
    .iter()
    .map(|(wp, _)| wp.clone())
    .collect();
gcreate_records::append_record(
    &mut records,
    &ws.name,
    &new_branch,
    name,
    &workspace_projects,
);
config::save_gcreate_records(&records)?;
```

注意：`name` 是用户原始输入参数，`new_branch` 是 prefix 后的最终名。

- [ ] **Step 2: 确认失败路径不写记录**

确认以下路径 **没有** `save_gcreate_records`：`precheck` 失败、`rollback` 分支、`update_workspace_branch` 失败。

- [ ] **Step 3: 编译**

```bash
cargo build
```

Expected: 成功

---

## Task 4: `glist` 默认列表

**Files:**
- Create: `src/commands/glist.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Modify: `src/i18n.rs`
- Test: `tests/cli_test.rs`

- [ ] **Step 1: CLI 接线（先让命令存在）**

`src/main.rs` 的 `Commands` 枚举增加：

```rust
/// List gcreate batch records
#[command(alias = "gli")]
Glist {
    /// Interactively delete branches from a selected gcreate record
    #[arg(long = "rm", conflicts_with = "rename")]
    rm: bool,
    /// Interactively rename branches from a selected gcreate record
    #[arg(long = "rename", conflicts_with = "rm")]
    rename: bool,
},
```

Dispatch：

```rust
Some(Commands::Glist { rm, rename }) => commands::glist::run(rm, rename),
```

`src/commands/mod.rs`：`pub mod glist;`

`tests/cli_test.rs`：

```rust
#[test]
fn test_glist_command_exists() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["glist"]).assert().success();
}

#[test]
fn test_glist_rm_and_rename_conflict() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["glist", "--rm", "--rename"])
        .assert()
        .failure();
}
```

- [ ] **Step 2: 实现 `glist::run` 列表模式**

`src/commands/glist.rs` 骨架：

```rust
pub fn run(rm: bool, rename: bool) -> Result<()> {
    if rm {
        return run_rm();
    }
    if rename {
        return run_rename();
    }
    run_list()
}
```

`run_list`：

1. `load_gcreate_records` + `load_workspaces`
2. 空 → `ui::info(&t("no_gcreate_records"))`
3. `sort_records_newest_first`
4. 打印表头与行：`WORKSPACE`、`BRANCH`、`CREATED`、`STATUS`
5. `workspace_exists` = `workspaces.workspaces.iter().any(|w| w.name == record.workspace)`
6. `missing-ws` 时 workspace 列显示 `name (missing)` 或单独 status 列 `missing-ws`

格式化 `created_at` 为本地 `YYYY-MM-DD HH:MM`（解析 RFC3339 失败则原样输出）。

- [ ] **Step 3: i18n**

`src/i18n.rs` 增加：

- `no_gcreate_records` — EN: `No gcreate records yet.` / ZH: `暂无 gcreate 批量创建记录。`
- `glist_status_ok` / `glist_status_partial` / `glist_status_missing_ws`（或统一用 `ok`/`partial`/`missing-ws` 字面量显示）

- [ ] **Step 4: 运行测试**

```bash
cargo test test_glist
cargo test
```

---

## Task 5: `glist --rm`

**Files:**
- Modify: `src/commands/glist.rs`
- Modify: `src/i18n.rs`

- [ ] **Step 1: 选择记录辅助函数**

```rust
fn select_record(records: &mut GcreateRecordsFile) -> Result<Option<usize>> {
    sort_records_newest_first(&mut records.records);
    let labels: Vec<String> = records
        .records
        .iter()
        .map(|r| format!("{}  {}  {}", r.workspace, r.branch, format_created(&r.created_at)))
        .collect();
    let idx = ui::select(&t("select_gcreate_record"), &labels)?;
    Ok(Some(idx))
}
```

- [ ] **Step 2: 预检查与删除逻辑**

`run_rm` 流程：

1. 无记录 → info 退出
2. `select_record` → 取 `record`
3. `ui::confirm` — 文案含 workspace、branch、`record.projects.len()`
4. 对每个 `project`：
   - `path` 不存在 → 若也无法验证分支，记硬错误；若分支已不存在 → `ui::info` skip
   - `!git::is_clean(path)` → 收集 `PrecheckFailure` 风格错误，最后 `bail`
   - `current_branch == record.branch` → `git::checkout(path, main_branch)`，`main_branch` 从 `projects.toml` 找该项目 `branches.main` 字面值
   - `git::branch_delete(path, &record.branch)`；已不存在则 skip
5. 任一硬错误 → **不** `remove_record_by_id`
6. 全部成功 → `remove_record_by_id` + `save_gcreate_records`
7. 若 `workspaces.toml` 中该 workspace 存在且 `branch == record.branch` → 设为首个快照项目在 `projects.toml` 的 `branches.main`（`find_project_by_name`  helper）

**无可操作项**（所有 path 不存在且分支都不存在）：二次 `confirm`「仅删除记录？」→ 只删元数据。

- [ ] **Step 3: 手动验证命令（可选）**

在有测试 worktree 的环境：

```bash
grove gcr demo-test
grove glist
grove glist --rm
```

---

## Task 6: `glist --rename`

**Files:**
- Modify: `src/commands/glist.rs`

- [ ] **Step 1: `run_rename` 流程**

1. 无记录 → info
2. `select_record`
3. 若 workspace 不在 `workspaces.toml` → `bail!(...)` （spec：拒绝 rename）
4. `ui::input` 新名，placeholder = `record.input`
5. `new_branch = config::apply_git_prefix(&input, &global)`
6. 预检查：所有 path 存在、干净、旧分支存在、新分支不存在
7. 逐项目 `git::branch_rename`；失败则对已改名项目 reverse rename
8. 更新 `record.branch` / `record.input`；`save_gcreate_records`
9. 若 `workspace.branch == old` → 更新为 `new_branch` 并 `save_workspaces`

- [ ] **Step 2: 编译与测试**

```bash
cargo test
cargo build
```

---

## Task 7: 工作区生命周期联动

**Files:**
- Modify: `src/commands/delete.rs`
- Modify: `src/commands/rename.rs`

- [ ] **Step 1: 删除工作区时清理记录**

在 `delete.rs` 的 `save_workspaces` **之后**：

```rust
let mut records = config::load_gcreate_records()?;
gcreate_records::purge_records_for_workspace(&mut records, &ws_name);
config::save_gcreate_records(&records)?;
```

- [ ] **Step 2: 重命名工作区时更新记录**

在 `rename.rs` 工作区 `name` 写入 `new_name` 之后、`save_workspaces` 之前或之后：

```rust
let mut records = config::load_gcreate_records()?;
gcreate_records::rename_records_workspace(&mut records, &old_name, &new_name);
config::save_gcreate_records(&records)?;
```

- [ ] **Step 3: 全量测试**

```bash
cargo test
```

---

## Task 8: README 与收尾

**Files:**
- Modify: `README.md`

- [ ] **Step 1: 更新中文命令表**

在「批量 Git 操作」表增加：

| `grove glist` | `grove gli` | 列出所有 `gcr` 批量创建记录（跨工作区） |
| `grove glist --rm` | | 交互选择并删除一次 `gcr` 在各项目创建的分支 |
| `grove glist --rename` | | 交互选择并重命名一次 `gcr` 创建的分支 |

- [ ] **Step 2: 增加示例输出**

```text
WORKSPACE     BRANCH    CREATED              STATUS
feature-x     demo2     2026-05-17 14:00     ok
feature-x     demo1     2026-05-17 10:30     partial
```

- [ ] **Step 3: 同步英文 README 段落**（若仓库双语维护）

- [ ] **Step 4: 最终验证**

```bash
cargo test
cargo clippy -- -D warnings
```

（若项目无 clippy CI，至少 `cargo test` 必须通过。）

---

## Spec 覆盖自检

| Spec 要求 | 任务 |
|-----------|------|
| `gcreate-records.toml` 独立文件 | Task 1 |
| 成功 gcreate 写入、失败不写 | Task 3 |
| `glist` 全局列表 + STATUS | Task 4 |
| `glist --rm` 交互删除 + 本地 -D | Task 5 |
| `glist --rename` + git-prefix | Task 2, 6 |
| `-w remove/rename` 联动 | Task 7 |
| `gli` 别名 | Task 4 |
| i18n | Task 4–6 |
| README | Task 8 |
| 工作区已删时 rename 拒绝 | Task 6 |
| `--rm` 仅删元数据二次确认 | Task 5 |

无 TBD / 占位步骤。
