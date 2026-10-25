use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "skill-sync",
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

    /// Show resolved manifests, locks, scopes and target directories
    #[arg(long)]
    pub verbose: bool,

    /// Sync only the central user configuration
    #[arg(long)]
    pub global: bool,

    /// Explicit skills directory (e.g. ~/.codex/skills/); conflicts with --global
    #[arg(long, value_name = "PATH", conflicts_with = "global")]
    pub target: Option<PathBuf>,

    /// Select agents (repeat or comma-separate); overrides [sync].agents
    #[arg(long = "agent", alias = "agents", value_delimiter = ',')]
    pub agents: Vec<String>,

    /// Show planned changes without executing (exit 2 if changes pending, 0 if up-to-date)
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm bulk installations and managed removals for CI usage
    #[arg(long)]
    pub force: bool,

    /// Sync only the nearest project manifest
    #[arg(long, conflicts_with = "global")]
    pub local: bool,
}
