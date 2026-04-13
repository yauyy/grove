use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::Cli;

pub fn run(shell: &str) -> Result<()> {
    let shell: Shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "ps" => Shell::PowerShell,
        other => anyhow::bail!(
            "Unknown shell: '{}'. Supported: bash, zsh, fish, powershell (or ps)",
            other
        ),
    };

    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut std::io::stdout());

    Ok(())
}
