pub mod models;

pub use models::*;

use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use std::fs;
use std::path::PathBuf;

/// Returns the path to the grove configuration directory (~/.grove/).
pub fn grove_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".grove"))
}

/// Ensures the grove configuration directories exist (~/.grove/ and ~/.grove/agents/).
pub fn ensure_dirs() -> Result<()> {
    let dir = grove_dir()?;
    fs::create_dir_all(&dir).context("Failed to create ~/.grove/")?;
    fs::create_dir_all(dir.join("agents")).context("Failed to create ~/.grove/agents/")?;
    Ok(())
}

/// Loads the global configuration from ~/.grove/config.toml.
/// Returns default values if the file does not exist.
pub fn load_global_config() -> Result<GlobalConfig> {
    let path = grove_dir()?.join("config.toml");
    if !path.exists() {
        return Ok(GlobalConfig::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: GlobalConfig =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}

/// Loads the projects file from ~/.grove/projects.toml.
/// Returns default (empty) if the file does not exist.
pub fn load_projects() -> Result<ProjectsFile> {
    let path = grove_dir()?.join("projects.toml");
    if !path.exists() {
        return Ok(ProjectsFile::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let pf: ProjectsFile =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(pf)
}

/// Loads the workspaces file from ~/.grove/workspaces.toml.
/// Returns default (empty) if the file does not exist.
pub fn load_workspaces() -> Result<WorkspacesFile> {
    let path = grove_dir()?.join("workspaces.toml");
    if !path.exists() {
        return Ok(WorkspacesFile::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let wf: WorkspacesFile =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(wf)
}

/// Saves the global configuration to ~/.grove/config.toml.
pub fn save_global_config(config: &GlobalConfig) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("config.toml");
    let content = toml::to_string(config).context("Failed to serialize global config")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Saves the projects file to ~/.grove/projects.toml.
pub fn save_projects(pf: &ProjectsFile) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("projects.toml");
    let content = toml::to_string(pf).context("Failed to serialize projects file")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Saves the workspaces file to ~/.grove/workspaces.toml.
pub fn save_workspaces(wf: &WorkspacesFile) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("workspaces.toml");
    let content = toml::to_string(wf).context("Failed to serialize workspaces file")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Resolves a workpath string, expanding ~ to the user's home directory.
pub fn resolve_workpath(workpath: &str) -> Result<PathBuf> {
    if let Some(rest) = workpath.strip_prefix('~') {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        Ok(home.join(rest))
    } else {
        Ok(PathBuf::from(workpath))
    }
}

/// Convert a name to a safe directory name.
/// On macOS: '/' becomes ':' (Finder displays ':' as '/')
/// On other platforms: '/' becomes '-'
pub fn safe_dir_name(name: &str) -> String {
    if cfg!(target_os = "macos") {
        name.replace('/', ":")
    } else {
        name.replace('/', "-")
    }
}

/// Expand date templates in config strings.
///
/// Supported tokens inside square brackets: YYYY, YY, MM, DD.
/// Separators are preserved, e.g. [YYYY-MM-DD] -> 2026-04-24.
pub fn expand_date_templates(value: &str) -> String {
    expand_date_templates_with_date(value, Local::now().date_naive())
}

pub fn expand_date_templates_with_date(value: &str, date: NaiveDate) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find('[') {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        if let Some(end) = after_start.find(']') {
            let pattern = &after_start[..end];
            output.push_str(&expand_date_pattern(pattern, date));
            rest = &after_start[end + 1..];
        } else {
            output.push_str(&rest[start..]);
            return output;
        }
    }

    output.push_str(rest);
    output
}

fn expand_date_pattern(pattern: &str, date: NaiveDate) -> String {
    pattern
        .replace("YYYY", &format!("{:04}", date.year()))
        .replace("YY", &format!("{:02}", date.year() % 100))
        .replace("MM", &format!("{:02}", date.month()))
        .replace("DD", &format!("{:02}", date.day()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_workpath_with_tilde() {
        let home = dirs::home_dir().unwrap();
        let resolved = resolve_workpath("~/my-workspaces").unwrap();
        assert_eq!(resolved, home.join("my-workspaces"));
    }

    #[test]
    fn test_resolve_workpath_absolute() {
        let resolved = resolve_workpath("/tmp/workspaces").unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/workspaces"));
    }

    #[test]
    fn test_expand_date_templates_with_slashes() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 24).unwrap();
        let expanded = expand_date_templates_with_date("feature/ymy/[YYYY/MM/DD]/", date);
        assert_eq!(expanded, "feature/ymy/2026/04/24/");
    }

    #[test]
    fn test_expand_date_templates_with_compact_and_dash_formats() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 24).unwrap();
        assert_eq!(
            expand_date_templates_with_date("feat/[YYYYMMDD]/", date),
            "feat/20260424/"
        );
        assert_eq!(
            expand_date_templates_with_date("fix/[YY-MM-DD]/", date),
            "fix/26-04-24/"
        );
    }

    #[test]
    fn test_save_and_load_projects_file() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_path = tmp.path().join("projects.toml");

        let pf = ProjectsFile {
            groups: vec![Group {
                name: "frontend".to_string(),
                order: 0,
            }],
            projects: vec![Project {
                name: "web-app".to_string(),
                path: "/home/user/web-app".to_string(),
                group: "frontend".to_string(),
                order: 0,
                tags: Vec::new(),
                agents_md: None,
                branches: BranchConfig {
                    main: "main".to_string(),
                    test: Some("develop".to_string()),
                    staging: None,
                    prod: None,
                },
            }],
        };

        // Save to temp file
        let content = toml::to_string(&pf).unwrap();
        fs::write(&projects_path, &content).unwrap();

        // Load from temp file
        let loaded_content = fs::read_to_string(&projects_path).unwrap();
        let loaded: ProjectsFile = toml::from_str(&loaded_content).unwrap();

        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.groups[0].name, "frontend");
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "web-app");
        assert_eq!(
            loaded.projects[0].branches.test,
            Some("develop".to_string())
        );
        assert_eq!(loaded.projects[0].branches.staging, None);
    }
}
