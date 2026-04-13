use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_output() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Multi-project git worktree workspace manager",
        ));
}

#[test]
fn test_no_args_shows_help() {
    Command::cargo_bin("grove")
        .unwrap()
        .assert()
        .success();
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
        .args(["add", "/nonexistent/path/that/does/not/exist"])
        .assert()
        .failure();
}

#[test]
fn test_aliases_ls() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("ls")
        .assert()
        .success();
}

#[test]
fn test_aliases_st() {
    Command::cargo_bin("grove")
        .unwrap()
        .arg("st")
        .assert()
        .success();
}
