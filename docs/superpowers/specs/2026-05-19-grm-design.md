# grm / gbranch：批量删除分支与当前分支查看

## 背景

`grove glist --rm` 通过交互选择 gcreate 记录来批量删除本地分支，适合「按一次 gcreate 操作」清理。但用户有时已知精确分支名，希望在工作区内直接删除，无需先查记录、再交互选择；也需要一次多选清理多条 gcreate 记录；还需要快速查看当前工作区各项目所在分支。

本设计新增：

- `grove grm [branch]` — 批量删除本地分支（多选 gcreate 记录，或按精确分支名删除）。
- `grove gbranch` — 查看当前工作区各项目的当前分支，全相同则单行输出。

## 目标

- `grove grm`：跨工作区展示 gcreate 记录，**多选**后批量删除（语义同 `glist --rm`，但支持一次选多条）。
- `grove grm <branch>`：在当前工作区各项目删除精确分支名 `<branch>`（不应用 `git-prefix`）。
- 删除成功后联动 gcreate 记录与 `workspaces.toml`，行为与 `glist --rm` 对齐。
- `grove gbranch`：只输出当前分支；各项目分支相同时单行，不同时逐项目展示。

## 非目标

- 删除或重命名远程分支。
- 模糊匹配、前缀匹配或 glob 模式。
- 对 `<branch>` 应用 `git-prefix`。
- 废弃或替换 `glist --rm`（两者并存）。
- 非交互式 `-y` 跳过确认。

## 命令

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove grm` | `grm` | 多选 gcreate 记录，批量删除对应本地分支 |
| `grove grm <branch>` | `grm <branch>` | 在当前工作区各项目删除本地分支 `<branch>` |
| `grove gbranch` | `gbr` | 查看当前工作区各项目的当前分支 |

## 行为：`grove grm`（无参数，多选删除）

### 数据来源

- 范围：全部工作区的 gcreate 记录（与 `glist` 相同）。
- 排序：`created_at` 新 → 旧。
- 无记录时输出 i18n 提示并退出。

### 交互流程

1. `dialoguer::MultiSelect`：选项标签为 `workspace  branch  created_at  status`（与 `glist --rm` 单选标签一致，附加 status）。
2. 默认均未选中；空格多选，回车确认。
3. 未选任何项 → 提示退出（不报错）。
4. 汇总 Confirm：`Delete N selected gcreate record(s)?`（i18n）。
5. 逐条处理选中记录（见下），**不因单条失败而中断**，结束后统一汇总。

### 逐条删除（每条记录）

与 `glist --rm` 相同：

- 项目来源：该记录的 gcreate 快照（`worktree_path`）。
- 预检查、切 main、`-D`、跳过不存在分支等语义不变。
- 该条全部硬检查通过 → 移除该 `id` 记录 + 按需更新 `workspaces.toml.branch`。
- 该条硬错误 → **保留该条记录**，计入失败；继续处理下一条。

### 结束汇总

与现有 `g*` 批量命令一致：

```text
成功 2，失败 1，跳过 0
```

任一失败时进程 exit code 非 0；成功删除的记录元数据已持久化，失败记录保留。

## 行为：`grove grm <branch>`

### 工作区解析

与其他 `g*` 命令一致：通过 `get_workspace_context()` 从 cwd 检测工作区，检测不到则交互选择。

### 分支名

- `<branch>` 为精确 Git 本地分支名。
- **不**经 `git-prefix` 转换。

### 流程

1. 解析当前工作区及项目列表（来自 `workspaces.toml`，非 gcreate 快照）。
2. `dialoguer::Confirm`：`Delete branch '<branch>' in workspace '<workspace>' across N projects?`（复用或等价于 `gcreate_delete_confirm` 文案）。
3. 预检查通过后逐项目执行删除（见「删除语义」）。
4. 全部硬检查通过 → 元数据联动（见「元数据联动」）。

### 删除语义

与 `glist --rm` 一致，项目来源改为当前工作区 `workspaces.toml` 中的 `worktree_path`。

#### 预检查

| 检查 | 失败行为 |
|------|----------|
| worktree 路径不存在 | info 跳过，不计入 operable |
| 工作区不干净 | 硬错误，中止，不改元数据 |
| 分支不存在 | info 跳过 |
| 当前在待删分支 | 先切到该项目 `branches.main`，再 `-D` |
| 切 main 失败 | 硬错误，中止 |

#### 执行

- 仅删除本地分支：`git branch -D <branch>`。
- 分支已不存在：跳过，计入成功清理。
- 存在无法删除的硬错误：保留元数据，汇总失败项目后 `bail!`。

#### 无可操作项目

若所有项目均因路径不存在或分支不存在而无法操作（`operable == 0`）：

- 二次 Confirm：是否仅清理匹配的 gcreate 记录（复用 `gcreate_delete_record_only` 文案）。
- 用户确认 → 仅删记录；拒绝 → 退出。

## 元数据联动

删除全部成功后：

### gcreate 记录

- 从 `gcreate-records.toml` 移除**所有**满足 `workspace == 当前工作区名` 且 `branch == <branch>` 的记录。
- 与 `glist --rm` 不同：`glist --rm` 删单条选中记录；`grm` 按 workspace+branch 批量清理，可一次删多条。

### workspaces.toml

若当前工作区仍存在且 `workspace.branch == <branch>`：

- 将 `workspace.branch` 设为工作区内首个项目在 `projects.toml` 中的 `branches.main` 字面值（与 `glist --rm` 一致）。

## 行为：`grove gbranch`（当前分支）

### 作用

在当前工作区各项目中读取 Git 当前分支，**仅输出分支信息**（不含 tracking、文件变更；那是 `gstatus` 的职责）。

### 工作区解析

与其他 `g*` 命令一致：`get_workspace_context()`。

### 分支读取

逐项目调用 `git::current_branch(worktree_path)`：

| 结果 | 展示值 |
|------|--------|
| 成功 | 分支名字符串 |
| detached HEAD / 读取失败 | `(detached)` |

worktree 路径不存在：该项目展示 `(unknown)`，并 `ui::error` 一行说明（不中断其他项目）。

### 聚合输出规则

收集全部项目的展示值后：

| 情况 | 输出 |
|------|------|
| 所有项目展示值相同 | **单行**：`feature/login` |
| 存在不同展示值 | **逐行**：`api: feature/login`（项目名 + 分支，按工作区项目顺序） |

单项目工作区视为「全部相同」，输出一行分支名。

### 示例

**全部相同：**

```text
feature/login
```

**分支不一致：**

```text
api: feature/login
web: develop
admin: feature/login
```

**含 detached：**

```text
api: feature/login
web: (detached)
```

### 与 `gstatus` 对比

| | `gstatus` (`gs`) | `gbranch` (`gbr`) |
|--|------------------|-------------------|
| 分支 | 每项目单独展示 | 相同则聚合为一行 |
| tracking | 有 | 无 |
| 工作区变更 | 有 | 无 |
| 输出量 | 详细 | 极简 |

---

## 与 `glist` / `glist --rm` 对比

| | `glist` | `glist --rm` | `grm`（无参） | `grm <branch>` |
|--|---------|-------------|--------------|----------------|
| 作用 | 只读列表 | 单选删除 | **多选删除** | 按名删除 |
| 范围 | 全局记录 | 全局记录 | 全局记录 | 当前工作区 |
| 项目来源 | — | gcreate 快照 | gcreate 快照 | `workspaces.toml` |
| 记录清理 | — | 单条 by id | 每条选中 by id | 所有匹配 workspace+branch |
| 批量失败 | — | 单条即中止 | 继续并汇总 | 单条即中止 |
| 删除语义 | — | 相同 | 相同 | 相同 |
| 元数据联动 | — | 相同 | 相同 | 相同 |

## 错误信息

与现有 `g*` 命令一致，带项目名：

```text
web: working tree has uncommitted changes
api: branch 'feat-login' does not exist (skipped)
admin: failed to switch to main before delete: ...
```

预检查失败时尽量一次列出所有问题。

## 实现结构

| 模块 | 职责 |
|------|------|
| `src/commands/branch_delete.rs`（新） | 抽取共享删除逻辑：`delete_branch_across_projects(projects, branch, projects_file) -> Result<DeleteOutcome>` |
| `src/commands/grm.rs`（新） | `grm` 入口：无参多选删除；有参调工作区删除 + 元数据联动 |
| `src/commands/gbranch.rs`（新） | `gbranch` 入口：收集分支 + 聚合输出 |
| `src/gcreate_records.rs` | 新增 `select_records_multi_interactive`（MultiSelect） |
| `src/commands/glist.rs` | `run_rm` 改为调用共享删除逻辑（项目来源仍为记录快照） |
| `src/commands/mod.rs` | 注册新模块 |
| `src/main.rs` | `Grm { branch: Option<String> }` 别名 `grm`；`Gbranch` 别名 `gbr` |
| `src/i18n.rs` | 多选确认、`gbranch` 标签等文案 |
| `README.md` | 命令表与示例 |

### gbranch 聚合逻辑（伪代码）

```rust
enum BranchDisplay {
    Name(String),
    Detached,
    Unknown,
}

fn format_branch_display(entries: &[(project_name, BranchDisplay)]) -> Vec<String> {
    let labels: Vec<String> = entries.iter().map(|(name, b)| match b {
        BranchDisplay::Name(s) => s.clone(),
        BranchDisplay::Detached => "(detached)".into(),
        BranchDisplay::Unknown => "(unknown)".into(),
    }).collect();

    if labels.len() > 0 && labels.iter().all(|l| l == &labels[0]) {
        vec![labels[0].clone()]
    } else {
        entries.iter().zip(labels).map(|((name, _), label)| {
            format!("{}: {}", name, label)
        }).collect()
    }
}
```

### 共享函数职责

```rust
// 伪代码
struct DeleteBranchOutcome {
    hard_errors: Vec<String>,
    operable: usize,
}

fn delete_branch_across_projects(
    items: &[(ProjectRef, PathBuf)],  // name + worktree_path
    branch: &str,
    projects_file: &ProjectsFile,
) -> Result<DeleteBranchOutcome>
```

`glist --rm` 与 `grm` 在删除完成后各自处理元数据：

- `glist --rm`：`remove_record_by_id` + 可选 `workspaces.toml` 更新。
- `grm`：`remove_records_by_workspace_branch` + 可选 `workspaces.toml` 更新。

可在 `gcreate_records.rs` 新增：

```rust
fn remove_records_by_workspace_branch(
    file: &mut GcreateRecordsFile,
    workspace: &str,
    branch: &str,
) -> usize  // 返回删除条数
```

## 测试

### grm

- `grm` 无参多选：选中 2 条记录均成功删除并清理元数据；未选中项保留。
- `grm` 无参：未选任何项 → 安静退出。
- `grm` 无参：一条失败一条成功 → 成功项已清理，失败项保留，汇总报告。
- `grm <branch>` 成功：各项目分支删除、gcreate 记录清理、`workspaces.toml.branch` 回退。
- 工作区不干净：硬错误，记录与 `workspaces.toml` 不变。
- 分支部分不存在：跳过 + info，其余正常删除。
- 多条匹配 gcreate 记录：一次 `grm` 全部清除。
- `operable == 0`：二次 Confirm 仅清记录路径。
- 共享删除逻辑：`glist --rm` 回归测试仍通过。

### gbranch

- 全部相同 → 单行分支名。
- 分支不一致 → `project: branch` 多行，顺序与工作区项目一致。
- 单项目工作区 → 单行。
- 含 `(detached)` / `(unknown)` → 视为不同值，走逐项目展示。
- 聚合逻辑单元测试（纯函数，不依赖 git 仓库）。

## 兼容性

- 不影响现有 `glist` / `glist --rm` / `glist --rename` / `gstatus`。
- 旧版无 `gcreate-records.toml` 时，`grm <branch>` 仍可直接删 Git 分支；元数据步骤 no-op。
