mod cli;
mod utils;

use std::{fmt, path::PathBuf};

use clap::Parser;
use cli::NoitdCli;
use color_eyre::{
    eyre::{bail, ContextCompat, Result},
    owo_colors::OwoColorize,
};
use inquire::MultiSelect;
use itertools::Itertools;
use noitad_lib::{
    config::Config, defines::APP_CONFIG_DIR, log::RotatingWriter, noita::mod_config::Mods,
};
use tracing::debug;
use tracing_subscriber::{prelude::*, EnvFilter};
use utils::group_equal_by_key;

fn get_save_dir(cfg: &Config) -> Result<PathBuf> {
    cfg.noita_path
        .save_dir()
        .context("Couldn't find Noita's save directory.")
}

macro_rules! exit_on_err {
    ($res:expr) => {{
        match $res {
            Ok(value) => value,
            Err(_) => std::process::exit(1),
        }
    }};
}

fn main() -> Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(RotatingWriter::new(
        3,
        APP_CONFIG_DIR.join("logs"),
        "noitd.log",
    )?);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env())
        .init();

    let cli = NoitdCli::parse();
    let mut cfg = Config::load()?;
    debug!(?cfg);

    match cli.command {
        cli::Command::Add { profile } => {
            cfg.profiles.add_profile(&profile, get_save_dir(&cfg)?)?;
            if cfg.active_profile.is_none() {
                cfg.active_profile = Some(profile.to_owned());
            }
            cfg.store()?;
            eprintln!("Added profile '{}'", profile);
        }
        cli::Command::Remove { profile } => {
            if cfg.active_profile.as_ref() == Some(&profile) {
                bail!("Cannot remove an active profile")
            }
            cfg.profiles.remove_profile(&profile)?;
            cfg.store()?;
            eprintln!("Removed profile '{}'", profile);
        }
        cli::Command::List => {
            if cfg.profiles.keys().len() == 0 {
                bail!("No profiles available")
            }
            println!(
                "{}",
                cfg.profiles
                    .keys()
                    .into_iter()
                    .map(|s| if cfg.active_profile.as_ref() == Some(s) {
                        format!("* {}", s.green())
                    } else {
                        format!("  {}", s)
                    })
                    .join("\n")
            );
        }
        cli::Command::Switch { profile } => {
            let noita_save_dir = get_save_dir(&cfg)?;
            let mut mod_list = cfg.profiles.get_profile(&profile)?;

            if cfg.active_profile_sync {
                mod_list.sync_with_noita(&noita_save_dir)?;
            }
            mod_list.overwrite_noita_mod_list(&noita_save_dir)?;
            cfg.active_profile = Some(profile.to_owned());

            cfg.store()?;

            eprintln!("Switched to profile '{}'", profile);
        }
        cli::Command::Edit { mut profile } => {
            if profile.is_none() {
                profile = cfg.active_profile.clone();
            }
            let profile = profile.context("No profile is available for edit")?;

            let mut mod_list = cfg.profiles.get_profile(&profile)?;
            let noita_save_dir = get_save_dir(&cfg)?;
            if cfg.active_profile_sync {
                mod_list.sync_with_noita(&noita_save_dir)?;
            }

            let (content, enabled) = ModsDisplay::get_vec_from(&mod_list);

            let selected = exit_on_err!(MultiSelect::new(">", content)
                .with_default(&enabled)
                .with_formatter(&|it| format!("{} mods are enabled in total", it.len()))
                .prompt());

            let dup_modids = group_equal_by_key(&selected, |it| it.1)
                .values()
                .filter(|it| it.len() > 1)
                .map(|it| it[0].1)
                .collect_vec();

            if dup_modids.len() > 0 {
                bail!(
                    "Multiple mods with same id were enabled:\n{}",
                    dup_modids.into_iter().join("\n")
                )
            }

            for i in selected
                .into_iter()
                .map(|it| it.0)
                .collect_vec() // T_T
                .into_iter()
            {
                mod_list.mods[i].enabled = true;
            }

            cfg.store()?;

            if cfg.active_profile.as_ref() == Some(&profile) {
                mod_list.overwrite_noita_mod_list(&noita_save_dir)?;
            }
        }
    };

    Ok(())
}

/// All this because inquire wouldn't let me just let me give it a closure where I can return a string from the vec's items.
#[derive(Debug, Clone)]
struct ModsDisplay<'a>(usize, &'a str, bool);

impl<'a> ModsDisplay<'a> {
    fn get_vec_from(value: &'a Mods) -> (Vec<Self>, Vec<usize>) {
        let mut content = vec![];
        let mut enabled = vec![];

        for (i, mod_) in value.mods.iter().enumerate() {
            content.push(Self(i, mod_.name.as_str(), mod_.workshop_item_id == 0));
            if mod_.enabled {
                enabled.push(i);
            }
        }

        (content, enabled)
    }
}

impl fmt::Display for ModsDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{} ({})",
            self.1,
            if self.2 { "Local" } else { "Steam" }
        ))
    }
}
