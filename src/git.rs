use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

/// Output from a git command execution.
#[derive(Debug)]
pub struct GitOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Run a git command in the given directory and return the output.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<GitOutput> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run git {:?} in {}", args, dir.display()))?;

    Ok(GitOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

/// Run a git command in the given directory and error if it fails.
pub fn run_git_checked(dir: &Path, args: &[&str]) -> Result<GitOutput> {
    let output = run_git(dir, args)?;
    if !output.success {
        bail!(
            "git {:?} failed in {}: {}",
            args,
            dir.display(),
            output.stderr
        );
    }
    Ok(output)
}

/// Check if a directory is a git repository.
pub fn is_git_repo(dir: &Path) -> bool {
    run_git(dir, &["rev-parse", "--git-dir"])
        .map(|o| o.success)
        .unwrap_or(false)
}

/// List remote branches for a repository.
pub fn list_remote_branches(dir: &Path) -> Result<Vec<String>> {
    let output = run_git_checked(dir, &["branch", "-r"])?;
    let branches = output
        .stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.contains("->"))
        .collect();
    Ok(branches)
}

/// Fetch from remote.
pub fn fetch(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["fetch"])?;
    Ok(())
}

/// Add a new worktree with --no-track.
pub fn worktree_add(
    repo_dir: &Path,
    worktree_path: &Path,
    branch: &str,
    start_point: &str,
) -> Result<()> {
    let wt_str = worktree_path
        .to_str()
        .context("Invalid worktree path encoding")?;
    run_git_checked(
        repo_dir,
        &["worktree", "add", "-b", branch, "--no-track", wt_str, start_point],
    )?;
    Ok(())
}

/// Remove a worktree.
pub fn worktree_remove(repo_dir: &Path, worktree_path: &Path) -> Result<()> {
    let wt_str = worktree_path
        .to_str()
        .context("Invalid worktree path encoding")?;
    run_git_checked(repo_dir, &["worktree", "remove", "--force", wt_str])?;
    Ok(())
}

/// Delete a local branch.
pub fn branch_delete(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["branch", "-D", branch])?;
    Ok(())
}

/// Check if a local branch exists.
pub fn branch_exists(dir: &Path, branch: &str) -> Result<bool> {
    let output = run_git(dir, &["rev-parse", "--verify", branch])?;
    Ok(output.success)
}

/// Stage all changes.
pub fn add_all(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["add", "-A"])?;
    Ok(())
}

/// Create a commit with the given message.
pub fn commit(dir: &Path, message: &str) -> Result<()> {
    run_git_checked(dir, &["commit", "-m", message])?;
    Ok(())
}

/// Push the current branch and set upstream.
pub fn push_upstream(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["push", "-u", "origin", branch])?;
    Ok(())
}

/// Pull from remote.
pub fn pull(dir: &Path) -> Result<()> {
    run_git_checked(dir, &["pull"])?;
    Ok(())
}

/// Get short status output.
pub fn status_short(dir: &Path) -> Result<String> {
    let output = run_git_checked(dir, &["status", "--short"])?;
    Ok(output.stdout)
}

/// Merge a branch into the current branch.
pub fn merge(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["merge", branch])?;
    Ok(())
}

/// Checkout a branch.
pub fn checkout(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["checkout", branch])?;
    Ok(())
}

/// Check if the working directory is clean (no uncommitted changes).
pub fn is_clean(dir: &Path) -> Result<bool> {
    let output = run_git(dir, &["status", "--porcelain"])?;
    Ok(output.stdout.is_empty())
}

/// Get the current branch name.
#[cfg(test)]
pub fn current_branch(dir: &Path) -> Result<String> {
    let output = run_git_checked(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a temporary git repo with an initial commit.
    fn create_test_repo() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        run_git_checked(dir, &["init"]).unwrap();
        run_git_checked(dir, &["config", "user.email", "test@test.com"]).unwrap();
        run_git_checked(dir, &["config", "user.name", "Test"]).unwrap();
        // Create initial commit so HEAD exists
        fs::write(dir.join("README.md"), "# Test").unwrap();
        run_git_checked(dir, &["add", "."]).unwrap();
        run_git_checked(dir, &["commit", "-m", "initial"]).unwrap();
        tmp
    }

    #[test]
    fn test_is_git_repo_valid() {
        let tmp = create_test_repo();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn test_is_git_repo_invalid() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
    }

    #[test]
    fn test_current_branch() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();
        // Could be "main" or "master" depending on git config
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_is_clean() {
        let tmp = create_test_repo();
        assert!(is_clean(tmp.path()).unwrap());

        // Make the repo dirty
        fs::write(tmp.path().join("dirty.txt"), "dirty").unwrap();
        assert!(!is_clean(tmp.path()).unwrap());
    }

    #[test]
    fn test_branch_exists() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();
        assert!(branch_exists(tmp.path(), &branch).unwrap());
        assert!(!branch_exists(tmp.path(), "nonexistent-branch-xyz").unwrap());
    }

    #[test]
    fn test_status_short() {
        let tmp = create_test_repo();
        // Clean repo should have empty status
        let status = status_short(tmp.path()).unwrap();
        assert!(status.is_empty());

        // Add an untracked file
        fs::write(tmp.path().join("new.txt"), "new").unwrap();
        let status = status_short(tmp.path()).unwrap();
        assert!(status.contains("new.txt"));
    }

    #[test]
    fn test_worktree_add_and_remove() {
        let tmp = create_test_repo();
        let repo_dir = tmp.path();

        let wt_path = repo_dir.join("worktree-test");
        let main_branch = current_branch(repo_dir).unwrap();
        worktree_add(repo_dir, &wt_path, "test-branch", &main_branch).unwrap();

        // The worktree directory should exist
        assert!(wt_path.exists());

        // The branch should exist
        assert!(branch_exists(repo_dir, "test-branch").unwrap());

        // Remove the worktree
        worktree_remove(repo_dir, &wt_path).unwrap();
        assert!(!wt_path.exists());
    }
}
