use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "skills-sync", about = "Sync Claude skills from manifest")]
pub struct Cli {
    /// Generate a sample .skills.toml
    #[arg(long)]
    pub init: bool,

    /// Path to config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Show verbose output
    #[arg(long)]
    pub verbose: bool,
}
