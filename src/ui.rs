use console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, MultiSelect, Select};

/// Custom theme with highlighted active selection and checkbox style
fn theme() -> ColorfulTheme {
    ColorfulTheme {
        active_item_style: Style::new().cyan().bold(),
        active_item_prefix: console::style("❯ ".to_string()).cyan().bold(),
        checked_item_prefix: console::style("[✓] ".to_string()).green(),
        unchecked_item_prefix: console::style("[ ] ".to_string()),
        ..ColorfulTheme::default()
    }
}

/// Prompt for text input with a default value.
pub fn input(prompt: &str, default: &str) -> anyhow::Result<String> {
    let value = Input::<String>::with_theme(&theme())
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?;
    Ok(value)
}

/// Prompt for optional text input. Returns None if the user enters nothing.
pub fn input_optional(prompt: &str, placeholder: &str) -> anyhow::Result<Option<String>> {
    let prompt_text = if placeholder.is_empty() {
        prompt.to_string()
    } else {
        format!("{} ({})", prompt, placeholder)
    };
    let value = Input::<String>::with_theme(&theme())
        .with_prompt(prompt_text)
        .default(String::new())
        .allow_empty(true)
        .interact_text()?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

/// Prompt for a single selection from a list. Returns the selected index.
/// Active item is highlighted with cyan bold.
pub fn select(prompt: &str, items: &[String]) -> anyhow::Result<usize> {
    let idx = Select::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?;
    Ok(idx)
}

/// Prompt for multi-selection from a list. Returns the selected indices.
/// Active item is highlighted, checked items show green checkmark.
pub fn multi_select(
    prompt: &str,
    items: &[String],
    defaults: &[bool],
) -> anyhow::Result<Vec<usize>> {
    let indices = MultiSelect::with_theme(&theme())
        .with_prompt(prompt)
        .items(items)
        .defaults(defaults)
        .interact()?;
    Ok(indices)
}

/// Prompt for a yes/no confirmation.
pub fn confirm(prompt: &str, default: bool) -> anyhow::Result<bool> {
    let result = Confirm::with_theme(&theme())
        .with_prompt(prompt)
        .default(default)
        .interact()?;
    Ok(result)
}

/// Print a success message with a green checkmark.
pub fn success(msg: &str) {
    let style = Style::new().green();
    println!("{} {}", style.apply_to("\u{2713}"), msg);
}

/// Print an error message with a red cross.
pub fn error(msg: &str) {
    let style = Style::new().red();
    eprintln!("{} {}", style.apply_to("\u{2717}"), msg);
}

/// Print an info message with a cyan info symbol.
pub fn info(msg: &str) {
    let style = Style::new().cyan();
    println!("{} {}", style.apply_to("\u{2139}"), msg);
}

/// Print a warning message with a yellow warning symbol.
pub fn warn(msg: &str) {
    let style = Style::new().yellow();
    println!("{} {}", style.apply_to("\u{26A0}"), msg);
}

/// Print a bold header.
pub fn header(msg: &str) {
    let style = Style::new().bold();
    println!("{}", style.apply_to(msg));
}

/// Print a batch operation summary showing succeeded and failed counts.
pub fn batch_summary(succeeded: usize, failed: usize) {
    let green = Style::new().green();
    let red = Style::new().red();
    println!(
        "{} succeeded, {} failed",
        green.apply_to(succeeded),
        red.apply_to(failed)
    );
}
