# grm：按分支名批量删除本地分支

## 背景

`grove glist --rm` 通过交互选择 gcreate 记录来批量删除本地分支，适合「按一次 gcreate 操作」清理。但用户有时已知精确分支名，希望在工作区内直接删除，无需先查记录、再交互选择。

本设计新增 `grove grm [branch]`，在当前工作区各项目中按精确分支名删除本地分支，删除语义与元数据联动与 `glist --rm` 一致。

## 目标

- `grove grm`：与 `grove glist` 相同，跨工作区列出全部 gcreate 记录。
- `grove grm <branch>`：在当前工作区各项目删除精确分支名 `<branch>`（不应用 `git-prefix`）。
- 删除成功后联动 gcreate 记录与 `workspaces.toml`，行为与 `glist --rm` 对齐。

## 非目标

- 删除或重命名远程分支。
- 模糊匹配、前缀匹配或 glob 模式。
- 对 `<branch>` 应用 `git-prefix`。
- 废弃或替换 `glist --rm`（两者并存）。
- 非交互式 `-y` 跳过确认。

## 命令

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove grm` | `grm` | 列出全部 gcreate 记录（与 `grove glist` 相同输出） |
| `grove grm <branch>` | `grm <branch>` | 在当前工作区各项目删除本地分支 `<branch>` |

## 行为：`grove grm`（无参数）

- 直接复用 `glist` 的列表逻辑（`run_list`）。
- 范围：全部工作区；排序：`created_at` 新 → 旧。
- 输出列：`WORKSPACE`、`BRANCH`、`CREATED`、`STATUS`。
- 无记录时输出 i18n 提示。

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

## 与 `glist --rm` 对比

| | `glist --rm` | `grm <branch>` |
|--|-------------|----------------|
| 入口 | 全局 gcreate 记录交互选择 | CLI 精确分支名 |
| 项目来源 | gcreate 记录快照 | 当前 `workspaces.toml` |
| 记录清理 | 删除选中的单条（by id） | 删除所有匹配 workspace+branch 的记录 |
| 无参数 | 需 `--rm` flag | 展示 glist 列表 |
| 删除语义 | 相同 | 相同 |
| 元数据联动 | 相同 | 相同 |

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
| `src/commands/grm.rs`（新） | `grm` 入口：无参调列表；有参调工作区删除 + 元数据联动 |
| `src/commands/glist.rs` | `run_rm` 改为调用共享删除逻辑（项目来源仍为记录快照） |
| `src/commands/mod.rs` | 注册新模块 |
| `src/main.rs` | `Grm { branch: Option<String> }`，别名 `grm` |
| `src/i18n.rs` | 如需新增文案（优先复用 `gcreate_delete_confirm` 等） |
| `README.md` | 命令表与示例 |

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

- `grm` 无参：与 `glist` 列表输出一致（可共用测试 helper）。
- `grm <branch>` 成功：各项目分支删除、gcreate 记录清理、`workspaces.toml.branch` 回退。
- 工作区不干净：硬错误，记录与 `workspaces.toml` 不变。
- 分支部分不存在：跳过 + info，其余正常删除。
- 多条匹配 gcreate 记录：一次 `grm` 全部清除。
- `operable == 0`：二次 Confirm 仅清记录路径。
- 共享删除逻辑：`glist --rm` 回归测试仍通过。

## 兼容性

- 不影响现有 `glist` / `glist --rm` / `glist --rename`。
- 旧版无 `gcreate-records.toml` 时，`grm <branch>` 仍可直接删 Git 分支；元数据步骤 no-op。
