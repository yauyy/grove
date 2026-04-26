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

fn start_point_candidates(branch: &str) -> (String, String) {
    if let Some(local_branch) = branch.strip_prefix("origin/") {
        return (branch.to_string(), local_branch.to_string());
    }

    (format!("origin/{}", branch), branch.to_string())
}

fn remote_start_point_refs(branch: &str) -> (String, String, String) {
    let (_remote_ref, local_branch) = start_point_candidates(branch);
    let remote_short = format!("origin/{}", local_branch);
    let remote_full = format!("refs/remotes/{}", remote_short);
    (remote_short, remote_full, local_branch)
}

/// Resolve the best start point for branch creation.
/// Prefers `origin/{branch}` (remote latest) over local branch.
pub fn resolve_remote_start_point(dir: &Path, branch: &str) -> Result<String> {
    let (remote_ref, local_branch) = start_point_candidates(branch);
    if run_git(dir, &["rev-parse", "--verify", &remote_ref])?.success {
        return Ok(remote_ref);
    }

    if run_git(dir, &["rev-parse", "--verify", &local_branch])?.success {
        return Ok(local_branch);
    }

    bail!(
        "cannot resolve start point '{}' in {}",
        branch,
        dir.display()
    );
}

/// Resolve a remote-only start point and fail if the remote branch is missing.
pub fn resolve_remote_start_point_checked(dir: &Path, branch: &str) -> Result<String> {
    let (remote_short, remote_full, _local_branch) = remote_start_point_refs(branch);
    if run_git(dir, &["rev-parse", "--verify", &remote_full])?.success {
        return Ok(remote_short);
    }

    bail!(
        "cannot resolve remote start point '{}' in {}",
        branch,
        dir.display()
    );
}

/// Resolve a start point and fail if neither remote nor local branch exists.
#[allow(dead_code)]
pub fn resolve_start_point_checked(dir: &Path, branch: &str) -> Result<String> {
    let (remote_ref, local_branch) = start_point_candidates(branch);
    if run_git(dir, &["rev-parse", "--verify", &remote_ref])?.success {
        return Ok(remote_ref);
    }

    if run_git(dir, &["rev-parse", "--verify", &local_branch])?.success {
        return Ok(local_branch);
    }

    bail!(
        "cannot resolve start point '{}' in {}",
        branch,
        dir.display()
    );
}

/// Add a new worktree with a new branch (--no-track).
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
        &[
            "worktree",
            "add",
            "-b",
            branch,
            "--no-track",
            wt_str,
            start_point,
        ],
    )?;
    Ok(())
}

/// Add a worktree using an existing branch.
pub fn worktree_add_existing(repo_dir: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    let wt_str = worktree_path
        .to_str()
        .context("Invalid worktree path encoding")?;
    run_git_checked(repo_dir, &["worktree", "add", wt_str, branch])?;
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

/// Prune stale worktree entries.
pub fn worktree_prune(repo_dir: &Path) -> Result<()> {
    run_git_checked(repo_dir, &["worktree", "prune"])?;
    Ok(())
}

/// Delete a local branch.
pub fn branch_delete(dir: &Path, branch: &str) -> Result<()> {
    run_git_checked(dir, &["branch", "-D", branch])?;
    Ok(())
}

/// Rename a local branch.
pub fn branch_rename(dir: &Path, old_name: &str, new_name: &str) -> Result<()> {
    run_git_checked(dir, &["branch", "-m", old_name, new_name])?;
    Ok(())
}

/// Check if a local branch exists.
pub fn branch_exists(dir: &Path, branch: &str) -> Result<bool> {
    let full_ref = format!("refs/heads/{}", branch);
    let output = run_git(dir, &["rev-parse", "--verify", &full_ref])?;
    Ok(output.success)
}

/// Check if an origin remote-tracking branch exists.
pub fn remote_branch_exists(dir: &Path, branch: &str) -> Result<bool> {
    let branch = branch.strip_prefix("origin/").unwrap_or(branch);
    let full_ref = format!("refs/remotes/origin/{}", branch);
    Ok(run_git(dir, &["rev-parse", "--verify", &full_ref])?.success)
}

/// Repair worktree administrative files after moving worktree directories.
pub fn worktree_repair(repo_dir: &Path) -> Result<()> {
    run_git_checked(repo_dir, &["worktree", "repair"])?;
    Ok(())
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

/// Return whether the index contains staged changes.
pub fn has_staged_changes(dir: &Path) -> Result<bool> {
    let output = run_git(dir, &["diff", "--cached", "--quiet"])?;
    if !output.success && !output.stderr.is_empty() {
        bail!(
            "git diff --cached --quiet failed in {}: {}",
            dir.display(),
            output.stderr
        );
    }
    Ok(!output.success)
}

/// Get staged diff context suitable for commit message generation.
pub fn staged_diff_summary(dir: &Path) -> Result<String> {
    let stat = run_git_checked(dir, &["diff", "--cached", "--stat"])?;
    let names = run_git_checked(dir, &["diff", "--cached", "--name-status"])?;
    Ok(format!("{}\n{}", stat.stdout, names.stdout)
        .trim()
        .to_string())
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

/// Pull a specific remote branch using fast-forward only.
pub fn pull_ff_only(dir: &Path, remote: &str, branch: &str) -> Result<()> {
    run_git_checked(dir, &["pull", "--ff-only", remote, branch])?;
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

/// Checkout a new branch from the given start point.
pub fn checkout_new_branch(dir: &Path, branch: &str, start_point: &str) -> Result<()> {
    run_git_checked(dir, &["checkout", "--no-track", "-b", branch, start_point])?;
    Ok(())
}

/// Check if the working directory is clean (no uncommitted changes).
pub fn is_clean(dir: &Path) -> Result<bool> {
    let output = run_git(dir, &["status", "--porcelain"])?;
    if !output.success {
        bail!(
            "git status --porcelain failed in {}: {}",
            dir.display(),
            output.stderr
        );
    }
    Ok(output.stdout.is_empty())
}

/// Get the current branch name.
pub fn current_branch(dir: &Path) -> Result<String> {
    let output = run_git_checked(dir, &["symbolic-ref", "--short", "HEAD"])?;
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
    fn test_current_branch_runtime_helper() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();

        assert!(!branch.is_empty());
    }

    #[test]
    fn test_current_branch_errors_when_detached() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let head = run_git_checked(dir, &["rev-parse", "HEAD"]).unwrap().stdout;

        run_git_checked(dir, &["checkout", "--detach", &head]).unwrap();

        assert!(current_branch(dir).is_err());
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
    fn test_checkout_new_branch_does_not_set_upstream() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_ref = format!("refs/remotes/origin/{}", main_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        checkout_new_branch(
            dir,
            "feature/no-upstream",
            &format!("origin/{}", main_branch),
        )
        .unwrap();

        let upstream = run_git(
            dir,
            &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        )
        .unwrap();
        assert!(!upstream.success);
    }

    #[test]
    fn test_resolve_existing_start_point_checked() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();

        let start_point = resolve_start_point_checked(dir, &main_branch).unwrap();

        assert_eq!(start_point, main_branch);
    }

    #[test]
    fn test_resolve_start_point_checked_prefers_remote() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_ref = format!("refs/remotes/origin/{}", main_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        let start_point = resolve_start_point_checked(dir, &main_branch).unwrap();

        assert_eq!(start_point, format!("origin/{}", main_branch));
    }

    #[test]
    fn test_resolve_start_point_checked_accepts_origin_prefixed_remote() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_branch = format!("origin/{}", main_branch);
        let remote_ref = format!("refs/remotes/{}", remote_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        let start_point = resolve_start_point_checked(dir, &remote_branch).unwrap();

        assert_eq!(start_point, remote_branch);
    }

    #[test]
    fn test_resolve_start_point_checked_origin_prefixed_falls_back_to_local() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();

        let start_point =
            resolve_start_point_checked(dir, &format!("origin/{}", main_branch)).unwrap();

        assert_eq!(start_point, main_branch);
    }

    #[test]
    fn test_resolve_remote_start_point_accepts_origin_prefixed_remote() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_branch = format!("origin/{}", main_branch);
        let remote_ref = format!("refs/remotes/{}", remote_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        let start_point = resolve_remote_start_point(dir, &remote_branch).unwrap();

        assert_eq!(start_point, remote_branch);
        assert!(!start_point.starts_with("origin/origin/"));
    }

    #[test]
    fn test_resolve_remote_start_point_checked_returns_origin_for_local_input() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_ref = format!("refs/remotes/origin/{}", main_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        let start_point = resolve_remote_start_point_checked(dir, &main_branch).unwrap();

        assert_eq!(start_point, format!("origin/{}", main_branch));
    }

    #[test]
    fn test_resolve_remote_start_point_checked_accepts_origin_prefixed_remote() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        let remote_branch = format!("origin/{}", main_branch);
        let remote_ref = format!("refs/remotes/{}", remote_branch);
        run_git_checked(dir, &["update-ref", &remote_ref, "HEAD"]).unwrap();

        let start_point = resolve_remote_start_point_checked(dir, &remote_branch).unwrap();

        assert_eq!(start_point, remote_branch);
        assert!(!start_point.starts_with("origin/origin/"));
    }

    #[test]
    fn test_resolve_remote_start_point_checked_errors_without_remote_ref() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();

        let err = resolve_remote_start_point_checked(dir, &main_branch).unwrap_err();

        assert!(err
            .to_string()
            .contains("cannot resolve remote start point"));
    }

    #[test]
    fn test_resolve_remote_start_point_checked_rejects_local_origin_branch() {
        let tmp = create_test_repo();
        let dir = tmp.path();
        let main_branch = current_branch(dir).unwrap();
        run_git_checked(dir, &["branch", &format!("origin/{}", main_branch)]).unwrap();

        let err = resolve_remote_start_point_checked(dir, &main_branch).unwrap_err();

        assert!(err
            .to_string()
            .contains("cannot resolve remote start point"));
    }

    #[test]
    fn test_resolve_start_point_checked_errors_when_missing() {
        let tmp = create_test_repo();

        assert!(resolve_start_point_checked(tmp.path(), "missing-branch").is_err());
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
    fn test_is_clean_errors_for_non_git_directory() {
        let tmp = TempDir::new().unwrap();
        let err = is_clean(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("git status --porcelain failed"));
    }

    #[test]
    fn test_branch_exists() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();
        assert!(branch_exists(tmp.path(), &branch).unwrap());
        assert!(!branch_exists(tmp.path(), "nonexistent-branch-xyz").unwrap());
    }

    #[test]
    fn test_branch_exists_ignores_tags() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();
        run_git_checked(tmp.path(), &["tag", "v1"]).unwrap();

        assert!(branch_exists(tmp.path(), &branch).unwrap());
        assert!(!branch_exists(tmp.path(), "v1").unwrap());
    }

    #[test]
    fn test_remote_branch_exists_detects_origin_tracking_ref() {
        let tmp = create_test_repo();
        let branch = current_branch(tmp.path()).unwrap();
        let remote_ref = format!("refs/remotes/origin/{}", branch);
        run_git_checked(tmp.path(), &["update-ref", &remote_ref, "HEAD"]).unwrap();

        assert!(remote_branch_exists(tmp.path(), &branch).unwrap());
        assert!(remote_branch_exists(tmp.path(), &format!("origin/{}", branch)).unwrap());
        assert!(!branch_exists(tmp.path(), &format!("origin/{}", branch)).unwrap());
        assert!(!remote_branch_exists(tmp.path(), "missing-branch").unwrap());
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
    fn test_branch_rename() {
        let tmp = create_test_repo();
        let dir = tmp.path();

        // Create a branch to rename
        run_git_checked(dir, &["branch", "old-branch"]).unwrap();
        assert!(branch_exists(dir, "old-branch").unwrap());

        // Rename it
        branch_rename(dir, "old-branch", "new-branch").unwrap();
        assert!(!branch_exists(dir, "old-branch").unwrap());
        assert!(branch_exists(dir, "new-branch").unwrap());
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
