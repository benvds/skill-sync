use crate::error::SkillsSyncError;
use std::path::Path;

/// The documented sample .skills.toml template.
/// Tutorial-style with full header, usage section, format explanation,
/// and commented examples -- self-teaching for new users.
const INIT_TEMPLATE: &str = r#"# ============================================================================
# skills-sync manifest
# ============================================================================
#
# This file tells skills-sync which skill repos to install.
# Run `skills-sync` to sync all listed repos to your skills directory.
#
# -- Usage --------------------------------------------------------------------
#
#   skills-sync              Sync all repos listed below
#   skills-sync --dry-run    Preview changes without applying
#   skills-sync --verbose    Show detailed sync output
#   skills-sync --global     Sync to ~/.agents/skills/
#   skills-sync --target ~/.codex/skills  Use an explicit skills directory
#
# Skills are installed next to this file in an existing skills directory.
# From a project root, the target is .agents/skills/.
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
#   skills-sync detects the layout automatically.
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
        return Err(SkillsSyncError::ManifestAlreadyExists {
            path: path.to_owned(),
        }
        .into());
    }

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(SkillsSyncError::Io)?;
    }
    std::fs::write(path, INIT_TEMPLATE).map_err(SkillsSyncError::Io)?;

    eprintln!("Created {}", path.display());
    Ok(())
}
