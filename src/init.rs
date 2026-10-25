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
#   skill-sync --global     Sync the central user configuration
#   skill-sync --local      Sync only this project's manifest
#   skill-sync --target ~/.codex/skills  Use an explicit skills directory
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
    write_template(path, INIT_TEMPLATE)
}

pub fn run_init_global(path: &Path) -> crate::error::Result<()> {
    let settings =
        "[sync]\nscope = \"user\"\nagents = [\"codex\", \"claude-code\"]\nmode = \"copy\"\n\n";
    write_template(
        path,
        &INIT_TEMPLATE.replace(
            "[skills]\nrepos = [",
            &format!("{settings}[skills]\nrepos = ["),
        ),
    )
}

fn write_template(path: &Path, template: &str) -> crate::error::Result<()> {
    if path.exists() {
        return Err(SkillSyncError::ManifestAlreadyExists {
            path: path.to_owned(),
        }
        .into());
    }

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(SkillSyncError::Io)?;
    }
    std::fs::write(path, template).map_err(SkillSyncError::Io)?;

    eprintln!("Created {}", path.display());
    Ok(())
}
