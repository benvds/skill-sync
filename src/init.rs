use crate::error::SkillSyncError;
use std::path::Path;

/// The documented sample .skills.toml template.
/// Tutorial-style with full header, usage section, format explanation,
/// and commented examples -- self-teaching for new users.
const INIT_TEMPLATE: &str = r#"# ============================================================================
# skill-sync manifest
# ============================================================================
#
# This file tells skill-sync which skill repos to install.
# Run `skill-sync` to sync all listed repos to your skills directory.
#
# -- Usage --------------------------------------------------------------------
#
#   skill-sync              Sync all repos listed below
#   skill-sync --dry-run    Preview changes without applying
#   skill-sync --verbose    Show detailed sync output
#   skill-sync --global     Sync to ~/.claude/skills/ instead of local
#
# -- Format -------------------------------------------------------------------
#
#   Repos use "owner/repo" format (GitHub repos):
#
#     [skills]
#     repos = [
#         "owner/single-skill-repo",    # repo with SKILL.md at root
#         "owner/multi-skill-monorepo", # repo with skill subdirectories
#     ]
#
#   Single-skill repos have a SKILL.md at the repository root.
#   Monorepos contain subdirectories, each with their own SKILL.md.
#   skill-sync detects the layout automatically.
#
#   For monorepo skills, specify the subdirectory path:
#
#     [skills]
#     repos = [
#         "anthropics/skills/skills/pdf",     # specific skill in a monorepo
#         "anthropics/skills/skills/docx",     # another skill from same monorepo
#     ]
#
# -- Examples -----------------------------------------------------------------
#
#   Uncomment the repos below to try them, or add your own:
#
#     # "anthropics/claude-skills"                    # single-skill repo
#     # "example-org/skill-collection"                # monorepo (auto-detect)
#     # "anthropics/skills/skills/skill-creator",     # specific monorepo skill
#
# ============================================================================

[skills]
repos = [
    # Add your skill repos here, one per line:
    # "owner/repo",
]
"#;

/// Generate a sample .skills.toml at the given path.
///
/// Returns an error if the file already exists (refuses to overwrite).
pub fn run_init(path: &Path) -> crate::error::Result<()> {
    if path.exists() {
        return Err(SkillSyncError::ManifestAlreadyExists {
            path: path.to_owned(),
        }
        .into());
    }

    std::fs::write(path, INIT_TEMPLATE).map_err(SkillSyncError::Io)?;

    eprintln!("Created {}", path.display());
    Ok(())
}
