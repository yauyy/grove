pub mod models;

pub use models::*;

use anyhow::{Context, Result};
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
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let config: GlobalConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(config)
}

/// Loads the projects file from ~/.grove/projects.toml.
/// Returns default (empty) if the file does not exist.
pub fn load_projects() -> Result<ProjectsFile> {
    let path = grove_dir()?.join("projects.toml");
    if !path.exists() {
        return Ok(ProjectsFile::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let pf: ProjectsFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(pf)
}

/// Loads the workspaces file from ~/.grove/workspaces.toml.
/// Returns default (empty) if the file does not exist.
pub fn load_workspaces() -> Result<WorkspacesFile> {
    let path = grove_dir()?.join("workspaces.toml");
    if !path.exists() {
        return Ok(WorkspacesFile::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let wf: WorkspacesFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(wf)
}

/// Saves the global configuration to ~/.grove/config.toml.
pub fn save_global_config(config: &GlobalConfig) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("config.toml");
    let content = toml::to_string(config).context("Failed to serialize global config")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Saves the projects file to ~/.grove/projects.toml.
pub fn save_projects(pf: &ProjectsFile) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("projects.toml");
    let content = toml::to_string(pf).context("Failed to serialize projects file")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Saves the workspaces file to ~/.grove/workspaces.toml.
pub fn save_workspaces(wf: &WorkspacesFile) -> Result<()> {
    ensure_dirs()?;
    let path = grove_dir()?.join("workspaces.toml");
    let content = toml::to_string(wf).context("Failed to serialize workspaces file")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
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
