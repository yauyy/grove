use std::collections::HashMap;

use crate::config;

/// Get current language from config, fallback to "en"
pub fn current_lang() -> String {
    config::load_global_config()
        .map(|c| c.language)
        .unwrap_or_else(|_| "en".to_string())
}

/// Get a translated string by key
pub fn t(key: &str) -> String {
    let lang = current_lang();
    let translations = get_translations(&lang);
    translations
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_else(|| key.to_string())
}

fn get_translations(lang: &str) -> HashMap<&'static str, &'static str> {
    match lang {
        "zh" => zh(),
        _ => en(),
    }
}

fn en() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();

    // grove add
    m.insert("fetching_remote", "Fetching remote branches...");
    m.insert("project_name", "Project name");
    m.insert("select_group", "Select group");
    m.insert("new_group", "+ New group");
    m.insert("ungrouped", "Ungrouped");
    m.insert("group_name", "Group name");
    m.insert("main_branch", "Main branch");
    m.insert("test_branch", "Test branch (leave empty to skip)");
    m.insert("staging_branch", "Staging branch (leave empty to skip)");
    m.insert("prod_branch", "Prod branch (leave empty to skip)");
    m.insert("agents_md_path", "Path to agents.md");
    m.insert("press_enter_skip", "press Enter to skip");
    m.insert("field_required", "This field is required.");
    m.insert("skipped", "Skipped");
    m.insert("branch_exists", "origin/{} exists");
    m.insert("branch_not_found", "origin/{} not found on remote");
    m.insert("continue_branch", "Continue with this branch anyway?");
    m.insert("continue_anyway", "Continue anyway?");
    m.insert("project_added", "Added project '{}'");
    m.insert("project_already_registered", "Project already registered: {}");
    m.insert("project_name_exists", "A project named '{}' already exists");
    m.insert("not_git_repo", "Not a git repository: {}");
    m.insert("path_not_exist", "Path does not exist: {}");
    m.insert("configure_agents", "Configure agents.md for this project?");

    // grove list
    m.insert("no_projects", "No projects registered. Use 'grove add <path>' to register a project.");

    // grove remove
    m.insert("select_project_remove", "Select project to remove");
    m.insert("project_in_workspaces", "Project '{}' is used in workspaces: {}");
    m.insert("remove_anyway", "Remove anyway?");
    m.insert("cancelled", "Cancelled.");
    m.insert("project_removed", "Project '{}' removed.");

    // grove group
    m.insert("group_exists", "Group '{}' already exists.");
    m.insert("group_created", "Group '{}' created.");
    m.insert("no_groups", "No groups to remove.");
    m.insert("select_group_remove", "Select group to remove");
    m.insert("projects_become_ungrouped", "{} project(s) in this group will become ungrouped.");
    m.insert("group_removed", "Group '{}' removed.");
    m.insert("groups_reordered", "Groups reordered.");
    m.insert("move_which_group", "Move which group?");
    m.insert("move_to_position", "Move to position");

    // grove create
    m.insert("workspace_name", "Workspace name");
    m.insert("select_projects", "Select projects (space to toggle, enter to confirm)");
    m.insert("branch_name", "Branch name");
    m.insert("creating_worktrees", "Creating worktrees...");
    m.insert("agents_generated", "AGENTS.md generated.");
    m.insert("workspace_created", "Workspace created at: {}");
    m.insert("no_projects_registered", "No projects registered. Use 'grove add <path>' first.");
    m.insert("workspace_exists", "Workspace '{}' already exists.");
    m.insert("no_projects_selected", "No projects selected.");

    // grove delete
    m.insert("no_workspaces", "No workspaces to delete.");
    m.insert("select_workspace_delete", "Select workspace to delete");
    m.insert("has_uncommitted", "{}: has uncommitted changes");
    m.insert("delete_with_changes", "There are uncommitted changes. Delete anyway?");
    m.insert("cleaning_up", "Cleaning up...");
    m.insert("worktree_removed", "{}: worktree removed");
    m.insert("branch_deleted", "{}: branch '{}' deleted");
    m.insert("directory_removed", "Directory removed: {}");
    m.insert("workspace_deleted", "Workspace '{}' deleted.");

    // grove rename (workspace)
    m.insert("new_workspace_name", "New workspace name");
    m.insert("workspace_name_exists", "Workspace name '{}' already exists.");
    m.insert("rename_branch_too", "Rename corresponding branch as well?");
    m.insert("ws_rename_success", "Workspace '{}' renamed to '{}'.");
    m.insert("ws_rename_branch_failed", "{}: failed to rename branch: {}");

    // grove grename (branch)
    m.insert("new_branch_name", "New branch name");
    m.insert("branch_already_exists", "Branch '{}' already exists in project '{}'");
    m.insert("rename_confirm", "Rename branch from '{}' to '{}' in {} project(s)?");
    m.insert("workspace_branch_renamed", "Workspace '{}' branch renamed to '{}'");

    // grove delete (branch prompt)
    m.insert("delete_local_branch", "Delete local branch '{}'? (remote branches will not be affected)");

    // grove status
    m.insert("no_workspaces_status", "No workspaces. Create one with 'grove create'.");
    m.insert("clean", "clean");
    m.insert("changes", "{} changes");
    m.insert("missing", "missing");

    // batch git
    m.insert("select_workspace", "Select a workspace");
    m.insert("no_workspaces_found", "No workspaces found. Create one with 'grove create'.");
    m.insert("commit_message", "Commit message");
    m.insert("nothing_to_commit", "{}: nothing to commit");
    m.insert("merge_to_env", "Merge to which environment?");
    m.insert("no_common_env", "No common environment branch configured across all workspace projects.");
    m.insert("missing_envs", "{}: missing environments: {}");

    // workspace edit
    m.insert("no_workspaces_edit", "No workspaces found. Create one with 'grove create'.");
    m.insert("select_workspace_edit", "Select workspace to edit");
    m.insert("edit_projects", "Edit project selection (space to toggle, enter to confirm)");
    m.insert("no_changes", "No changes.");
    m.insert("uncommitted_changes", "Project '{}' has uncommitted changes. Commit or stash before removing.");

    // grove move
    m.insert("select_project_move", "Select project to move");
    m.insert("move_to_group", "Move to group");
    m.insert("project_moved", "Project '{}' moved to '{}'.");

    // grove config
    m.insert("config_edit_opening", "Opening {} with {}");
    m.insert("config_edited", "{} edited successfully");
    m.insert("workpath_forward_only", "Note: changing workpath is forward-only. Existing workspaces remain at their original location.");

    // grove code
    m.insert("select_workspace_code", "Select workspace to open");
    m.insert("opening_editor", "Opening {} in editor...");

    // grove language
    m.insert("language_set", "Language set to: {}");
    m.insert("language_invalid", "Invalid language: '{}'. Valid options: en, zh");

    // placeholders
    m.insert("placeholder_workspace_name", "enter workspace name");
    m.insert("placeholder_branch_name", "defaults to workspace name");
    m.insert("placeholder_project_name", "e.g. my-project");
    m.insert("placeholder_group_name", "e.g. frontend");

    m
}

fn zh() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();

    // grove add
    m.insert("fetching_remote", "正在获取远程分支...");
    m.insert("project_name", "项目名称");
    m.insert("select_group", "选择分组");
    m.insert("new_group", "+ 新建分组");
    m.insert("ungrouped", "未分组");
    m.insert("group_name", "分组名称");
    m.insert("main_branch", "主分支");
    m.insert("test_branch", "测试分支（留空跳过）");
    m.insert("staging_branch", "预发分支（留空跳过）");
    m.insert("prod_branch", "正式分支（留空跳过）");
    m.insert("agents_md_path", "agents.md 路径");
    m.insert("press_enter_skip", "按回车跳过");
    m.insert("field_required", "此字段为必填项。");
    m.insert("skipped", "已跳过");
    m.insert("branch_exists", "origin/{} 存在");
    m.insert("branch_not_found", "origin/{} 在远程不存在");
    m.insert("continue_branch", "是否继续使用此分支？");
    m.insert("continue_anyway", "是否继续？");
    m.insert("project_added", "项目 '{}' 添加成功");
    m.insert("project_already_registered", "项目已注册: {}");
    m.insert("project_name_exists", "项目名 '{}' 已存在");
    m.insert("not_git_repo", "不是 Git 仓库: {}");
    m.insert("path_not_exist", "路径不存在: {}");
    m.insert("configure_agents", "是否为此项目配置 agents.md？");

    // grove list
    m.insert("no_projects", "暂无已注册的项目。使用 'grove add <路径>' 注册项目。");

    // grove remove
    m.insert("select_project_remove", "选择要移除的项目");
    m.insert("project_in_workspaces", "项目 '{}' 正在被以下工作区使用: {}");
    m.insert("remove_anyway", "是否仍然移除？");
    m.insert("cancelled", "已取消。");
    m.insert("project_removed", "项目 '{}' 已移除。");

    // grove group
    m.insert("group_exists", "分组 '{}' 已存在。");
    m.insert("group_created", "分组 '{}' 创建成功。");
    m.insert("no_groups", "暂无分组可删除。");
    m.insert("select_group_remove", "选择要删除的分组");
    m.insert("projects_become_ungrouped", "该分组中有 {} 个项目将变为未分组。");
    m.insert("group_removed", "分组 '{}' 已删除。");
    m.insert("groups_reordered", "分组已重新排序。");
    m.insert("move_which_group", "移动哪个分组？");
    m.insert("move_to_position", "移动到位置");

    // grove create
    m.insert("workspace_name", "工作区名称");
    m.insert("select_projects", "选择项目（空格勾选，回车确认）");
    m.insert("branch_name", "分支名称");
    m.insert("creating_worktrees", "正在创建工作区...");
    m.insert("agents_generated", "AGENTS.md 已生成。");
    m.insert("workspace_created", "工作区已创建: {}");
    m.insert("no_projects_registered", "暂无已注册的项目。请先使用 'grove add <路径>'。");
    m.insert("workspace_exists", "工作区 '{}' 已存在。");
    m.insert("no_projects_selected", "未选择任何项目。");

    // grove delete
    m.insert("no_workspaces", "暂无工作区可删除。");
    m.insert("select_workspace_delete", "选择要删除的工作区");
    m.insert("has_uncommitted", "{}: 存在未提交的更改");
    m.insert("delete_with_changes", "存在未提交的更改，是否仍然删除？");
    m.insert("cleaning_up", "正在清理...");
    m.insert("worktree_removed", "{}: 工作区已移除");
    m.insert("branch_deleted", "{}: 分支 '{}' 已删除");
    m.insert("directory_removed", "目录已删除: {}");
    m.insert("workspace_deleted", "工作区 '{}' 已删除。");

    // grove rename (workspace)
    m.insert("new_workspace_name", "新工作区名称");
    m.insert("workspace_name_exists", "工作区名称 '{}' 已存在。");
    m.insert("rename_branch_too", "是否同时重命名对应分支？");
    m.insert("ws_rename_success", "工作区 '{}' 已重命名为 '{}'。");
    m.insert("ws_rename_branch_failed", "{}: 分支重命名失败: {}");

    // grove grename (branch)
    m.insert("new_branch_name", "新分支名称");
    m.insert("branch_already_exists", "分支 '{}' 在项目 '{}' 中已存在");
    m.insert("rename_confirm", "是否将 {} 个项目的分支从 '{}' 重命名为 '{}'？");
    m.insert("workspace_branch_renamed", "工作区 '{}' 的分支已重命名为 '{}'");

    // grove delete (branch prompt)
    m.insert("delete_local_branch", "是否同时删除本地分支 '{}'？（该操作不会删除远程分支）");

    // grove status
    m.insert("no_workspaces_status", "暂无工作区。使用 'grove create' 创建。");
    m.insert("clean", "干净");
    m.insert("changes", "{} 处更改");
    m.insert("missing", "缺失");

    // batch git
    m.insert("select_workspace", "选择一个工作区");
    m.insert("no_workspaces_found", "暂无工作区。使用 'grove create' 创建。");
    m.insert("commit_message", "提交信息");
    m.insert("nothing_to_commit", "{}: 没有需要提交的内容");
    m.insert("merge_to_env", "合并到哪个环境？");
    m.insert("no_common_env", "所有工作区项目中没有共同配置的环境分支。");
    m.insert("missing_envs", "{}: 缺少环境配置: {}");

    // workspace edit
    m.insert("no_workspaces_edit", "暂无工作区。使用 'grove create' 创建。");
    m.insert("select_workspace_edit", "选择要编辑的工作区");
    m.insert("edit_projects", "编辑项目选择（空格勾选，回车确认）");
    m.insert("no_changes", "无变更。");
    m.insert("uncommitted_changes", "项目 '{}' 存在未提交的更改。请先提交或暂存。");

    // grove move
    m.insert("select_project_move", "选择要移动的项目");
    m.insert("move_to_group", "移动到分组");
    m.insert("project_moved", "项目 '{}' 已移动到 '{}'。");

    // grove config
    m.insert("config_edit_opening", "正在使用 {} 打开 {}");
    m.insert("config_edited", "{} 编辑完成");
    m.insert("workpath_forward_only", "注意：更改工作区路径仅对后续新建的工作区生效。");

    // grove code
    m.insert("select_workspace_code", "选择要打开的工作区");
    m.insert("opening_editor", "正在打开 {}...");

    // grove language
    m.insert("language_set", "语言已设置为: {}");
    m.insert("language_invalid", "无效的语言: '{}'。可选项: en, zh");

    // placeholders
    m.insert("placeholder_workspace_name", "请输入工作区名称");
    m.insert("placeholder_branch_name", "默认与工作区同名");
    m.insert("placeholder_project_name", "例如 my-project");
    m.insert("placeholder_group_name", "例如 frontend");

    m
}
