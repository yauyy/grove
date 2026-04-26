use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Global configuration stored in ~/.grove/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub workpath: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub git_prefix: String,
    #[serde(default = "default_commit_message_tool")]
    pub commit_message_tool: String,
    #[serde(default)]
    pub auto_go_work: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub branch_presets: BTreeMap<String, String>,
}

fn default_language() -> String {
    // Detect from system locale
    if let Ok(lang) = std::env::var("LANG") {
        if lang.starts_with("zh") {
            return "zh".to_string();
        }
    }
    if let Ok(lang) = std::env::var("LC_ALL") {
        if lang.starts_with("zh") {
            return "zh".to_string();
        }
    }
    "en".to_string()
}

fn default_commit_message_tool() -> String {
    "manual".to_string()
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            workpath: "~/grove-workspaces".to_string(),
            language: default_language(),
            git_prefix: String::new(),
            commit_message_tool: default_commit_message_tool(),
            auto_go_work: false,
            branch_presets: BTreeMap::new(),
        }
    }
}

/// Branch naming configuration for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchConfig {
    pub main: String,
    #[serde(default, flatten)]
    pub aliases: BTreeMap<String, String>,
}

impl BranchConfig {
    pub fn get(&self, name: &str) -> Option<&str> {
        if name == "main" {
            Some(self.main.as_str())
        } else {
            self.aliases.get(name).map(String::as_str)
        }
    }

    #[allow(dead_code)]
    pub fn set_alias(&mut self, name: impl Into<String>, branch: impl Into<String>) -> bool {
        let name = name.into();
        if name.trim().is_empty() || name == "main" {
            return false;
        }
        self.aliases.insert(name, branch.into());
        true
    }
}

/// A registered project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub branch_aliases: BTreeMap<String, String>,
    pub branches: BranchConfig,
}

/// A project group for organizing projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub order: u32,
}

/// The projects.toml file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectsFile {
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

/// A project within a workspace (worktree instance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProject {
    pub name: String,
    pub worktree_path: String,
}

/// A workspace representing a set of worktrees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub created_at: String,
    #[serde(default)]
    pub projects: Vec<WorkspaceProject>,
}

/// The workspaces.toml file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspacesFile {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_roundtrip() {
        let config = GlobalConfig {
            workpath: "/tmp/my-workspaces".to_string(),
            language: "en".to_string(),
            git_prefix: String::new(),
            commit_message_tool: "manual".to_string(),
            auto_go_work: false,
            branch_presets: BTreeMap::new(),
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: GlobalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.workpath, "/tmp/my-workspaces");
    }

    #[test]
    fn test_global_config_default_contains_grove_workspaces() {
        let config = GlobalConfig::default();
        assert!(config.workpath.contains("grove-workspaces"));
    }

    #[test]
    fn test_projects_file_roundtrip() {
        let pf = ProjectsFile {
            groups: vec![Group {
                name: "backend".to_string(),
                order: 0,
            }],
            projects: vec![Project {
                name: "api".to_string(),
                path: "/home/user/api".to_string(),
                group: "backend".to_string(),
                order: 1,
                tags: vec!["go".to_string()],
                agents_md: Some("/home/user/api/agents.md".to_string()),
                branch_aliases: BTreeMap::new(),
                branches: BranchConfig {
                    main: "main".to_string(),
                    aliases: BTreeMap::from([
                        ("test".to_string(), "test".to_string()),
                        ("staging".to_string(), "staging".to_string()),
                    ]),
                },
            }],
        };
        let toml_str = toml::to_string(&pf).unwrap();
        let parsed: ProjectsFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.projects.len(), 1);
        assert_eq!(parsed.projects[0].name, "api");
        assert_eq!(parsed.projects[0].tags, vec!["go"]);
        assert_eq!(parsed.projects[0].branches.main, "main");
        assert_eq!(parsed.projects[0].branches.get("test"), Some("test"));
        assert_eq!(parsed.projects[0].branches.get("staging"), Some("staging"));
        assert_eq!(parsed.projects[0].branches.get("prod"), None);
        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].name, "backend");
    }

    #[test]
    fn test_workspaces_file_roundtrip() {
        let wf = WorkspacesFile {
            workspaces: vec![Workspace {
                name: "feature-x".to_string(),
                branch: "feature/x".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                projects: vec![WorkspaceProject {
                    name: "api".to_string(),
                    worktree_path: "/tmp/ws/api".to_string(),
                }],
            }],
        };
        let toml_str = toml::to_string(&wf).unwrap();
        let parsed: WorkspacesFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.workspaces[0].name, "feature-x");
        assert_eq!(parsed.workspaces[0].projects.len(), 1);
        assert_eq!(parsed.workspaces[0].projects[0].name, "api");
    }

    #[test]
    fn test_empty_projects_file_deserializes() {
        // An empty TOML string should deserialize to default ProjectsFile
        let parsed: ProjectsFile = toml::from_str("").unwrap();
        assert!(parsed.groups.is_empty());
        assert!(parsed.projects.is_empty());
    }

    #[test]
    fn test_group_order_defaults_to_zero() {
        let toml_str = r#"
[[groups]]
name = "frontend"
"#;

        let parsed: ProjectsFile = toml::from_str(toml_str).unwrap();

        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].name, "frontend");
        assert_eq!(parsed.groups[0].order, 0);
    }

    #[test]
    fn test_workspace_projects_defaults_to_empty() {
        let toml_str = r#"
[[workspaces]]
name = "feature-x"
branch = "feature/x"
created_at = "2024-01-01T00:00:00Z"
"#;

        let parsed: WorkspacesFile = toml::from_str(toml_str).unwrap();

        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.workspaces[0].name, "feature-x");
        assert!(parsed.workspaces[0].projects.is_empty());
    }

    #[test]
    fn test_legacy_project_config_defaults_missing_fields() {
        let toml_str = r#"
[[projects]]
name = "api"
path = "/tmp/api"

[projects.branches]
main = "main"
test = "test-master"
staging = "pre"
prod = "master"
"#;

        let parsed: ProjectsFile = toml::from_str(toml_str).unwrap();
        let project = &parsed.projects[0];

        assert!(project.tags.is_empty());
        assert!(project.branch_aliases.is_empty());
        assert_eq!(project.branches.get("test"), Some("test-master"));
        assert_eq!(project.branches.get("staging"), Some("pre"));
        assert_eq!(project.branches.get("prod"), Some("master"));
    }

    #[test]
    fn test_branch_config_optional_fields_omitted_in_toml() {
        let bc = BranchConfig {
            main: "main".to_string(),
            aliases: BTreeMap::new(),
        };
        let toml_str = toml::to_string(&bc).unwrap();
        assert!(!toml_str.contains("test"));
        assert!(!toml_str.contains("staging"));
        assert!(!toml_str.contains("prod"));
        assert!(toml_str.contains("main"));
    }

    #[test]
    fn test_branch_config_accepts_extra_mappings() {
        let toml_str = r#"
main = "master"
test = "test-master"
staging = "pre"
prod = "master"
master = "main"
"#;

        let parsed: BranchConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(parsed.main, "master");
        assert_eq!(parsed.get("test"), Some("test-master"));
        assert_eq!(parsed.get("staging"), Some("pre"));
        assert_eq!(parsed.get("prod"), Some("master"));
        assert_eq!(parsed.get("master"), Some("main"));
        assert_eq!(parsed.get("missing"), None);
    }

    #[test]
    fn test_branch_config_set_alias_rejects_invalid_names() {
        let mut branches = BranchConfig {
            main: "master".to_string(),
            aliases: BTreeMap::new(),
        };

        assert!(branches.set_alias("test", "test-master"));
        assert_eq!(branches.get("test"), Some("test-master"));

        assert!(!branches.set_alias("main", "develop"));
        assert_eq!(branches.main, "master");
        assert!(!branches.aliases.contains_key("main"));

        assert!(!branches.set_alias("", "x"));
    }

    #[test]
    fn test_project_branch_aliases_roundtrip() {
        let pf = ProjectsFile {
            groups: Vec::new(),
            projects: vec![Project {
                name: "api".to_string(),
                path: "/tmp/api".to_string(),
                group: String::new(),
                order: 0,
                tags: Vec::new(),
                agents_md: None,
                branch_aliases: std::collections::BTreeMap::from([(
                    "test-master".to_string(),
                    "test".to_string(),
                )]),
                branches: BranchConfig {
                    main: "master".to_string(),
                    aliases: std::collections::BTreeMap::from([(
                        "test".to_string(),
                        "test-master".to_string(),
                    )]),
                },
            }],
        };

        let toml_str = toml::to_string(&pf).unwrap();
        let parsed: ProjectsFile = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            parsed.projects[0].branch_aliases.get("test-master"),
            Some(&"test".to_string())
        );
        assert_eq!(parsed.projects[0].branches.get("test"), Some("test-master"));
    }
}
