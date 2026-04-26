# Branch Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add configurable branch presets, project branch input aliases, `gswitch`, `gcreate`, and target-aware `gmerge`/`gpush` behavior to Grove.

**Architecture:** Keep Grove's current command layout, but introduce a focused branch-target resolver so all branch commands share one parsing rule. Extend config models to support global presets and per-project alias/mapping data, then update Git commands to precheck before mutating state and print explicit success summaries.

**Tech Stack:** Rust 2021, `clap`, `serde`, `toml`, `anyhow`, Git CLI through `std::process::Command`, existing `dialoguer`-based UI helpers.

---

## File Map

- Modify `src/config/models.rs`: extend `GlobalConfig`, replace fixed `BranchConfig` with required `main` plus flattened extra branch mappings, add project `branch_aliases`, and update config tests.
- Modify `src/config/mod.rs`: add default branch preset helpers and keep date-prefix behavior unchanged.
- Create `src/branch_target.rs`: centralize branch preset lookup, target resolution, batch precheck result types, and display labels.
- Modify `src/git.rs`: make `current_branch` available at runtime, add checked start-point resolution and branch creation helpers used by rollback.
- Modify `src/workspace.rs`: update environment helpers to use extensible branch mappings while preserving current `test/staging/prod` behavior.
- Modify `src/commands/git_ops.rs`: add `gmerge(target)`, `gpush(target)`, `gswitch(target)`, `gcreate(name)`, prechecks, rollback, and success summaries.
- Modify `src/main.rs`: add CLI arguments for `gmerge`, `gpush`, and new commands `gswitch`/`gcreate`.
- Modify `src/commands/add.rs`: keep existing prompts but write into the new `BranchConfig` shape.
- Modify `src/commands/create.rs`, `src/commands/rename.rs`, `src/commands/workspace_edit.rs`, `src/commands/sync.rs`: adjust branch config field access after `BranchConfig` changes.
- Modify `src/i18n.rs`: add user-facing messages for branch presets, precheck failures, switch/create summaries, and target labels.
- Modify `README.md`: document new config, commands, examples, and success output.
- Modify `tests/cli_test.rs` and module unit tests: cover config compatibility, target resolution, CLI signatures, prechecks, and summaries.

Do not commit during implementation unless the user explicitly asks for commits.

## Task 1: Config Model And Defaults

**Files:**
- Modify: `src/config/models.rs`
- Modify: `src/config/mod.rs`
- Test: `src/config/models.rs`
- Test: `src/config/mod.rs`

- [ ] **Step 1: Write failing config model tests**

Add tests that prove old branch config still works, new branch mappings deserialize, project aliases deserialize, and default presets are available.

```rust
#[test]
fn test_branch_config_accepts_extra_mappings() {
    let toml_str = r#"
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"
"#;

    let parsed: BranchConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(parsed.main, "master");
    assert_eq!(parsed.get("test"), Some("test-master"));
    assert_eq!(parsed.get("staging"), Some("pre"));
    assert_eq!(parsed.get("prod"), Some("master"));
    assert_eq!(parsed.get("master"), Some("main"));
    assert_eq!(parsed.get("missing"), None);
}

#[test]
fn test_project_branch_aliases_roundtrip() {
    let pf = ProjectsFile {
        groups: Vec::new(),
        projects: vec![Project {
            name: "api".to_string(),
            path: "/tmp/api".to_string(),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            agents_md: None,
            branch_aliases: std::collections::BTreeMap::from([(
                "test-master".to_string(),
                "test".to_string(),
            )]),
            branches: BranchConfig {
                main: "master".to_string(),
                aliases: std::collections::BTreeMap::from([(
                    "test".to_string(),
                    "test-master".to_string(),
                )]),
            },
        }],
    };

    let toml_str = toml::to_string(&pf).unwrap();
    let parsed: ProjectsFile = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.projects[0].branch_aliases.get("test-master"), Some(&"test".to_string()));
    assert_eq!(parsed.projects[0].branches.get("test"), Some("test-master"));
}
```

Add this test in `src/config/mod.rs`:

```rust
#[test]
fn test_default_branch_presets_include_current_environments() {
    let presets = default_branch_presets();

    assert_eq!(presets.get("test"), Some(&"Test branch".to_string()));
    assert_eq!(presets.get("staging"), Some(&"Staging branch".to_string()));
    assert_eq!(presets.get("prod"), Some(&"Prod branch".to_string()));
}
```

- [ ] **Step 2: Run config tests and verify they fail**

Run:

```bash
cargo test config::models::tests::test_branch_config_accepts_extra_mappings config::models::tests::test_project_branch_aliases_roundtrip config::tests::test_default_branch_presets_include_current_environments
```

Expected: FAIL because `BranchConfig.aliases`, `Project.branch_aliases`, and `default_branch_presets()` do not exist yet.

- [ ] **Step 3: Implement extensible config types**

In `src/config/models.rs`, add `BTreeMap` and update the config structs:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
```

Replace `GlobalConfig` with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub workpath: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub git_prefix: String,
    #[serde(default = "default_commit_message_tool")]
    pub commit_message_tool: String,
    #[serde(default)]
    pub auto_go_work: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub branch_presets: BTreeMap<String, String>,
}
```

Replace `BranchConfig` with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    pub main: String,
    #[serde(default, flatten)]
    pub aliases: BTreeMap<String, String>,
}

impl BranchConfig {
    pub fn get(&self, name: &str) -> Option<&str> {
        if name == "main" {
            Some(self.main.as_str())
        } else {
            self.aliases.get(name).map(String::as_str)
        }
    }

    pub fn set_alias(&mut self, name: impl Into<String>, branch: impl Into<String>) {
        self.aliases.insert(name.into(), branch.into());
    }
}
```

Update `Project`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub branch_aliases: BTreeMap<String, String>,
    pub branches: BranchConfig,
}
```

Update `Default for GlobalConfig`:

```rust
impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            workpath: "~/grove-workspaces".to_string(),
            language: default_language(),
            git_prefix: String::new(),
            commit_message_tool: default_commit_message_tool(),
            auto_go_work: false,
            branch_presets: BTreeMap::new(),
        }
    }
}
```

Update every `Project` literal in tests to include:

```rust
branch_aliases: BTreeMap::new(),
```

Update every `BranchConfig` literal from:

```rust
BranchConfig {
    main: "main".to_string(),
    test: Some("test".to_string()),
    staging: None,
    prod: None,
}
```

to:

```rust
BranchConfig {
    main: "main".to_string(),
    aliases: BTreeMap::from([("test".to_string(), "test".to_string())]),
}
```

- [ ] **Step 4: Add preset helpers**

In `src/config/mod.rs`, import `BTreeMap` and add:

```rust
use std::collections::BTreeMap;
```

Add helpers below `load_global_config()`:

```rust
pub fn default_branch_presets() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("test".to_string(), "Test branch".to_string()),
        ("staging".to_string(), "Staging branch".to_string()),
        ("prod".to_string(), "Prod branch".to_string()),
    ])
}

pub fn effective_branch_presets(config: &GlobalConfig) -> BTreeMap<String, String> {
    if config.branch_presets.is_empty() {
        default_branch_presets()
    } else {
        config.branch_presets.clone()
    }
}
```

- [ ] **Step 5: Run config tests and full unit tests**

Run:

```bash
cargo test config
```

Expected: PASS for config tests.

Run:

```bash
cargo test
```

Expected: Some compile failures may remain in command modules that still access `branches.test`, `branches.staging`, or `branches.prod`. Fix only the model-test-related compile failures in this task; command behavior is handled in later tasks.

## Task 2: Branch Target Resolver

**Files:**
- Create: `src/branch_target.rs`
- Modify: `src/main.rs`
- Test: `src/branch_target.rs`

- [ ] **Step 1: Write resolver tests**

Create `src/branch_target.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project};
    use std::collections::BTreeMap;

    fn project() -> Project {
        Project {
            name: "api".to_string(),
            path: "/tmp/api".to_string(),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            agents_md: None,
            branch_aliases: BTreeMap::from([(
                "test-master".to_string(),
                "test".to_string(),
            )]),
            branches: BranchConfig {
                main: "master".to_string(),
                aliases: BTreeMap::from([
                    ("test".to_string(), "test-master".to_string()),
                    ("pre".to_string(), "release-api".to_string()),
                ]),
            },
        }
    }

    #[test]
    fn resolves_input_alias_then_branch_mapping() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "test-master");

        assert_eq!(resolved.input, "test-master");
        assert_eq!(resolved.logical.as_deref(), Some("test"));
        assert_eq!(resolved.branch, "test-master");
        assert_eq!(resolved.source, ResolveSource::ProjectInputAlias);
    }

    #[test]
    fn resolves_direct_branch_mapping() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "pre");

        assert_eq!(resolved.logical.as_deref(), Some("pre"));
        assert_eq!(resolved.branch, "release-api");
        assert_eq!(resolved.source, ResolveSource::ProjectBranchMapping);
    }

    #[test]
    fn resolves_preset_key_as_real_branch_when_project_mapping_missing() {
        let presets = BTreeMap::from([("prod".to_string(), "正式环境".to_string())]);
        let resolved = resolve_target(&project(), &presets, "prod");

        assert_eq!(resolved.logical.as_deref(), Some("prod"));
        assert_eq!(resolved.branch, "prod");
        assert_eq!(resolved.source, ResolveSource::BranchPresetFallback);
    }

    #[test]
    fn falls_back_to_explicit_real_branch() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "hotfix/x");

        assert_eq!(resolved.logical, None);
        assert_eq!(resolved.branch, "hotfix/x");
        assert_eq!(resolved.source, ResolveSource::ExplicitBranch);
    }

    #[test]
    fn display_label_includes_mapping_context() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "test-master");

        assert_eq!(resolved.summary_label(), "test-master -> test -> test-master");
    }
}
```

- [ ] **Step 2: Run resolver tests and verify they fail**

Run:

```bash
cargo test branch_target
```

Expected: FAIL because resolver types and module registration do not exist.

- [ ] **Step 3: Implement resolver module**

In `src/main.rs`, add:

```rust
mod branch_target;
```

Implement `src/branch_target.rs`:

```rust
use std::collections::BTreeMap;

use crate::config::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveSource {
    ProjectInputAlias,
    ProjectBranchMapping,
    BranchPresetFallback,
    ExplicitBranch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBranch {
    pub input: String,
    pub logical: Option<String>,
    pub branch: String,
    pub source: ResolveSource,
}

impl ResolvedBranch {
    pub fn summary_label(&self) -> String {
        match (&self.logical, &self.source) {
            (Some(logical), ResolveSource::ProjectInputAlias) => {
                format!("{} -> {} -> {}", self.input, logical, self.branch)
            }
            (Some(logical), ResolveSource::ProjectBranchMapping) if logical != &self.branch => {
                format!("{} -> {}", logical, self.branch)
            }
            (Some(logical), ResolveSource::BranchPresetFallback) => {
                format!("{} -> {}", logical, self.branch)
            }
            _ => self.branch.clone(),
        }
    }
}

pub fn resolve_target(
    project: &Project,
    branch_presets: &BTreeMap<String, String>,
    target: &str,
) -> ResolvedBranch {
    if let Some(logical) = project.branch_aliases.get(target) {
        if let Some(branch) = project.branches.get(logical) {
            return ResolvedBranch {
                input: target.to_string(),
                logical: Some(logical.clone()),
                branch: branch.to_string(),
                source: ResolveSource::ProjectInputAlias,
            };
        }
    }

    if let Some(branch) = project.branches.get(target) {
        return ResolvedBranch {
            input: target.to_string(),
            logical: Some(target.to_string()),
            branch: branch.to_string(),
            source: ResolveSource::ProjectBranchMapping,
        };
    }

    if branch_presets.contains_key(target) {
        return ResolvedBranch {
            input: target.to_string(),
            logical: Some(target.to_string()),
            branch: target.to_string(),
            source: ResolveSource::BranchPresetFallback,
        };
    }

    ResolvedBranch {
        input: target.to_string(),
        logical: None,
        branch: target.to_string(),
        source: ResolveSource::ExplicitBranch,
    }
}
```

- [ ] **Step 4: Run resolver tests**

Run:

```bash
cargo test branch_target
```

Expected: PASS.

## Task 3: Git Helpers For Precheck And Rollback

**Files:**
- Modify: `src/git.rs`
- Test: `src/git.rs`

- [ ] **Step 1: Write Git helper tests**

Add tests in `src/git.rs`:

```rust
#[test]
fn test_current_branch_runtime_helper() {
    let tmp = create_test_repo();
    let branch = current_branch(tmp.path()).unwrap();

    assert!(!branch.is_empty());
}

#[test]
fn test_checkout_new_branch_from_start_point() {
    let tmp = create_test_repo();
    let dir = tmp.path();
    let main_branch = current_branch(dir).unwrap();

    checkout_new_branch(dir, "feature/test", &main_branch).unwrap();

    assert_eq!(current_branch(dir).unwrap(), "feature/test");
    assert!(branch_exists(dir, "feature/test").unwrap());
}

#[test]
fn test_resolve_existing_start_point_checked() {
    let tmp = create_test_repo();
    let dir = tmp.path();
    let main_branch = current_branch(dir).unwrap();

    let start_point = resolve_start_point_checked(dir, &main_branch).unwrap();

    assert_eq!(start_point, main_branch);
}
```

- [ ] **Step 2: Run Git helper tests and verify they fail**

Run:

```bash
cargo test git::tests::test_current_branch_runtime_helper git::tests::test_checkout_new_branch_from_start_point git::tests::test_resolve_existing_start_point_checked
```

Expected: FAIL because `checkout_new_branch` and `resolve_start_point_checked` do not exist, and `current_branch` is `cfg(test)` only.

- [ ] **Step 3: Implement helpers**

In `src/git.rs`, remove `#[cfg(test)]` from `current_branch`.

Add:

```rust
pub fn checkout_new_branch(dir: &Path, branch: &str, start_point: &str) -> Result<()> {
    run_git_checked(dir, &["checkout", "-b", branch, start_point])?;
    Ok(())
}

pub fn resolve_start_point_checked(dir: &Path, branch: &str) -> Result<String> {
    let remote_ref = format!("origin/{}", branch);
    if run_git(dir, &["rev-parse", "--verify", &remote_ref])?.success {
        return Ok(remote_ref);
    }

    if run_git(dir, &["rev-parse", "--verify", branch])?.success {
        return Ok(branch.to_string());
    }

    bail!(
        "cannot resolve start point '{}' in {}",
        branch,
        dir.display()
    );
}
```

- [ ] **Step 4: Run Git helper tests**

Run:

```bash
cargo test git::tests
```

Expected: PASS.

## Task 4: Update Existing Code For Extensible Branch Config

**Files:**
- Modify: `src/commands/add.rs`
- Modify: `src/commands/create.rs`
- Modify: `src/commands/rename.rs`
- Modify: `src/commands/workspace_edit.rs`
- Modify: `src/commands/sync.rs`
- Modify: `src/workspace.rs`
- Modify: `src/commands/tags.rs`
- Test: existing module tests

- [ ] **Step 1: Replace fixed branch field reads**

Update fixed reads:

```rust
project.branches.test.as_ref()
project.branches.staging.as_ref()
project.branches.prod.as_ref()
```

to:

```rust
project.branches.get("test")
project.branches.get("staging")
project.branches.get("prod")
```

In `src/workspace.rs`, implement:

```rust
pub fn get_env_branch<'a>(project: &'a Project, env_name: &str) -> Option<&'a str> {
    project.branches.get(env_name)
}
```

Update callers that expected `Option<&String>` to use `Option<&str>` and call `.to_string()` when ownership is needed.

- [ ] **Step 2: Update project creation**

In `src/commands/add.rs`, import `BTreeMap`:

```rust
use std::collections::BTreeMap;
```

Replace `BranchConfig` construction:

```rust
let mut branch_aliases = BTreeMap::new();
if let Some(branch) = test_branch {
    branch_aliases.insert("test".to_string(), branch);
}
if let Some(branch) = staging_branch {
    branch_aliases.insert("staging".to_string(), branch);
}
if let Some(branch) = prod_branch {
    branch_aliases.insert("prod".to_string(), branch);
}

let project = Project {
    name: name.clone(),
    path: path_str,
    group,
    order,
    tags: detect_project_tags(&resolved),
    agents_md,
    branch_aliases: BTreeMap::new(),
    branches: BranchConfig {
        main: main_branch,
        aliases: branch_aliases,
    },
};
```

- [ ] **Step 3: Update test fixtures**

Every `Project` fixture needs:

```rust
branch_aliases: BTreeMap::new(),
```

Every `BranchConfig` fixture needs:

```rust
BranchConfig {
    main: "main".to_string(),
    aliases: BTreeMap::new(),
}
```

or:

```rust
BranchConfig {
    main: "main".to_string(),
    aliases: BTreeMap::from([("test".to_string(), "develop".to_string())]),
}
```

- [ ] **Step 4: Run full compile and tests**

Run:

```bash
cargo test
```

Expected: PASS before command behavior changes begin.

## Task 5: CLI Signatures And Command Stubs

**Files:**
- Modify: `src/main.rs`
- Modify: `src/commands/git_ops.rs`
- Test: `tests/cli_test.rs`

- [ ] **Step 1: Write CLI tests for new command signatures**

Add tests:

```rust
#[test]
fn test_gpush_accepts_optional_target() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["gpush", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No workspaces").or(predicate::str::contains("暂无工作区")));
}

#[test]
fn test_gmerge_accepts_optional_target() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["gmerge", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No workspaces").or(predicate::str::contains("暂无工作区")));
}

#[test]
fn test_gswitch_command_exists() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["gswitch", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No workspaces").or(predicate::str::contains("暂无工作区")));
}

#[test]
fn test_gcreate_command_exists() {
    Command::cargo_bin("grove")
        .unwrap()
        .args(["gcreate", "feature-x"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No workspaces").or(predicate::str::contains("暂无工作区")));
}
```

- [ ] **Step 2: Run CLI tests and verify they fail**

Run:

```bash
cargo test test_gpush_accepts_optional_target test_gmerge_accepts_optional_target test_gswitch_command_exists test_gcreate_command_exists
```

Expected: FAIL because clap does not accept those signatures yet.

- [ ] **Step 3: Update `Commands` enum**

In `src/main.rs`, change:

```rust
Gmerge,
Gpush,
```

to:

```rust
Gmerge {
    /// Target preset, alias, logical branch, or real branch
    target: Option<String>,
},
Gpush {
    /// Target preset, alias, logical branch, or real branch
    target: Option<String>,
},
```

Add:

```rust
/// Switch all projects to a target branch
#[command(alias = "gsw")]
Gswitch {
    /// Target preset, alias, logical branch, or real branch
    target: String,
},

/// Create and switch to a new branch in all projects
#[command(alias = "gcr")]
Gcreate {
    /// New branch name, git-prefix is applied if configured
    name: String,
},
```

Update dispatch:

```rust
Some(Commands::Gmerge { ref target }) => commands::git_ops::gmerge(target.clone()),
Some(Commands::Gpush { ref target }) => commands::git_ops::gpush(target.clone()),
Some(Commands::Gswitch { ref target }) => commands::git_ops::gswitch(target),
Some(Commands::Gcreate { ref name }) => commands::git_ops::gcreate(name),
```

- [ ] **Step 4: Add temporary stubs**

In `src/commands/git_ops.rs`, change signatures:

Change `pub fn gpush() -> Result<()>` to `pub fn gpush(target: Option<String>) -> Result<()>`, then add this as the first line inside the function body:

```rust
let _target = target;
```

Leave the rest of the existing `gpush` body unchanged until Task 7.

Change `pub fn gmerge() -> Result<()>` to `pub fn gmerge(target: Option<String>) -> Result<()>`, then add this as the first line inside the function body:

```rust
let _target = target;
```

Leave the rest of the existing `gmerge` body unchanged until Task 10.

Add stubs:

```rust
pub fn gswitch(_target: &str) -> Result<()> {
    let _ = get_workspace_context()?;
    anyhow::bail!("gswitch is not implemented yet")
}

pub fn gcreate(_name: &str) -> Result<()> {
    let _ = get_workspace_context()?;
    anyhow::bail!("gcreate is not implemented yet")
}
```

- [ ] **Step 5: Run CLI tests**

Run:

```bash
cargo test test_gpush_accepts_optional_target test_gmerge_accepts_optional_target test_gswitch_command_exists test_gcreate_command_exists
```

Expected: PASS because commands parse and fail only after workspace lookup or explicit stub error.

## Task 6: Shared Batch Context And Precheck Helpers

**Files:**
- Modify: `src/commands/git_ops.rs`
- Test: `src/commands/git_ops.rs`

- [ ] **Step 1: Add focused structs**

In `src/commands/git_ops.rs`, add near `get_workspace_context()`:

```rust
#[derive(Debug, Clone)]
struct ProjectBranchPlan {
    wp: WorkspaceProject,
    project: Project,
    resolved: crate::branch_target::ResolvedBranch,
}

#[derive(Debug, Clone)]
struct PrecheckFailure {
    project: String,
    message: String,
}

fn report_precheck_failures(failures: &[PrecheckFailure]) -> Result<()> {
    for failure in failures {
        ui::error(&format!("{}: {}", failure.project, failure.message));
    }
    anyhow::bail!("precheck failed for {} project(s)", failures.len())
}
```

- [ ] **Step 2: Add planner helper**

Add:

```rust
fn plan_existing_branch_targets(
    projects: &[(WorkspaceProject, Project)],
    target: &str,
) -> Result<Vec<ProjectBranchPlan>> {
    let global = config::load_global_config()?;
    let presets = config::effective_branch_presets(&global);
    let mut plans = Vec::new();
    let mut failures = Vec::new();

    for (wp, project) in projects {
        let resolved = crate::branch_target::resolve_target(project, &presets, target);
        let wt_path = Path::new(&wp.worktree_path);
        match git::branch_exists(wt_path, &resolved.branch) {
            Ok(true) => plans.push(ProjectBranchPlan {
                wp: wp.clone(),
                project: project.clone(),
                resolved,
            }),
            Ok(false) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: format!("branch '{}' does not exist", resolved.branch),
            }),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }
    }

    if failures.is_empty() {
        Ok(plans)
    } else {
        report_precheck_failures(&failures)
    }
}
```

- [ ] **Step 3: Add clean-worktree precheck**

Add:

```rust
fn precheck_clean_worktrees(projects: &[(WorkspaceProject, Project)]) -> Result<()> {
    let mut failures = Vec::new();

    for (wp, _project) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::is_clean(wt_path) {
            Ok(true) => {}
            Ok(false) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: "working tree has uncommitted changes; commit or stash before switching".to_string(),
            }),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        report_precheck_failures(&failures)
    }
}
```

- [ ] **Step 4: Run compile**

Run:

```bash
cargo test --no-run
```

Expected: PASS compile, even if helper functions are not all used yet.

## Task 7: Implement `gpush [target]`

**Files:**
- Modify: `src/commands/git_ops.rs`
- Modify: `src/git.rs` if needed
- Test: `src/commands/git_ops.rs`

- [ ] **Step 1: Replace `gpush` body**

Use the workspace record when no target is passed, resolve per project, precheck existence, then push each resolved branch.

```rust
pub fn gpush(target: Option<String>) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let target_input = target.unwrap_or_else(|| ws.branch.clone());
    let plans = plan_existing_branch_targets(&projects, &target_input)?;

    println!("gpush target: {}", target_input);
    println!();

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for plan in &plans {
        let wt_path = Path::new(&plan.wp.worktree_path);
        match git::push_upstream(wt_path, &plan.resolved.branch) {
            Ok(()) => {
                ui::success(&format!(
                    "{}: pushed {} -> origin/{} (target: {})",
                    plan.wp.name,
                    plan.resolved.branch,
                    plan.resolved.branch,
                    target_input
                ));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to push {} -> origin/{} (target: {}): {}",
                    plan.wp.name,
                    plan.resolved.branch,
                    plan.resolved.branch,
                    target_input,
                    e
                ));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}
```

- [ ] **Step 2: Add a unit test for output formatting helper if extracted**

If extracting a helper such as `format_push_success`, test:

```rust
#[test]
fn test_format_push_success_includes_target_and_remote() {
    assert_eq!(
        format_push_success("api", "test-master", "test"),
        "api: pushed test-master -> origin/test-master (target: test)"
    );
}
```

- [ ] **Step 3: Run relevant tests**

Run:

```bash
cargo test gpush branch_target
```

Expected: PASS.

## Task 8: Implement `gswitch <target>`

**Files:**
- Modify: `src/commands/git_ops.rs`
- Test: `src/commands/git_ops.rs`

- [ ] **Step 1: Add workspace update helper**

Add:

```rust
fn update_workspace_branch(workspace_name: &str, branch: &str) -> Result<()> {
    let mut workspaces_file = config::load_workspaces()?;
    let ws = workspaces_file
        .workspaces
        .iter_mut()
        .find(|ws| ws.name == workspace_name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;
    ws.branch = branch.to_string();
    config::save_workspaces(&workspaces_file)?;
    Ok(())
}
```

- [ ] **Step 2: Implement `gswitch`**

```rust
pub fn gswitch(target: &str) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    precheck_clean_worktrees(&projects)?;
    let plans = plan_existing_branch_targets(&projects, target)?;

    let mut switched: Vec<(String, String, String)> = Vec::new();

    for plan in &plans {
        let wt_path = Path::new(&plan.wp.worktree_path);
        let original = git::current_branch(wt_path)?;
        match git::checkout(wt_path, &plan.resolved.branch) {
            Ok(()) => {
                ui::success(&format!(
                    "{}: switched {} -> {} (target: {})",
                    plan.wp.name, original, plan.resolved.branch, target
                ));
                switched.push((plan.wp.name.clone(), plan.wp.worktree_path.clone(), original));
            }
            Err(e) => {
                ui::error(&format!("{}: {}", plan.wp.name, e));
                rollback_checkouts(&switched);
                anyhow::bail!("gswitch failed; rolled back changed projects");
            }
        }
    }

    update_workspace_branch(&ws.name, target)?;
    ui::success(&format!("Workspace '{}' branch set to '{}'", ws.name, target));
    Ok(())
}
```

Add rollback helper:

```rust
fn rollback_checkouts(switched: &[(String, String, String)]) {
    for (project_name, worktree_path, original_branch) in switched.iter().rev() {
        let path = Path::new(worktree_path);
        if let Err(e) = git::checkout(path, original_branch) {
            ui::error(&format!(
                "{}: rollback checkout to '{}' failed: {}",
                project_name, original_branch, e
            ));
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test gswitch git::tests
```

Expected: PASS.

## Task 9: Implement `gcreate <name>`

**Files:**
- Modify: `src/commands/git_ops.rs`
- Test: `src/commands/git_ops.rs`

- [ ] **Step 1: Add branch-name helper**

Add:

```rust
fn apply_git_prefix(input: &str, global: &config::GlobalConfig) -> String {
    let git_prefix = config::expand_date_templates(&global.git_prefix);
    if git_prefix.is_empty() || input.starts_with(&git_prefix) {
        input.to_string()
    } else {
        format!("{}{}", git_prefix, input)
    }
}
```

Add test if placed in testable module:

```rust
#[test]
fn test_apply_git_prefix_keeps_existing_prefix() {
    let mut global = config::GlobalConfig::default();
    global.git_prefix = "feature/".to_string();

    assert_eq!(apply_git_prefix("feature/login", &global), "feature/login");
    assert_eq!(apply_git_prefix("login", &global), "feature/login");
}
```

- [ ] **Step 2: Add new-branch precheck helper**

```rust
fn precheck_new_branch_absent(
    projects: &[(WorkspaceProject, Project)],
    branch: &str,
) -> Result<()> {
    let mut failures = Vec::new();

    for (wp, _project) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::branch_exists(wt_path, branch) {
            Ok(false) => {}
            Ok(true) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: format!("new branch '{}' already exists", branch),
            }),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        report_precheck_failures(&failures)
    }
}
```

- [ ] **Step 3: Implement `gcreate`**

```rust
pub fn gcreate(name: &str) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let global = config::load_global_config()?;
    let new_branch = apply_git_prefix(name, &global);

    precheck_clean_worktrees(&projects)?;

    for (wp, _project) in &projects {
        git::fetch(Path::new(&wp.worktree_path))
            .map_err(|e| anyhow::anyhow!("{}: fetch failed: {}", wp.name, e))?;
    }

    precheck_new_branch_absent(&projects, &new_branch)?;

    let mut start_points = Vec::new();
    let mut failures = Vec::new();
    for (wp, project) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::resolve_start_point_checked(wt_path, &project.branches.main) {
            Ok(start_point) => start_points.push((wp.clone(), start_point)),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }
    }
    if !failures.is_empty() {
        return report_precheck_failures(&failures);
    }

    let mut created: Vec<(String, String, String)> = Vec::new();

    for ((wp, _project), (_, start_point)) in projects.iter().zip(start_points.iter()) {
        let wt_path = Path::new(&wp.worktree_path);
        let original = git::current_branch(wt_path)?;
        match git::checkout_new_branch(wt_path, &new_branch, start_point) {
            Ok(()) => {
                ui::success(&format!(
                    "{}: created {} from {}",
                    wp.name, new_branch, start_point
                ));
                created.push((wp.name.clone(), wp.worktree_path.clone(), original));
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                rollback_created_branches(&created, &new_branch);
                anyhow::bail!("gcreate failed; rolled back changed projects");
            }
        }
    }

    update_workspace_branch(&ws.name, &new_branch)?;
    ui::success(&format!("Workspace '{}' branch set to '{}'", ws.name, new_branch));
    Ok(())
}
```

Add rollback helper:

```rust
fn rollback_created_branches(created: &[(String, String, String)], new_branch: &str) {
    for (project_name, worktree_path, original_branch) in created.iter().rev() {
        let path = Path::new(worktree_path);
        if let Err(e) = git::checkout(path, original_branch) {
            ui::error(&format!(
                "{}: rollback checkout to '{}' failed: {}",
                project_name, original_branch, e
            ));
            continue;
        }
        if let Err(e) = git::branch_delete(path, new_branch) {
            ui::error(&format!(
                "{}: rollback delete branch '{}' failed: {}",
                project_name, new_branch, e
            ));
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test gcreate git::tests
```

Expected: PASS.

## Task 10: Implement Target-Aware `gmerge`

**Files:**
- Modify: `src/commands/git_ops.rs`
- Modify: `src/workspace.rs`
- Test: `src/commands/git_ops.rs`

- [ ] **Step 1: Replace no-arg menu source**

In `gmerge`, when `target` is `None`, use:

```rust
let global = config::load_global_config()?;
let presets = config::effective_branch_presets(&global);
let options: Vec<String> = presets
    .iter()
    .map(|(name, description)| format!("{:<8} {}", name, description))
    .collect();
let keys: Vec<String> = presets.keys().cloned().collect();
let idx = ui::select(&t("merge_to_env"), &options)?;
let target_input = keys[idx].clone();
```

When `target` is `Some`, use that value directly.

- [ ] **Step 2: Resolve source and target branches**

Implement helper:

```rust
fn plan_merge_targets(
    projects: &[(WorkspaceProject, Project)],
    source_input: &str,
    target_input: &str,
) -> Result<Vec<(WorkspaceProject, Project, crate::branch_target::ResolvedBranch, crate::branch_target::ResolvedBranch)>> {
    let global = config::load_global_config()?;
    let presets = config::effective_branch_presets(&global);
    let mut plans = Vec::new();
    let mut failures = Vec::new();

    for (wp, project) in projects {
        let source = crate::branch_target::resolve_target(project, &presets, source_input);
        let target = crate::branch_target::resolve_target(project, &presets, target_input);
        let wt_path = Path::new(&wp.worktree_path);

        for (label, branch) in [("source", &source.branch), ("target", &target.branch)] {
            match git::branch_exists(wt_path, branch) {
                Ok(true) => {}
                Ok(false) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: format!("{} branch '{}' does not exist", label, branch),
                }),
                Err(e) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: e.to_string(),
                }),
            }
        }

        plans.push((wp.clone(), project.clone(), source, target));
    }

    if failures.is_empty() {
        Ok(plans)
    } else {
        report_precheck_failures(&failures)
    }
}
```

- [ ] **Step 3: Replace `gmerge` body**

The body should:

```rust
pub fn gmerge(target: Option<String>) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    precheck_clean_worktrees(&projects)?;

    let target_input = match target {
        Some(target) => target,
        None => select_branch_preset()?,
    };

    let plans = plan_merge_targets(&projects, &ws.branch, &target_input)?;

    println!("gmerge target: {}", target_input);
    println!();

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, _project, source, target) in &plans {
        let wt_path = Path::new(&wp.worktree_path);
        let original = git::current_branch(wt_path)?;

        let result = (|| -> Result<()> {
            git::fetch(wt_path)?;
            git::checkout(wt_path, &target.branch)?;
            git::pull_ff_only(wt_path, "origin", &target.branch)?;
            git::merge(wt_path, &source.branch)?;
            git::checkout(wt_path, &original)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                ui::success(&format!(
                    "{}: merged {} -> {} (target: {})",
                    wp.name, source.branch, target.branch, target_input
                ));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                let _ = git::checkout(wt_path, &original);
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}
```

Extract `select_branch_preset()` to keep `gmerge` readable.

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test gmerge branch_target
```

Expected: PASS.

## Task 11: i18n And User-Facing Messages

**Files:**
- Modify: `src/i18n.rs`
- Modify: `src/commands/git_ops.rs`

- [ ] **Step 1: Add translation keys**

In English map:

```rust
m.insert("branch_target", "Branch target");
m.insert("precheck_failed", "Precheck failed for {} project(s)");
m.insert("switch_success", "{}: switched {} -> {} (target: {})");
m.insert("create_success", "{}: created {} from {}");
m.insert("push_success", "{}: pushed {} -> origin/{} (target: {})");
m.insert("merge_success", "{}: merged {} -> {} (target: {})");
```

In Chinese map:

```rust
m.insert("branch_target", "分支目标");
m.insert("precheck_failed", "{} 个项目预检查失败");
m.insert("switch_success", "{}: 已从 {} 切换到 {} (target: {})");
m.insert("create_success", "{}: 已从 {} 创建 {}");
m.insert("push_success", "{}: 已推送 {} -> origin/{} (target: {})");
m.insert("merge_success", "{}: 已合并 {} -> {} (target: {})");
```

- [ ] **Step 2: Replace hardcoded success strings where practical**

Use translations for final user-facing messages. Keep low-level Git errors as raw error strings.

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test i18n git_ops
```

Expected: PASS.

## Task 12: README Updates

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update feature list**

Add bullets in Chinese and English sections:

```markdown
- **分支预设与别名** — 全局配置常用分支选项，每个项目可映射到不同真实分支
- **批量分支切换/创建** — `gswitch` 统一切换，`gcreate` 基于最新主分支事务式创建
```

English:

```markdown
- **Branch presets and aliases** — Configure shared branch choices while each project maps them to its own real branch
- **Batch branch switch/create** — `gswitch` switches all projects, `gcreate` creates from latest main with rollback
```

- [ ] **Step 2: Update config examples**

Add to `config.toml` example:

```toml
[branch_presets]
test = "测试环境"
staging = "预发环境"
prod = "正式环境"
master = "主分支"
```

Update `projects.toml` example:

```toml
[projects.branches]
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"

[projects.branch_aliases]
test-master = "test"
```

- [ ] **Step 3: Update command reference**

Add:

```markdown
| `grove gswitch <target>` | `grove gsw <target>` | 所有项目切换到目标分支 |
| `grove gcreate <name>` | `grove gcr <name>` | 所有项目基于最新主分支创建并切换到新分支 |
| `grove gpush [target]` | `grove gp [target]` | 推送当前或指定分支 |
| `grove gmerge [target]` | `grove gm [target]` | 合并工作分支到交互选择或指定目标分支 |
```

- [ ] **Step 4: Add success output examples**

Add:

```markdown
`gpush test` 会显示每个项目实际推送的分支：

```text
gpush target: test

api: pushed test-master -> origin/test-master (target: test)
web: pushed develop -> origin/develop (target: test)

Result: 2 succeeded, 0 failed
```
```

Add matching `gmerge` example from the spec.

- [ ] **Step 5: Run doc-adjacent checks**

Run:

```bash
cargo test --test cli_test
```

Expected: PASS.

## Task 13: Full Verification

**Files:**
- All modified files

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, then rerun `cargo fmt --check`.

- [ ] **Step 2: Lint**

Run:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Test**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Review diff**

Run:

```bash
git status --short
git diff -- README.md src tests docs/superpowers
```

Expected: only intended files are changed. `src/commands/gowork.rs` may already contain pre-existing user changes; do not revert it unless the user explicitly asks.

- [ ] **Step 5: Stop for user review**

Report verification results and ask the user whether they want commits or further changes. Do not create commits unless the user explicitly requests them.
