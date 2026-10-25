use clap::Parser;
use skills_sync::{
    agents,
    cli::Cli,
    config,
    deployment::DeploymentLock,
    engine::{self, PreparedSources},
    error::{Result, SkillsSyncError},
    github::{self, AuthMethod},
    init,
    manifest::{self, Scope},
    metadata::lock_path_for_manifest,
};
use std::collections::BTreeMap;

fn main() {
    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .build(),
        )
    }));
    match run(Cli::parse()) {
        Ok(pending) => {
            if pending {
                std::process::exit(2);
            }
        }
        Err(err) => {
            eprintln!("{err:?}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<bool> {
    let cwd = std::env::current_dir().map_err(SkillsSyncError::Io)?;
    let home = dirs::home_dir()
        .ok_or_else(|| SkillsSyncError::Config("cannot resolve home directory".into()))?;
    for agent in &cli.agents {
        agents::validate_agent(agent)?;
    }
    if cli.init {
        let path = cli.config.clone().unwrap_or(if cli.global {
            config::central_manifest_path()?
        } else {
            cwd.join(".skills.toml")
        });
        if cli.global {
            init::run_init_global(&path)?;
        } else {
            init::run_init(&path)?;
        }
        return Ok(false);
    }
    let central = config::find_central_manifest()?;
    let local = config::find_project_manifest(&cwd);
    let mut locations = Vec::new();
    if let Some(path) = &cli.config {
        let path = config::discover_manifest(Some(path))?;
        let is_central = central
            .as_ref()
            .is_some_and(|p| p.canonicalize().ok() == path.canonicalize().ok());
        locations.push((
            path,
            if is_central {
                Scope::User
            } else {
                Scope::Project
            },
        ));
    } else {
        if !cli.local {
            if let Some(path) = central {
                locations.push((path, Scope::User));
            }
        }
        if !cli.global {
            if let Some(path) = local {
                if !locations
                    .iter()
                    .any(|(p, _)| p.canonicalize().ok() == path.canonicalize().ok())
                {
                    locations.push((path, Scope::Project));
                }
            }
        }
    }
    if locations.is_empty() {
        return Err(SkillsSyncError::ManifestNotFound {
            searched: vec![cwd.join(".skills.toml"), config::central_manifest_path()?],
        }
        .into());
    }
    if cli.target.is_some() && locations.len() > 1 {
        return Err(SkillsSyncError::Config(
            "--target with multiple manifests is ambiguous; select --local or --config".into(),
        )
        .into());
    }
    let mut github_auth = None;
    let mut jobs = Vec::new();
    let mut claimed_targets = BTreeMap::new();
    let mut skill_names = BTreeMap::new();
    for (path, default_scope) in locations {
        let path = agents::absolute_path(&path, &cwd)?;
        let content = std::fs::read_to_string(&path).map_err(SkillsSyncError::Io)?;
        let manifest = manifest::parse_and_validate(&content, &path)?;
        let mut scope = if cli.global {
            Scope::User
        } else if cli.local {
            Scope::Project
        } else {
            manifest
                .sync
                .as_ref()
                .and_then(|s| s.scope)
                .unwrap_or(default_scope)
        };
        let targets = if let Some(target) = &cli.target {
            vec![agents::expand_path(&target.to_string_lossy(), &home, &cwd)?]
        } else {
            agents::target_directories(
                &manifest,
                &path,
                scope,
                &cli.agents,
                &home,
                &config::config_home()?,
            )?
        };
        if scope == Scope::Project
            && targets
                .iter()
                .any(|target| !target.starts_with(path.parent().unwrap()))
        {
            if cli.target.is_some() && manifest.sync.is_none() && !cli.local {
                // Legacy explicit-target manifests were also used as user configurations.
                scope = Scope::User;
            } else {
                return Err(SkillsSyncError::Config("project targets must remain inside the manifest directory; use a user-scoped manifest for external targets".into()).into());
            }
        }
        for target in &targets {
            if let Some(other) = claimed_targets.insert(target.clone(), path.clone()) {
                return Err(SkillsSyncError::Config(format!(
                    "{} and {} both manage {}; choose distinct targets",
                    other.display(),
                    path.display(),
                    target.display()
                ))
                .into());
            }
        }
        let lock_path = lock_path_for_manifest(&path);
        // A v1 lock belonged to one target; never pretend it already owns every new target.
        let legacy_target = if cli.global && manifest.sync.is_none() {
            config::global_target_dir()?
        } else if let Some(target) = &cli.target {
            agents::expand_path(&target.to_string_lossy(), &home, &cwd)?
        } else {
            config::target_dir_for_manifest(&path)
        };
        let mut lock =
            DeploymentLock::read(&lock_path, &legacy_target).map_err(SkillsSyncError::Io)?;
        lock.bind_scope(scope, path.parent().unwrap())
            .map_err(SkillsSyncError::Io)?;
        if cli.verbose {
            eprintln!(
                "manifest: {}\nlock: {}\nscope: {:?}",
                path.display(),
                lock_path.display(),
                scope
            );
            for target in &targets {
                eprintln!("target: {}", target.display());
            }
        }
        // Empty manifests need no HTTP client, credentials, or Git access.
        let sources = if manifest.skills.repos.is_empty() {
            None
        } else {
            let (token, auth) = github_auth.get_or_insert_with(github::discover_token);
            if matches!(auth, AuthMethod::Unauthenticated) {
                eprintln!("warning: running unauthenticated (GitHub rate limits apply)");
            }
            let client = github::GitHubClient::new(token.clone(), auth.clone())
                .map_err(|e| SkillsSyncError::HttpClient(e.to_string()))?;
            Some(PreparedSources::fetch(
                &manifest,
                &client,
                token.as_deref(),
            )?)
        };
        let skills = sources.as_ref().map(|s| s.skills.as_slice()).unwrap_or(&[]);
        for skill in skills {
            if let Some(other) = skill_names.insert(skill.directory.clone(), path.clone()) {
                eprintln!(
                    "warning: skill `{}` occurs in {} and {}; agents decide precedence",
                    skill.directory,
                    other.display(),
                    path.display()
                );
            }
        }
        let plan = engine::plan(skills, &targets, &lock);
        for op in &plan.operations {
            println!("{:?}: {} -> {}", op.action, op.skill, op.target.display());
        }
        for conflict in &plan.conflicts {
            eprintln!("conflict: {conflict}");
        }
        if plan.operations.is_empty() && plan.conflicts.is_empty() {
            println!("everything up to date ({} copies)", plan.skips);
        }
        jobs.push((lock_path, targets, lock, sources, plan));
    }
    // All manifests are planned before the first target is changed.
    let conflicts: Vec<_> = jobs
        .iter()
        .flat_map(|job| job.4.conflicts.iter().cloned())
        .collect();
    if !conflicts.is_empty() {
        return Err(SkillsSyncError::Config(conflicts.join("\n")).into());
    }
    if cli.dry_run {
        return Ok(jobs.iter().any(|job| !job.4.operations.is_empty()));
    }
    let mut errors = Vec::new();
    for (lock_path, targets, mut lock, sources, plan) in jobs {
        let skills = sources.as_ref().map(|s| s.skills.as_slice()).unwrap_or(&[]);
        if let Err(err) = engine::apply(&plan, skills, &targets, &mut lock, &lock_path, cli.force) {
            errors.push(err.to_string());
        }
    }
    if !errors.is_empty() {
        return Err(SkillsSyncError::Config(errors.join("\n")).into());
    }
    Ok(false)
}
