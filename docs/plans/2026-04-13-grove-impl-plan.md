# Grove Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a cross-platform Rust CLI tool that manages multi-project git worktree workspaces with batch git operations.

**Architecture:** CLI built with clap (derive macros) for command parsing, dialoguer for interactive terminal UI, TOML files in ~/.grove/ for persistent state. Git operations via std::process::Command calling the system git binary. Each command is a separate module under src/commands/.

**Tech Stack:** Rust, clap 4 (derive), dialoguer, console, serde + toml, dirs, anyhow, chrono

**Design Reference:** `docs/plans/2026-04-13-grove-design.md`

---

## Task 1: Project Scaffold + Clap CLI Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/commands/mod.rs`

**Step 1: Initialize Cargo project**

Run:
```bash
cd /Users/yau/Documents/me/tools
cargo init --name grove
```
Expected: `Cargo.toml` and `src/main.rs` created

**Step 2: Add dependencies to Cargo.toml**

Replace `Cargo.toml` with:

```toml
[package]
name = "grove"
version = "0.1.0"
edition = "2021"
description = "Multi-project git worktree workspace manager"

[dependencies]
clap = { version = "4", features = ["derive"] }
dialoguer = { version = "0.11", features = ["fuzzy-select"] }
console = "0.15"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

**Step 3: Write the clap CLI definition in main.rs**

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "grove", about = "Multi-project git worktree workspace manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Edit an existing workspace (shortcut for workspace editing)
    #[arg(short = 'w', value_name = "NAME")]
    workspace: Option<Option<String>>,

    /// Create a new workspace (shortcut for create)
    #[arg(short = 'c', value_name = "NAME")]
    create: Option<Option<String>>,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a local git project
    Add {
        /// Path to the git repository
        path: String,
    },
    /// Remove a registered project
    Remove,
    /// List all registered projects
    #[command(alias = "ls")]
    List,
    /// Group management
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },
    /// Move a project between groups
    Move {
        /// Project name
        project: Option<String>,
    },
    /// Create a new workspace
    Create {
        /// Workspace name
        name: Option<String>,
    },
    /// Delete a workspace
    Delete,
    /// View workspace status
    #[command(alias = "st")]
    Status,
    /// Sync remote main branch into work branch
    #[command(alias = "sy")]
    Sync,
    /// Merge work branch to environment branch
    #[command(alias = "gm")]
    Gmerge,
    /// Git status for all workspace projects
    #[command(alias = "gs")]
    Gstatus,
    /// Git add for all workspace projects
    #[command(alias = "ga")]
    Gadd,
    /// Git commit for all workspace projects
    #[command(alias = "gc")]
    Gcommit,
    /// Git push for all workspace projects
    #[command(alias = "gp")]
    Gpush,
    /// Git pull for all workspace projects
    #[command(alias = "gl")]
    Gpull,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Generate shell completion scripts
    Completion {
        /// Shell type (bash, zsh, fish, powershell)
        shell: String,
    },
}

#[derive(Subcommand)]
enum GroupAction {
    /// Create a new group
    Add { name: String },
    /// Remove a group
    Remove,
    /// List all groups
    List,
    /// Reorder groups
    Reorder,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// List all configuration values
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle shortcut flags first
    if cli.workspace.is_some() {
        let name = cli.workspace.unwrap();
        return commands::workspace_edit::run(name);
    }

    if cli.create.is_some() {
        let name = cli.create.unwrap();
        return commands::create::run(name);
    }

    match cli.command {
        Some(Commands::Add { path }) => commands::add::run(&path),
        Some(Commands::Remove) => commands::remove::run(),
        Some(Commands::List) => commands::list::run(),
        Some(Commands::Group { action }) => match action {
            GroupAction::Add { name } => commands::group::add(&name),
            GroupAction::Remove => commands::group::remove(),
            GroupAction::List => commands::group::list(),
            GroupAction::Reorder => commands::group::reorder(),
        },
        Some(Commands::Move { project }) => commands::mov::run(project),
        Some(Commands::Create { name }) => commands::create::run(name),
        Some(Commands::Delete) => commands::delete::run(),
        Some(Commands::Status) => commands::status::run(),
        Some(Commands::Sync) => commands::sync::run(),
        Some(Commands::Gmerge) => commands::git_ops::gmerge(),
        Some(Commands::Gstatus) => commands::git_ops::gstatus(),
        Some(Commands::Gadd) => commands::git_ops::gadd(),
        Some(Commands::Gcommit) => commands::git_ops::gcommit(),
        Some(Commands::Gpush) => commands::git_ops::gpush(),
        Some(Commands::Gpull) => commands::git_ops::gpull(),
        Some(Commands::Config { action }) => match action {
            ConfigAction::Set { key, value } => commands::config::set(&key, &value),
            ConfigAction::List => commands::config::list(),
        },
        Some(Commands::Completion { shell }) => commands::completion::run(&shell),
        None => {
            println!("Use 'grove help' for usage information.");
            Ok(())
        }
    }
}
```

**Step 4: Create the commands module stub**

Create `src/commands/mod.rs`:

```rust
pub mod add;
pub mod completion;
pub mod config;
pub mod create;
pub mod delete;
pub mod git_ops;
pub mod group;
pub mod list;
pub mod mov;
pub mod remove;
pub mod status;
pub mod sync;
pub mod workspace_edit;
```

Create stub files for each command module. Every file has the same pattern:

`src/commands/add.rs`:
```rust
use anyhow::Result;

pub fn run(_path: &str) -> Result<()> {
    println!("grove add: not yet implemented");
    Ok(())
}
```

`src/commands/create.rs`:
```rust
use anyhow::Result;

pub fn run(_name: Option<String>) -> Result<()> {
    println!("grove create: not yet implemented");
    Ok(())
}
```

`src/commands/list.rs`:
```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("grove list: not yet implemented");
    Ok(())
}
```

`src/commands/remove.rs`:
```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("grove remove: not yet implemented");
    Ok(())
}
```

`src/commands/group.rs`:
```rust
use anyhow::Result;

pub fn add(_name: &str) -> Result<()> {
    println!("grove group add: not yet implemented");
    Ok(())
}

pub fn remove() -> Result<()> {
    println!("grove group remove: not yet implemented");
    Ok(())
}

pub fn list() -> Result<()> {
    println!("grove group list: not yet implemented");
    Ok(())
}

pub fn reorder() -> Result<()> {
    println!("grove group reorder: not yet implemented");
    Ok(())
}
```

`src/commands/mov.rs`:
```rust
use anyhow::Result;

pub fn run(_project: Option<String>) -> Result<()> {
    println!("grove move: not yet implemented");
    Ok(())
}
```

`src/commands/delete.rs`:
```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("grove delete: not yet implemented");
    Ok(())
}
```

`src/commands/status.rs`:
```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("grove status: not yet implemented");
    Ok(())
}
```

`src/commands/sync.rs`:
```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("grove sync: not yet implemented");
    Ok(())
}
```

`src/commands/git_ops.rs`:
```rust
use anyhow::Result;

pub fn gmerge() -> Result<()> {
    println!("grove gmerge: not yet implemented");
    Ok(())
}

pub fn gstatus() -> Result<()> {
    println!("grove gstatus: not yet implemented");
    Ok(())
}

pub fn gadd() -> Result<()> {
    println!("grove gadd: not yet implemented");
    Ok(())
}

pub fn gcommit() -> Result<()> {
    println!("grove gcommit: not yet implemented");
    Ok(())
}

pub fn gpush() -> Result<()> {
    println!("grove gpush: not yet implemented");
    Ok(())
}

pub fn gpull() -> Result<()> {
    println!("grove gpull: not yet implemented");
    Ok(())
}
```

`src/commands/config.rs`:
```rust
use anyhow::Result;

pub fn set(_key: &str, _value: &str) -> Result<()> {
    println!("grove config set: not yet implemented");
    Ok(())
}

pub fn list() -> Result<()> {
    println!("grove config list: not yet implemented");
    Ok(())
}
```

`src/commands/workspace_edit.rs`:
```rust
use anyhow::Result;

pub fn run(_name: Option<String>) -> Result<()> {
    println!("grove -w: not yet implemented");
    Ok(())
}
```

`src/commands/completion.rs`:
```rust
use anyhow::Result;

pub fn run(_shell: &str) -> Result<()> {
    println!("grove completion: not yet implemented");
    Ok(())
}
```

**Step 5: Verify project compiles**

Run: `cargo build`
Expected: Compiles with no errors

**Step 6: Test CLI help output**

Run: `cargo run -- help`
Expected: Shows all subcommands with descriptions

Run: `cargo run -- --help`
Expected: Shows -w and -c flags plus subcommands

**Step 7: Commit**

```bash
git init
git add Cargo.toml src/
git commit -m "feat: scaffold grove CLI with clap command definitions"
```

---

## Task 2: Config Data Models + Serialization

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/models.rs`

**Step 1: Write tests for config model serialization**

Create `src/config/models.rs`:

```rust
use serde::{Deserialize, Serialize};

// === Global Config ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    pub workpath: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            workpath: home
                .join("grove-workspaces")
                .to_string_lossy()
                .to_string(),
        }
    }
}

// === Branch Config ===

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BranchConfig {
    pub main: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staging: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prod: Option<String>,
}

// === Project ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub order: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md: Option<String>,
    pub branches: BranchConfig,
}

// === Group ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub order: u32,
}

// === Projects File (contains both groups and projects) ===

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectsFile {
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

// === Workspace Project Entry ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceProject {
    pub name: String,
    pub worktree_path: String,
}

// === Workspace ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub created_at: String,
    #[serde(default)]
    pub projects: Vec<WorkspaceProject>,
}

// === Workspaces File ===

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WorkspacesFile {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_roundtrip() {
        let config = GlobalConfig {
            workpath: "/home/user/grove-workspaces".to_string(),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: GlobalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.workpath, "/home/user/grove-workspaces");
    }

    #[test]
    fn test_global_config_default_contains_grove_workspaces() {
        let config = GlobalConfig::default();
        assert!(config.workpath.contains("grove-workspaces"));
    }

    #[test]
    fn test_projects_file_roundtrip() {
        let file = ProjectsFile {
            groups: vec![
                Group { name: "Frontend".to_string(), order: 1 },
                Group { name: "Backend".to_string(), order: 2 },
            ],
            projects: vec![Project {
                name: "web-app".to_string(),
                path: "/projects/web-app".to_string(),
                group: "Frontend".to_string(),
                order: 1,
                agents_md: Some("~/.grove/agents/web-app.md".to_string()),
                branches: BranchConfig {
                    main: "origin/master".to_string(),
                    test: Some("origin/test".to_string()),
                    staging: None,
                    prod: None,
                },
            }],
        };

        let toml_str = toml::to_string_pretty(&file).unwrap();
        let parsed: ProjectsFile = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.groups.len(), 2);
        assert_eq!(parsed.projects.len(), 1);
        assert_eq!(parsed.projects[0].name, "web-app");
        assert_eq!(parsed.projects[0].branches.main, "origin/master");
        assert_eq!(parsed.projects[0].branches.test, Some("origin/test".to_string()));
        assert_eq!(parsed.projects[0].branches.staging, None);
    }

    #[test]
    fn test_workspaces_file_roundtrip() {
        let file = WorkspacesFile {
            workspaces: vec![Workspace {
                name: "feature-login".to_string(),
                branch: "feature-login".to_string(),
                created_at: "2026-04-13".to_string(),
                projects: vec![
                    WorkspaceProject {
                        name: "web-app".to_string(),
                        worktree_path: "/workspaces/feature-login/web-app".to_string(),
                    },
                ],
            }],
        };

        let toml_str = toml::to_string_pretty(&file).unwrap();
        let parsed: WorkspacesFile = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.workspaces[0].name, "feature-login");
        assert_eq!(parsed.workspaces[0].projects.len(), 1);
    }

    #[test]
    fn test_empty_projects_file_deserializes() {
        let toml_str = "";
        let parsed: ProjectsFile = toml::from_str(toml_str).unwrap();
        assert!(parsed.groups.is_empty());
        assert!(parsed.projects.is_empty());
    }

    #[test]
    fn test_branch_config_optional_fields_omitted_in_toml() {
        let config = BranchConfig {
            main: "origin/main".to_string(),
            test: None,
            staging: None,
            prod: None,
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(!toml_str.contains("test"));
        assert!(!toml_str.contains("staging"));
        assert!(!toml_str.contains("prod"));
    }
}
```

**Step 2: Create the config module**

Create `src/config/mod.rs`:

```rust
pub mod models;
```

Update `src/main.rs` to add `mod config;` near the top (after `mod commands;`):

```rust
mod config;
```

**Step 3: Run tests to verify they pass**

Run: `cargo test config::models::tests`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add src/config/
git commit -m "feat: add config data models with serde serialization"
```

---

## Task 3: Config File I/O + Default Paths

**Files:**
- Modify: `src/config/mod.rs`

**Step 1: Write tests for config I/O**

Update `src/config/mod.rs` with full implementation and tests:

```rust
pub mod models;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use models::*;

/// Returns the grove config directory (~/.grove/)
pub fn grove_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".grove"))
}

/// Ensures ~/.grove/ and ~/.grove/agents/ directories exist
pub fn ensure_dirs() -> Result<()> {
    let dir = grove_dir()?;
    fs::create_dir_all(&dir)?;
    fs::create_dir_all(dir.join("agents"))?;
    Ok(())
}

fn config_path() -> Result<PathBuf> {
    Ok(grove_dir()?.join("config.toml"))
}

fn projects_path() -> Result<PathBuf> {
    Ok(grove_dir()?.join("projects.toml"))
}

fn workspaces_path() -> Result<PathBuf> {
    Ok(grove_dir()?.join("workspaces.toml"))
}

// === Load functions ===

pub fn load_global_config() -> Result<GlobalConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(GlobalConfig::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let config: GlobalConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}

pub fn load_projects() -> Result<ProjectsFile> {
    let path = projects_path()?;
    if !path.exists() {
        return Ok(ProjectsFile::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let file: ProjectsFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(file)
}

pub fn load_workspaces() -> Result<WorkspacesFile> {
    let path = workspaces_path()?;
    if !path.exists() {
        return Ok(WorkspacesFile::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let file: WorkspacesFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(file)
}

// === Save functions ===

pub fn save_global_config(config: &GlobalConfig) -> Result<()> {
    ensure_dirs()?;
    let path = config_path()?;
    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn save_projects(file: &ProjectsFile) -> Result<()> {
    ensure_dirs()?;
    let path = projects_path()?;
    let content = toml::to_string_pretty(file)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn save_workspaces(file: &WorkspacesFile) -> Result<()> {
    ensure_dirs()?;
    let path = workspaces_path()?;
    let content = toml::to_string_pretty(file)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Resolve workpath: expand ~ to home directory
pub fn resolve_workpath(workpath: &str) -> Result<PathBuf> {
    if workpath.starts_with('~') {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(&workpath[2..]))
    } else {
        Ok(PathBuf::from(workpath))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper: override grove_dir for testing is complex,
    // so we test resolve_workpath and serialization directly.

    #[test]
    fn test_resolve_workpath_with_tilde() {
        let result = resolve_workpath("~/grove-workspaces").unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("grove-workspaces"));
    }

    #[test]
    fn test_resolve_workpath_absolute() {
        let result = resolve_workpath("/tmp/workspaces").unwrap();
        assert_eq!(result, PathBuf::from("/tmp/workspaces"));
    }

    #[test]
    fn test_save_and_load_projects_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("projects.toml");

        let file = ProjectsFile {
            groups: vec![Group { name: "Test".to_string(), order: 1 }],
            projects: vec![],
        };

        let content = toml::to_string_pretty(&file).unwrap();
        fs::write(&path, &content).unwrap();

        let loaded_content = fs::read_to_string(&path).unwrap();
        let loaded: ProjectsFile = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.groups[0].name, "Test");
    }
}
```

**Step 2: Run tests**

Run: `cargo test config::tests`
Expected: All 3 tests pass

**Step 3: Commit**

```bash
git add src/config/
git commit -m "feat: add config file I/O with load/save functions"
```

---

## Task 4: Git Command Wrappers

**Files:**
- Create: `src/git.rs`

**Step 1: Write git command wrappers with tests**

Create `src/git.rs`:

```rust
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Result of a git command execution
pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Run a git command in a specific directory
pub fn run_git(dir: &Path, args: &[&str]) -> Result<GitOutput> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute git {:?} in {}", args, dir.display()))?;

    Ok(GitOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

/// Run a git command, returning error if it fails
pub fn run_git_checked(dir: &Path, args: &[&str]) -> Result<String> {
    let output = run_git(dir, args)?;
    if !output.success {
        bail!(
            "git {} failed in {}:\n{}",
            args.join(" "),
            dir.display(),
            output.stderr
        );
    }
    Ok(output.stdout)
}

/// Check if a directory is a valid git repository
pub fn is_git_repo(dir: &Path) -> bool {
    run_git(dir, &["rev-parse", "--git-dir"])
        .map(|o| o.success)
        .unwrap_or(false)
}

/// List remote branches
pub fn list_remote_branches(dir: &Path) -> Result<Vec<String>> {
    let output = run_git_checked(dir, &["branch", "-r", "--format=%(refname:short)"])?;
    Ok(output.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// Fetch from remote
pub fn fetch(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["fetch", "origin"])?;
    Ok(())
}

/// Create a worktree with a new branch, no tracking
pub fn worktree_add(
    repo_dir: &Path,
    worktree_path: &Path,
    branch: &str,
    start_point: &str,
) -> Result<()> {
    run_git_checked(
        repo_dir,
        &[
            "worktree",
            "add",
            &worktree_path.to_string_lossy(),
            "-b",
            branch,
            "--no-track",
            start_point,
        ],
    )?;
    Ok(())
}

/// Remove a worktree
pub fn worktree_remove(repo_dir: &Path, worktree_path: &Path) -> Result<()> {
    run_git_checked(
        repo_dir,
        &["worktree", "remove", &worktree_path.to_string_lossy()],
    )?;
    Ok(())
}

/// Delete a local branch
pub fn branch_delete(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["branch", "-d", branch])?;
    Ok(())
}

/// Check if a local branch exists
pub fn branch_exists(dir: &Path, branch: &str) -> Result<bool> {
    let output = run_git(dir, &["rev-parse", "--verify", branch])?;
    Ok(output.success)
}

/// Git add all files
pub fn add_all(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["add", "."])?;
    Ok(())
}

/// Git commit with message
pub fn commit(dir: &Path, message: &str) -> Result<()> {
    run_git_checked(dir, &["commit", "-m", message])?;
    Ok(())
}

/// Git push with upstream tracking
pub fn push_upstream(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["push", "-u", "origin", branch])?;
    Ok(())
}

/// Git pull
pub fn pull(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["pull"])?;
    Ok(())
}

/// Git status --short
pub fn status_short(dir: &Path) -> Result<String> {
    run_git_checked(dir, &["status", "--short"])
}

/// Git merge a branch
pub fn merge(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["merge", branch])?;
    Ok(())
}

/// Git checkout a branch
pub fn checkout(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["checkout", branch])?;
    Ok(())
}

/// Check if working tree is clean (no uncommitted changes)
pub fn is_clean(dir: &Path) -> Result<bool> {
    let output = run_git_checked(dir, &["status", "--porcelain"])?;
    Ok(output.is_empty())
}

/// Get current branch name
pub fn current_branch(dir: &Path) -> Result<String> {
    run_git_checked(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        Command::new("git")
            .current_dir(dir.path())
            .args(["init"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir.path())
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir.path())
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();
        // Create initial commit
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .current_dir(dir.path())
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir.path())
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn test_is_git_repo_valid() {
        let dir = create_test_repo();
        assert!(is_git_repo(dir.path()));
    }

    #[test]
    fn test_is_git_repo_invalid() {
        let dir = TempDir::new().unwrap();
        assert!(!is_git_repo(dir.path()));
    }

    #[test]
    fn test_current_branch() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        // Default branch is usually "main" or "master"
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_is_clean() {
        let dir = create_test_repo();
        assert!(is_clean(dir.path()).unwrap());

        // Create a dirty file
        fs::write(dir.path().join("dirty.txt"), "dirty").unwrap();
        assert!(!is_clean(dir.path()).unwrap());
    }

    #[test]
    fn test_branch_exists() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch_exists(dir.path(), &branch).unwrap());
        assert!(!branch_exists(dir.path(), "nonexistent-branch").unwrap());
    }

    #[test]
    fn test_status_short() {
        let dir = create_test_repo();
        let status = status_short(dir.path()).unwrap();
        assert!(status.is_empty()); // clean repo

        fs::write(dir.path().join("new.txt"), "new").unwrap();
        let status = status_short(dir.path()).unwrap();
        assert!(status.contains("new.txt"));
    }

    #[test]
    fn test_worktree_add_and_remove() {
        let dir = create_test_repo();
        let wt_path = dir.path().join("worktree-test");
        let branch = current_branch(dir.path()).unwrap();

        worktree_add(dir.path(), &wt_path, "test-branch", &branch).unwrap();
        assert!(wt_path.exists());
        assert!(branch_exists(dir.path(), "test-branch").unwrap());

        worktree_remove(dir.path(), &wt_path).unwrap();
        assert!(!wt_path.exists());
    }
}
```

**Step 2: Add git module to main.rs**

Add `mod git;` to `src/main.rs` (after `mod config;`).

**Step 3: Run tests**

Run: `cargo test git::tests`
Expected: All 7 tests pass

**Step 4: Commit**

```bash
git add src/git.rs src/main.rs
git commit -m "feat: add git command wrappers with tests"
```

---

## Task 5: UI Interaction Helpers

**Files:**
- Create: `src/ui.rs`

**Step 1: Write UI helper module**

Create `src/ui.rs`:

```rust
use anyhow::{bail, Result};
use console::Style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};

/// Get a consistent theme for all dialogs
fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

/// Prompt user to input text with an optional default value
pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut builder = Input::<String>::with_theme(&theme()).with_prompt(prompt);
    if let Some(d) = default {
        builder = builder.default(d.to_string());
    }
    let result = builder.interact_text()?;
    Ok(result)
}

/// Prompt user to input text, allow empty (returns None if empty)
pub fn input_optional(prompt: &str, placeholder: &str) -> Result<Option<String>> {
    let result: String = Input::with_theme(&theme())
        .with_prompt(prompt)
        .default(String::new())
        .show_default(false)
        .with_initial_text("")
        .allow_empty(true)
        .with_prompt(format!("{} [{}]", prompt, placeholder))
        .interact_text()?;

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

/// Single select from a list of options. Returns the selected index.
pub fn select(prompt: &str, items: &[String]) -> Result<usize> {
    if items.is_empty() {
        bail!("No items to select from");
    }
    let selection = Select::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?;
    Ok(selection)
}

/// Multi-select from a list of options. Returns indices of selected items.
/// `defaults` indicates which items are pre-selected.
pub fn multi_select(prompt: &str, items: &[String], defaults: &[bool]) -> Result<Vec<usize>> {
    if items.is_empty() {
        bail!("No items to select from");
    }
    let selection = MultiSelect::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .defaults(defaults)
        .interact()?;
    Ok(selection)
}

/// Yes/No confirmation
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    let result = Confirm::with_theme(&theme())
        .with_prompt(prompt)
        .default(default)
        .interact()?;
    Ok(result)
}

/// Print a success message
pub fn success(msg: &str) {
    let style = Style::new().green();
    println!("  {} {}", style.apply_to("✓"), msg);
}

/// Print an error message
pub fn error(msg: &str) {
    let style = Style::new().red();
    println!("  {} {}", style.apply_to("✗"), msg);
}

/// Print an info message
pub fn info(msg: &str) {
    let style = Style::new().cyan();
    println!("  {} {}", style.apply_to("ℹ"), msg);
}

/// Print a warning message
pub fn warn(msg: &str) {
    let style = Style::new().yellow();
    println!("  {} {}", style.apply_to("⚠"), msg);
}

/// Print a section header
pub fn header(msg: &str) {
    let style = Style::new().bold();
    println!("\n{}", style.apply_to(msg));
}

/// Print a batch operation summary
pub fn batch_summary(succeeded: usize, failed: usize) {
    println!();
    if failed == 0 {
        success(&format!("All {} succeeded", succeeded));
    } else {
        warn(&format!("{} succeeded, {} failed", succeeded, failed));
    }
}
```

**Step 2: Add ui module to main.rs**

Add `mod ui;` to `src/main.rs` (after `mod git;`).

**Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add src/ui.rs src/main.rs
git commit -m "feat: add terminal UI interaction helpers"
```

---

## Task 6: Workspace Detection + AGENTS.md Merge

**Files:**
- Create: `src/workspace.rs`

**Step 1: Write workspace utilities with tests**

Create `src/workspace.rs`:

```rust
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::config::models::*;

/// Detect which workspace the current directory belongs to
pub fn detect_workspace(cwd: &Path) -> Result<Option<Workspace>> {
    let ws_file = config::load_workspaces()?;
    let global = config::load_global_config()?;
    let workpath = config::resolve_workpath(&global.workpath)?;

    for ws in &ws_file.workspaces {
        let ws_path = workpath.join(&ws.name);
        if cwd.starts_with(&ws_path) {
            return Ok(Some(ws.clone()));
        }
    }
    Ok(None)
}

/// Get or select a workspace. If cwd is inside one, use it. Otherwise, prompt.
pub fn get_or_select_workspace() -> Result<Workspace> {
    let cwd = std::env::current_dir()?;

    if let Some(ws) = detect_workspace(&cwd)? {
        return Ok(ws);
    }

    // Not in a workspace, let user select
    let ws_file = config::load_workspaces()?;
    if ws_file.workspaces.is_empty() {
        bail!("No workspaces found. Create one with 'grove create'.");
    }

    let names: Vec<String> = ws_file.workspaces.iter().map(|w| w.name.clone()).collect();
    let idx = crate::ui::select("Select a workspace", &names)?;
    Ok(ws_file.workspaces[idx].clone())
}

/// Merge agents.md files from selected projects into a single AGENTS.md
pub fn merge_agents_md(projects: &[&Project], output_path: &Path) -> Result<()> {
    let mut content = String::new();
    let mut has_content = false;

    for project in projects {
        if let Some(agents_path) = &project.agents_md {
            let resolved = resolve_path(agents_path)?;
            if resolved.exists() {
                if has_content {
                    content.push_str("\n---\n\n");
                }
                content.push_str(&format!("# {}\n\n", project.name));
                let agents_content = fs::read_to_string(&resolved)?;
                content.push_str(&agents_content);
                content.push('\n');
                has_content = true;
            }
        }
    }

    if has_content {
        fs::write(output_path, content)?;
    } else if output_path.exists() {
        // Remove stale AGENTS.md if no agents content
        fs::remove_file(output_path)?;
    }

    Ok(())
}

/// Resolve a path that may start with ~
fn resolve_path(path: &str) -> Result<PathBuf> {
    if path.starts_with('~') {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
        Ok(home.join(&path[2..]))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Compute intersection of environment branches across projects.
/// Returns a list of environment names that ALL given projects have configured.
pub fn common_environments(projects_file: &ProjectsFile, project_names: &[String]) -> Vec<String> {
    let mut envs = vec!["test", "staging", "prod"];

    for name in project_names {
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == *name) {
            envs.retain(|env| match *env {
                "test" => project.branches.test.is_some(),
                "staging" => project.branches.staging.is_some(),
                "prod" => project.branches.prod.is_some(),
                _ => false,
            });
        }
    }

    envs.into_iter().map(|s| s.to_string()).collect()
}

/// Get the environment branch for a project by environment name
pub fn get_env_branch(project: &Project, env_name: &str) -> Option<String> {
    match env_name {
        "test" => project.branches.test.clone(),
        "staging" => project.branches.staging.clone(),
        "prod" => project.branches.prod.clone(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_merge_agents_md_single() {
        let dir = TempDir::new().unwrap();
        let agents_path = dir.path().join("agents-a.md");
        fs::write(&agents_path, "Use TypeScript for this project.").unwrap();

        let project = Project {
            name: "web-app".to_string(),
            path: "/tmp".to_string(),
            group: String::new(),
            order: 0,
            agents_md: Some(agents_path.to_string_lossy().to_string()),
            branches: BranchConfig {
                main: "origin/main".to_string(),
                test: None,
                staging: None,
                prod: None,
            },
        };

        let output = dir.path().join("AGENTS.md");
        merge_agents_md(&[&project], &output).unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("# web-app"));
        assert!(content.contains("Use TypeScript"));
    }

    #[test]
    fn test_merge_agents_md_multiple() {
        let dir = TempDir::new().unwrap();

        let a_path = dir.path().join("a.md");
        fs::write(&a_path, "Frontend rules.").unwrap();
        let b_path = dir.path().join("b.md");
        fs::write(&b_path, "Backend rules.").unwrap();

        let pa = Project {
            name: "frontend".to_string(),
            path: "/tmp".to_string(),
            group: String::new(),
            order: 0,
            agents_md: Some(a_path.to_string_lossy().to_string()),
            branches: BranchConfig { main: "origin/main".to_string(), test: None, staging: None, prod: None },
        };
        let pb = Project {
            name: "backend".to_string(),
            path: "/tmp".to_string(),
            group: String::new(),
            order: 0,
            agents_md: Some(b_path.to_string_lossy().to_string()),
            branches: BranchConfig { main: "origin/main".to_string(), test: None, staging: None, prod: None },
        };

        let output = dir.path().join("AGENTS.md");
        merge_agents_md(&[&pa, &pb], &output).unwrap();

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("# frontend"));
        assert!(content.contains("Frontend rules."));
        assert!(content.contains("---"));
        assert!(content.contains("# backend"));
        assert!(content.contains("Backend rules."));
    }

    #[test]
    fn test_merge_agents_md_no_agents() {
        let dir = TempDir::new().unwrap();
        let project = Project {
            name: "bare".to_string(),
            path: "/tmp".to_string(),
            group: String::new(),
            order: 0,
            agents_md: None,
            branches: BranchConfig { main: "origin/main".to_string(), test: None, staging: None, prod: None },
        };

        let output = dir.path().join("AGENTS.md");
        merge_agents_md(&[&project], &output).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn test_common_environments_all_have_test() {
        let file = ProjectsFile {
            groups: vec![],
            projects: vec![
                Project {
                    name: "a".to_string(),
                    path: "/tmp".to_string(),
                    group: String::new(),
                    order: 0,
                    agents_md: None,
                    branches: BranchConfig {
                        main: "origin/main".to_string(),
                        test: Some("origin/test".to_string()),
                        staging: None,
                        prod: None,
                    },
                },
                Project {
                    name: "b".to_string(),
                    path: "/tmp".to_string(),
                    group: String::new(),
                    order: 0,
                    agents_md: None,
                    branches: BranchConfig {
                        main: "origin/main".to_string(),
                        test: Some("origin/test".to_string()),
                        staging: Some("origin/staging".to_string()),
                        prod: None,
                    },
                },
            ],
        };

        let envs = common_environments(&file, &["a".to_string(), "b".to_string()]);
        assert_eq!(envs, vec!["test"]);
    }

    #[test]
    fn test_common_environments_none() {
        let file = ProjectsFile {
            groups: vec![],
            projects: vec![
                Project {
                    name: "a".to_string(),
                    path: "/tmp".to_string(),
                    group: String::new(),
                    order: 0,
                    agents_md: None,
                    branches: BranchConfig {
                        main: "origin/main".to_string(),
                        test: Some("origin/test".to_string()),
                        staging: None,
                        prod: None,
                    },
                },
                Project {
                    name: "b".to_string(),
                    path: "/tmp".to_string(),
                    group: String::new(),
                    order: 0,
                    agents_md: None,
                    branches: BranchConfig {
                        main: "origin/main".to_string(),
                        test: None,
                        staging: Some("origin/staging".to_string()),
                        prod: None,
                    },
                },
            ],
        };

        let envs = common_environments(&file, &["a".to_string(), "b".to_string()]);
        assert!(envs.is_empty());
    }
}
```

**Step 2: Add workspace module to main.rs**

Add `mod workspace;` to `src/main.rs` (after `mod ui;`).

**Step 3: Run tests**

Run: `cargo test workspace::tests`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add src/workspace.rs src/main.rs
git commit -m "feat: add workspace detection and AGENTS.md merge logic"
```

---

## Task 7: `grove add` Command

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Implement the add command**

Replace `src/commands/add.rs`:

```rust
use anyhow::{bail, Result};
use std::path::Path;

use crate::config;
use crate::config::models::*;
use crate::git;
use crate::ui;

pub fn run(path: &str) -> Result<()> {
    // 1. Resolve and validate the path
    let repo_path = std::fs::canonicalize(path)
        .map_err(|_| anyhow::anyhow!("Path does not exist: {}", path))?;

    if !git::is_git_repo(&repo_path) {
        bail!("Not a valid git repository: {}", repo_path.display());
    }

    // 2. Check if already registered
    let mut projects_file = config::load_projects()?;
    let repo_str = repo_path.to_string_lossy().to_string();

    if projects_file.projects.iter().any(|p| p.path == repo_str) {
        bail!("Project already registered: {}. Use 'grove list' to view.", repo_str);
    }

    // 3. Auto-detect project name
    let default_name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".to_string());

    let name = ui::input("Project name", Some(&default_name))?;

    // Check for name conflict
    if projects_file.projects.iter().any(|p| p.name == name) {
        bail!("A project named '{}' already exists.", name);
    }

    // 4. Select group
    let group = select_group(&mut projects_file)?;

    // 5. Select remote main branch
    ui::header("Remote Branch Configuration");
    git::fetch(&repo_path).ok(); // Best effort fetch
    let remote_branches = git::list_remote_branches(&repo_path)?;

    if remote_branches.is_empty() {
        bail!("No remote branches found. Add a remote first.");
    }

    let main_idx = ui::select("Select remote main branch", &remote_branches)?;
    let main_branch = remote_branches[main_idx].clone();

    // 6. Configure environment branches (optional)
    let test = select_env_branch("Test environment branch", &remote_branches)?;
    let staging = select_env_branch("Staging environment branch", &remote_branches)?;
    let prod = select_env_branch("Production environment branch", &remote_branches)?;

    // 7. Configure agents.md (optional)
    let agents_md = if ui::confirm("Configure agents.md for this project?", false)? {
        let agents_path = ui::input("Path to agents.md file", None)?;
        let resolved = std::fs::canonicalize(&agents_path).ok();
        if let Some(ref p) = resolved {
            if !p.exists() {
                ui::warn(&format!("File does not exist yet: {}", p.display()));
            }
        }
        Some(agents_path)
    } else {
        None
    };

    // 8. Calculate order
    let order = projects_file
        .projects
        .iter()
        .filter(|p| p.group == group)
        .count() as u32
        + 1;

    // 9. Save
    let project = Project {
        name: name.clone(),
        path: repo_str,
        group,
        order,
        agents_md,
        branches: BranchConfig {
            main: main_branch,
            test,
            staging,
            prod,
        },
    };

    projects_file.projects.push(project);
    config::save_projects(&projects_file)?;

    ui::success(&format!("Project '{}' registered successfully!", name));
    Ok(())
}

fn select_group(projects_file: &mut ProjectsFile) -> Result<String> {
    let mut options: Vec<String> = projects_file
        .groups
        .iter()
        .map(|g| g.name.clone())
        .collect();
    options.push("+ New group".to_string());
    options.push("Ungrouped".to_string());

    let idx = ui::select("Select group", &options)?;

    if idx == options.len() - 1 {
        // Ungrouped
        Ok(String::new())
    } else if idx == options.len() - 2 {
        // New group
        let name = ui::input("New group name", None)?;
        let order = projects_file.groups.len() as u32 + 1;
        projects_file.groups.push(Group { name: name.clone(), order });
        Ok(name)
    } else {
        Ok(options[idx].clone())
    }
}

fn select_env_branch(prompt: &str, branches: &[String]) -> Result<Option<String>> {
    let mut options = vec!["Skip (none)".to_string()];
    options.extend(branches.iter().cloned());

    let idx = ui::select(prompt, &options)?;
    if idx == 0 {
        Ok(None)
    } else {
        Ok(Some(options[idx].clone()))
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat: implement grove add command"
```

---

## Task 8: `grove list` Command

**Files:**
- Modify: `src/commands/list.rs`

**Step 1: Implement the list command**

Replace `src/commands/list.rs`:

```rust
use anyhow::Result;
use console::Style;

use crate::config;

pub fn run() -> Result<()> {
    let projects_file = config::load_projects()?;

    if projects_file.projects.is_empty() {
        println!("No projects registered. Use 'grove add <path>' to register a project.");
        return Ok(());
    }

    let bold = Style::new().bold();
    let dim = Style::new().dim();

    // Collect groups (sorted by order)
    let mut groups = projects_file.groups.clone();
    groups.sort_by_key(|g| g.order);

    // Print grouped projects
    for group in &groups {
        let group_projects: Vec<_> = projects_file
            .projects
            .iter()
            .filter(|p| p.group == group.name)
            .collect();

        if group_projects.is_empty() {
            continue;
        }

        println!("\n{}", bold.apply_to(&group.name));
        for project in group_projects {
            print_project(project, &dim);
        }
    }

    // Print ungrouped projects
    let ungrouped: Vec<_> = projects_file
        .projects
        .iter()
        .filter(|p| p.group.is_empty())
        .collect();

    if !ungrouped.is_empty() {
        println!("\n{}", bold.apply_to("Ungrouped"));
        for project in ungrouped {
            print_project(project, &dim);
        }
    }

    Ok(())
}

fn print_project(project: &crate::config::models::Project, dim: &Style) {
    println!("  {} {}", project.name, dim.apply_to(&project.path));
    let mut branches = vec![format!("main: {}", project.branches.main)];
    if let Some(ref t) = project.branches.test {
        branches.push(format!("test: {}", t));
    }
    if let Some(ref s) = project.branches.staging {
        branches.push(format!("staging: {}", s));
    }
    if let Some(ref p) = project.branches.prod {
        branches.push(format!("prod: {}", p));
    }
    println!("    {}", dim.apply_to(branches.join(" | ")));
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/list.rs
git commit -m "feat: implement grove list command"
```

---

## Task 9: `grove remove` Command

**Files:**
- Modify: `src/commands/remove.rs`

**Step 1: Implement the remove command**

Replace `src/commands/remove.rs`:

```rust
use anyhow::{bail, Result};

use crate::config;
use crate::ui;

pub fn run() -> Result<()> {
    let mut projects_file = config::load_projects()?;

    if projects_file.projects.is_empty() {
        bail!("No projects registered.");
    }

    let names: Vec<String> = projects_file.projects.iter().map(|p| {
        if p.group.is_empty() {
            p.name.clone()
        } else {
            format!("{} ({})", p.name, p.group)
        }
    }).collect();

    let idx = ui::select("Select project to remove", &names)?;
    let project_name = projects_file.projects[idx].name.clone();

    // Check if project is in any workspace
    let ws_file = config::load_workspaces()?;
    let in_workspaces: Vec<String> = ws_file
        .workspaces
        .iter()
        .filter(|ws| ws.projects.iter().any(|p| p.name == project_name))
        .map(|ws| ws.name.clone())
        .collect();

    if !in_workspaces.is_empty() {
        ui::warn(&format!(
            "Project '{}' is used in workspaces: {}",
            project_name,
            in_workspaces.join(", ")
        ));
        if !ui::confirm("Remove anyway?", false)? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    projects_file.projects.remove(idx);
    config::save_projects(&projects_file)?;

    ui::success(&format!("Project '{}' removed.", project_name));
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/remove.rs
git commit -m "feat: implement grove remove command"
```

---

## Task 10: `grove group` Subcommands

**Files:**
- Modify: `src/commands/group.rs`

**Step 1: Implement group subcommands**

Replace `src/commands/group.rs`:

```rust
use anyhow::{bail, Result};
use console::Style;

use crate::config;
use crate::ui;

pub fn add(name: &str) -> Result<()> {
    let mut projects_file = config::load_projects()?;

    if projects_file.groups.iter().any(|g| g.name == name) {
        bail!("Group '{}' already exists.", name);
    }

    let order = projects_file.groups.len() as u32 + 1;
    projects_file
        .groups
        .push(crate::config::models::Group { name: name.to_string(), order });
    config::save_projects(&projects_file)?;

    ui::success(&format!("Group '{}' created.", name));
    Ok(())
}

pub fn remove() -> Result<()> {
    let mut projects_file = config::load_projects()?;

    if projects_file.groups.is_empty() {
        bail!("No groups to remove.");
    }

    let names: Vec<String> = projects_file.groups.iter().map(|g| g.name.clone()).collect();
    let idx = ui::select("Select group to remove", &names)?;
    let group_name = names[idx].clone();

    // Move projects in this group to ungrouped
    let affected = projects_file
        .projects
        .iter()
        .filter(|p| p.group == group_name)
        .count();

    if affected > 0 {
        ui::info(&format!(
            "{} project(s) in this group will become ungrouped.",
            affected
        ));
    }

    for project in &mut projects_file.projects {
        if project.group == group_name {
            project.group = String::new();
        }
    }

    projects_file.groups.remove(idx);
    config::save_projects(&projects_file)?;

    ui::success(&format!("Group '{}' removed.", group_name));
    Ok(())
}

pub fn list() -> Result<()> {
    let projects_file = config::load_projects()?;
    let bold = Style::new().bold();
    let dim = Style::new().dim();

    if projects_file.groups.is_empty() && projects_file.projects.is_empty() {
        println!("No groups or projects registered.");
        return Ok(());
    }

    let mut groups = projects_file.groups.clone();
    groups.sort_by_key(|g| g.order);

    for group in &groups {
        let count = projects_file
            .projects
            .iter()
            .filter(|p| p.group == group.name)
            .count();
        println!(
            "  {} {}",
            bold.apply_to(&group.name),
            dim.apply_to(format!("({} projects)", count))
        );
    }

    let ungrouped = projects_file
        .projects
        .iter()
        .filter(|p| p.group.is_empty())
        .count();
    if ungrouped > 0 {
        println!(
            "  {} {}",
            bold.apply_to("Ungrouped"),
            dim.apply_to(format!("({} projects)", ungrouped))
        );
    }

    Ok(())
}

pub fn reorder() -> Result<()> {
    let mut projects_file = config::load_projects()?;

    if projects_file.groups.len() < 2 {
        bail!("Need at least 2 groups to reorder.");
    }

    projects_file.groups.sort_by_key(|g| g.order);

    ui::info("Select the group you want to move:");
    let names: Vec<String> = projects_file.groups.iter().map(|g| g.name.clone()).collect();
    let from_idx = ui::select("Move which group?", &names)?;

    let positions: Vec<String> = (1..=names.len())
        .map(|i| format!("Position {}", i))
        .collect();
    let to_idx = ui::select("Move to position", &positions)?;

    let group = projects_file.groups.remove(from_idx);
    projects_file.groups.insert(to_idx, group);

    // Recalculate orders
    for (i, g) in projects_file.groups.iter_mut().enumerate() {
        g.order = i as u32 + 1;
    }

    config::save_projects(&projects_file)?;
    ui::success("Groups reordered.");
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/group.rs
git commit -m "feat: implement grove group subcommands"
```

---

## Task 11: `grove move` Command

**Files:**
- Modify: `src/commands/mov.rs`

**Step 1: Implement move command**

Replace `src/commands/mov.rs`:

```rust
use anyhow::{bail, Result};

use crate::config;
use crate::ui;

pub fn run(project: Option<String>) -> Result<()> {
    let mut projects_file = config::load_projects()?;

    if projects_file.projects.is_empty() {
        bail!("No projects registered.");
    }

    // Select or find the project
    let project_idx = if let Some(ref name) = project {
        projects_file
            .projects
            .iter()
            .position(|p| p.name == *name)
            .ok_or_else(|| anyhow::anyhow!("Project '{}' not found.", name))?
    } else {
        let names: Vec<String> = projects_file.projects.iter().map(|p| p.name.clone()).collect();
        ui::select("Select project to move", &names)?
    };

    // Build group options
    let mut options: Vec<String> = projects_file
        .groups
        .iter()
        .map(|g| g.name.clone())
        .collect();
    options.push("Ungrouped".to_string());

    let target_idx = ui::select("Move to group", &options)?;

    let new_group = if target_idx == options.len() - 1 {
        String::new()
    } else {
        options[target_idx].clone()
    };

    projects_file.projects[project_idx].group = new_group.clone();

    // Recalculate order within the new group
    let order = projects_file
        .projects
        .iter()
        .filter(|p| p.group == new_group)
        .count() as u32;
    projects_file.projects[project_idx].order = order;

    config::save_projects(&projects_file)?;

    let display_group = if new_group.is_empty() {
        "Ungrouped".to_string()
    } else {
        new_group
    };
    ui::success(&format!(
        "Project '{}' moved to '{}'.",
        projects_file.projects[project_idx].name, display_group
    ));
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/mov.rs
git commit -m "feat: implement grove move command"
```

---

## Task 12: `grove create` Command

**Files:**
- Modify: `src/commands/create.rs`

**Step 1: Implement the create command**

Replace `src/commands/create.rs`:

```rust
use anyhow::{bail, Result};
use std::fs;

use crate::config;
use crate::config::models::*;
use crate::git;
use crate::ui;
use crate::workspace;

pub fn run(name: Option<String>) -> Result<()> {
    let projects_file = config::load_projects()?;
    let mut ws_file = config::load_workspaces()?;
    let global = config::load_global_config()?;

    if projects_file.projects.is_empty() {
        bail!("No projects registered. Use 'grove add <path>' first.");
    }

    // 1. Get workspace name
    let ws_name = match name {
        Some(n) => n,
        None => ui::input("Workspace name", None)?,
    };

    if ws_name.is_empty() {
        bail!("Workspace name cannot be empty.");
    }

    if ws_file.workspaces.iter().any(|w| w.name == ws_name) {
        bail!("Workspace '{}' already exists.", ws_name);
    }

    // 2. Multi-select projects (grouped display)
    let (display_items, project_indices) = build_grouped_project_list(&projects_file);
    let defaults = vec![false; display_items.len()];
    let selected = ui::multi_select("Select projects (space to toggle, enter to confirm)", &display_items, &defaults)?;

    if selected.is_empty() {
        bail!("No projects selected.");
    }

    let selected_projects: Vec<&Project> = selected
        .iter()
        .map(|&i| &projects_file.projects[project_indices[i]])
        .collect();

    // 3. Prompt for branch name (default = workspace name)
    let branch = ui::input(&format!("Branch name [{}]", ws_name), Some(&ws_name))?;

    // 4. Create workspace directory
    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(&ws_name);

    if ws_dir.exists() {
        bail!("Directory already exists: {}", ws_dir.display());
    }

    fs::create_dir_all(&ws_dir)?;

    // 5. Create worktrees for each selected project
    ui::header("Creating worktrees...");
    let mut ws_projects = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for project in &selected_projects {
        let repo_path = std::path::Path::new(&project.path);
        let wt_path = ws_dir.join(&project.name);

        // Check if branch already exists
        if git::branch_exists(repo_path, &branch)? {
            ui::error(&format!(
                "{}: branch '{}' already exists. Skipping.",
                project.name, branch
            ));
            failed += 1;
            continue;
        }

        // Fetch origin
        if let Err(e) = git::fetch(repo_path) {
            ui::error(&format!("{}: fetch failed: {}", project.name, e));
            failed += 1;
            continue;
        }

        // Create worktree
        match git::worktree_add(repo_path, &wt_path, &branch, &project.branches.main) {
            Ok(()) => {
                ui::success(&format!("{} -> {}", project.name, wt_path.display()));
                ws_projects.push(WorkspaceProject {
                    name: project.name.clone(),
                    worktree_path: wt_path.to_string_lossy().to_string(),
                });
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: worktree creation failed: {}", project.name, e));
                failed += 1;
            }
        }
    }

    // 6. Merge agents.md
    let agents_output = ws_dir.join("AGENTS.md");
    workspace::merge_agents_md(&selected_projects, &agents_output)?;
    if agents_output.exists() {
        ui::info("AGENTS.md generated.");
    }

    // 7. Save workspace record
    let ws = Workspace {
        name: ws_name.clone(),
        branch,
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        projects: ws_projects,
    };
    ws_file.workspaces.push(ws);
    config::save_workspaces(&ws_file)?;

    // 8. Summary
    ui::batch_summary(succeeded, failed);
    ui::info(&format!("Workspace created at: {}", ws_dir.display()));

    Ok(())
}

/// Build display items for grouped project selection.
/// Returns (display strings, mapping from display index to projects_file.projects index)
pub fn build_grouped_project_list(projects_file: &ProjectsFile) -> (Vec<String>, Vec<usize>) {
    let mut items = Vec::new();
    let mut indices = Vec::new();

    // Sorted groups
    let mut groups = projects_file.groups.clone();
    groups.sort_by_key(|g| g.order);

    for group in &groups {
        for (idx, project) in projects_file.projects.iter().enumerate() {
            if project.group == group.name {
                items.push(format!("[{}] {}", group.name, project.name));
                indices.push(idx);
            }
        }
    }

    // Ungrouped
    for (idx, project) in projects_file.projects.iter().enumerate() {
        if project.group.is_empty() {
            items.push(project.name.clone());
            indices.push(idx);
        }
    }

    (items, indices)
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/create.rs
git commit -m "feat: implement grove create command"
```

---

## Task 13: `grove -w` (Edit Workspace) Command

**Files:**
- Modify: `src/commands/workspace_edit.rs`

**Step 1: Implement workspace edit command**

Replace `src/commands/workspace_edit.rs`:

```rust
use anyhow::{bail, Result};
use std::path::Path;

use crate::config;
use crate::config::models::*;
use crate::git;
use crate::ui;
use crate::workspace;
use crate::commands::create::build_grouped_project_list;

pub fn run(name: Option<String>) -> Result<()> {
    let projects_file = config::load_projects()?;
    let mut ws_file = config::load_workspaces()?;
    let global = config::load_global_config()?;

    if ws_file.workspaces.is_empty() {
        bail!("No workspaces found. Create one with 'grove create'.");
    }

    // 1. Select workspace
    let ws_idx = if let Some(ref n) = name {
        ws_file
            .workspaces
            .iter()
            .position(|w| w.name == *n)
            .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found.", n))?
    } else {
        let names: Vec<String> = ws_file.workspaces.iter().map(|w| w.name.clone()).collect();
        ui::select("Select workspace to edit", &names)?
    };

    let ws = &ws_file.workspaces[ws_idx];
    let branch = ws.branch.clone();
    let ws_name = ws.name.clone();
    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(&ws_name);

    // 2. Show multi-select with current projects pre-checked
    let (display_items, project_indices) = build_grouped_project_list(&projects_file);

    let current_project_names: Vec<String> = ws.projects.iter().map(|p| p.name.clone()).collect();

    let defaults: Vec<bool> = project_indices
        .iter()
        .map(|&idx| {
            let project_name = &projects_file.projects[idx].name;
            current_project_names.contains(project_name)
        })
        .collect();

    let selected = ui::multi_select(
        "Edit project selection (space to toggle, enter to confirm)",
        &display_items,
        &defaults,
    )?;

    let selected_names: Vec<String> = selected
        .iter()
        .map(|&i| projects_file.projects[project_indices[i]].name.clone())
        .collect();

    // 3. Determine additions and removals
    let to_add: Vec<&str> = selected_names
        .iter()
        .filter(|n| !current_project_names.contains(n))
        .map(|n| n.as_str())
        .collect();

    let to_remove: Vec<&str> = current_project_names
        .iter()
        .filter(|n| !selected_names.contains(n))
        .map(|n| n.as_str())
        .collect();

    if to_add.is_empty() && to_remove.is_empty() {
        ui::info("No changes.");
        return Ok(());
    }

    // 4. Check removals for uncommitted changes
    for name in &to_remove {
        if let Some(ws_proj) = ws.projects.iter().find(|p| p.name == *name) {
            let wt_path = Path::new(&ws_proj.worktree_path);
            if wt_path.exists() && !git::is_clean(wt_path)? {
                bail!(
                    "Project '{}' has uncommitted changes. Commit or stash before removing.",
                    name
                );
            }
        }
    }

    // 5. Process removals
    let mut succeeded = 0;
    let mut failed = 0;

    for name in &to_remove {
        if let Some(ws_proj) = ws.projects.iter().find(|p| p.name == *name) {
            if let Some(project) = projects_file.projects.iter().find(|p| p.name == *name) {
                let repo_path = Path::new(&project.path);
                let wt_path = Path::new(&ws_proj.worktree_path);
                match git::worktree_remove(repo_path, wt_path) {
                    Ok(()) => {
                        ui::success(&format!("Removed: {}", name));
                        succeeded += 1;
                    }
                    Err(e) => {
                        ui::error(&format!("{}: removal failed: {}", name, e));
                        failed += 1;
                    }
                }
            }
        }
    }

    // 6. Process additions
    for name in &to_add {
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == *name) {
            let repo_path = Path::new(&project.path);
            let wt_path = ws_dir.join(name);

            if let Err(e) = git::fetch(repo_path) {
                ui::error(&format!("{}: fetch failed: {}", name, e));
                failed += 1;
                continue;
            }

            match git::worktree_add(repo_path, &wt_path, &branch, &project.branches.main) {
                Ok(()) => {
                    ui::success(&format!("Added: {}", name));
                    succeeded += 1;
                }
                Err(e) => {
                    ui::error(&format!("{}: worktree creation failed: {}", name, e));
                    failed += 1;
                }
            }
        }
    }

    // 7. Update workspace record
    let mut new_projects: Vec<WorkspaceProject> = ws.projects
        .iter()
        .filter(|p| !to_remove.contains(&p.name.as_str()))
        .cloned()
        .collect();

    for name in &to_add {
        let wt_path = ws_dir.join(name);
        if wt_path.exists() {
            new_projects.push(WorkspaceProject {
                name: name.to_string(),
                worktree_path: wt_path.to_string_lossy().to_string(),
            });
        }
    }

    ws_file.workspaces[ws_idx].projects = new_projects;
    config::save_workspaces(&ws_file)?;

    // 8. Regenerate AGENTS.md
    let all_selected: Vec<&crate::config::models::Project> = selected_names
        .iter()
        .filter_map(|n| projects_file.projects.iter().find(|p| p.name == *n))
        .collect();
    let agents_output = ws_dir.join("AGENTS.md");
    workspace::merge_agents_md(&all_selected, &agents_output)?;

    ui::batch_summary(succeeded, failed);
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/workspace_edit.rs
git commit -m "feat: implement grove -w workspace edit command"
```

---

## Task 14: `grove delete` Command

**Files:**
- Modify: `src/commands/delete.rs`

**Step 1: Implement delete command**

Replace `src/commands/delete.rs`:

```rust
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::config;
use crate::git;
use crate::ui;

pub fn run() -> Result<()> {
    let mut ws_file = config::load_workspaces()?;
    let projects_file = config::load_projects()?;
    let global = config::load_global_config()?;

    if ws_file.workspaces.is_empty() {
        bail!("No workspaces to delete.");
    }

    let names: Vec<String> = ws_file.workspaces.iter().map(|w| w.name.clone()).collect();
    let idx = ui::select("Select workspace to delete", &names)?;
    let ws = &ws_file.workspaces[idx];

    // Check for uncommitted changes
    let mut has_dirty = false;
    for ws_proj in &ws.projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        if wt_path.exists() {
            if let Ok(clean) = git::is_clean(wt_path) {
                if !clean {
                    ui::warn(&format!("{}: has uncommitted changes", ws_proj.name));
                    has_dirty = true;
                }
            }
        }
    }

    if has_dirty {
        if !ui::confirm("There are uncommitted changes. Delete anyway?", false)? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let branch = ws.branch.clone();

    // Remove worktrees and branches
    ui::header("Cleaning up...");
    for ws_proj in &ws.projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        if let Some(project) = projects_file.projects.iter().find(|p| p.name == ws_proj.name) {
            let repo_path = Path::new(&project.path);

            if wt_path.exists() {
                match git::worktree_remove(repo_path, wt_path) {
                    Ok(()) => ui::success(&format!("{}: worktree removed", ws_proj.name)),
                    Err(e) => ui::error(&format!("{}: worktree removal failed: {}", ws_proj.name, e)),
                }
            }

            // Delete branch
            match git::branch_delete(repo_path, &branch) {
                Ok(()) => ui::success(&format!("{}: branch '{}' deleted", ws_proj.name, branch)),
                Err(_) => {} // Branch may not exist or may have been merged
            }
        }
    }

    // Remove workspace directory
    let workpath = config::resolve_workpath(&global.workpath)?;
    let ws_dir = workpath.join(&ws.name);
    if ws_dir.exists() {
        fs::remove_dir_all(&ws_dir)?;
        ui::success(&format!("Directory removed: {}", ws_dir.display()));
    }

    // Remove from workspaces.toml
    ws_file.workspaces.remove(idx);
    config::save_workspaces(&ws_file)?;

    ui::success(&format!("Workspace '{}' deleted.", names[idx]));
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/delete.rs
git commit -m "feat: implement grove delete command"
```

---

## Task 15: `grove status` Command

**Files:**
- Modify: `src/commands/status.rs`

**Step 1: Implement status command**

Replace `src/commands/status.rs`:

```rust
use anyhow::Result;
use console::Style;
use std::path::Path;

use crate::config;
use crate::git;

pub fn run() -> Result<()> {
    let ws_file = config::load_workspaces()?;

    if ws_file.workspaces.is_empty() {
        println!("No workspaces. Create one with 'grove create'.");
        return Ok(());
    }

    let bold = Style::new().bold();
    let dim = Style::new().dim();
    let green = Style::new().green();
    let yellow = Style::new().yellow();

    for ws in &ws_file.workspaces {
        println!(
            "\n{} {} {}",
            bold.apply_to(&ws.name),
            dim.apply_to(format!("(branch: {})", ws.branch)),
            dim.apply_to(format!("[{}]", ws.created_at))
        );

        for ws_proj in &ws.projects {
            let wt_path = Path::new(&ws_proj.worktree_path);
            let status = if wt_path.exists() {
                match git::status_short(wt_path) {
                    Ok(s) if s.is_empty() => green.apply_to("clean").to_string(),
                    Ok(s) => {
                        let changes = s.lines().count();
                        yellow
                            .apply_to(format!("{} changes", changes))
                            .to_string()
                    }
                    Err(_) => "error reading status".to_string(),
                }
            } else {
                "missing".to_string()
            };

            println!("  {} - {}", ws_proj.name, status);
        }
    }

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/status.rs
git commit -m "feat: implement grove status command"
```

---

## Task 16: Batch Git Operations (gstatus, gadd, gcommit, gpush, gpull)

**Files:**
- Modify: `src/commands/git_ops.rs`

**Step 1: Implement all simple batch git operations**

Replace `src/commands/git_ops.rs`:

```rust
use anyhow::{bail, Result};
use console::Style;
use std::path::Path;

use crate::config;
use crate::config::models::*;
use crate::git;
use crate::ui;
use crate::workspace;

/// Get the current workspace and its projects with repo paths
fn get_workspace_context() -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;

    let matched: Vec<(WorkspaceProject, Project)> = ws
        .projects
        .iter()
        .filter_map(|ws_proj| {
            projects_file
                .projects
                .iter()
                .find(|p| p.name == ws_proj.name)
                .map(|p| (ws_proj.clone(), p.clone()))
        })
        .collect();

    if matched.is_empty() {
        bail!("No projects found in workspace '{}'.", ws.name);
    }

    Ok((ws, matched))
}

pub fn gstatus() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let bold = Style::new().bold();

    ui::header(&format!("Workspace: {} (branch: {})", ws.name, ws.branch));

    for (ws_proj, _) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        println!("\n  {}", bold.apply_to(&ws_proj.name));

        match git::status_short(wt_path) {
            Ok(status) if status.is_empty() => {
                ui::success("Working tree clean");
            }
            Ok(status) => {
                for line in status.lines() {
                    println!("    {}", line);
                }
            }
            Err(e) => {
                ui::error(&format!("Failed: {}", e));
            }
        }
    }

    Ok(())
}

pub fn gadd() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    ui::header(&format!("git add . — {}", ws.name));

    let mut succeeded = 0;
    let mut failed = 0;

    for (ws_proj, _) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        match git::add_all(wt_path) {
            Ok(()) => {
                ui::success(&ws_proj.name);
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", ws_proj.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gcommit() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;

    let message = ui::input("Commit message", None)?;
    if message.is_empty() {
        bail!("Commit message cannot be empty.");
    }

    ui::header(&format!("git commit — {}", ws.name));

    let mut succeeded = 0;
    let mut failed = 0;

    for (ws_proj, _) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);

        // Check if there is anything to commit
        match git::status_short(wt_path) {
            Ok(status) if status.is_empty() => {
                ui::info(&format!("{}: nothing to commit", ws_proj.name));
                continue;
            }
            _ => {}
        }

        match git::commit(wt_path, &message) {
            Ok(()) => {
                ui::success(&ws_proj.name);
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", ws_proj.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gpush() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    ui::header(&format!("git push -u origin {} — {}", ws.branch, ws.name));

    let mut succeeded = 0;
    let mut failed = 0;

    for (ws_proj, _) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        match git::push_upstream(wt_path, &ws.branch) {
            Ok(()) => {
                ui::success(&format!("{} -> origin/{}", ws_proj.name, ws.branch));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", ws_proj.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gpull() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    ui::header(&format!("git pull — {}", ws.name));

    let mut succeeded = 0;
    let mut failed = 0;

    for (ws_proj, _) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        match git::pull(wt_path) {
            Ok(()) => {
                ui::success(&ws_proj.name);
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", ws_proj.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gmerge() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let projects_file = config::load_projects()?;

    // Compute common environments
    let project_names: Vec<String> = projects.iter().map(|(wp, _)| wp.name.clone()).collect();
    let envs = workspace::common_environments(&projects_file, &project_names);

    if envs.is_empty() {
        // Check which projects are missing which environments
        for (ws_proj, project) in &projects {
            let missing: Vec<&str> = ["test", "staging", "prod"]
                .iter()
                .filter(|env| workspace::get_env_branch(project, env).is_none())
                .copied()
                .collect();
            if !missing.is_empty() {
                ui::warn(&format!(
                    "{}: missing environments: {}",
                    ws_proj.name,
                    missing.join(", ")
                ));
            }
        }
        bail!("No common environment branch configured across all workspace projects.");
    }

    // Select target environment
    let env_idx = ui::select("Merge to which environment?", &envs)?;
    let target_env = &envs[env_idx];

    ui::header(&format!(
        "Merging {} -> {} environment",
        ws.branch, target_env
    ));

    let mut succeeded = 0;
    let mut failed = 0;

    for (ws_proj, project) in &projects {
        let wt_path = Path::new(&ws_proj.worktree_path);
        let env_branch = match workspace::get_env_branch(project, target_env) {
            Some(b) => b,
            None => continue,
        };

        // Extract the local branch name from remote ref (e.g., origin/test -> test)
        let local_env_branch = env_branch
            .strip_prefix("origin/")
            .unwrap_or(&env_branch);

        ui::info(&format!("{}: merging {} -> {}", ws_proj.name, ws.branch, local_env_branch));

        // Fetch
        if let Err(e) = git::fetch(wt_path) {
            ui::error(&format!("{}: fetch failed: {}", ws_proj.name, e));
            failed += 1;
            continue;
        }

        // Checkout env branch
        if let Err(e) = git::checkout(wt_path, local_env_branch) {
            ui::error(&format!("{}: checkout {} failed: {}", ws_proj.name, local_env_branch, e));
            failed += 1;
            continue;
        }

        // Merge work branch
        match git::merge(wt_path, &ws.branch) {
            Ok(()) => {
                ui::success(&format!("{}: merged", ws_proj.name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: merge failed: {}", ws_proj.name, e));
                failed += 1;
            }
        }

        // Switch back to work branch
        if let Err(e) = git::checkout(wt_path, &ws.branch) {
            ui::error(&format!("{}: failed to switch back to {}: {}", ws_proj.name, ws.branch, e));
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/git_ops.rs
git commit -m "feat: implement batch git operations (gadd, gcommit, gpush, gpull, gstatus, gmerge)"
```

---

## Task 17: `grove sync` Command

**Files:**
- Modify: `src/commands/sync.rs`

**Step 1: Implement sync command**

Replace `src/commands/sync.rs`:

```rust
use anyhow::Result;
use std::path::Path;

use crate::config;
use crate::git;
use crate::ui;
use crate::workspace;

pub fn run() -> Result<()> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;

    ui::header(&format!("Syncing workspace: {} (branch: {})", ws.name, ws.branch));

    let mut succeeded = 0;
    let mut failed = 0;

    for ws_proj in &ws.projects {
        let wt_path = Path::new(&ws_proj.worktree_path);

        let project = match projects_file.projects.iter().find(|p| p.name == ws_proj.name) {
            Some(p) => p,
            None => {
                ui::error(&format!("{}: project not found in registry", ws_proj.name));
                failed += 1;
                continue;
            }
        };

        ui::info(&format!("{}: fetching + merging {}", ws_proj.name, project.branches.main));

        // Fetch
        if let Err(e) = git::fetch(wt_path) {
            ui::error(&format!("{}: fetch failed: {}", ws_proj.name, e));
            failed += 1;
            continue;
        }

        // Merge remote main branch
        match git::merge(wt_path, &project.branches.main) {
            Ok(()) => {
                ui::success(&format!("{}: synced", ws_proj.name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: merge failed: {}", ws_proj.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/sync.rs
git commit -m "feat: implement grove sync command"
```

---

## Task 18: `grove config` Command

**Files:**
- Modify: `src/commands/config.rs`

**Step 1: Implement config command**

Replace `src/commands/config.rs`:

```rust
use anyhow::{bail, Result};
use std::fs;

use crate::config;
use crate::ui;

pub fn set(key: &str, value: &str) -> Result<()> {
    match key {
        "workpath" => {
            let mut global = config::load_global_config()?;
            global.workpath = value.to_string();

            // Ensure directory exists
            let resolved = config::resolve_workpath(value)?;
            if !resolved.exists() {
                fs::create_dir_all(&resolved)?;
                ui::info(&format!("Created directory: {}", resolved.display()));
            }

            config::save_global_config(&global)?;
            ui::success(&format!("workpath = {}", value));
            ui::info("This change only affects newly created workspaces.");
        }
        _ => {
            bail!("Unknown config key: '{}'. Available keys: workpath", key);
        }
    }
    Ok(())
}

pub fn list() -> Result<()> {
    let global = config::load_global_config()?;
    let resolved = config::resolve_workpath(&global.workpath)?;

    println!("workpath = {} ({})", global.workpath, resolved.display());

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/commands/config.rs
git commit -m "feat: implement grove config command"
```

---

## Task 19: `grove completion` Command

**Files:**
- Modify: `src/commands/completion.rs`
- Modify: `src/main.rs` (extract Cli struct for reuse)

**Step 1: Implement completion generation**

Replace `src/commands/completion.rs`:

```rust
use anyhow::{bail, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::Cli;

pub fn run(shell: &str) -> Result<()> {
    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "ps" => Shell::PowerShell,
        _ => bail!("Unsupported shell: {}. Use: bash, zsh, fish, powershell", shell),
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "grove", &mut io::stdout());
    Ok(())
}
```

**Step 2: Add clap_complete dependency**

Add to `Cargo.toml` under `[dependencies]`:

```toml
clap_complete = "4"
```

**Step 3: Make Cli struct public in main.rs**

Change `struct Cli` to `pub struct Cli` in `src/main.rs`.

**Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles with no errors

**Step 5: Test completion output**

Run: `cargo run -- completion bash | head -5`
Expected: Outputs bash completion script

**Step 6: Commit**

```bash
git add Cargo.toml src/commands/completion.rs src/main.rs
git commit -m "feat: implement grove completion command for shell tab completion"
```

---

## Task 20: Integration Test + Final Build Verification

**Files:**
- Create: `tests/cli_test.rs`

**Step 1: Write basic CLI integration tests**

Create `tests/cli_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_output() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-project git worktree workspace manager"));
}

#[test]
fn test_list_empty() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("list")
        .assert()
        .success();
}

#[test]
fn test_status_empty() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_config_list() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("workpath"));
}

#[test]
fn test_completion_bash() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["completion", "bash"])
        .assert()
        .success();
}

#[test]
fn test_completion_zsh() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["completion", "zsh"])
        .assert()
        .success();
}

#[test]
fn test_add_invalid_path() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["add", "/nonexistent/path"])
        .assert()
        .failure();
}

#[test]
fn test_aliases() {
    // Test 'ls' alias for 'list'
    Command::cargo_bin("grove")
        .unwrap()
        .arg("ls")
        .assert()
        .success();

    // Test 'st' alias for 'status'
    Command::cargo_bin("grove")
        .unwrap()
        .arg("st")
        .assert()
        .success();
}

#[test]
fn test_no_args_shows_usage() {
    Command::cargo_bin("grove")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("grove help"));
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All unit tests and integration tests pass

**Step 3: Build release binary**

Run: `cargo build --release`
Expected: Release binary at `target/release/grove`

**Step 4: Test release binary**

Run: `./target/release/grove --help`
Expected: Shows help output

**Step 5: Commit**

```bash
git add tests/
git commit -m "test: add CLI integration tests"
```

**Step 6: Final full test run**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All tests pass, no clippy warnings

---

## Summary

| Task | What | Key Files |
|------|------|-----------|
| 1 | Project scaffold + clap CLI | `Cargo.toml`, `src/main.rs`, `src/commands/mod.rs` |
| 2 | Config data models | `src/config/models.rs` |
| 3 | Config file I/O | `src/config/mod.rs` |
| 4 | Git command wrappers | `src/git.rs` |
| 5 | UI interaction helpers | `src/ui.rs` |
| 6 | Workspace detection + AGENTS.md merge | `src/workspace.rs` |
| 7 | `grove add` | `src/commands/add.rs` |
| 8 | `grove list` | `src/commands/list.rs` |
| 9 | `grove remove` | `src/commands/remove.rs` |
| 10 | `grove group *` | `src/commands/group.rs` |
| 11 | `grove move` | `src/commands/mov.rs` |
| 12 | `grove create` | `src/commands/create.rs` |
| 13 | `grove -w` (edit workspace) | `src/commands/workspace_edit.rs` |
| 14 | `grove delete` | `src/commands/delete.rs` |
| 15 | `grove status` | `src/commands/status.rs` |
| 16 | Batch git ops | `src/commands/git_ops.rs` |
| 17 | `grove sync` | `src/commands/sync.rs` |
| 18 | `grove config` | `src/commands/config.rs` |
| 19 | `grove completion` | `src/commands/completion.rs` |
| 20 | Integration tests + final verification | `tests/cli_test.rs` |
