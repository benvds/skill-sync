use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "skills-sync",
    about = "Sync Agent Skills from a manifest",
    version
)]
pub struct Cli {
    /// Generate a sample .skills.toml
    #[arg(long)]
    pub init: bool,

    /// Path to config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Show verbose output including SHA comparisons and git command output
    #[arg(long)]
    pub verbose: bool,

    /// Sync to ~/.agents/skills/ instead of detecting the target from the manifest
    #[arg(long)]
    pub global: bool,

    /// Explicit skills directory (e.g. ~/.codex/skills/); conflicts with --global
    #[arg(long, value_name = "PATH", conflicts_with = "global")]
    pub target: Option<PathBuf>,

    /// Show planned changes without executing (exit 2 if changes pending, 0 if up-to-date)
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts for CI usage
    #[arg(long)]
    pub force: bool,

    /// Detect the target from the manifest directory (default); conflicts with --global
    #[arg(long, conflicts_with = "global")]
    pub local: bool,
}
