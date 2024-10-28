use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    author,
    version,
    about = "Noita mod manager",
    arg_required_else_help = true
)]
pub struct NoitdCli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Add a mod profile
    #[command()]
    Add {
        #[arg()]
        profile: String,
    },
    /// Remove an existing mod profile
    #[command(alias = "rm")]
    Remove {
        #[arg()]
        profile: String,
    },
    /// List all existing mod profiles
    #[command(alias = "ls")]
    List,
    /// Switch to an existing mod profile
    #[command()]
    Switch {
        #[arg()]
        profile: String,
    },
    /// Edit an existing mod profile
    #[command()]
    Edit {
        #[arg(short, long)]
        profile: Option<String>,
    },
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        command: Option<ConfigCommand>,
        /// Print the path to the config file
        #[arg(short, long)]
        path: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    /// Set the path to the game's location
    #[command()]
    NoitaPath,
}
