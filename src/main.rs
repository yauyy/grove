mod commands;
mod config;
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

    /// Merge current branch into environment for all projects
    #[command(alias = "gm")]
    Gmerge,

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
    Gpush,

    /// Pull all projects
    #[command(alias = "gl")]
    Gpull,

    /// Generate/update go.work for the current workspace
    #[command(alias = "gw")]
    Gowork,

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
        Some(Commands::Gmerge) => commands::git_ops::gmerge(),
        Some(Commands::Grename) => commands::rename::grename(),
        Some(Commands::Gstatus) => commands::git_ops::gstatus(),
        Some(Commands::Gadd) => commands::git_ops::gadd(),
        Some(Commands::Gcommit) => commands::git_ops::gcommit(),
        Some(Commands::Gpush) => commands::git_ops::gpush(),
        Some(Commands::Gpull) => commands::git_ops::gpull(),
        Some(Commands::Gowork) => commands::gowork::run(),
        Some(Commands::Tags) => commands::tags::run(),
        Some(Commands::Config { action }) => match action {
            ConfigCommands::Set { ref key, ref value } => commands::config::set(key, value),
            ConfigCommands::List => commands::config::list(),
            ConfigCommands::Edit { ref file } => commands::config::edit(file.as_deref()),
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
