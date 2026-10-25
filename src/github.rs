use crate::error::SkillSyncError;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;

/// How the GitHub token was discovered.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    /// Token from GITHUB_TOKEN environment variable.
    EnvVar,
    /// Token from `gh auth token` CLI command.
    GhCli,
    /// No token found — unauthenticated (60 req/hr limit).
    Unauthenticated,
}

/// Discover a GitHub token using the priority chain:
/// 1. GITHUB_TOKEN env var
/// 2. `gh auth token` CLI
/// 3. Unauthenticated
pub fn discover_token() -> (Option<String>, AuthMethod) {
    // 1. Environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return (Some(token), AuthMethod::EnvVar);
        }
    }

    // 2. gh CLI
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return (Some(token), AuthMethod::GhCli);
            }
        }
    }

    // 3. Unauthenticated
    (None, AuthMethod::Unauthenticated)
}

/// A single entry in a Git tree.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TreeEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub sha: String,
}

/// Response from the GitHub Trees API.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TreeResponse {
    pub sha: String,
    pub tree: Vec<TreeEntry>,
    pub truncated: bool,
}

/// Basic repository information from the GitHub API.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RepoInfo {
    pub default_branch: String,
}

/// GitHub API client with authentication and rate limit handling.
pub struct GitHubClient {
    client: Client,
    base_url: String,
    token: Option<String>,
    pub auth_method: AuthMethod,
}

impl GitHubClient {
    /// Create a new GitHub API client.
    ///
    /// Sets User-Agent to "skill-sync/{version}" to avoid 403 responses.
    pub fn new(token: Option<String>, auth_method: AuthMethod) -> Result<Self, reqwest::Error> {
        Self::with_base_url(token, auth_method, "https://api.github.com")
    }

    /// Alternate API endpoint for embedding and deterministic integration tests.
    pub fn with_base_url(
        token: Option<String>,
        auth_method: AuthMethod,
        base_url: &str,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .user_agent(format!("skill-sync/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            token,
            auth_method,
        })
    }

    /// Get repository information (default branch, etc.).
    pub fn get_repo_info(&self, owner: &str, repo: &str) -> crate::error::Result<RepoInfo> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        let response = self.send_request(&url, &format!("{}/{}", owner, repo))?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(SkillSyncError::RepoNotFound {
                repo: format!("{}/{}", owner, repo),
            }
            .into());
        }

        if status == 403 || status == 429 {
            if let Some(err) = check_rate_limit(&response) {
                return Err(err.into());
            }
        }

        if !response.status().is_success() {
            let body = response.text().unwrap_or_default();
            return Err(SkillSyncError::GitHubApi {
                status,
                message: body,
                repo: format!("{}/{}", owner, repo),
            }
            .into());
        }

        response.json::<RepoInfo>().map_err(|e| {
            SkillSyncError::GitHubApi {
                status,
                message: e.to_string(),
                repo: format!("{}/{}", owner, repo),
            }
            .into()
        })
    }

    /// Get the repository tree (file listing) for a given branch.
    ///
    /// Uses `?recursive=1` to get the full tree in one call.
    pub fn get_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> crate::error::Result<TreeResponse> {
        let url = format!(
            "{}/repos/{}/{}/git/trees/{}?recursive=1",
            self.base_url, owner, repo, branch
        );
        let response = self.send_request(&url, &format!("{}/{}", owner, repo))?;
        let status = response.status().as_u16();

        if status == 404 {
            return Err(SkillSyncError::RepoNotFound {
                repo: format!("{}/{}", owner, repo),
            }
            .into());
        }

        if status == 403 || status == 429 {
            if let Some(err) = check_rate_limit(&response) {
                return Err(err.into());
            }
        }

        if !response.status().is_success() {
            let body = response.text().unwrap_or_default();
            return Err(SkillSyncError::GitHubApi {
                status,
                message: body,
                repo: format!("{}/{}", owner, repo),
            }
            .into());
        }

        response.json::<TreeResponse>().map_err(|e| {
            SkillSyncError::GitHubApi {
                status,
                message: e.to_string(),
                repo: format!("{}/{}", owner, repo),
            }
            .into()
        })
    }

    /// Send an authenticated request to the GitHub API.
    fn send_request(
        &self,
        url: &str,
        repo: &str,
    ) -> crate::error::Result<reqwest::blocking::Response> {
        let mut req = self
            .client
            .get(url)
            .header(ACCEPT, "application/vnd.github+json");

        if let Some(ref token) = self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        req.send().map_err(|e| {
            SkillSyncError::NetworkError {
                repo: repo.to_string(),
                reason: e.to_string(),
            }
            .into()
        })
    }
}

/// Check response headers for rate limit exhaustion.
///
/// Returns Some(SkillSyncError::RateLimited) if remaining is 0.
fn check_rate_limit(response: &reqwest::blocking::Response) -> Option<SkillSyncError> {
    let remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    let reset = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    if remaining == Some(0) {
        Some(SkillSyncError::RateLimited {
            remaining: 0,
            reset_epoch: reset,
        })
    } else {
        None
    }
}
