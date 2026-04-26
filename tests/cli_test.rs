use assert_cmd::Command;
use predicates::prelude::*;

fn workspace_context_failure() -> impl Predicate<str> {
    predicate::str::contains("No workspaces")
        .or(predicate::str::contains("暂无工作区"))
        .or(predicate::str::contains("not a terminal"))
}

fn grove_cmd() -> (tempfile::TempDir, Command) {
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("grove").unwrap();
    cmd.env("HOME", home.path());
    (home, cmd)
}

#[test]
fn test_help_output() {
    let (_home, mut cmd) = grove_cmd();

    cmd.arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Multi-project git worktree workspace manager",
        ));
}

#[test]
fn test_no_args_shows_help() {
    let (_home, mut cmd) = grove_cmd();

    cmd.assert().success();
}

#[test]
fn test_list_empty() {
    let (_home, mut cmd) = grove_cmd();

    cmd.arg("list").assert().success();
}

#[test]
fn test_status_empty() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["-w", "status"]).assert().success();
}

#[test]
fn test_config_list() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("workpath"));
}

#[test]
fn test_completion_bash() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["completion", "bash"]).assert().success();
}

#[test]
fn test_completion_zsh() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["completion", "zsh"]).assert().success();
}

#[test]
fn test_add_invalid_path() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["add", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .failure();
}

#[test]
fn test_aliases_ls() {
    let (_home, mut cmd) = grove_cmd();

    cmd.arg("ls").assert().success();
}

#[test]
fn test_aliases_st() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["-w", "st"]).assert().success();
}

#[test]
fn test_gpush_accepts_optional_target() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["gpush", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gmerge_accepts_optional_target() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["gmerge", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gswitch_command_exists() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["gswitch", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gcreate_command_exists() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["gcreate", "feature-x"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}
