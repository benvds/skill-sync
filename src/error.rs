use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum SkillsSyncError {
    #[error("manifest not found")]
    #[diagnostic(
        code(skills_sync::manifest::not_found),
        help("Run `skills-sync --init` to create a sample manifest")
    )]
    ManifestNotFound { searched: Vec<PathBuf> },

    #[error("invalid TOML syntax")]
    #[diagnostic(code(skills_sync::manifest::invalid_toml))]
    InvalidToml {
        #[source_code]
        src: miette::NamedSource<String>,
        #[label("syntax error here")]
        span: miette::SourceSpan,
    },

    #[error("invalid repo format: `{repo}`")]
    #[diagnostic(
        code(skills_sync::manifest::invalid_repo),
        help("Repos must match `owner/repo` format (e.g., `anthropics/claude-skills`)")
    )]
    InvalidRepo { repo: String },

    #[error("duplicate repo: `{repo}`")]
    #[diagnostic(
        code(skills_sync::manifest::duplicate_repo),
        help("Remove the duplicate entry from your manifest")
    )]
    DuplicateRepo { repo: String },

    #[error("manifest already exists: {}", path.display())]
    #[diagnostic(
        code(skills_sync::init::already_exists),
        help("Remove or rename the existing file first")
    )]
    ManifestAlreadyExists { path: PathBuf },

    #[error(transparent)]
    #[diagnostic(code(skills_sync::io))]
    Io(#[from] std::io::Error),
}

pub type Result<T> = miette::Result<T>;
