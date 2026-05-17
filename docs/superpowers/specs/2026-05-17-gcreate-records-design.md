# gcreate 批量创建记录与 glist 管理

## 背景

`grove gcreate`（`gcr`）会在当前工作区的所有项目中批量创建并切换到同一新分支，成功后更新 `workspaces.toml` 中的 `branch`。用户需要：

- 记录每次成功的 `gcr` 操作，便于跨工作区查询；
- 按「一次 gcr」为单位查看、删除或重命名当时创建的全部分支。

本设计不改动 `gcr` 的核心创建流程（fetch、预检查、回滚），仅在成功后追加元数据，并新增 `glist` 命令族。

## 目标

- 每次成功的 `gcreate` 写入一条持久化记录。
- `grove glist` 全局列出所有记录，并标注所属工作区。
- `grove glist --rm`：交互选择 → 确认 → 在各项目删除该次创建的分支，并移除记录。
- `grove glist --rename`：交互选择 → 输入新名 → 在各项目重命名分支，并更新记录与工作区 `branch`（若匹配）。

## 非目标

- 删除或重命名远程分支（`origin/*`）。
- 非交互式 `--rm <name>` / `--rename <name>`。
- 追踪非 `gcreate` 创建的分支。
- 与 `grove grename` 自动同步记录。

## 存储方案

采用独立文件 `~/.grove/gcreate-records.toml`（方案 A）。

理由：全局查询简单；不膨胀 `workspaces.toml`；记录生命周期与工作区配置解耦，通过 `workspace` 字段关联。

### 文件结构

```toml
[[records]]
id = "550e8400-e29b-41d4-a716-446655440000"
workspace = "feature-x"
branch = "demo1"
input = "demo1"
created_at = "2026-05-17T10:30:00+08:00"
projects = [
  { name = "api", worktree_path = "/Users/me/grove-workspaces/feature-x/api" },
  { name = "web", worktree_path = "/Users/me/grove-workspaces/feature-x/web" },
]
```

| 字段 | 说明 |
|------|------|
| `id` | UUID，交互选择内部主键，避免同名记录冲突 |
| `workspace` | 创建时的工作区名 |
| `branch` | 最终分支名（已应用 `git-prefix`） |
| `input` | 用户 CLI 输入的原始名 |
| `created_at` | ISO 8601 时间戳 |
| `projects` | 创建时项目快照：`name` + `worktree_path` |

### 读写 API

- `config::load_gcreate_records() -> Result<GcreateRecordsFile>`
- `config::save_gcreate_records(&GcreateRecordsFile) -> Result<()>`
- 模型：`GcreateRecord`、`GcreateRecordProject`、`GcreateRecordsFile { records: Vec<GcreateRecord> }`

## gcreate 写入时机

在现有 `gcreate` 中，当且仅当以下条件全部满足后追加记录：

1. 所有项目已创建并切换到新分支；
2. `workspaces.toml` 中工作区 `branch` 已更新成功。

以下情况 **不写入**：

- 预检查失败（未创建任何分支）；
- 执行中失败且已回滚；
- `workspaces.toml` 更新失败（即使已回滚分支）。

写入内容：当前工作区名、最终 `branch`/`input`、`created_at`、当时工作区内所有项目的 `name` 与 `worktree_path` 快照。

## 命令

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove glist` | `grove gli` | 列出全部 gcreate 记录（跨工作区） |
| `grove glist --rm` | | 交互删除一条记录对应的全部分支 |
| `grove glist --rename` | | 交互重命名一条记录对应的全部分支 |

`glist` 与 `grove list`（`ls`，列项目）独立。不使用 `gl` 作为别名（已被 `gpull` 占用）。

### `grove glist`（默认）

- 范围：**全部工作区**的记录（用户选择 B）。
- 排序：`created_at` 新 → 旧。
- 输出列：`WORKSPACE`、`BRANCH`、`CREATED`；可选状态（见下）。

示例：

```text
WORKSPACE     BRANCH    CREATED              STATUS
feature-x     demo2     2026-05-17 14:00     ok
feature-x     demo1     2026-05-17 10:30     partial
hotfix-ws     demo1     2026-05-16 09:15     missing-ws
```

状态含义：

| STATUS | 含义 |
|--------|------|
| `ok` | 快照中各项目分支均存在 |
| `partial` | 部分项目分支已不存在 |
| `missing-ws` | `workspaces.toml` 中已无该工作区 |

无记录时输出 i18n 提示（如 `No gcreate records yet.` / 中文等价文案）。

### `grove glist --rm`

1. 无记录 → 提示退出。
2. `dialoguer::Select`：展示 `workspace  branch  created_at`。
3. `dialoguer::Confirm`：`Delete branch '<branch>' in workspace '<workspace>' across N projects?`
4. 预检查通过后执行删除（见「删除语义」）。
5. 全部处理成功 → 从 `gcreate-records.toml` 移除该 `id`。

`--rm` 与 `--rename` 互斥；与默认列表互斥（带 flag 时不重复打印全表，直接进入选择）。

### `grove glist --rename`

1. 无记录 → 提示退出。
2. `Select` 选择记录。
3. `dialoguer::Input`：`New branch name:`（placeholder 为当前 `input` 或 `branch`）。
4. 经 `git-prefix` 得到最终新分支名（规则与 `gcreate` 一致）。
5. 预检查通过后各项目 `git branch -m <old> <new>`。
6. 更新记录 `branch`/`input`；若 `workspaces.toml` 中该工作区 `branch == old` → 改为新名。

## 删除语义（`--rm`）

### 预检查

| 检查 | 失败行为 |
|------|----------|
| 快照 `worktree_path` 存在 | 路径不存在且分支无法操作 → 硬错误；分支已不存在 → **跳过并 info** |
| 工作区干净 | 中止，不删记录 |
| 当前在待删分支 | 先切到该项目 `main` 对应本地分支，再 `-D` |
| 切 main 失败 | 硬错误，中止 |

### 执行

- 仅删除 **本地** 分支：`git branch -D <branch>`（用户已 Confirm）。
- 分支已不存在：跳过，计入成功清理。
- 存在无法删除的硬错误：**保留记录**，汇总失败项目。

### workspaces.toml

`--rm` 成功后，若工作区仍存在且 `workspace.branch == 被删分支`：

- 将 `workspace.branch` 设为首个快照项目在 `projects.toml` 中的 `branches.main` 字面值（与新建工作区默认分支字段一致）。

## 重命名语义（`--rename`）

### 预检查

| 检查 | 失败行为 |
|------|----------|
| worktree 存在 | 中止 |
| 工作区干净 | 中止 |
| 旧分支在所有快照项目中存在 | 任一缺失则中止（不允许 partial rename） |
| 新分支（含 prefix）在所有项目中不存在 | 中止 |

### 执行与回滚

- 各项目 `git branch -m <old> <new>`。
- 任一失败：对已改名项目 best-effort `branch -m` 回旧名；**不更新记录**。
- 全部成功：更新记录；按需更新 `workspaces.toml` 的 `branch`。

## 工作区生命周期联动

| 事件 | 记录处理 |
|------|----------|
| `grove -w remove` | 删除该 `workspace` 名下的所有记录 |
| `grove -w rename <old> <new>` | 将所有记录中 `workspace == old` 改为 `new` |

## 边界：工作区已删

- `glist` 仍显示，`WORKSPACE` 标 `(missing)` 或 `STATUS=missing-ws`。
- `--rm`：优先按快照 `worktree_path` 操作；若无可操作项，二次 Confirm 后仅删除元数据记录。
- `--rename`：工作区不在 `workspaces.toml` 时拒绝执行（无法可靠更新 `branch`）。

## 错误信息

与现有 `g*` 命令一致，带项目名：

```text
web: working tree has uncommitted changes
api: branch 'demo1' does not exist (skipped)
admin: failed to switch to main before delete: ...
```

预检查失败时尽量一次列出所有问题。

## 实现位置（指引）

| 模块 | 职责 |
|------|------|
| `src/config/models.rs` | `GcreateRecord*` 结构体 |
| `src/config/mod.rs` | load/save `gcreate-records.toml` |
| `src/commands/git_ops.rs` | `gcreate` 成功后 append |
| `src/commands/glist.rs`（新） | `glist` / `--rm` / `--rename` |
| `src/commands/delete.rs` | 工作区删除时 purge 记录 |
| `src/commands/rename.rs` | 工作区重命名时更新 `workspace` 字段 |
| `src/main.rs` | `Glist` 子命令与 `gli` 别名 |
| `src/i18n.rs` | 文案 |
| `README.md` | 命令表与示例 |

## 测试

- `GcreateRecordsFile` TOML roundtrip。
- `gcreate` 成功 append、失败/回滚不 append（单元/集成测）。
- `glist` 空列表、多工作区排序、状态 `ok` / `partial` / `missing-ws`。
- `--rm`：干净检查失败保留记录；全成功后记录删除且 `workspace.branch` 联动。
- `--rename`：新名冲突中止；成功更新记录与 `workspaces.toml`。
- `-w remove` / `-w rename` 触发记录同步。

## 兼容性

- 旧版 Grove 无 `gcreate-records.toml` 时视为空列表。
- 不影响现有 `gcr`/`gsw`/`gpush` 行为。
