use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::Cli;

pub fn run(shell: &str) -> Result<()> {
    let shell_type: Shell = match shell.to_lowercase().as_str() {
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
    generate(shell_type, &mut cmd, &name, &mut std::io::stdout());

    // Append custom workspace completion for shells that support it
    match shell_type {
        Shell::Zsh => print_zsh_workspace_completion(&name),
        Shell::Bash => print_bash_workspace_completion(&name),
        Shell::Fish => print_fish_workspace_completion(&name),
        _ => {}
    }

    Ok(())
}

fn print_zsh_workspace_completion(name: &str) {
    println!(r#"
# Custom workspace completion for grove code/delete/-w
_grove_workspaces() {{
    local workspaces_file="$HOME/.grove/workspaces.toml"
    if [[ -f "$workspaces_file" ]]; then
        local -a ws_names
        ws_names=(${{(f)"$(grep '^name = ' "$workspaces_file" | sed 's/name = "//;s/"//')"}} )
        _describe 'workspace' ws_names
    fi
}}

# Override completion for commands that take workspace names
if (( $+functions[_{name}_commands] )); then
    _{name}_orig_commands=$functions[_{name}_commands]
fi
"#, name = name);
}

fn print_bash_workspace_completion(name: &str) {
    println!(r#"
# Custom workspace completion for grove code/delete/-w
_{name}_workspace_completions() {{
    local workspaces_file="$HOME/.grove/workspaces.toml"
    if [[ -f "$workspaces_file" ]]; then
        grep '^name = ' "$workspaces_file" | sed 's/name = "//;s/"//'
    fi
}}

# Hook into bash completion for code subcommand
__{name}_orig_complete=${{complete_function:-_{name}}}
_{name}_custom() {{
    local cur="${{COMP_WORDS[$COMP_CWORD]}}"
    local prev="${{COMP_WORDS[$COMP_CWORD-1]}}"
    if [[ "$prev" == "code" || "$prev" == "-w" || "$prev" == "--workspace" ]]; then
        COMPREPLY=($(compgen -W "$(_{name}_workspace_completions)" -- "$cur"))
        return
    fi
}}
"#, name = name);
}

fn print_fish_workspace_completion(name: &str) {
    println!(r#"
# Custom workspace completion for grove code
complete -c {name} -n "__fish_seen_subcommand_from code" -xa "(grep '^name = ' ~/.grove/workspaces.toml 2>/dev/null | sed 's/name = \"//;s/\"//')"
complete -c {name} -s w -l workspace -xa "(grep '^name = ' ~/.grove/workspaces.toml 2>/dev/null | sed 's/name = \"//;s/\"//')"
"#, name = name);
}
