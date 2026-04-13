use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::config::{self, BranchConfig, Group, Project};
use crate::git;
use crate::i18n::t;
use crate::ui;

pub fn run(path: &str) -> Result<()> {
    // 1. Resolve and validate the path
    let resolved = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("{}", t("path_not_exist").replace("{}", path)))?;

    if !resolved.is_dir() {
        bail!("Path is not a directory: {}", resolved.display());
    }

    if !git::is_git_repo(&resolved) {
        bail!("{}", t("not_git_repo").replace("{}", &resolved.to_string_lossy()));
    }

    let path_str = resolved.to_string_lossy().to_string();

    // 2. Check if already registered
    let mut pf = config::load_projects()?;
    if pf.projects.iter().any(|p| p.path == path_str) {
        bail!("{}", t("project_already_registered").replace("{}", &path_str));
    }

    // 3. Auto-detect project name from directory name, let user modify
    let dir_name = resolved
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let name = ui::input(&t("project_name"), &dir_name)?;

    // Check for duplicate name
    if pf.projects.iter().any(|p| p.name == name) {
        bail!("{}", t("project_name_exists").replace("{}", &name));
    }

    // 4. Select group
    let group = select_group(&mut pf)?;

    // 5. Fetch remote (best effort) to validate branches later
    ui::info(&t("fetching_remote"));
    let _ = git::fetch(&resolved);

    let remote_branches = git::list_remote_branches(&resolved).unwrap_or_default();
    let clean_branches: Vec<String> = remote_branches
        .iter()
        .filter_map(|b| b.strip_prefix("origin/").map(|s| s.to_string()))
        .collect();

    // 6. Main branch - direct input + validation
    let current = git::current_branch(&resolved).unwrap_or_else(|_| "main".to_string());
    let main_branch = input_branch_with_validation(
        &t("main_branch"),
        &current,
        &clean_branches,
        true,
    )?.unwrap(); // safe: required=true guarantees Some

    // 7. Environment branches - direct input + validation (optional)
    let test_branch = input_branch_with_validation(
        &t("test_branch"),
        "",
        &clean_branches,
        false,
    )?;
    let staging_branch = input_branch_with_validation(
        &t("staging_branch"),
        "",
        &clean_branches,
        false,
    )?;
    let prod_branch = input_branch_with_validation(
        &t("prod_branch"),
        "",
        &clean_branches,
        false,
    )?;

    // 8. Optional agents.md configuration
    let agents_md = ui::input_optional(&t("agents_md_path"), &t("press_enter_skip"))?;

    if let Some(ref md_path) = agents_md {
        let md_resolved = PathBuf::from(md_path);
        if !md_resolved.exists() {
            ui::warn(&format!("agents.md not found at: {}", md_path));
            if !ui::confirm(&t("continue_anyway"), false)? {
                bail!("Aborted");
            }
        }
    }

    // 9. Calculate order and save
    let order = pf
        .projects
        .iter()
        .filter(|p| p.group == group)
        .map(|p| p.order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    let project = Project {
        name: name.clone(),
        path: path_str,
        group,
        order,
        agents_md,
        branches: BranchConfig {
            main: main_branch,
            test: test_branch,
            staging: staging_branch,
            prod: prod_branch,
        },
    };

    pf.projects.push(project);
    config::save_projects(&pf)?;

    ui::success(&t("project_added").replace("{}", &name));
    Ok(())
}

/// Prompt user to input a branch name directly, then validate against remote branches.
/// If `required` is true, empty input is not allowed.
/// Returns None for optional branches when input is empty.
fn input_branch_with_validation(
    prompt: &str,
    default: &str,
    remote_branches: &[String],
    required: bool,
) -> Result<Option<String>> {
    loop {
        let value = if default.is_empty() {
            ui::input_optional(prompt, "")?.unwrap_or_default()
        } else {
            ui::input(prompt, default)?
        };

        if value.is_empty() {
            if required {
                ui::error(&t("field_required"));
                continue;
            }
            ui::info(&t("skipped"));
            return Ok(None);
        }

        // Validate: check if origin/<branch> exists in remote
        if remote_branches.contains(&value) {
            ui::success(&t("branch_exists").replace("{}", &value));
        } else if !remote_branches.is_empty() {
            ui::warn(&t("branch_not_found").replace("{}", &value));
            if !ui::confirm(&t("continue_branch"), true)? {
                continue;
            }
        }
        // If remote_branches is empty, skip validation silently

        return Ok(Some(value));
    }
}

/// Prompt user to select a group from existing groups, create a new one, or choose ungrouped.
fn select_group(pf: &mut config::ProjectsFile) -> Result<String> {
    let mut options: Vec<String> = pf.groups.iter().map(|g| g.name.clone()).collect();
    options.push(t("new_group"));
    options.push(t("ungrouped"));

    let idx = ui::select(&t("select_group"), &options)?;

    if idx == options.len() - 1 {
        Ok(String::new())
    } else if idx == options.len() - 2 {
        let group_name = ui::input_with_placeholder(&t("group_name"), &t("placeholder_group_name"))?;
        if group_name.is_empty() {
            bail!("Group name cannot be empty");
        }
        if pf.groups.iter().any(|g| g.name == group_name) {
            bail!("{}", t("group_exists").replace("{}", &group_name));
        }
        let order = pf.groups.iter().map(|g| g.order).max().map(|m| m + 1).unwrap_or(0);
        pf.groups.push(Group {
            name: group_name.clone(),
            order,
        });
        Ok(group_name)
    } else {
        Ok(options[idx].clone())
    }
}
