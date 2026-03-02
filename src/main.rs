use clap::Parser;
use skills_sync::cli::Cli;
use skills_sync::config::discover_manifest;
use skills_sync::error::SkillsSyncError;
use skills_sync::github::{self, AuthMethod};
use skills_sync::init::run_init;
use skills_sync::metadata;
use skills_sync::resolver;
use skills_sync::sync;
use skills_sync::{manifest, resolver::skill_key};
use std::collections::{HashMap, HashSet};

fn main() {
    // Install miette error handler for fancy diagnostics
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(2)
                .build(),
        )
    }));

    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> skills_sync::error::Result<()> {
    if cli.init {
        let path = std::path::PathBuf::from(".skills.toml");
        return run_init(&path);
    }

    let manifest_path = discover_manifest(cli.config.as_deref())?;
    let content =
        std::fs::read_to_string(&manifest_path).map_err(SkillsSyncError::Io)?;
    let manifest = manifest::parse_and_validate(&content, &manifest_path)?;

    // Discover GitHub authentication
    let (token, auth_method) = github::discover_token();
    if matches!(auth_method, AuthMethod::Unauthenticated) {
        eprintln!("warning: running unauthenticated (60 requests/hour limit)");
        eprintln!(
            "  hint: set GITHUB_TOKEN or install gh CLI for 5,000 requests/hour"
        );
    }
    if cli.verbose {
        eprintln!("auth: {:?}", auth_method);
    }

    // Clone token before moving it into GitHubClient so we can pass it to the sync engine
    let sync_token = token.clone();

    let client = github::GitHubClient::new(token, auth_method)
        .map_err(|e| SkillsSyncError::HttpClient(e.to_string()))?;

    // Resolve target directory: global (~/.claude/skills/) or local (.claude/skills/)
    let target_dir_buf: std::path::PathBuf = if cli.global {
        let home = dirs::home_dir().ok_or_else(|| SkillsSyncError::SyncFailed {
            repo: "--global".to_string(),
            reason: "cannot resolve home directory for --global target".to_string(),
        })?;
        home.join(".claude").join("skills")
    } else {
        std::path::PathBuf::from(".claude/skills")
    };
    let target_dir = target_dir_buf.as_path();

    // Process each repo from manifest
    let mut all_actions: Vec<resolver::SyncAction> = Vec::new();
    let mut failed_repos: Vec<(String, String)> = Vec::new();
    let mut branch_map: HashMap<String, String> = HashMap::new();
    let mut manifest_repos: HashSet<String> = HashSet::new();

    for repo_entry in &manifest.skills.repos {
        // Track all repos from manifest (before any early continues)
        manifest_repos.insert(repo_entry.clone());
        // Split owner/repo
        let parts: Vec<&str> = repo_entry.splitn(2, '/').collect();
        let (owner, repo_name) = (parts[0], parts[1]);

        // Get repo info (default branch)
        let repo_info = match client.get_repo_info(owner, repo_name) {
            Ok(info) => info,
            Err(e) => {
                // Rate limit errors fail immediately
                if let Some(rl) = e.downcast_ref::<SkillsSyncError>() {
                    if matches!(rl, SkillsSyncError::RateLimited { .. }) {
                        return Err(e);
                    }
                }
                eprintln!("warning: {} — {}, skipping", repo_entry, e);
                failed_repos.push((repo_entry.clone(), format!("{}", e)));
                continue;
            }
        };

        branch_map.insert(repo_entry.clone(), repo_info.default_branch.clone());

        if cli.verbose {
            eprintln!(
                "{}: default branch = {}",
                repo_entry, repo_info.default_branch
            );
        }

        // Get tree
        let tree = match client.get_tree(owner, repo_name, &repo_info.default_branch) {
            Ok(t) => t,
            Err(e) => {
                if let Some(rl) = e.downcast_ref::<SkillsSyncError>() {
                    if matches!(rl, SkillsSyncError::RateLimited { .. }) {
                        return Err(e);
                    }
                }
                eprintln!("warning: {} — {}, skipping", repo_entry, e);
                failed_repos.push((repo_entry.clone(), format!("{}", e)));
                continue;
            }
        };

        // Detect skills
        let remote_skills = resolver::detect_skills(&tree);
        if remote_skills.is_empty() {
            eprintln!("warning: {} — no SKILL.md found, skipping", repo_entry);
            continue;
        }

        if cli.verbose {
            let skill_type = if remote_skills.len() == 1
                && remote_skills[0].subdirectory.is_none()
            {
                "single-skill".to_string()
            } else {
                format!("monorepo ({} skills)", remote_skills.len())
            };
            eprintln!("{}: detected as {}", repo_entry, skill_type);
        }

        // Load local metadata for this repo's skills
        let local_metadata = load_local_metadata(repo_entry, target_dir);

        // Build repo skills for resolver
        let repo_skills = resolver::RepoSkills {
            repo: repo_entry.clone(),
            skills: remote_skills,
            commit_sha: tree.sha.clone(),
        };

        // Resolve sync actions
        let actions = resolver::resolve(&repo_skills, &local_metadata);
        all_actions.extend(actions);
    }

    // Report failed repos
    if !failed_repos.is_empty() {
        eprintln!("\nfailed repos:");
        for (repo, reason) in &failed_repos {
            eprintln!("  {} — {}", repo, reason);
        }
    }

    // Report sync plan summary
    let installs = all_actions
        .iter()
        .filter(|a| matches!(a, resolver::SyncAction::Install { .. }))
        .count();
    let updates = all_actions
        .iter()
        .filter(|a| matches!(a, resolver::SyncAction::Update { .. }))
        .count();
    let removes = all_actions
        .iter()
        .filter(|a| matches!(a, resolver::SyncAction::Remove { .. }))
        .count();
    let skips = all_actions
        .iter()
        .filter(|a| matches!(a, resolver::SyncAction::Skip { .. }))
        .count();

    if installs > 0 || updates > 0 || removes > 0 {
        println!(
            "sync plan: {} install, {} update, {} remove, {} up-to-date",
            installs, updates, removes, skips
        );
    } else if skips > 0 {
        println!("everything up to date ({} skills)", skips);
    }

    // Execute sync plan (handles install, update, remove, orphan detection, and cleanup)
    sync::execute_sync_plan(
        &all_actions,
        target_dir,
        sync_token.as_deref(),
        &branch_map,
        &manifest_repos,
    )?;

    // Exit non-zero if any repo failed at the API level (separate from sync failures)
    if !failed_repos.is_empty() {
        return Err(SkillsSyncError::GitHubApi {
            status: 0,
            message: format!("{} repo(s) failed", failed_repos.len()),
            repo: failed_repos
                .iter()
                .map(|(r, _)| r.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }
        .into());
    }

    Ok(())
}

/// Load local metadata for skills from a specific repo.
///
/// Scans the given target_dir for sibling lock files (*.lock) and returns
/// those matching the given repo. Lock files are stored as siblings to skill
/// directories at the target_dir level (e.g., target_dir/owner--repo.lock).
fn load_local_metadata(repo: &str, target_dir: &std::path::Path) -> HashMap<String, metadata::SkillMeta> {
    let mut result = HashMap::new();

    if !target_dir.exists() {
        return result;
    }

    if let Ok(entries) = std::fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Look for sibling lock files (*.lock) instead of lock files inside directories
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == metadata::META_EXTENSION {
                        if let Ok(meta) = metadata::read_metadata(&path) {
                            if meta.source == repo {
                                let key = skill_key(repo, &meta.subdirectory);
                                result.insert(key, meta);
                            }
                        }
                    }
                }
            }
        }
    }

    result
}
