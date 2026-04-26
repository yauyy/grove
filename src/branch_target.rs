use std::collections::BTreeMap;

use crate::config::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveSource {
    ProjectInputAlias,
    ProjectBranchMapping,
    BranchPresetFallback,
    ExplicitBranch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBranch {
    pub input: String,
    pub logical: Option<String>,
    pub branch: String,
    pub source: ResolveSource,
}

impl ResolvedBranch {
    #[allow(dead_code)]
    pub fn summary_label(&self) -> String {
        match (&self.logical, &self.source) {
            (Some(logical), ResolveSource::ProjectInputAlias) => {
                format!("{} -> {} -> {}", self.input, logical, self.branch)
            }
            (Some(logical), ResolveSource::ProjectBranchMapping) if logical != &self.branch => {
                format!("{} -> {}", logical, self.branch)
            }
            _ => self.branch.clone(),
        }
    }
}

pub fn resolve_target(
    project: &Project,
    branch_presets: &BTreeMap<String, String>,
    target: &str,
) -> ResolvedBranch {
    if let Some(logical) = project.branch_aliases.get(target) {
        if let Some(branch) = project.branches.get(logical) {
            return ResolvedBranch {
                input: target.to_string(),
                logical: Some(logical.clone()),
                branch: branch.to_string(),
                source: ResolveSource::ProjectInputAlias,
            };
        }
    }

    if let Some(branch) = project.branches.get(target) {
        return ResolvedBranch {
            input: target.to_string(),
            logical: Some(target.to_string()),
            branch: branch.to_string(),
            source: ResolveSource::ProjectBranchMapping,
        };
    }

    if branch_presets.contains_key(target) {
        return ResolvedBranch {
            input: target.to_string(),
            logical: Some(target.to_string()),
            branch: target.to_string(),
            source: ResolveSource::BranchPresetFallback,
        };
    }

    ResolvedBranch {
        input: target.to_string(),
        logical: None,
        branch: target.to_string(),
        source: ResolveSource::ExplicitBranch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchConfig, Project};
    use std::collections::BTreeMap;

    fn project() -> Project {
        Project {
            name: "api".to_string(),
            path: "/tmp/api".to_string(),
            group: String::new(),
            order: 0,
            tags: Vec::new(),
            agents_md: None,
            branch_aliases: BTreeMap::from([("test-master".to_string(), "test".to_string())]),
            branches: BranchConfig {
                main: "master".to_string(),
                aliases: BTreeMap::from([
                    ("test".to_string(), "test-master".to_string()),
                    ("pre".to_string(), "release-api".to_string()),
                    ("develop".to_string(), "develop".to_string()),
                ]),
            },
        }
    }

    #[test]
    fn resolves_input_alias_then_branch_mapping() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "test-master");

        assert_eq!(resolved.input, "test-master");
        assert_eq!(resolved.logical.as_deref(), Some("test"));
        assert_eq!(resolved.branch, "test-master");
        assert_eq!(resolved.source, ResolveSource::ProjectInputAlias);
    }

    #[test]
    fn resolves_direct_branch_mapping() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "pre");

        assert_eq!(resolved.logical.as_deref(), Some("pre"));
        assert_eq!(resolved.branch, "release-api");
        assert_eq!(resolved.source, ResolveSource::ProjectBranchMapping);
    }

    #[test]
    fn resolves_preset_key_as_real_branch_when_project_mapping_missing() {
        let presets = BTreeMap::from([("prod".to_string(), "正式环境".to_string())]);
        let resolved = resolve_target(&project(), &presets, "prod");

        assert_eq!(resolved.logical.as_deref(), Some("prod"));
        assert_eq!(resolved.branch, "prod");
        assert_eq!(resolved.source, ResolveSource::BranchPresetFallback);
    }

    #[test]
    fn falls_back_to_explicit_real_branch() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "hotfix/x");

        assert_eq!(resolved.logical, None);
        assert_eq!(resolved.branch, "hotfix/x");
        assert_eq!(resolved.source, ResolveSource::ExplicitBranch);
    }

    #[test]
    fn display_label_includes_mapping_context() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "test-master");

        assert_eq!(
            resolved.summary_label(),
            "test-master -> test -> test-master"
        );
    }

    #[test]
    fn display_label_shows_branch_mapping_when_names_differ() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "pre");

        assert_eq!(resolved.summary_label(), "pre -> release-api");
    }

    #[test]
    fn display_label_omits_branch_mapping_when_names_match() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "develop");

        assert_eq!(resolved.summary_label(), "develop");
    }

    #[test]
    fn display_label_omits_preset_fallback_mapping() {
        let presets = BTreeMap::from([("prod".to_string(), "正式环境".to_string())]);
        let resolved = resolve_target(&project(), &presets, "prod");

        assert_eq!(resolved.summary_label(), "prod");
    }

    #[test]
    fn display_label_shows_explicit_branch() {
        let resolved = resolve_target(&project(), &BTreeMap::new(), "hotfix/x");

        assert_eq!(resolved.summary_label(), "hotfix/x");
    }
}
