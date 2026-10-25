use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "skill-sync",
    about = "Sync Agent Skills from a manifest",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Emit one JSON result on stdout; diagnostics remain on stderr
    #[arg(long, global = true)]
    pub json: bool,

    /// Generate a sample .skills.toml
    #[arg(long, global = true, conflicts_with = "dry_run")]
    pub init: bool,

    /// Path to config file
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Show resolved manifests, locks, scopes and target directories
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Sync only the central user configuration
    #[arg(long, global = true)]
    pub global: bool,

    /// Explicit skills directory (e.g. ~/.codex/skills/); conflicts with --global
    #[arg(long, global = true, value_name = "PATH", conflicts_with = "global")]
    pub target: Option<PathBuf>,

    /// Select agents (repeat or comma-separate); overrides [sync].agents
    #[arg(long = "agent", global = true, alias = "agents", value_delimiter = ',')]
    pub agents: Vec<String>,

    /// Require an unchanged portable source lock; cached commits work offline
    #[arg(long, global = true)]
    pub frozen: bool,

    /// Include file differences against the installed baseline; implies preview
    #[arg(long, global = true)]
    pub diff: bool,

    /// Maximum persistent repository cache size in MiB (0 disables retention)
    #[arg(long, global = true, default_value_t = 512)]
    pub cache_limit_mib: u32,

    /// Show planned changes without executing (exit 2 if changes pending, 0 if up-to-date)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Confirm bulk installations and managed removals for CI usage
    #[arg(long, global = true)]
    pub force: bool,

    /// Sync only the nearest project manifest
    #[arg(long, global = true, conflicts_with = "global")]
    pub local: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Inspect local installations without network access or filesystem changes
    #[command(alias = "doctor")]
    Status,
    /// Add a source without installing it; --dry-run previews the manifest edit
    Add { source: String },
    /// Remove an exact source selection; installed copies are reconciled by sync
    Remove { source: String },
    /// Create a portable source lock without changing installed copies
    Lock,
    /// Explicitly refresh the portable source lock to current source revisions
    Update,
}
