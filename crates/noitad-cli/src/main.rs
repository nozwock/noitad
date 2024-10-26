mod cli;

use std::path::PathBuf;

use clap::Parser;
use cli::NoitdCli;
use color_eyre::{
    eyre::{bail, ContextCompat, Result},
    owo_colors::OwoColorize,
};
use itertools::Itertools;
use noitad_lib::{config::Config, defines::APP_CONFIG_DIR, log::RotatingWriter};
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn get_save_dir(cfg: &Config) -> Result<PathBuf> {
    cfg.noita_path
        .save_dir()
        .context("Couldn't find Noita's save directory.")
}

fn main() -> Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(RotatingWriter::new(
        3,
        APP_CONFIG_DIR.join("logs"),
        "noitd.log",
    )?);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env())
        .init();

    let cli = NoitdCli::parse();
    let mut cfg = Config::load()?;
    debug!(?cfg);

    match cli.command {
        cli::Command::Add { profile } => {
            cfg.profiles.add_profile(&profile, get_save_dir(&cfg)?)?;
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
        cli::Command::Edit { profile } => todo!(),
    };

    Ok(())
}
