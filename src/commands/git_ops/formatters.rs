//! User-facing message formatters for batch git operations.
//!
//! Templates live in [`crate::i18n`] and are filled in via positional `{}`
//! placeholders. Keep these functions side-effect free so the rest of the
//! command code stays focused on git work.

use crate::i18n::t;

pub(super) fn format_push_success(project: &str, branch: &str, target: &str) -> String {
    t("push_success")
        .replacen("{}", project, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", target, 1)
}

pub(super) fn format_gpush_skipped(project: &str, branch: &str, target: &str) -> String {
    t("push_skipped_no_commits")
        .replacen("{}", project, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", target, 1)
}

pub(super) fn format_gpush_local_behind(
    project: &str,
    branch: &str,
    behind: usize,
    target: &str,
) -> String {
    t("push_local_behind")
        .replacen("{}", project, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", &behind.to_string(), 1)
        .replacen("{}", target, 1)
}

pub(super) fn format_gswitch_success(
    project: &str,
    original: &str,
    branch: &str,
    target: &str,
) -> String {
    t("switch_success")
        .replacen("{}", project, 1)
        .replacen("{}", original, 1)
        .replacen("{}", branch, 1)
        .replacen("{}", target, 1)
}

pub(super) fn format_gmerge_success(
    project: &str,
    source: &str,
    target: &str,
    target_input: &str,
) -> String {
    t("merge_success")
        .replacen("{}", project, 1)
        .replacen("{}", source, 1)
        .replacen("{}", target, 1)
        .replacen("{}", target_input, 1)
}

pub(super) fn format_gmerge_skipped(
    project: &str,
    source: &str,
    target: &str,
    target_input: &str,
) -> String {
    t("merge_skipped_no_commits")
        .replacen("{}", project, 1)
        .replacen("{}", source, 1)
        .replacen("{}", target, 1)
        .replacen("{}", target_input, 1)
}

pub(super) fn format_gcreate_success(project: &str, new_branch: &str, start_point: &str) -> String {
    t("create_success")
        .replacen("{}", project, 1)
        .replacen("{}", new_branch, 1)
        .replacen("{}", start_point, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_push_success_includes_target_and_remote() {
        assert_eq!(
            format_push_success("api", "test-master", "test"),
            "api: pushed test-master -> origin/test-master (target: test)"
        );
    }

    #[test]
    fn test_format_gswitch_success_includes_original_resolved_and_target() {
        assert_eq!(
            format_gswitch_success("api", "main", "feature-api", "feature"),
            "api: switched main -> feature-api (target: feature)"
        );
    }

    #[test]
    fn test_format_gpush_local_behind_renders_behind_count() {
        let line = format_gpush_local_behind("api", "feature", 3, "feature");
        assert!(line.contains("api"));
        assert!(line.contains("feature"));
        assert!(line.contains('3'));
    }

    #[test]
    fn test_format_gmerge_skipped_mentions_branches() {
        let line = format_gmerge_skipped("api", "feature", "test", "test");
        assert!(line.contains("feature"));
        assert!(line.contains("test"));
    }
}
