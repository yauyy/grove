# Grove

**中文** | [English](#english)

> 多项目 Git Worktree 工作区管理器

Grove 让你将多个本地 Git 仓库绑定在一起，统一创建工作分支、统一执行 Git 操作。告别在多个仓库之间反复切换的痛苦。

## 功能特性

- **多项目绑定** — 注册多个本地 Git 项目，按分组管理
- **一键创建工作区** — 交互式多选项目，自动为每个项目创建 worktree 分支
- **批量 Git 操作** — 一条命令同时对所有项目执行 add / commit / push / pull / merge
- **环境分支管理** — 配置测试、预发、正式环境分支，一键合并发布
- **AGENTS.md 合并** — 为每个项目配置 AI 代理描述，创建工作区时自动合并
- **多语言支持** — 支持中文和英文界面，自动检测系统语言
- **VS Code 集成** — `grove code` 一键用 VS Code 打开工作区
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
- 直接输入远程主分支名（自动校验是否存在）
- 配置环境分支（测试 / 预发 / 正式，可选，直接输入 + 校验）
- 配置 agents.md（可选）

### 2. 创建工作区

```bash
grove create feature-login
# 或使用快捷方式
grove -c feature-login
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

# 批量推送到远程同名分支
grove gp

# 合并到测试环境
grove gm
```

### 4. 同步与管理

```bash
# 同步远程主分支更新到工作分支
grove sync

# 编辑工作区（添加/移除项目）
grove -w feature-login

# 用 VS Code 打开工作区
grove code feature-login

# 查看所有工作区状态
grove st

# 删除工作区（清理 worktree + 分支）
grove delete
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

# 直接编辑配置文件
grove config edit           # 编辑 projects.toml
grove config edit config    # 编辑 config.toml
grove config edit workspaces # 编辑 workspaces.toml
```

## 命令参考

### 项目管理

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove add <path>` | | 注册本地 Git 项目 |
| `grove remove` | | 移除已注册的项目 |
| `grove list` | `grove ls` | 列出所有项目（按分组展示） |

### 分组管理

| 命令 | 说明 |
|------|------|
| `grove group add <name>` | 创建分组 |
| `grove group remove` | 删除分组 |
| `grove group list` | 列出所有分组 |
| `grove group reorder` | 调整分组顺序 |
| `grove move` | 移动项目到其他分组 |

### 工作区管理

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove create [name]` | `grove -c` | 创建新工作区 |
| `grove -w [name]` | | 编辑已有工作区（添加/移除项目） |
| `grove delete` | | 删除工作区 |
| `grove status` | `grove st` | 查看工作区状态 |

### 批量 Git 操作

| 命令 | 别名 | 说明 |
|------|------|------|
| `grove sync` | `grove sy` | 同步远程主分支（fetch + merge） |
| `grove gmerge` | `grove gm` | 合并工作分支到环境分支 |
| `grove gstatus` | `grove gs` | 查看所有项目 git status |
| `grove gadd` | `grove ga` | 所有项目 git add . |
| `grove gcommit` | `grove gc` | 统一提交消息 |
| `grove gpush` | `grove gp` | 推送到远程同名分支 |
| `grove gpull` | `grove gl` | 拉取远程更新 |

### 配置与工具

| 命令 | 说明 |
|------|------|
| `grove config set workpath <path>` | 设置工作区根目录（仅影响新建工作区） |
| `grove config set git-prefix <prefix>` | 设置 Git 分支前缀（如 `feat-`） |
| `grove config list` | 查看当前配置 |
| `grove config edit [file]` | 编辑配置文件（projects/config/workspaces） |
| `grove code [name]` | 用 VS Code 打开工作区 |
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

[projects.branches]
main = "main"
test = "develop"           # 可选
staging = "staging"        # 可选
prod = "production"        # 可选
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

1. **禁止自动跟踪** — worktree 创建时使用 `--no-track`，避免意外关联远程分支
2. **继续并汇总** — 批量操作不因单个失败而中断，执行完后统一报告
3. **极简输入** — 所有常用命令提供短别名
4. **配置仅向前** — `config set workpath` 仅影响新创建的工作区
5. **多语言** — 中英文界面，自动检测系统语言，可手动切换
6. **跨平台** — 使用 `PathBuf` 处理路径，兼容 macOS 和 Windows

---

<a id="english"></a>

# Grove (English)

> Multi-project Git worktree workspace manager

Grove binds multiple local Git repositories together, creating unified work branches and executing Git operations across all projects at once.

## Features

- **Multi-project binding** — Register multiple local Git projects, organized by groups
- **One-click workspace creation** — Interactive multi-select, auto-creates worktree branches
- **Batch Git operations** — Single command for add / commit / push / pull / merge across all projects
- **Environment branch management** — Configure test / staging / production branches, merge with one command
- **AGENTS.md merging** — Per-project AI agent descriptions, auto-merged on workspace creation
- **i18n support** — Chinese and English UI, auto-detects system locale
- **VS Code integration** — `grove code` opens workspace in VS Code
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

Interactive prompts guide you through: group selection, branch name input with remote validation, environment branches (optional), agents.md (optional).

### 2. Create a Workspace

```bash
grove create feature-login
```

1. Space to multi-select projects → Enter
2. Enter branch name (defaults to workspace name) → Enter
3. Worktrees created automatically with `--no-track`

### 3. Batch Operations

```bash
cd ~/grove-workspaces/feature-login

grove gs          # git status for all
grove ga          # git add . for all
grove gc          # git commit (unified message)
grove gp          # git push to remote
grove gm          # merge to environment branch
```

### 4. Sync & Manage

```bash
grove sync        # merge remote main into work branch
grove -w name     # edit workspace (add/remove projects)
grove code name   # open workspace in VS Code
grove st          # view all workspace status
grove delete      # delete workspace + cleanup
```

### 5. Language & Config

```bash
grove language zh           # switch to Chinese
grove language en           # switch to English

# Set git branch prefix (auto-prepended when creating workspaces)
grove config set git-prefix feat-
# e.g. workspace "login" → branch defaults to "feat-login"

grove config edit           # edit projects.toml
grove config edit config    # edit config.toml
```

## Command Reference

| Command | Alias | Description |
|---------|-------|-------------|
| `grove add <path>` | | Register a project |
| `grove remove` | | Remove a project |
| `grove list` | `ls` | List projects (grouped) |
| `grove group add/remove/list/reorder` | | Group management |
| `grove move` | `mv` | Move project between groups |
| `grove create [name]` | `-c` | Create workspace |
| `grove -w [name]` | | Edit workspace |
| `grove delete` | | Delete workspace |
| `grove status` | `st` | Workspace status |
| `grove sync` | `sy` | Sync remote main branch |
| `grove gmerge` | `gm` | Merge to environment branch |
| `grove gstatus` | `gs` | Batch git status |
| `grove gadd` | `ga` | Batch git add |
| `grove gcommit` | `gc` | Batch git commit |
| `grove gpush` | `gp` | Batch git push |
| `grove gpull` | `gl` | Batch git pull |
| `grove code [name]` | | Open workspace in VS Code |
| `grove language <en/zh>` | | Set display language |
| `grove config set/list` | | Configuration (workpath, git-prefix) |
| `grove config edit [file]` | | Edit config file in editor |
| `grove completion <shell>` | | Shell completions |

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

[projects.branches]
main = "main"
test = "develop"           # optional
staging = "staging"        # optional
prod = "production"        # optional
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
