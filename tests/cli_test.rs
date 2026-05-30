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
fn test_gswitch_accepts_optional_target() {
    let (_home, mut cmd) = grove_cmd();

    cmd.arg("gswitch")
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

#[test]
fn test_glist_command_exists() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["glist"]).assert().success();
}

#[test]
fn test_glist_rm_and_rename_conflict() {
    let (_home, mut cmd) = grove_cmd();

    cmd.args(["glist", "--rm", "--rename"])
        .assert()
        .failure();
}

#[test]
fn test_grm_command_exists() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("grm").assert().success();
}

#[test]
fn test_grm_with_branch_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["grm", "feature-x"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gbranch_command_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("gbranch")
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gbranch_alias_gbr() {
    let (_home, mut cmd) = grove_cmd();
    cmd.arg("gbr")
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gmerge_all_flag_parses() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["gmerge", "--all", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gmerge_push_flag_parses() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["gmerge", "--push", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_gmerge_all_push_flags_parse() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["gmerge", "--all", "--push", "test"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_config_preset_list_shows_defaults() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["config", "preset", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("staging"))
        .stdout(predicate::str::contains("prod"));
}

#[test]
fn test_config_preset_set_then_list() {
    let home = tempfile::tempdir().unwrap();

    Command::cargo_bin("grove")
        .unwrap()
        .env("HOME", home.path())
        .args(["config", "preset", "set", "gray", "Gray release"])
        .assert()
        .success();

    // A fresh process with the same HOME must see the persisted preset, and the
    // built-in defaults must still be present (seed-on-first-edit).
    Command::cargo_bin("grove")
        .unwrap()
        .env("HOME", home.path())
        .args(["config", "preset", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gray"))
        .stdout(predicate::str::contains("test"));
}

#[test]
fn test_config_preset_rm_missing_fails() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["config", "preset", "rm", "does-not-exist"])
        .assert()
        .failure();
}

#[test]
fn test_worktree_list_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["worktree", "list"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_worktree_alias_gwt() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["gwt", "ls"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}

#[test]
fn test_worktree_prune_requires_workspace() {
    let (_home, mut cmd) = grove_cmd();
    cmd.args(["worktree", "prune"])
        .assert()
        .code(1)
        .stderr(workspace_context_failure());
}
