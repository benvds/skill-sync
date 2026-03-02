use clap::Parser;
use owo_colors::{OwoColorize, Stream};
use skills_sync::cli::Cli;
use skills_sync::config::discover_manifest;
use skills_sync::error::SkillsSyncError;
use skills_sync::github::{self, AuthMethod};
use skills_sync::init::run_init;
use skills_sync::manifest::parse_repo_entry;
use skills_sync::metadata::{self, lock_path_for_manifest, read_lock_file, LockFile};
use skills_sync::resolver;
use skills_sync::sync;
use skills_sync::manifest;
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

    // Derive lock file path from manifest path (.skills.toml → .skills.lock)
    let lock_path = lock_path_for_manifest(&manifest_path);

    // Discover GitHub authentication
    let (token, auth_method) = github::discover_token();
    if matches!(auth_method, AuthMethod::Unauthenticated) {
        eprintln!(
            "{} running unauthenticated (60 requests/hour limit)",
            "warning:".if_supports_color(Stream::Stderr, |t| t.yellow())
        );
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

    // Read consolidated lock file once (returns empty if not found yet)
    let lock_file = read_lock_file(&lock_path).map_err(SkillsSyncError::Io)?;

    // Process each repo from manifest
    let mut all_actions: Vec<resolver::SyncAction> = Vec::new();
    let mut failed_repos: Vec<(String, String)> = Vec::new();
    let mut branch_map: HashMap<String, String> = HashMap::new();
    let mut manifest_repos: HashSet<String> = HashSet::new();

    for repo_entry in &manifest.skills.repos {
        // Parse entry into (owner, repo_name, optional subdirectory)
        let (owner, repo_name, manifest_subdir) = parse_repo_entry(repo_entry);

        // Use owner/repo slug (not full entry) for manifest_repos so orphan detection
        // matches against meta.source which also stores owner/repo
        let repo_slug = format!("{}/{}", owner, repo_name);
        manifest_repos.insert(repo_slug.clone());

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
                eprintln!(
                    "{} {} — {}, skipping",
                    "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
                    repo_slug,
                    e
                );
                failed_repos.push((repo_slug.clone(), format!("{}", e)));
                continue;
            }
        };

        branch_map.insert(repo_slug.clone(), repo_info.default_branch.clone());

        if cli.verbose {
            eprintln!(
                "{}: default branch = {}",
                repo_slug, repo_info.default_branch
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
                eprintln!(
                    "{} {} — {}, skipping",
                    "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
                    repo_slug,
                    e
                );
                failed_repos.push((repo_slug.clone(), format!("{}", e)));
                continue;
            }
        };

        // Detect or use specified skills
        let remote_skills = if let Some(subdir) = manifest_subdir {
            // Manifest specifies exact subdirectory — skip Trees API detection
            vec![resolver::RemoteSkill {
                subdirectory: Some(subdir.to_string()),
                tree_sha: tree.sha.clone(),
            }]
        } else {
            // No subdirectory specified — detect skills via Trees API
            let detected = resolver::detect_skills(&tree);
            if detected.is_empty() {
                eprintln!(
                    "{} {} — no SKILL.md found, skipping",
                    "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
                    repo_slug
                );
                continue;
            }
            detected
        };

        if cli.verbose {
            let skill_type = if remote_skills.len() == 1
                && remote_skills[0].subdirectory.is_none()
            {
                "single-skill".to_string()
            } else {
                format!("monorepo ({} skills)", remote_skills.len())
            };
            eprintln!("{}: detected as {}", repo_slug, skill_type);
        }

        // Load local metadata from consolidated lock file (filter by repo_slug)
        let local_metadata = load_local_metadata(&repo_slug, &lock_file);

        // Build repo skills for resolver
        let repo_skills = resolver::RepoSkills {
            repo: repo_slug.clone(),
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

    // Scan for global orphans (repos removed from manifest entirely)
    // Must happen before dry-run check so dry-run output is complete
    let lock_file_for_orphans = read_lock_file(&lock_path).map_err(SkillsSyncError::Io)?;
    let global_orphans = sync::scan_orphans(target_dir, &manifest_repos, &lock_file_for_orphans);

    // Add global orphan Remove actions to all_actions (skip any already covered by resolver)
    for orphan in &global_orphans {
        let already_covered = all_actions.iter().any(|a| {
            if let resolver::SyncAction::Remove { repo, subdirectory, .. } = a {
                sync::skill_dir_name(repo, subdirectory) == orphan.dir_name
            } else {
                false
            }
        });
        if !already_covered {
            all_actions.push(resolver::SyncAction::Remove {
                repo: orphan.source.clone(),
                subdirectory: None,
                local_sha: String::new(),
            });
        }
    }

    // Dry-run short-circuit: print planned actions and exit without touching disk
    if cli.dry_run {
        for action in &all_actions {
            match action {
                resolver::SyncAction::Install { repo, subdirectory, .. } => {
                    let name = resolver::skill_key(repo, subdirectory);
                    println!(
                        "{} {} (install)",
                        "+".if_supports_color(Stream::Stdout, |t| t.green()),
                        name
                    );
                }
                resolver::SyncAction::Update { repo, subdirectory, old_sha, new_sha } => {
                    let name = resolver::skill_key(repo, subdirectory);
                    if cli.verbose {
                        println!(
                            "{} {} (update: {}..{})",
                            "~".if_supports_color(Stream::Stdout, |t| t.yellow()),
                            name,
                            &old_sha[..8.min(old_sha.len())],
                            &new_sha[..8.min(new_sha.len())]
                        );
                    } else {
                        println!(
                            "{} {} (update)",
                            "~".if_supports_color(Stream::Stdout, |t| t.yellow()),
                            name
                        );
                    }
                }
                resolver::SyncAction::Remove { repo, subdirectory, .. } => {
                    let name = resolver::skill_key(repo, subdirectory);
                    println!(
                        "{} {} (remove)",
                        "-".if_supports_color(Stream::Stdout, |t| t.red()),
                        name
                    );
                }
                resolver::SyncAction::Skip { .. } => {}
            }
        }
        let pending = all_actions
            .iter()
            .filter(|a| !matches!(a, resolver::SyncAction::Skip { .. }))
            .count();
        if pending > 0 {
            std::process::exit(2);
        }
        return Ok(());
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
        println!(
            "{}",
            format!("everything up to date ({} skills)", skips)
                .if_supports_color(Stream::Stdout, |t| t.green())
        );
    }

    // Execute sync plan (handles install, update, remove, orphan detection, and cleanup)
    sync::execute_sync_plan(
        &all_actions,
        target_dir,
        sync_token.as_deref(),
        &branch_map,
        &manifest_repos,
        &lock_path,
        cli.force,
        cli.verbose,
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
/// Reads entries from the consolidated lock file that match the given repo source.
/// Returns a HashMap keyed by skill_key for use with the resolver.
fn load_local_metadata(repo: &str, lock_file: &LockFile) -> HashMap<String, metadata::SkillMeta> {
    lock_file
        .skills
        .iter()
        .filter(|(_, meta)| meta.source == repo)
        .map(|(key, meta)| (key.clone(), meta.clone()))
        .collect()
}
