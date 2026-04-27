//! AI-assisted commit message generation.
//!
//! `gcommit` can ask one of the supported CLI tools (codex, claude, copilot,
//! cursor) to draft a message for the staged changes across all workspace
//! projects. The runner is bounded by [`COMMIT_MESSAGE_TOOL_TIMEOUT`] so a
//! hung subprocess can't hold up the user's commit flow.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{Project, WorkspaceProject};
use crate::git;
use crate::i18n::t;

/// Maximum wall time we let an AI CLI tool consume while drafting a commit
/// message. Past this, we kill the child and fall back to manual input.
pub(super) const COMMIT_MESSAGE_TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// Normalise a configured tool name into one of the canonical identifiers used
/// internally. Unknown values map to `"manual"` so callers know to skip AI
/// generation entirely.
pub fn normalize_commit_message_tool(tool: &str) -> &'static str {
    match tool.trim().to_ascii_lowercase().as_str() {
        "codex" => "codex",
        "claude" | "claude-code" | "claude_code" => "claude",
        "copilot" | "copilot-cli" | "copilot_cli" => "copilot",
        "cursor" | "cursor-cli" | "cursor_cli" | "cursorcli" => "cursor",
        _ => "manual",
    }
}

pub(super) fn generate_commit_message(
    tool: &str,
    projects: &[(WorkspaceProject, Project)],
) -> Result<String> {
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

fn build_command(tool: &str, prompt: &str) -> Result<Command> {
    let mut command = match tool {
        "codex" => {
            let mut c = Command::new("codex");
            c.args(["exec", prompt]);
            c
        }
        "claude" => {
            let mut c = Command::new("claude");
            c.args(["-p", prompt]);
            c
        }
        "copilot" => {
            let mut c = Command::new("gh");
            c.args(["copilot", "suggest", "-t", "git", prompt]);
            c
        }
        "cursor" => {
            let mut c = Command::new("cursor-agent");
            c.args(["-p", prompt]);
            c
        }
        _ => anyhow::bail!("unsupported commit message tool: {}", tool),
    };
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    Ok(command)
}

fn run_commit_message_tool(tool: &str, prompt: &str) -> Result<String> {
    let mut command = build_command(tool, prompt)?;
    let mut child = command.spawn()?;
    let deadline = Instant::now() + COMMIT_MESSAGE_TOOL_TIMEOUT;

    loop {
        match child.try_wait()? {
            Some(_status) => break,
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!(
                        "{}",
                        t("ai_commit_tool_timeout")
                            .replacen("{}", tool, 1)
                            .replacen("{}", &COMMIT_MESSAGE_TOOL_TIMEOUT.as_secs().to_string(), 1)
                    );
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    let output = child.wait_with_output()?;
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

    #[test]
    fn test_build_command_rejects_manual_tool() {
        assert!(build_command("manual", "noop").is_err());
    }
}
