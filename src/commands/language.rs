use anyhow::{bail, Result};

use crate::config;
use crate::i18n::t;
use crate::ui;

pub fn run(lang: &str) -> Result<()> {
    match lang {
        "en" | "zh" => {
            let mut global = config::load_global_config()?;
            global.language = lang.to_string();
            config::save_global_config(&global)?;
            ui::success(&t("language_set").replace("{}", lang));
            Ok(())
        }
        _ => {
            bail!("{}", t("language_invalid").replace("{}", lang));
        }
    }
}
