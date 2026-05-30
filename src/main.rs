mod branch_target;
mod commands;
mod config;
mod gcreate_records;
mod git;
mod i18n;
mod ui;
mod workspace;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "grove",
    version,
    about = "Multi-project git worktree workspace manager"
)]
pub struct Cli {
    /// Workspace operations: -w [create|remove|rename|status|code|edit|<name>]
    #[arg(short = 'w', long = "workspace", num_args = 0..=2)]
    workspace: Option<Vec<String>>,

    /// Create a workspace (shortcut for -w create)
    #[arg(short = 'c', long = "create")]
    create: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a project
    Add {
        /// Path to the project directory
        path: String,
    },

    /// Remove a project
    #[command(alias = "rm")]
    Remove,

    /// List all projects
    #[command(alias = "ls")]
    List,

    /// Manage project groups
    Group {
        #[command(subcommand)]
        action: GroupCommands,
    },

    /// Move a project to another group
    #[command(alias = "mv")]
    Move {
        /// Project name (interactive if omitted)
        project: Option<String>,
    },

    /// Sync all projects (fetch + merge)
    #[command(alias = "sy")]
    Sync,

    /// Merge current branch into environment for selected projects
    #[command(alias = "gm")]
    Gmerge {
        /// Target preset, alias, logical branch, or real branch
        target: Option<String>,
        /// After a successful merge, also push the target branch to origin
        #[arg(short = 'p', long = "push")]
        push: bool,
        /// Merge all workspace projects without the interactive multi-select
        #[arg(short = 'a', long = "all")]
        all: bool,
    },

    /// Rename branch for all projects in a workspace
    #[command(alias = "grn")]
    Grename,

    /// Show git status for all projects
    #[command(alias = "gs")]
    Gstatus,

    /// Stage changes in all projects
    #[command(alias = "ga")]
    Gadd,

    /// Commit staged changes in all projects
    #[command(alias = "gc")]
    Gcommit,

    /// Push all projects
    #[command(alias = "gp")]
    Gpush {
        /// Target preset, alias, logical branch, or real branch
        target: Option<String>,
    },

    /// Switch all projects to a target branch
    #[command(alias = "gsw")]
    Gswitch {
        /// Target preset, alias, logical branch, or real branch; omit to pick from gcreate records
        target: Option<String>,
    },

    /// Create and switch to a new branch in all projects
    #[command(alias = "gcr")]
    Gcreate {
        /// New branch name, git-prefix is applied if configured
        name: String,
    },

    /// List gcreate batch records across all workspaces
    #[command(alias = "gli")]
    Glist {
        /// Interactively delete branches from a selected gcreate record
        #[arg(long = "rm", conflicts_with = "rename")]
        rm: bool,
        /// Interactively rename branches from a selected gcreate record
        #[arg(long = "rename", conflicts_with = "rm")]
        rename: bool,
    },

    /// Remove local branches (multi-select gcreate records, or by exact branch name)
    Grm {
        /// Exact local branch name to delete in the current workspace
        branch: Option<String>,
    },

    /// Show current branch(es) for all projects in the workspace
    #[command(alias = "gbr")]
    Gbranch,

    /// Pull all projects
    #[command(alias = "gl")]
    Gpull,

    /// Generate/update go.work for the current workspace
    #[command(alias = "gw")]
    Gowork,

    /// Manage individual project worktrees in the current workspace
    #[command(alias = "gwt")]
    Worktree {
        #[command(subcommand)]
        action: WorktreeCommands,
    },

    /// Auto-detect and update project tags
    Tags,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Generate shell completions
    Completion {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        shell: String,
    },

    /// Set display language (en/zh)
    Language {
        /// Language code: en or zh
        lang: String,
    },
}

#[derive(Subcommand)]
enum GroupCommands {
    /// Add a project to a group
    Add {
        /// Group name
        name: String,
    },
    /// Remove a project from a group
    Remove,
    /// List all groups
    List,
    /// Reorder projects within a group
    Reorder,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// List all configuration values
    List,
    /// Open a config file in your editor
    Edit {
        /// File to edit: projects, config, workspaces (default: projects)
        file: Option<String>,
    },
    /// Manage environment branch presets
    Preset {
        #[command(subcommand)]
        action: PresetCommands,
    },
}

#[derive(Subcommand)]
enum PresetCommands {
    /// Add or update an environment preset
    Set {
        /// Preset name (e.g. test, staging, prod, gray)
        name: String,
        /// Human-readable description shown in the merge menu
        description: String,
    },
    /// Remove an environment preset
    #[command(alias = "remove")]
    Rm {
        /// Preset name to remove
        name: String,
    },
    /// List configured environment presets
    List,
}

#[derive(Subcommand)]
enum WorktreeCommands {
    /// List worktrees for each project in the current workspace
    #[command(alias = "ls")]
    List,
    /// Add a project's worktree to the current workspace
    Add {
        /// Project name (interactive if omitted)
        project: Option<String>,
    },
    /// Remove a project's worktree from the current workspace
    #[command(alias = "remove")]
    Rm {
        /// Project name (interactive if omitted)
        project: Option<String>,
        /// Discard uncommitted changes when removing
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Prune stale worktree administrative entries
    Prune,
    /// Repair worktree links after moving directories
    Repair,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle -w (workspace operations)
    if let Some(ref args) = cli.workspace {
        return match args.first().map(|s| s.as_str()) {
            Some("create") | Some("c") => commands::create::run(args.get(1).cloned()),
            Some("remove") | Some("rm") => commands::delete::run(),
            Some("rename") | Some("rn") => commands::rename::run(),
            Some("status") | Some("st") => commands::status::run(),
            Some("code") => commands::code::run(args.get(1).cloned()),
            Some("edit") => commands::workspace_edit::run(args.get(1).cloned()),
            Some(name) => commands::workspace_edit::run(Some(name.to_string())),
            None => commands::workspace_edit::run(None),
        };
    }

    // Handle -c (shortcut for -w create)
    if let Some(ref name) = cli.create {
        return commands::create::run(name.clone());
    }

    match cli.command {
        Some(Commands::Add { ref path }) => commands::add::run(path),
        Some(Commands::Remove) => commands::remove::run(),
        Some(Commands::List) => commands::list::run(),
        Some(Commands::Group { action }) => match action {
            GroupCommands::Add { ref name } => commands::group::add(name),
            GroupCommands::Remove => commands::group::remove(),
            GroupCommands::List => commands::group::list(),
            GroupCommands::Reorder => commands::group::reorder(),
        },
        Some(Commands::Move { ref project }) => commands::mov::run(project.clone()),
        Some(Commands::Sync) => commands::sync::run(),
        Some(Commands::Gmerge {
            ref target,
            push,
            all,
        }) => commands::git_ops::gmerge(target.clone(), push, all),
        Some(Commands::Grename) => commands::rename::grename(),
        Some(Commands::Gstatus) => commands::git_ops::gstatus(),
        Some(Commands::Gadd) => commands::git_ops::gadd(),
        Some(Commands::Gcommit) => commands::git_ops::gcommit(),
        Some(Commands::Gpush { ref target }) => commands::git_ops::gpush(target.clone()),
        Some(Commands::Gswitch { ref target }) => {
            commands::git_ops::gswitch(target.as_deref())
        }
        Some(Commands::Gcreate { ref name }) => commands::git_ops::gcreate(name),
        Some(Commands::Glist { rm, rename }) => commands::glist::run(rm, rename),
        Some(Commands::Grm { ref branch }) => commands::grm::run(branch.as_deref()),
        Some(Commands::Gbranch) => commands::gbranch::run(),
        Some(Commands::Gpull) => commands::git_ops::gpull(),
        Some(Commands::Gowork) => commands::gowork::run(),
        Some(Commands::Worktree { action }) => match action {
            WorktreeCommands::List => commands::worktree::list(),
            WorktreeCommands::Add { ref project } => commands::worktree::add(project.clone()),
            WorktreeCommands::Rm { ref project, force } => {
                commands::worktree::remove(project.clone(), force)
            }
            WorktreeCommands::Prune => commands::worktree::prune(),
            WorktreeCommands::Repair => commands::worktree::repair(),
        },
        Some(Commands::Tags) => commands::tags::run(),
        Some(Commands::Config { action }) => match action {
            ConfigCommands::Set { ref key, ref value } => commands::config::set(key, value),
            ConfigCommands::List => commands::config::list(),
            ConfigCommands::Edit { ref file } => commands::config::edit(file.as_deref()),
            ConfigCommands::Preset { action } => match action {
                PresetCommands::Set {
                    ref name,
                    ref description,
                } => commands::config::preset_set(name, description),
                PresetCommands::Rm { ref name } => commands::config::preset_rm(name),
                PresetCommands::List => commands::config::preset_list(),
            },
        },
        Some(Commands::Completion { ref shell }) => commands::completion::run(shell),
        Some(Commands::Language { ref lang }) => commands::language::run(lang),
        None => {
            // No command given, print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
