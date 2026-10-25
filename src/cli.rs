use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "skill-sync", about = "Sync Claude skills from manifest", version)]
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

    /// Sync to global target (~/.claude/skills/) instead of local (.claude/skills/)
    #[arg(long)]
    pub global: bool,

    /// Show planned changes without executing (exit 2 if changes pending, 0 if up-to-date)
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts for CI usage
    #[arg(long)]
    pub force: bool,

    /// Sync to local target (.claude/skills/), mutually exclusive with --global
    #[arg(long, conflicts_with = "global")]
    pub local: bool,
}
