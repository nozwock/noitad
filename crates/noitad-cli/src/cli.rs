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
    #[command()]
    Remove {
        #[arg()]
        profile: String,
    },
    /// List all existing mod profiles
    #[command()]
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
        #[arg()]
        profile: String,
    },
}
