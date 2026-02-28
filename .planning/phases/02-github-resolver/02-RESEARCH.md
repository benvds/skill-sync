# Phase 2: GitHub + Resolver - Research

**Researched:** 2026-02-28
**Domain:** GitHub REST API integration, sync plan resolution
**Confidence:** HIGH

## Summary

Phase 2 adds two capabilities: (1) a GitHub API client that authenticates, calls the Trees API to detect repo structure (single-skill vs monorepo), and fetches HEAD SHAs; (2) a pure resolver that compares remote state against local metadata to produce a typed sync plan (Install/Update/Remove/Skip actions). The project already uses synchronous Rust with serde, so `reqwest` blocking with JSON features is the natural HTTP client. The GitHub Trees API returns a flat list of tree entries when called with `?recursive=1`, making SKILL.md detection a simple filter over the response. Rate limit information comes back in response headers (`x-ratelimit-remaining`, `x-ratelimit-reset`) and must be extracted reactively on 403/429 responses.

**Primary recommendation:** Use `reqwest::blocking::Client` with `json` feature, call `GET /repos/{owner}/{repo}` for default branch + `GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1` for structure detection, and build a pure `resolve()` function that takes remote state + local metadata and returns `Vec<SyncAction>`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Token discovery priority: GITHUB_TOKEN env var > `gh auth token` CLI > unauthenticated
- Silent by default — auth source only shown with --verbose
- Warn once when running unauthenticated: hint about 60 req/hr limit and how to authenticate
- Private repos work when a token is present; clear error if private repo accessed without token
- Detection details (single-skill vs monorepo, skill counts) only shown with --verbose
- No-SKILL.md warning printed inline per repo during the run
- Root SKILL.md wins: if SKILL.md exists at repo root, treat as single-skill regardless of subdirectories
- No rename detection for monorepo subdirectories — renamed skill = orphan remove + fresh install
- Four action types: Install (new skill), Update (SHA changed), Remove (orphan), Skip (unchanged)
- Resolver returns a pure data struct (e.g., Vec<SyncAction>) — no display logic in the resolver
- Sequential repo processing (no async/parallel)
- Skip actions tracked explicitly so downstream consumers can report "N skills up to date"
- Rate limit hit: fail immediately with quota info (remaining count, reset time) and auth hint. No retry
- Per-repo API failure (404, network error): skip with inline warning, continue processing remaining repos
- Failed repos summarized at end of run
- Exit code non-zero if any repo failed, even if others succeeded (CI-friendly)
- Reactive rate limit handling — no pre-check of /rate_limit endpoint

### Claude's Discretion
- Sync action struct design (what fields each action carries beyond the basics)
- HTTP client choice (sync vs async, library selection)
- Internal error type design for GitHub API errors (extending existing miette/thiserror pattern)
- Trees API response parsing approach

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| REPO-01 | Tool detects single-skill repos (SKILL.md at root) via GitHub Trees API | Trees API with `?recursive=1` returns flat list; filter for `path == "SKILL.md"` and `type == "blob"` |
| REPO-02 | Tool detects monorepo skills (subdirectories containing SKILL.md, one level deep) via Trees API | Filter tree entries matching `{subdir}/SKILL.md` pattern with exactly one path separator |
| REPO-03 | Tool skips repos with no SKILL.md found and warns the user | Check if filtered tree entries are empty; emit warning and continue |
| REPO-04 | Tool handles GitHub API rate limits gracefully with clear error messages showing remaining quota | Parse `x-ratelimit-remaining` and `x-ratelimit-reset` headers from 403/429 responses |
| SYNC-03 | Tool updates outdated skills by comparing local SHA against remote HEAD SHA | Resolver compares `SkillMeta.sha` against tree SHA from API response |
| SYNC-07 | Sync is idempotent — re-running with no changes produces no side effects | Resolver emits `Skip` action when local SHA matches remote SHA |
| ERR-06 | GitHub API requests include User-Agent header to avoid 403 responses | Set `User-Agent: skills-sync/{version}` via `reqwest::blocking::ClientBuilder::user_agent()` |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.12 | HTTP client for GitHub API calls | De facto Rust HTTP client; `blocking` feature matches project's sync architecture |
| serde | 1 (already in project) | JSON deserialization of API responses | Already used for metadata; `Deserialize` on API response structs |
| serde_json | 1 (already in project) | JSON parsing | Already a dependency |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chrono | 0.4 (already in project) | Parse `x-ratelimit-reset` epoch timestamps | Rate limit error messages with human-readable reset time |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| reqwest blocking | ureq | Lighter dependency, but reqwest is more widely used and has better JSON integration via serde |
| reqwest blocking | reqwest async + tokio | Would add tokio runtime; project explicitly chose sync architecture |

**Installation:**
```bash
cargo add reqwest --features blocking,json
```

No new dependencies beyond reqwest. All other needs (serde, serde_json, chrono, thiserror, miette) are already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure
```
src/
├── github.rs        # GitHub API client: auth, HTTP calls, response types
├── resolver.rs      # Pure sync resolver: remote state + local state → Vec<SyncAction>
├── cli.rs           # (existing) CLI arg parsing
├── config.rs        # (existing) Manifest discovery
├── error.rs         # (existing, extended) New GitHub error variants
├── manifest.rs      # (existing) Manifest parsing
├── metadata.rs      # (existing) Per-skill metadata I/O
├── lib.rs           # (existing, extended) New module declarations
└── main.rs          # (existing, extended) Wire GitHub + resolver into run()
```

### Pattern 1: Layered API Client
**What:** Separate the HTTP transport (github.rs) from the business logic (resolver.rs). The GitHub module knows about API endpoints and response shapes; the resolver knows about sync semantics.
**When to use:** Always — this separation enables testing the resolver with mock data without HTTP calls.
**Example:**
```rust
// github.rs — API types and client
pub struct GitHubClient {
    client: reqwest::blocking::Client,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TreeResponse {
    pub sha: String,
    pub tree: Vec<TreeEntry>,
    pub truncated: bool,
}

#[derive(Debug, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    pub default_branch: String,
}
```

### Pattern 2: Token Discovery Chain
**What:** Try GITHUB_TOKEN env var first, then `gh auth token` subprocess, then unauthenticated.
**When to use:** At client construction time, once per run.
**Example:**
```rust
fn discover_token() -> Option<String> {
    // 1. Environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Some(token);
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
                return Some(token);
            }
        }
    }
    // 3. Unauthenticated
    None
}
```

### Pattern 3: Pure Resolver Function
**What:** A pure function that takes remote repo state and local metadata, returns sync actions. No I/O inside the resolver.
**When to use:** Core sync logic.
**Example:**
```rust
pub fn resolve(
    repo: &str,
    remote_skills: &[RemoteSkill],
    local_metadata: &HashMap<String, SkillMeta>,
) -> Vec<SyncAction> {
    let mut actions = Vec::new();
    // Install/Update/Skip for each remote skill
    for skill in remote_skills {
        let key = skill.install_key(repo);
        match local_metadata.get(&key) {
            None => actions.push(SyncAction::Install { /* ... */ }),
            Some(meta) if meta.sha != skill.sha => actions.push(SyncAction::Update { /* ... */ }),
            Some(_) => actions.push(SyncAction::Skip { /* ... */ }),
        }
    }
    // Remove orphans: local skills not in remote
    for (key, meta) in local_metadata {
        if meta.source == repo && !remote_skills.iter().any(|s| s.install_key(repo) == *key) {
            actions.push(SyncAction::Remove { /* ... */ });
        }
    }
    actions
}
```

### Anti-Patterns to Avoid
- **Mixing HTTP calls into the resolver:** Makes it untestable. Keep the resolver pure.
- **Parsing response bodies for rate limit info:** Rate limit data is in headers, not the JSON body.
- **Using `unwrap()` on API responses:** Network calls fail; always handle errors explicitly.
- **Hardcoding "main" as default branch:** Repos may use "master" or other branch names. Always query the API.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP client | Custom TCP/TLS | reqwest::blocking | Connection pooling, redirect handling, TLS, timeouts |
| JSON parsing | Manual string parsing | serde_json with Deserialize derive | Type safety, error handling, nested structures |
| User-Agent header | Manual header construction | ClientBuilder::user_agent() | Handles encoding, validates header value |
| URL construction | String concatenation | format!() with validated inputs | Manifest already validates owner/repo format |

**Key insight:** The GitHub API is well-documented and stable. The complexity is in error handling (rate limits, 404s, auth failures), not in the HTTP calls themselves.

## Common Pitfalls

### Pitfall 1: Missing User-Agent Header
**What goes wrong:** GitHub returns 403 Forbidden for requests without a User-Agent header.
**Why it happens:** reqwest doesn't set a User-Agent by default.
**How to avoid:** Set it once via `ClientBuilder::user_agent("skills-sync/0.1.0")`.
**Warning signs:** 403 responses on the very first API call.

### Pitfall 2: Confusing Tree SHA with Commit SHA
**What goes wrong:** Using the tree SHA (from Trees API) when you need the commit SHA (for metadata tracking), or vice versa.
**Why it happens:** The Trees API returns tree object SHAs, but `SkillMeta.sha` should track the commit SHA to detect updates.
**How to avoid:** Use `GET /repos/{owner}/{repo}/git/refs/heads/{branch}` to get the HEAD commit SHA, then use the tree SHA from the Trees API only for structure detection.
**Warning signs:** SHAs never matching between local metadata and remote, causing unnecessary updates.

### Pitfall 3: Not Handling the `type` Field in Tree Entries
**What goes wrong:** Treating all tree entries as files. The `type` field can be "blob" (file), "tree" (directory), or "commit" (submodule).
**Why it happens:** Only looking at `path` without checking `type`.
**How to avoid:** Filter for `type == "blob"` when looking for SKILL.md files. Filter for `type == "tree"` when looking for subdirectories.
**Warning signs:** Directories being treated as skills, or submodules causing false positives.

### Pitfall 4: Auth Token Leaking in Error Messages
**What goes wrong:** Token appears in error output or debug logs when API calls fail.
**Why it happens:** Default debug formatting of request headers.
**How to avoid:** Never include the token in error messages. Log the auth method ("token", "gh-cli", "unauthenticated") but not the token value.
**Warning signs:** Token visible in stderr output.

### Pitfall 5: Rate Limit Headers Missing on Error Responses
**What goes wrong:** Assuming rate limit headers are always present on 403/429 responses.
**Why it happens:** Some 403 responses are not rate-limit related (e.g., repo not found with bad auth).
**How to avoid:** Check for the presence of `x-ratelimit-remaining` header before parsing. Differentiate rate-limit 403 from other 403 errors.
**Warning signs:** Panics when parsing missing headers.

## Code Examples

### GitHub API Client Construction
```rust
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT, USER_AGENT};

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .user_agent(format!("skills-sync/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client, token })
    }

    fn get(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let mut req = self.client.get(url)
            .header(ACCEPT, "application/vnd.github+json");
        if let Some(ref token) = self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }
        req
    }
}
```

### Trees API Call with Rate Limit Handling
```rust
pub fn get_tree(&self, owner: &str, repo: &str, branch: &str) -> Result<TreeResponse> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        owner, repo, branch
    );
    let response = self.get(&url).send()?;

    if response.status() == 403 || response.status() == 429 {
        // Check for rate limit headers
        let remaining = response.headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let reset = response.headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok());

        if remaining == Some(0) {
            return Err(SkillsSyncError::RateLimited { remaining: 0, reset_epoch: reset });
        }
    }

    response.error_for_status_ref()?;
    Ok(response.json()?)
}
```

### SKILL.md Detection from Tree
```rust
/// Detect repo structure from tree entries.
/// Returns list of RemoteSkill (root skill or monorepo subdirectory skills).
pub fn detect_skills(tree: &TreeResponse) -> Vec<RemoteSkill> {
    // Check root first — root SKILL.md wins
    let has_root_skill = tree.tree.iter().any(|e| {
        e.path == "SKILL.md" && e.entry_type == "blob"
    });

    if has_root_skill {
        return vec![RemoteSkill { subdirectory: None, sha: tree.sha.clone() }];
    }

    // Check one-level-deep subdirectories
    tree.tree.iter()
        .filter(|e| {
            e.entry_type == "blob"
                && e.path.ends_with("/SKILL.md")
                && e.path.matches('/').count() == 1
        })
        .map(|e| {
            let subdir = e.path.strip_suffix("/SKILL.md").unwrap().to_string();
            RemoteSkill { subdirectory: Some(subdir), sha: tree.sha.clone() }
        })
        .collect()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| GitHub API v3 with basic auth | Token-based auth (Bearer) | 2020 | Basic auth deprecated; use personal access tokens or GitHub App tokens |
| `type` field as string | Still string in REST API | Stable | No change — "blob", "tree", "commit" are the valid values |
| Single rate limit bucket | Per-resource rate limits | 2021 | `x-ratelimit-resource` header tells you which bucket was consumed |

**Deprecated/outdated:**
- Basic authentication (username:password) — removed November 2020
- API preview headers (Accept: application/vnd.github.VERSION+json) — previews graduated to stable

## Open Questions

1. **Commit SHA tracking strategy**
   - What we know: The Trees API returns the tree SHA, not the commit SHA. The repo API returns `default_branch` name. The refs API (`/git/refs/heads/{branch}`) returns the commit SHA.
   - What's unclear: Whether to make a separate API call for the commit SHA or use the tree SHA for change detection.
   - Recommendation: Use the commit SHA from `/git/refs/heads/{branch}` for metadata tracking (matches git conventions). Use the tree from the Trees API for structure detection. This costs one extra API call per repo but ensures correct SHA semantics. Alternatively, use `GET /repos/{owner}/{repo}/commits/{branch}` which returns the commit SHA and tree SHA in one call, but is a heavier response.

2. **Orphan detection scope**
   - What we know: Remove actions are for skills that are installed locally but no longer in the manifest or no longer in the remote repo.
   - What's unclear: Whether orphan detection in Phase 2 resolver should handle "removed from manifest" (comparing manifest repos vs installed skills) or just "removed from remote repo" (skill directory disappeared from monorepo).
   - Recommendation: Handle both: (a) installed skills whose repo is no longer in the manifest → Remove, and (b) monorepo subdirectories that no longer have SKILL.md → Remove. The resolver receives both the manifest repos and local metadata, so it can detect both cases.

## Sources

### Primary (HIGH confidence)
- [GitHub Trees API Docs](https://docs.github.com/en/rest/git/trees) — endpoint format, response shape, recursive parameter, truncation handling
- [GitHub Rate Limits Docs](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) — rate limit headers, 60/5000 req/hr limits, 403/429 status codes
- [reqwest blocking ClientBuilder](https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html) — user_agent(), build(), default_headers()
- [GitHub Refs API](https://docs.github.com/en/rest/git/refs) — commit SHA from refs/heads/{branch}
- [GitHub Repositories API](https://docs.github.com/en/rest/repos/repos) — default_branch field

### Secondary (MEDIUM confidence)
- [gh auth token docs](https://cli.github.com/manual/gh_auth_token) — token discovery via gh CLI

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — reqwest is well-established, project already uses serde
- Architecture: HIGH — layered client + pure resolver is standard Rust pattern
- Pitfalls: HIGH — GitHub API behavior is well-documented
- API endpoints: HIGH — verified against official GitHub REST API docs

**Research date:** 2026-02-28
**Valid until:** 2026-03-28 (stable APIs, unlikely to change)
