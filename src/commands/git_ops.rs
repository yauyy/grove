use anyhow::Result;
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

    let mut matched = Vec::new();
    for wp in &ws.projects {
        if let Some(proj) = projects_file.projects.iter().find(|p| p.name == wp.name) {
            matched.push((wp.clone(), proj.clone()));
        }
    }

    Ok((ws, matched))
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
        match generate_commit_message(&tool, &projects) {
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

pub fn gpush() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, _proj) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        match git::push_upstream(wt_path, &ws.branch) {
            Ok(()) => {
                ui::success(&format!("{}: pushed to origin/{}", wp.name, ws.branch));
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

pub fn gmerge() -> Result<()> {
    let (ws, projects) = get_workspace_context()?;
    let projects_file = config::load_projects()?;

    // Collect project names for common_environments
    let project_names: Vec<String> = projects.iter().map(|(wp, _)| wp.name.clone()).collect();
    let envs = workspace::common_environments(&projects_file, &project_names);

    if envs.is_empty() {
        ui::error(&t("no_common_env"));
        // Show which projects are missing which environments
        let env_names = ["test", "staging", "prod"];
        for env_name in &env_names {
            let missing: Vec<&str> = projects
                .iter()
                .filter(|(_, proj)| workspace::get_env_branch(proj, env_name).is_none())
                .map(|(wp, _)| wp.name.as_str())
                .collect();
            if !missing.is_empty() {
                ui::warn(&format!("{}: missing in {}", env_name, missing.join(", ")));
            }
        }
        anyhow::bail!("Cannot merge without common environments");
    }

    let env_names: Vec<String> = envs.clone();
    let idx = ui::select(&t("merge_to_env"), &env_names)?;
    let target_env = &envs[idx];

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (wp, proj) in &projects {
        let wt_path = Path::new(&wp.worktree_path);
        let env_branch_remote = match workspace::get_env_branch(proj, target_env) {
            Some(b) => b.clone(),
            None => {
                ui::error(&format!("{}: no {} branch configured", wp.name, target_env));
                failed += 1;
                continue;
            }
        };

        // Determine local branch name (strip "origin/" prefix if present)
        let local_env_branch = env_branch_remote
            .strip_prefix("origin/")
            .unwrap_or(&env_branch_remote)
            .to_string();

        let work_branch = ws.branch.clone();

        // Perform fetch, checkout env branch, merge work branch, checkout back
        let result = (|| -> Result<()> {
            git::fetch(wt_path)?;
            git::checkout(wt_path, &local_env_branch)?;
            git::pull_ff_only(wt_path, "origin", &local_env_branch)?;
            git::merge(wt_path, &work_branch)?;
            git::checkout(wt_path, &work_branch)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                ui::success(&format!(
                    "{}: merged {} into {}",
                    wp.name, work_branch, local_env_branch
                ));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", wp.name, e));
                // Try to checkout back to work branch on failure
                let _ = git::checkout(wt_path, &work_branch);
                failed += 1;
            }
        }
    }

    ui::batch_summary(succeeded, failed);
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
}
