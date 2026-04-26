use anyhow::{Context, Result};
use console::Style;
use std::path::Path;
use std::process::Command;

use crate::config::{self, Project, Workspace, WorkspaceProject};
use crate::git;
use crate::i18n::t;
use crate::ui;
use crate::workspace;

/// Resolved workspace context: the workspace plus matched (WorkspaceProject, Project) pairs.
fn get_workspace_context() -> Result<(Workspace, Vec<(WorkspaceProject, Project)>)> {
    let ws = workspace::get_or_select_workspace()?;
    let projects_file = config::load_projects()?;
    let matched = match_workspace_projects(&ws, &projects_file)?;

    Ok((ws, matched))
}

fn match_workspace_projects(
    ws: &Workspace,
    projects_file: &config::ProjectsFile,
) -> Result<Vec<(WorkspaceProject, Project)>> {
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

    Ok(matched)
}

#[derive(Debug, Clone)]
struct ProjectBranchPlan {
    wp: WorkspaceProject,
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
    let project_count = failures
        .iter()
        .map(|failure| failure.project.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    anyhow::bail!(t("precheck_failed").replace("{}", &project_count.to_string()))
}

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
        report_precheck_failures(&failures)?;
        unreachable!("report_precheck_failures always returns an error")
    }
}

fn select_branch_preset() -> Result<String> {
    let global = config::load_global_config()?;
    let entries = config::effective_branch_preset_entries(&global);
    let options: Vec<String> = entries
        .iter()
        .map(|(name, description)| format!("{:<8} {}", name, description))
        .collect();
    let keys: Vec<String> = entries.into_iter().map(|(name, _)| name).collect();
    let idx = ui::select(&t("merge_to_env"), &options)?;
    Ok(keys[idx].clone())
}

fn plan_merge_targets(
    projects: &[(WorkspaceProject, Project)],
    source_input: &str,
    target_input: &str,
) -> Result<
    Vec<(
        WorkspaceProject,
        crate::branch_target::ResolvedBranch,
        crate::branch_target::ResolvedBranch,
    )>,
> {
    let global = config::load_global_config()?;
    let presets = config::effective_branch_presets(&global);
    let mut plans = Vec::new();
    let mut failures = Vec::new();

    for (wp, project) in projects {
        let source = crate::branch_target::resolve_target(project, &presets, source_input);
        let target = crate::branch_target::resolve_target(project, &presets, target_input);
        let wt_path = Path::new(&wp.worktree_path);

        match git::branch_exists(wt_path, &source.branch) {
            Ok(true) => {}
            Ok(false) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: format!("source branch '{}' does not exist", source.branch),
            }),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }

        match git::branch_exists(wt_path, &target.branch) {
            Ok(true) => {}
            Ok(false) => match git::remote_branch_exists(wt_path, &target.branch) {
                Ok(true) => {}
                Ok(false) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: format!("target branch '{}' does not exist", target.branch),
                }),
                Err(e) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: e.to_string(),
                }),
            },
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }

        plans.push((wp.clone(), source, target));
    }

    if failures.is_empty() {
        Ok(plans)
    } else {
        report_precheck_failures(&failures)?;
        unreachable!("report_precheck_failures always returns an error")
    }
}

fn prefetch_projects(projects: &[(WorkspaceProject, Project)]) -> Result<()> {
    let mut failures = Vec::new();

    for (wp, _project) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        if let Err(e) = git::fetch(wt_path) {
            failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: format!("fetch failed: {}", e),
            });
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        report_precheck_failures(&failures)
    }
}

fn precheck_clean_worktrees(projects: &[(WorkspaceProject, Project)]) -> Result<()> {
    let mut failures = Vec::new();

    for (wp, _project) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::is_clean(wt_path) {
            Ok(true) => {}
            Ok(false) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: "working tree has uncommitted changes; commit or stash before switching"
                    .to_string(),
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

fn apply_git_prefix(input: &str, global: &config::GlobalConfig) -> String {
    let git_prefix = config::expand_date_templates(&global.git_prefix);
    if git_prefix.is_empty() || input.starts_with(&git_prefix) {
        input.to_string()
    } else {
        format!("{}{}", git_prefix, input)
    }
}

fn precheck_new_branch_absent(
    projects: &[(WorkspaceProject, Project)],
    branch: &str,
) -> Result<()> {
    let mut failures = Vec::new();

    for (wp, _project) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::branch_exists(wt_path, branch) {
            Ok(false) => match git::remote_branch_exists(wt_path, branch) {
                Ok(false) => {}
                Ok(true) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: format!("remote branch 'origin/{}' already exists", branch),
                }),
                Err(e) => failures.push(PrecheckFailure {
                    project: wp.name.clone(),
                    message: e.to_string(),
                }),
            },
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

#[derive(Debug, Clone)]
struct CheckoutRollback {
    project_name: String,
    worktree_path: String,
    original_branch: String,
}

fn update_workspace_branch(workspace_name: &str, branch: &str) -> Result<()> {
    let mut workspaces_file = config::load_workspaces()?;
    let ws = workspaces_file
        .workspaces
        .iter_mut()
        .find(|ws| ws.name == workspace_name)
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;
    ws.branch = branch.to_string();
    config::save_workspaces(&workspaces_file).with_context(|| {
        format!(
            "Failed to save workspace '{}' branch '{}'",
            workspace_name, branch
        )
    })?;
    Ok(())
}

fn rollback_checkouts(switched: &[CheckoutRollback]) -> Vec<String> {
    let mut failures = Vec::new();
    for rollback in switched.iter().rev() {
        let path = Path::new(&rollback.worktree_path);
        if let Err(e) = git::checkout(path, &rollback.original_branch) {
            ui::error(&format!(
                "{}: rollback checkout to '{}' failed: {}",
                rollback.project_name, rollback.original_branch, e
            ));
            failures.push(rollback.project_name.clone());
        }
    }
    failures
}

fn rollback_created_branches(created: &[CheckoutRollback], new_branch: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for rollback in created.iter().rev() {
        let path = Path::new(&rollback.worktree_path);
        if let Err(e) = git::checkout(path, &rollback.original_branch) {
            ui::error(&format!(
                "{}: rollback checkout to '{}' failed: {}",
                rollback.project_name, rollback.original_branch, e
            ));
            failures.push(rollback.project_name.clone());
            continue;
        }

        if let Err(e) = git::branch_delete(path, new_branch) {
            ui::error(&format!(
                "{}: rollback delete branch '{}' failed: {}",
                rollback.project_name, new_branch, e
            ));
            failures.push(rollback.project_name.clone());
        }
    }
    failures
}

pub fn gstatus() -> Result<()> {
    let (_ws, projects) = get_workspace_context()?;
    let bold = Style::new().bold();

    for (wp, _proj) in &projects {
        println!("{}", bold.apply_to(&wp.name));
        let wt_path = Path::new(&wp.worktree_path);
        match git::status_short(wt_path) {
            Ok(output) => {
                if output.is_empty() {
                    println!("  Working tree clean");
                } else {
                    for line in output.lines() {
                        println!("  {}", line);
                    }
                }
            }
            Err(e) => {
                ui::error(&format!("  {}: {}", wp.name, e));
            }
        }
    }

    Ok(())
}

pub fn gadd() -> Result<()> {
    let (_ws, projects) = get_workspace_context()?;
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, _proj) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::add_all(wt_path) {
            Ok(()) => {
                ui::success(&format!("{}: staged all changes", wp.name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gcommit() -> Result<()> {
    let (_ws, projects) = get_workspace_context()?;
    let global = config::load_global_config()?;
    let tool = normalize_commit_message_tool(&global.commit_message_tool);
    let message_default = if tool == "manual" {
        String::new()
    } else {
        match generate_commit_message(tool, &projects) {
            Ok(message) => message,
            Err(e) => {
                ui::warn(&format!("AI commit message generation failed: {}", e));
                String::new()
            }
        }
    };

    let message = ui::input(&t("commit_message"), &message_default)?;
    if message.is_empty() {
        anyhow::bail!("Commit message cannot be empty");
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, _proj) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::has_staged_changes(wt_path) {
            Ok(has_staged_changes) => {
                if !has_staged_changes {
                    ui::info(&t("nothing_to_commit").replace("{}", &wp.name));
                    continue;
                }
                match git::commit(wt_path, &message) {
                    Ok(()) => {
                        ui::success(&format!("{}: committed", wp.name));
                        succeeded += 1;
                    }
                    Err(e) => {
                        ui::error(&format!("{}: {}", wp.name, e));
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

fn format_push_success(project: &str, branch: &str, target: &str) -> String {
    t("push_success")
        .replacen("{}", project, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", target, 1)
}

fn format_gswitch_success(project: &str, original: &str, branch: &str, target: &str) -> String {
    t("switch_success")
        .replacen("{}", project, 1)
        .replacen("{}", original, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", target, 1)
}

fn format_gmerge_success(project: &str, source: &str, target: &str, target_input: &str) -> String {
    t("merge_success")
        .replacen("{}", project, 1)
        .replacen("{}", source, 1)
        .replacen("{}", target, 1)
        .replacen("{}", target_input, 1)
}

fn format_gcreate_success(project: &str, new_branch: &str, start_point: &str) -> String {
    t("create_success")
        .replacen("{}", project, 1)
        .replacen("{}", new_branch, 1)
        .replacen("{}", start_point, 1)
}

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
        let branch = &plan.resolved.branch;
        match git::push_upstream(wt_path, branch) {
            Ok(()) => {
                ui::success(&format_push_success(&plan.wp.name, branch, &target_input));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to push {} -> origin/{} (target: {}): {}",
                    plan.wp.name, branch, branch, target_input, e
                ));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gpull() -> Result<()> {
    let (_ws, projects) = get_workspace_context()?;
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, _proj) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::pull(wt_path) {
            Ok(()) => {
                ui::success(&format!("{}: pulled", wp.name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gmerge(target: Option<String>) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    precheck_clean_worktrees(&projects)?;
    prefetch_projects(&projects)?;
    let target_input = match target {
        Some(target) => target,
        None => select_branch_preset()?,
    };
    let plans = plan_merge_targets(&projects, &ws.branch, &target_input)?;

    println!("gmerge target: {}", target_input);
    println!();

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, source, target) in &plans {
        let wt_path = Path::new(&wp.worktree_path);
        let original = match git::current_branch(wt_path) {
            Ok(original) => original,
            Err(e) => {
                ui::error(&format!("{}: failed to get current branch: {}", wp.name, e));
                failed += 1;
                continue;
            }
        };

        let result = (|| -> Result<()> {
            git::checkout(wt_path, &target.branch)?;
            git::pull_ff_only(wt_path, "origin", &target.branch)?;
            git::merge(wt_path, &source.branch)?;
            git::checkout(wt_path, &original)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                ui::success(&format_gmerge_success(
                    &wp.name,
                    &source.branch,
                    &target.branch,
                    &target_input,
                ));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to merge {} -> {} (target: {}): {}",
                    wp.name, source.branch, target.branch, target_input, e
                ));
                if let Err(checkout_err) = git::checkout(wt_path, &original) {
                    ui::error(&format!(
                        "{}: checkout back to '{}' failed: {}",
                        wp.name, original, checkout_err
                    ));
                }
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
    Ok(())
}

pub fn gswitch(target: &str) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    precheck_clean_worktrees(&projects)?;
    let plans = plan_existing_branch_targets(&projects, target)?;

    let mut originals = Vec::new();
    for plan in &plans {
        let wt_path = Path::new(&plan.wp.worktree_path);
        match git::current_branch(wt_path) {
            Ok(original) => originals.push(CheckoutRollback {
                project_name: plan.wp.name.clone(),
                worktree_path: plan.wp.worktree_path.clone(),
                original_branch: original,
            }),
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to get current branch: {}",
                    plan.wp.name, e
                ));
                anyhow::bail!("gswitch failed before checkout; no projects were changed");
            }
        }
    }

    let mut switched: Vec<CheckoutRollback> = Vec::new();

    for (plan, original) in plans.iter().zip(originals) {
        let wt_path = Path::new(&plan.wp.worktree_path);
        match git::checkout(wt_path, &plan.resolved.branch) {
            Ok(()) => {
                ui::success(&format_gswitch_success(
                    &plan.wp.name,
                    &original.original_branch,
                    &plan.resolved.branch,
                    target,
                ));
                switched.push(original);
            }
            Err(e) => {
                ui::error(&format!("{}: {}", plan.wp.name, e));
                let rollback_failures = rollback_checkouts(&switched);
                if rollback_failures.is_empty() {
                    anyhow::bail!("gswitch failed; rolled back changed projects");
                }
                anyhow::bail!(
                    "gswitch failed; rollback incomplete for: {}",
                    rollback_failures.join(", ")
                );
            }
        }
    }

    if let Err(e) = update_workspace_branch(&ws.name, target) {
        ui::error(&format!(
            "workspace branch update failed after checkout: {}",
            e
        ));
        let rollback_failures = rollback_checkouts(&switched);
        if rollback_failures.is_empty() {
            anyhow::bail!(
                "workspace branch update failed after checkout; rolled back changed projects"
            );
        }
        anyhow::bail!(
            "workspace branch update failed after checkout; rollback incomplete for: {}",
            rollback_failures.join(", ")
        );
    }
    ui::success(&format!(
        "Workspace '{}' branch set to '{}'",
        ws.name, target
    ));
    Ok(())
}

pub fn gcreate(name: &str) -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let global = config::load_global_config()?;
    let new_branch = apply_git_prefix(name, &global);

    precheck_clean_worktrees(&projects)?;

    for (wp, _project) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        git::fetch(wt_path).with_context(|| format!("{}: fetch failed", wp.name))?;
    }

    precheck_new_branch_absent(&projects, &new_branch)?;

    let mut start_points = Vec::new();
    let mut failures = Vec::new();
    for (wp, project) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::resolve_remote_start_point_checked(wt_path, &project.branches.main) {
            Ok(start_point) => start_points.push(start_point),
            Err(e) => failures.push(PrecheckFailure {
                project: wp.name.clone(),
                message: e.to_string(),
            }),
        }
    }
    if !failures.is_empty() {
        report_precheck_failures(&failures)?;
    }

    let mut originals = Vec::new();
    for (wp, _project) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::current_branch(wt_path) {
            Ok(original) => originals.push(CheckoutRollback {
                project_name: wp.name.clone(),
                worktree_path: wp.worktree_path.clone(),
                original_branch: original,
            }),
            Err(e) => {
                ui::error(&format!("{}: failed to get current branch: {}", wp.name, e));
                anyhow::bail!("gcreate failed before branch creation; no projects were changed");
            }
        }
    }

    let mut created = Vec::new();
    for (((wp, _project), start_point), original) in
        projects.iter().zip(start_points.iter()).zip(originals)
    {
        let wt_path = Path::new(&wp.worktree_path);
        match git::checkout_new_branch(wt_path, &new_branch, start_point) {
            Ok(()) => {
                ui::success(&format_gcreate_success(&wp.name, &new_branch, start_point));
                created.push(original);
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                let rollback_failures = rollback_created_branches(&created, &new_branch);
                if rollback_failures.is_empty() {
                    anyhow::bail!("gcreate failed; rolled back created branches");
                }
                anyhow::bail!(
                    "gcreate failed; rollback incomplete for: {}",
                    rollback_failures.join(", ")
                );
            }
        }
    }

    if let Err(e) = update_workspace_branch(&ws.name, &new_branch) {
        ui::error(&format!(
            "workspace branch update failed after branch creation: {}",
            e
        ));
        let rollback_failures = rollback_created_branches(&created, &new_branch);
        if rollback_failures.is_empty() {
            anyhow::bail!(
                "workspace branch update failed after branch creation; rolled back created branches"
            );
        }
        anyhow::bail!(
            "workspace branch update failed after branch creation; rollback incomplete for: {}",
            rollback_failures.join(", ")
        );
    }

    ui::success(&format!(
        "Workspace '{}' branch set to '{}'",
        ws.name, new_branch
    ));
    Ok(())
}

pub fn normalize_commit_message_tool(tool: &str) -> &'static str {
    match tool.trim().to_ascii_lowercase().as_str() {
        "codex" => "codex",
        "claude" | "claude-code" | "claude_code" => "claude",
        "copilot" | "copilot-cli" | "copilot_cli" => "copilot",
        "cursor" | "cursor-cli" | "cursor_cli" | "cursorcli" => "cursor",
        _ => "manual",
    }
}

fn generate_commit_message(tool: &str, projects: &[(WorkspaceProject, Project)]) -> Result<String> {
    let mut sections = Vec::new();
    for (wp, _proj) in projects {
        let wt_path = Path::new(&wp.worktree_path);
        if git::has_staged_changes(wt_path)? {
            let summary = git::staged_diff_summary(wt_path)?;
            if !summary.is_empty() {
                sections.push(format!("Project: {}\n{}", wp.name, summary));
            }
        }
    }

    if sections.is_empty() {
        anyhow::bail!("no staged changes found");
    }

    let prompt = format!(
        "Generate one concise git commit message for these staged changes. \
Use Conventional Commits style when possible. Output only the commit message, no explanation.\n\n{}",
        sections.join("\n\n")
    );

    run_commit_message_tool(tool, &prompt)
}

fn run_commit_message_tool(tool: &str, prompt: &str) -> Result<String> {
    let output = match tool {
        "codex" => Command::new("codex").args(["exec", prompt]).output(),
        "claude" => Command::new("claude").args(["-p", prompt]).output(),
        "copilot" => Command::new("gh")
            .args(["copilot", "suggest", "-t", "git", prompt])
            .output(),
        "cursor" => Command::new("cursor-agent").args(["-p", prompt]).output(),
        _ => anyhow::bail!("unsupported commit message tool: {}", tool),
    }?;

    if !output.status.success() {
        anyhow::bail!(
            "{} exited with status {}: {}",
            tool,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let message = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    if message.is_empty() {
        anyhow::bail!("{} returned an empty commit message", tool);
    }

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        git::run_git_checked(dir, &["init"]).unwrap();
        git::run_git_checked(dir, &["config", "user.email", "test@test.com"]).unwrap();
        git::run_git_checked(dir, &["config", "user.name", "Test"]).unwrap();
        std::fs::write(dir.join("README.md"), "# Test").unwrap();
        git::run_git_checked(dir, &["add", "."]).unwrap();
        git::run_git_checked(dir, &["commit", "-m", "initial"]).unwrap();
        tmp
    }

    fn test_project(name: &str) -> Project {
        Project {
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            agents_md: None,
            branch_aliases: BTreeMap::new(),
            branches: config::BranchConfig {
                main: "main".to_string(),
                aliases: BTreeMap::new(),
            },
        }
    }

    fn test_workspace(project_names: &[&str]) -> Workspace {
        Workspace {
            name: "foo".to_string(),
            branch: "feature".to_string(),
            created_at: "2026-04-26T00:00:00Z".to_string(),
            projects: project_names
                .iter()
                .map(|name| WorkspaceProject {
                    name: (*name).to_string(),
                    worktree_path: format!("/tmp/{}", name),
                })
                .collect(),
        }
    }

    #[test]
    fn test_match_workspace_projects_errors_for_missing_projects() {
        let ws = test_workspace(&["api", "web"]);
        let projects_file = config::ProjectsFile {
            groups: Vec::new(),
            projects: vec![test_project("worker")],
        };

        let err = match_workspace_projects(&ws, &projects_file).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Workspace 'foo' references missing project(s): api, web"
        );
    }

    #[test]
    fn test_match_workspace_projects_returns_all_matches() {
        let ws = test_workspace(&["api", "web"]);
        let projects_file = config::ProjectsFile {
            groups: Vec::new(),
            projects: vec![test_project("web"), test_project("api")],
        };

        let matched = match_workspace_projects(&ws, &projects_file).unwrap();

        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].0.name, "api");
        assert_eq!(matched[0].1.name, "api");
        assert_eq!(matched[1].0.name, "web");
        assert_eq!(matched[1].1.name, "web");
    }

    #[test]
    fn test_report_precheck_failures_reports_project_count() {
        let failures = vec![
            PrecheckFailure {
                project: "api".to_string(),
                message: "branch 'test' does not exist".to_string(),
            },
            PrecheckFailure {
                project: "web".to_string(),
                message: "working tree has uncommitted changes".to_string(),
            },
        ];

        let err = report_precheck_failures(&failures).unwrap_err();

        assert_eq!(err.to_string(), "Precheck failed for 2 project(s)");
    }

    #[test]
    fn test_format_push_success_includes_target_and_remote() {
        assert_eq!(
            format_push_success("api", "test-master", "test"),
            "api: pushed test-master -> origin/test-master (target: test)"
        );
    }

    #[test]
    fn test_format_gswitch_success_includes_original_resolved_and_target() {
        assert_eq!(
            format_gswitch_success("api", "main", "feature-api", "feature"),
            "api: switched main -> feature-api (target: feature)"
        );
    }

    #[test]
    fn test_apply_git_prefix_keeps_existing_prefix() {
        let global = config::GlobalConfig {
            git_prefix: "feature/".to_string(),
            ..Default::default()
        };

        assert_eq!(apply_git_prefix("feature/login", &global), "feature/login");
        assert_eq!(apply_git_prefix("login", &global), "feature/login");
    }

    #[test]
    fn test_normalize_commit_message_tool_defaults_to_manual() {
        assert_eq!(normalize_commit_message_tool(""), "manual");
        assert_eq!(normalize_commit_message_tool("unknown"), "manual");
    }

    #[test]
    fn test_normalize_commit_message_tool_accepts_ai_tools() {
        assert_eq!(normalize_commit_message_tool("codex"), "codex");
        assert_eq!(normalize_commit_message_tool("Claude"), "claude");
        assert_eq!(normalize_commit_message_tool("copilot-cli"), "copilot");
        assert_eq!(normalize_commit_message_tool("cursorcli"), "cursor");
    }

    #[test]
    fn test_plan_merge_targets_resolves_source_and_target_per_project() {
        let repo = create_test_repo();
        git::run_git_checked(repo.path(), &["branch", "feature-api"]).unwrap();
        git::run_git_checked(repo.path(), &["branch", "release-api"]).unwrap();

        let mut project = test_project("api");
        project
            .branches
            .aliases
            .insert("feature".to_string(), "feature-api".to_string());
        project
            .branches
            .aliases
            .insert("prod".to_string(), "release-api".to_string());
        let wp = WorkspaceProject {
            name: "api".to_string(),
            worktree_path: repo.path().display().to_string(),
        };

        let plans = plan_merge_targets(&[(wp, project)], "feature", "prod").expect("merge plan");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].1.branch, "feature-api");
        assert_eq!(plans[0].2.branch, "release-api");
    }

    #[test]
    fn test_plan_merge_targets_accepts_remote_only_target_branch() {
        let repo = create_test_repo();
        git::run_git_checked(repo.path(), &["branch", "feature-api"]).unwrap();
        git::run_git_checked(
            repo.path(),
            &["update-ref", "refs/remotes/origin/release-api", "HEAD"],
        )
        .unwrap();

        let mut project = test_project("api");
        project
            .branches
            .aliases
            .insert("feature".to_string(), "feature-api".to_string());
        project
            .branches
            .aliases
            .insert("prod".to_string(), "release-api".to_string());
        let wp = WorkspaceProject {
            name: "api".to_string(),
            worktree_path: repo.path().display().to_string(),
        };

        let plans = plan_merge_targets(&[(wp, project)], "feature", "prod").expect("merge plan");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].1.branch, "feature-api");
        assert_eq!(plans[0].2.branch, "release-api");
    }

    #[test]
    fn test_plan_merge_targets_reports_all_missing_source_and_target_branches() {
        let repo = create_test_repo();

        let mut project = test_project("api");
        project
            .branches
            .aliases
            .insert("feature".to_string(), "feature-api".to_string());
        project
            .branches
            .aliases
            .insert("prod".to_string(), "release-api".to_string());
        let wp = WorkspaceProject {
            name: "api".to_string(),
            worktree_path: repo.path().display().to_string(),
        };

        let err = plan_merge_targets(&[(wp, project)], "feature", "prod").unwrap_err();

        assert_eq!(err.to_string(), "Precheck failed for 1 project(s)");
    }
}
