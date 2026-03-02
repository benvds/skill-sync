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

    #[error("GitHub API rate limit exceeded")]
    #[diagnostic(
        code(skills_sync::github::rate_limited),
        help("Rate limit hit: {remaining} requests remaining, resets at epoch {reset_epoch:?}. Authenticate with GITHUB_TOKEN or `gh auth login` for 5,000 req/hr (vs 60 unauthenticated).")
    )]
    RateLimited { remaining: u64, reset_epoch: Option<i64> },

    #[error("GitHub API error for `{repo}`: {status} {message}")]
    #[diagnostic(code(skills_sync::github::api_error))]
    GitHubApi { status: u16, message: String, repo: String },

    #[error("repository not found: `{repo}`")]
    #[diagnostic(
        code(skills_sync::github::repo_not_found),
        help("Check the repo name is correct and you have access. Private repos require a GITHUB_TOKEN.")
    )]
    RepoNotFound { repo: String },

    #[error("network error for `{repo}`: {reason}")]
    #[diagnostic(code(skills_sync::github::network))]
    NetworkError { repo: String, reason: String },

    #[error("HTTP client error: {0}")]
    #[diagnostic(code(skills_sync::github::http_client))]
    HttpClient(String),

    #[error(transparent)]
    #[diagnostic(code(skills_sync::io))]
    Io(#[from] std::io::Error),

    #[error("git command failed: `git {args}`")]
    #[diagnostic(
        code(skills_sync::sync::git_failed),
        help("{stderr}")
    )]
    GitFailed { args: String, stderr: String },

    #[error("git not found — ensure git is installed and in PATH")]
    #[diagnostic(
        code(skills_sync::sync::git_not_found),
        help("Install git: https://git-scm.com/downloads")
    )]
    GitNotFound { source: std::io::Error },

    #[error("failed to rename `{}`", path.display())]
    #[diagnostic(code(skills_sync::sync::rename_failed))]
    RenameFailed {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to remove skill at `{}`", path.display())]
    #[diagnostic(code(skills_sync::sync::remove_failed))]
    RemoveFailed {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("sync failed for `{repo}`: {reason}")]
    #[diagnostic(code(skills_sync::sync::failed))]
    SyncFailed { repo: String, reason: String },
}

pub type Result<T> = miette::Result<T>;
