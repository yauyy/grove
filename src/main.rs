mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "grove", about = "Multi-project git worktree workspace manager")]
pub struct Cli {
    /// Edit a workspace configuration (interactive if name omitted)
    #[arg(short = 'w', long = "workspace")]
    workspace: Option<Option<String>>,

    /// Create a shortcut for the current directory (interactive if name omitted)
    #[arg(short = 'c', long = "create")]
    create: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a project to the current workspace
    Add {
        /// Path to the project directory
        path: String,
    },

    /// Remove a project from the current workspace
    Remove,

    /// List all projects in the current workspace
    #[command(alias = "ls")]
    List,

    /// Manage project groups
    Group {
        #[command(subcommand)]
        action: GroupCommands,
    },

    /// Move to a project directory
    #[command(alias = "mv")]
    Move {
        /// Project name (interactive if omitted)
        project: Option<String>,
    },

    /// Create a new worktree branch for a project
    Create {
        /// Branch name (interactive if omitted)
        name: Option<String>,
    },

    /// Delete a worktree branch
    Delete,

    /// Show status of all projects
    #[command(alias = "st")]
    Status,

    /// Sync all projects (fetch + rebase)
    #[command(alias = "sy")]
    Sync,

    /// Merge current branch into main for all projects
    #[command(alias = "gm")]
    Gmerge,

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle top-level flags first
    if let Some(ref name) = cli.workspace {
        return commands::workspace_edit::run(name.clone());
    }

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
        Some(Commands::Create { ref name }) => commands::create::run(name.clone()),
        Some(Commands::Delete) => commands::delete::run(),
        Some(Commands::Status) => commands::status::run(),
        Some(Commands::Sync) => commands::sync::run(),
        Some(Commands::Gmerge) => commands::git_ops::gmerge(),
        Some(Commands::Gstatus) => commands::git_ops::gstatus(),
        Some(Commands::Gadd) => commands::git_ops::gadd(),
        Some(Commands::Gcommit) => commands::git_ops::gcommit(),
        Some(Commands::Gpush) => commands::git_ops::gpush(),
        Some(Commands::Gpull) => commands::git_ops::gpull(),
        Some(Commands::Config { action }) => match action {
            ConfigCommands::Set { ref key, ref value } => commands::config::set(key, value),
            ConfigCommands::List => commands::config::list(),
        },
        Some(Commands::Completion { ref shell }) => commands::completion::run(shell),
        None => {
            // No command given, print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
