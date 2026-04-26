# Grove

**中文** | [English](#english)

> 多项目 Git Worktree 工作区管理器

Grove 让你将多个本地 Git 仓库绑定在一起，统一创建工作分支、统一执行 Git 操作。告别在多个仓库之间反复切换的痛苦。

## 功能特性

- **多项目绑定** — 注册多个本地 Git 项目，按分组管理
- **一键创建工作区** — 交互式多选项目，自动为每个项目创建 worktree 分支
- **批量 Git 操作** — 一条命令同时对所有项目执行 add / commit / push / pull / merge
- **分支预设与别名** — 全局配置常用分支选项，每个项目可映射到不同真实分支
- **批量分支切换/创建** — `gswitch` 统一切换，`gcreate` 先 fetch，并基于各项目配置的 `origin/<main>` 创建，失败时回滚已创建分支
- **工作区重命名** — 重命名工作区及对应分支，自动修复 worktree 链接
- **AGENTS.md 合并** — 为每个项目配置 AI 代理描述，创建工作区时自动合并
- **多语言支持** — 支持中文和英文界面，自动检测系统语言
- **VS Code 集成** — 一键用 VS Code 打开工作区
- **跨平台** — 支持 macOS 和 Windows
- **极简操作** — 所有常用命令都有短别名，支持 Tab 补全

## 安装

### 从 GitHub Release 下载（推荐）

前往 [Releases](https://github.com/yauyy/grove/releases) 下载对应平台的二进制文件，解压后放入 PATH 即可。

### 通过 Cargo 安装

```bash
cargo install grove-cli
```

### 通过 Homebrew 安装（macOS）

```bash
brew tap yauyy/grove
brew install grove
```

### 从源码编译

```bash
git clone https://github.com/yauyy/grove.git
cd grove
cargo install --path .
```

**前提条件：** 系统需要已安装 Git。

## 快速开始

### 1. 注册项目

```bash
# 添加本地 Git 项目
grove add /path/to/frontend
grove add /path/to/backend
```

添加时会交互式引导你：

- 选择分组（前端 / 后端 / 新建分组）
- 输入主分支名（默认 master）
- 配置分支预设映射（测试 / 预发 / 正式，可选）
- 自动校验分支是否存在于远程
- 配置 agents.md（可选）

### 2. 创建工作区

```bash
grove -c feature-login
# 或
grove -w create feature-login
```

交互流程：

1. 空格多选需要的项目 → 回车确认
2. 输入分支名（默认同工作区名） → 回车
3. 自动创建 worktree，基于远程主分支，`--no-track`

### 3. 批量操作

```bash
# 进入工作区目录
cd ~/grove-workspaces/feature-login

# 查看所有项目状态
grove gs

# 批量暂存
grove ga

# 统一提交（所有项目用同一条消息）
grove gc

# 推送当前或指定目标分支
grove gp

# 合并工作分支到交互选择或指定目标分支
grove gm test
```

### 4. 同步与管理

```bash
# 同步远程主分支更新到工作分支
grove sync

# 编辑工作区（添加/移除项目）
grove -w feature-login

# 重命名工作区（可选同时重命名分支）
grove -w rename

# 用 VS Code 打开工作区
grove -w code feature-login

# 查看所有工作区状态
grove -w st

# 删除工作区（可选删除本地分支，不影响远程分支）
grove -w rm
```

### 5. 语言与配置

```bash
# 切换为中文界面
grove language zh

# 切换为英文界面
grove language en

# 设置 Git 分支前缀（创建工作区时分支名自动添加前缀）
grove config set git-prefix feat-
# 例如：工作区名 login → 分支名默认 feat-login

# 日期模板会按当天展开
grove config set git-prefix 'feature/ymy/[YYYYMMDD]/'
# 例如：2026-04-24 创建 login → feature/ymy/20260424/login

# 设置 gcommit 提交信息来源（manual/codex/claude/copilot/cursor）
grove config set commit-message-tool codex

# 创建工作区后自动生成/同步 go.work
grove config set auto-go-work true

# 为旧项目自动补 tags（目前识别 go.mod → tags = ["go"]）
grove tags

# 直接编辑配置文件
grove config edit           # 编辑 projects.toml
grove config edit config    # 编辑 config.toml
grove config edit workspaces # 编辑 workspaces.toml
```

## 命令参考

命令按三个维度组织：**项目**（顶级命令）、**工作区**（`-w`）、**Git 操作**（`g` 前缀）。

### 项目管理

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove add <path>` | | 注册本地 Git 项目 |
| `grove remove` | `grove rm` | 移除已注册的项目 |
| `grove list` | `grove ls` | 列出所有项目（按分组展示） |
| `grove move` | `grove mv` | 移动项目到其他分组 |
| `grove tags` | | 自动识别并更新项目 tags |

### 分组管理

| 命令 | 说明 |
|------|------|
| `grove group add <name>` | 创建分组 |
| `grove group remove` | 删除分组 |
| `grove group list` | 列出所有分组 |
| `grove group reorder` | 调整分组顺序 |

### 工作区管理（`-w`）

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove -w create [name]` | `grove -c [name]` | 创建新工作区 |
| `grove -w [name]` | | 编辑工作区（添加/移除项目） |
| `grove -w remove` | `grove -w rm` | 删除工作区 |
| `grove -w rename` | `grove -w rn` | 重命名工作区（可选同时重命名分支） |
| `grove -w status` | `grove -w st` | 查看所有工作区状态 |
| `grove -w code [name]` | | 用 VS Code 打开工作区 |

### 批量 Git 操作（`g` 前缀）

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove sync` | `grove sy` | 同步远程主分支（fetch + merge） |
| `grove gswitch <target>` | `grove gsw <target>` | 所有项目切换到目标分支 |
| `grove gcreate <name>` | `grove gcr <name>` | 先 fetch，所有项目基于配置的 `main` 对应远程起点 `origin/<main>` 创建并切换到新分支；失败时回滚已创建分支 |
| `grove grename` | `grove grn` | 重命名所有项目的分支 |
| `grove gstatus` | `grove gs` | 查看所有项目 git status |
| `grove gadd` | `grove ga` | 所有项目 git add -A |
| `grove gcommit` | `grove gc` | 统一提交消息 |
| `grove gpush [target]` | `grove gp [target]` | 推送当前或指定分支 |
| `grove gmerge [target]` | `grove gm [target]` | 合并工作分支到交互选择或指定目标分支 |
| `grove gpull` | `grove gl` | 拉取远程更新 |
| `grove gowork` | `grove gw` | 为当前工作区生成/更新 go.work |

#### 示例输出

```text
gpush target: test

✓ api: pushed test-master -> origin/test-master (target: test)
✓ web: pushed develop -> origin/develop (target: test)

2 succeeded, 0 failed
```

```text
gmerge target: test

✓ api: merged feature/login -> test-master (target: test)
✓ web: merged feature/login -> develop (target: test)

2 succeeded, 0 failed
```

### 配置与工具

| 命令 | 说明 |
|------|------|
| `grove config set workpath <path>` | 设置工作区根目录（仅影响新建工作区） |
| `grove config set git-prefix <prefix>` | 设置 Git 分支前缀（如 `feat-`，支持 `[YYYYMMDD]` / `[YYYY-MM-DD]` / `[YYYY/MM/DD]`） |
| `grove config set commit-message-tool <tool>` | 设置 gcommit 提交信息来源（manual/codex/claude/copilot/cursor） |
| `grove config set auto-go-work <true/false>` | 创建工作区后自动生成/同步 go.work |
| `grove config list` | 查看当前配置 |
| `grove config edit [file]` | 编辑配置文件（projects/config/workspaces） |
| `grove language <en/zh>` | 切换界面语言 |
| `grove completion <shell>` | 生成 Shell 补全脚本 |

## 配置文件

所有配置存储在 `~/.grove/` 目录下：

```
~/.grove/
├── config.toml          # 全局配置
├── projects.toml        # 已注册项目
├── workspaces.toml      # 工作区记录
└── agents/              # 各项目的 agents.md
```

### config.toml

```toml
workpath = "~/grove-workspaces"   # 工作区根目录
language = "zh"                   # 界面语言（en / zh）
git_prefix = "feat-"              # Git 分支前缀（可选，默认为空）
commit_message_tool = "manual"    # gcommit 提交信息来源
auto_go_work = false              # 创建工作区后自动同步 go.work

[branch_presets]
test = "测试环境"
staging = "预发环境"
prod = "正式环境"
master = "主分支"
```

### projects.toml

```toml
[[groups]]
name = "frontend"
order = 0

[[projects]]
name = "web-app"
path = "/Users/you/projects/web-app"
group = "frontend"
order = 0
tags = ["go"]              # 可选；grove tags 可自动识别 go.mod 并补充

[projects.branches]
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"

[projects.branch_aliases]
test-master = "test"
```

### workspaces.toml

```toml
[[workspaces]]
name = "login"
branch = "feat-login"
created_at = "2026-04-14"

[[workspaces.projects]]
name = "web-app"
worktree_path = "/Users/you/grove-workspaces/login/web-app"
```

默认工作区路径：

- macOS: `~/grove-workspaces`
- Windows: `C:\Users\<user>\grove-workspaces`

## Shell 补全

```bash
# Zsh
grove completion zsh > ~/.zsh/completions/_grove

# Bash
grove completion bash > ~/.bash_completion.d/grove.bash

# Fish
grove completion fish > ~/.config/fish/completions/grove.fish

# PowerShell
grove completion powershell | Out-File ~\grove.ps1
```

## 设计原则

1. **三维命令结构** — 项目（顶级命令）、工作区（`-w`）、Git 操作（`g` 前缀），职责分明
2. **禁止自动跟踪** — worktree 创建时使用 `--no-track`，避免意外关联远程分支
3. **继续并汇总** — 批量操作不因单个失败而中断，执行完后统一报告
4. **极简输入** — 所有常用命令提供短别名
5. **配置仅向前** — `config set workpath` 仅影响新创建的工作区
6. **多语言** — 中英文界面，自动检测系统语言，可手动切换
7. **跨平台** — 使用 `PathBuf` 处理路径，兼容 macOS 和 Windows

---

<a id="english"></a>

# Grove (English)

> Multi-project Git worktree workspace manager

Grove binds multiple local Git repositories together, creating unified work branches and executing Git operations across all projects at once.

## Features

- **Multi-project binding** — Register multiple local Git projects, organized by groups
- **One-click workspace creation** — Interactive multi-select, auto-creates worktree branches
- **Batch Git operations** — Single command for add / commit / push / pull / merge across all projects
- **Branch presets and aliases** — Configure shared branch choices while each project maps them to its own real branch
- **Batch branch switch/create** — `gswitch` switches all projects; `gcreate` fetches first, creates from each project's configured `origin/<main>`, and rolls back created branches on failure
- **Workspace renaming** — Rename workspace and optionally its branch, auto-repairs worktree links
- **AGENTS.md merging** — Per-project AI agent descriptions, auto-merged on workspace creation
- **i18n support** — Chinese and English UI, auto-detects system locale
- **VS Code integration** — Open workspace in VS Code with one command
- **Cross-platform** — macOS and Windows
- **Minimal typing** — Short aliases for all frequent commands, Tab completion

## Installation

### From GitHub Releases (Recommended)

Download the binary for your platform from [Releases](https://github.com/yauyy/grove/releases).

### Via Cargo

```bash
cargo install grove-cli
```

### Via Homebrew (macOS)

```bash
brew tap yauyy/grove
brew install grove
```

### From Source

```bash
git clone https://github.com/yauyy/grove.git
cd grove
cargo install --path .
```

**Prerequisite:** Git must be installed.

## Quick Start

### 1. Register Projects

```bash
grove add /path/to/frontend
grove add /path/to/backend
```

Interactive prompts guide you through: group selection, main branch (default master), branch preset mappings (optional), remote validation, agents.md (optional).

### 2. Create a Workspace

```bash
grove -c feature-login
# or
grove -w create feature-login
```

1. Space to multi-select projects → Enter
2. Enter branch name (defaults to workspace name) → Enter
3. Worktrees created automatically from remote main with `--no-track`

### 3. Batch Operations

```bash
cd ~/grove-workspaces/feature-login

grove gs          # git status for all
grove ga          # git add -A for all
grove gc          # git commit (unified message)
grove gp          # push current branch
grove gm test     # merge work branch to target branch
```

### 4. Sync & Manage

```bash
grove sync            # merge remote main into work branch
grove -w name         # edit workspace (add/remove projects)
grove -w rename       # rename workspace (optionally rename branch)
grove -w code name    # open workspace in VS Code
grove -w st           # view all workspace status
grove -w rm           # delete workspace
```

### 5. Language & Config

```bash
grove language zh           # switch to Chinese
grove language en           # switch to English

# Set git branch prefix (auto-prepended when creating workspaces)
grove config set git-prefix feat-
# e.g. workspace "login" → branch defaults to "feat-login"

# Date templates are expanded using today's date
grove config set git-prefix 'feature/ymy/[YYYYMMDD]/'
# e.g. on 2026-04-24 workspace "login" → "feature/ymy/20260424/login"

# Set gcommit message source (manual/codex/claude/copilot/cursor)
grove config set commit-message-tool codex

# Generate/sync go.work automatically after workspace creation
grove config set auto-go-work true

# Backfill tags for existing projects (go.mod → tags = ["go"])
grove tags

grove config edit           # edit projects.toml
grove config edit config    # edit config.toml
grove config edit workspaces # edit workspaces.toml
```

## Command Reference

Commands are organized in three dimensions: **Project** (top-level), **Workspace** (`-w`), and **Git operations** (`g` prefix).

### Project Management

| Command | Alias | Description |
|---------|-------|-------------|
| `grove add <path>` | | Register a project |
| `grove remove` | `rm` | Remove a project |
| `grove list` | `ls` | List projects (grouped) |
| `grove move` | `mv` | Move project between groups |
| `grove group add/remove/list/reorder` | | Group management |
| `grove tags` | | Auto-detect and update project tags |

### Workspace Management (`-w`)

| Command | Alias | Description |
|---------|-------|-------------|
| `grove -w create [name]` | `-c [name]` | Create workspace |
| `grove -w [name]` | | Edit workspace (add/remove projects) |
| `grove -w remove` | `-w rm` | Delete workspace |
| `grove -w rename` | `-w rn` | Rename workspace (optionally rename branch) |
| `grove -w status` | `-w st` | Workspace status overview |
| `grove -w code [name]` | | Open workspace in VS Code |

### Batch Git Operations (`g` prefix)

| Command | Alias | Description |
|---------|-------|-------------|
| `grove sync` | `sy` | Sync remote main branch (fetch + merge) |
| `grove gswitch <target>` | `grove gsw <target>` | Switch all projects to the target branch |
| `grove gcreate <name>` | `grove gcr <name>` | Fetch first, then create and switch all projects from each configured `main` remote start point `origin/<main>`; roll back created branches on failure |
| `grove grename` | `grn` | Rename branch across all projects |
| `grove gstatus` | `gs` | Batch git status |
| `grove gadd` | `ga` | Batch git add -A |
| `grove gcommit` | `gc` | Batch git commit |
| `grove gpush [target]` | `grove gp [target]` | Push the current or specified branch |
| `grove gmerge [target]` | `grove gm [target]` | Merge the work branch to an interactive or specified target branch |
| `grove gpull` | `gl` | Batch git pull |
| `grove gowork` | `gw` | Generate/update go.work for the current workspace |

#### Output Examples

```text
gpush target: test

✓ api: pushed test-master -> origin/test-master (target: test)
✓ web: pushed develop -> origin/develop (target: test)

2 succeeded, 0 failed
```

```text
gmerge target: test

✓ api: merged feature/login -> test-master (target: test)
✓ web: merged feature/login -> develop (target: test)

2 succeeded, 0 failed
```

### Configuration & Tools

| Command | Description |
|---------|-------------|
| `grove config set workpath <path>` | Set workspace root (affects new workspaces only) |
| `grove config set git-prefix <prefix>` | Set git branch prefix (supports `[YYYYMMDD]`, `[YYYY-MM-DD]`, `[YYYY/MM/DD]`) |
| `grove config set commit-message-tool <tool>` | Set gcommit message source (manual/codex/claude/copilot/cursor) |
| `grove config set auto-go-work <true/false>` | Generate/sync go.work after workspace creation |
| `grove config list` | View current config |
| `grove config edit [file]` | Edit config file in editor |
| `grove language <en/zh>` | Set display language |
| `grove completion <shell>` | Generate shell completions |

## Config Files

All configuration is stored in `~/.grove/`:

```
~/.grove/
├── config.toml          # Global config
├── projects.toml        # Registered projects
├── workspaces.toml      # Workspace records
└── agents/              # Per-project agents.md
```

### config.toml

```toml
workpath = "~/grove-workspaces"   # Workspace root directory
language = "en"                   # UI language (en / zh)
git_prefix = "feat-"              # Git branch prefix (optional, empty by default)
commit_message_tool = "manual"    # gcommit message source
auto_go_work = false              # Auto-sync go.work after workspace creation

[branch_presets]
test = "Test environment"
staging = "Staging environment"
prod = "Production environment"
master = "Main branch"
```

### projects.toml

```toml
[[groups]]
name = "frontend"
order = 0

[[projects]]
name = "web-app"
path = "/Users/you/projects/web-app"
group = "frontend"
order = 0
tags = ["go"]              # optional; grove tags can detect go.mod and backfill this

[projects.branches]
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"

[projects.branch_aliases]
test-master = "test"
```

### workspaces.toml

```toml
[[workspaces]]
name = "login"
branch = "feat-login"
created_at = "2026-04-14"

[[workspaces.projects]]
name = "web-app"
worktree_path = "/Users/you/grove-workspaces/login/web-app"
```

Default workspace path:

- macOS: `~/grove-workspaces`
- Windows: `C:\Users\<user>\grove-workspaces`

## License

[MIT](LICENSE)
