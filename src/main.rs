use clap::Parser;
use skill_sync::{
    agents,
    cli::Cli,
    cli::Command,
    config,
    deployment::DeploymentLock,
    engine,
    error::{Result, SkillSyncError},
    init,
    manifest::{self, Scope},
    metadata::lock_path_for_manifest,
    output::{ManifestReport, Report},
    run_lock::RunLocks,
    sources, status,
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
    let mut cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.use_stderr() && std::env::args_os().any(|arg| arg == "--json") {
                let mut report = Report::new("arguments", false);
                report.success = false;
                report.exit_code = 2;
                report.errors.push(error.to_string());
                emit_json(&report);
                std::process::exit(2);
            }
            error.exit();
        }
    };
    cli.dry_run |= cli.diff;
    let name = match &cli.command {
        Some(Command::Status) => "status",
        Some(Command::Add { .. }) => "add",
        Some(Command::Remove { .. }) => "remove",
        Some(Command::Lock) => "lock",
        Some(Command::Update) => "update",
        None if cli.init => "init",
        None => "sync",
    };
    let mut report = Report::new(name, cli.dry_run);
    match run(&cli, &mut report) {
        Ok(pending) => {
            report.pending_changes = pending;
            report.exit_code = if pending { 2 } else { 0 };
        }
        Err(error) => {
            report.success = false;
            report.exit_code = 1;
            report.errors.push(format!("{error:#}"));
            if !cli.json {
                eprintln!("{error:?}");
            }
        }
    }
    if cli.json {
        emit_json(&report);
    }
    std::process::exit(report.exit_code);
}

fn emit_json(report: &Report) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = serde_json::to_writer_pretty(&mut stdout, report)
        .map_err(std::io::Error::other)
        .and_then(|_| writeln!(stdout))
    {
        eprintln!("cannot write JSON result: {error}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli, report: &mut Report) -> Result<bool> {
    let local_status = matches!(cli.command, Some(Command::Status));
    let source_lock_command = matches!(cli.command, Some(Command::Lock | Command::Update));
    if cli.command.is_some() && cli.init {
        return Err(
            SkillSyncError::Config("--init cannot be combined with a subcommand".into()).into(),
        );
    }
    if cli.frozen && matches!(cli.command, Some(Command::Update)) {
        return Err(
            SkillSyncError::Config("update cannot be combined with --frozen".into()).into(),
        );
    }
    if cli.diff && cli.command.is_some() {
        return Err(SkillSyncError::Config("--diff is a sync preview option".into()).into());
    }
    if local_status && (cli.init || cli.dry_run || cli.force) {
        return Err(SkillSyncError::Config(
            "status cannot be combined with --init, --dry-run or --force".into(),
        )
        .into());
    }
    let cwd = std::env::current_dir().map_err(SkillSyncError::Io)?;
    let home = dirs::home_dir()
        .ok_or_else(|| SkillSyncError::Config("cannot resolve home directory".into()))?;
    for agent in &cli.agents {
        agents::validate_agent(agent)?;
    }
    let mut run_locks = RunLocks::new(&home);
    if cli.init {
        let path = cli.config.clone().unwrap_or(if cli.global {
            config::central_manifest_path()?
        } else {
            cwd.join(".skills.toml")
        });
        run_locks.acquire("manifest", &lock_path_for_manifest(&path))?;
        if cli.global {
            init::run_init_global(&path)?;
        } else {
            init::run_init(&path)?;
        }
        report.created = Some(agents::absolute_path(&path, &cwd)?);
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
        return Err(SkillSyncError::ManifestNotFound {
            searched: vec![cwd.join(".skills.toml"), config::central_manifest_path()?],
        }
        .into());
    }
    if cli.target.is_some() && locations.len() > 1 {
        return Err(SkillSyncError::Config(
            "--target with multiple manifests is ambiguous; select --local or --config".into(),
        )
        .into());
    }
    if let Some(Command::Add { source } | Command::Remove { source }) = &cli.command {
        if locations.len() != 1 {
            return Err(SkillSyncError::Config(
                "select one manifest with --local, --global or --config for add/remove".into(),
            )
            .into());
        }
        let path = agents::absolute_path(&locations[0].0, &cwd)?;
        if !cli.dry_run {
            run_locks.acquire("manifest", &lock_path_for_manifest(&path))?;
        }
        let edit = skill_sync::edit::preview(
            &path,
            source,
            matches!(cli.command, Some(Command::Remove { .. })),
            &cli.agents,
        )?;
        let changed = edit.before != edit.after;
        if !cli.json {
            println!(
                "{}: {}",
                if changed {
                    "Manifest edit"
                } else {
                    "Unchanged"
                },
                path.display()
            );
            if cli.dry_run {
                print!("{}", edit.after);
            }
        }
        if !cli.dry_run {
            skill_sync::edit::apply(&edit)?;
        }
        report.manifest_edits.push(edit);
        return Ok(cli.dry_run && changed);
    }
    let mut prepared_locks = Vec::new();
    let mut jobs = Vec::new();
    let mut claimed_targets = BTreeMap::new();
    let mut skill_names = BTreeMap::new();
    for (path, default_scope) in locations {
        let path = agents::absolute_path(&path, &cwd)?;
        if !local_status && !cli.dry_run {
            run_locks.acquire("manifest", &lock_path_for_manifest(&path))?;
        }
        let content = std::fs::read_to_string(&path).map_err(SkillSyncError::Io)?;
        let manifest = manifest::parse_and_validate(&content, &path)?;
        if source_lock_command {
            let frozen = cli.frozen
                || (matches!(cli.command, Some(Command::Lock))
                    && sources::lock_path(&path).exists());
            let sources = sources::prepare(
                &manifest,
                &path,
                &home,
                frozen,
                u64::from(cli.cache_limit_mib) * 1024 * 1024,
            )?;

            let portable_path = sources::lock_path(&path);
            let prepared = sources;
            let changed =
                sources::SourceLock::read(&portable_path).ok().as_ref() != Some(&prepared.lock);
            report.pending_changes |= changed;
            report.source_locks.push(portable_path.clone());
            report.source_resolutions.push(prepared.lock.clone());
            if !cli.json {
                println!(
                    "{}: {}",
                    if changed {
                        "Source lock update"
                    } else {
                        "Source lock unchanged"
                    },
                    portable_path.display()
                );
            }
            if cli.dry_run && changed && !cli.json {
                println!(
                    "{}",
                    toml::to_string_pretty(&prepared.lock)
                        .map_err(|e| SkillSyncError::Config(e.to_string()))?
                );
            }
            prepared_locks.push((portable_path, prepared.lock));
            continue;
        }

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
                return Err(SkillSyncError::Config("project targets must remain inside the manifest directory; use a user-scoped manifest for external targets".into()).into());
            }
        }
        for target in &targets {
            if let Some(other) = claimed_targets.insert(target.clone(), path.clone()) {
                return Err(SkillSyncError::Config(format!(
                    "{} and {} both manage {}; choose distinct targets",
                    other.display(),
                    path.display(),
                    target.display()
                ))
                .into());
            }
        }
        let lock_path = lock_path_for_manifest(&path);
        if !local_status && !cli.dry_run {
            run_locks.acquire("manifest", &lock_path)?;
        }
        // A v1 lock belonged to one target; never pretend it already owns every new target.
        let legacy_target = if cli.global && manifest.sync.is_none() {
            config::global_target_dir()?
        } else if let Some(target) = &cli.target {
            agents::expand_path(&target.to_string_lossy(), &home, &cwd)?
        } else {
            config::target_dir_for_manifest(&path)
        };
        let mut lock =
            DeploymentLock::read(&lock_path, &legacy_target).map_err(SkillSyncError::Io)?;
        lock.bind_scope(scope, path.parent().unwrap())
            .map_err(SkillSyncError::Io)?;
        if !local_status && !source_lock_command && !cli.dry_run {
            for target in targets
                .iter()
                .chain(lock.installations.iter().map(|entry| &entry.target))
            {
                run_locks.acquire("target", target)?;
            }
        }
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
        if local_status {
            let snapshot = status::inspect(&manifest.skills.repos, &targets, &lock);
            report.pending_changes |= snapshot.attention_required;
            if !cli.json {
                println!("Local status: {} (upstream not checked)", path.display());
                for entry in &snapshot.entries {
                    println!(
                        "{:?}: {}{}",
                        entry.state,
                        entry.target.join(&entry.directory).display(),
                        if entry.managed && !entry.selected {
                            " (no longer selected)"
                        } else {
                            ""
                        }
                    );
                    if let Some(detail) = &entry.detail {
                        println!("  {detail}");
                    }
                }
                for source in &snapshot.unresolved_sources {
                    println!("Unresolved: {source} (sync required to discover skills)");
                }
                if !snapshot.attention_required {
                    println!("local state matches recorded installations; remote revisions and collection membership not checked");
                }
            }
            report.manifests.push(ManifestReport {
                manifest: path,
                lock: lock_path,
                scope,
                targets,
                operations: Vec::new(),
                unchanged: 0,
                conflicts: Vec::new(),
                status: Some(snapshot),
            });
            continue;
        }
        let frozen = cli.frozen
            || (matches!(cli.command, Some(Command::Lock)) && sources::lock_path(&path).exists());
        let sources = sources::prepare(
            &manifest,
            &path,
            &home,
            frozen,
            u64::from(cli.cache_limit_mib) * 1024 * 1024,
        )?;
        let skills = sources.skills.as_slice();
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
        if cli.diff {
            for op in &plan.operations {
                let current = op.previous.as_ref().map(|p| op.target.join(&p.directory));
                let baseline = op
                    .previous
                    .as_ref()
                    .and_then(|p| p.content_hash.as_ref())
                    .filter(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                    .map(|hash| skill_sync::diff::baseline_root(&lock_path).join(hash));
                let desired = op.desired.map(|i| skills[i].path.as_path());
                let comparisons = if baseline.as_ref().is_some_and(|p| p.exists()) {
                    vec![
                        ("upstream", baseline.as_deref(), desired),
                        ("local", baseline.as_deref(), current.as_deref()),
                    ]
                } else {
                    vec![("installed_to_proposed", current.as_deref(), desired)]
                };
                for (comparison, before, after) in comparisons {
                    let files = skill_sync::diff::compare(before, after)?;
                    if !files.is_empty() {
                        if !cli.json {
                            println!("{} {} ({comparison})", op.skill, op.target.display());
                            for file in &files {
                                println!(
                                    "{}{}\n{}",
                                    file.file,
                                    if file.mode_changed {
                                        " (executable mode changed)"
                                    } else {
                                        ""
                                    },
                                    file.patch
                                );
                            }
                        }
                        report.diffs.push(skill_sync::diff::SkillDiff {
                            skill: op.skill.clone(),
                            target: op.target.clone(),
                            comparison,
                            files,
                        });
                    }
                }
            }
        }
        if !cli.json {
            for op in &plan.operations {
                println!("{:?}: {} -> {}", op.action, op.skill, op.target.display());
            }
            for conflict in &plan.conflicts {
                eprintln!("conflict: {conflict}");
            }
            if plan.operations.is_empty() && plan.conflicts.is_empty() {
                println!("everything up to date ({} copies)", plan.skips);
            }
        }
        report.pending_changes |= !plan.operations.is_empty();
        report.manifests.push(ManifestReport {
            manifest: path,
            lock: lock_path.clone(),
            scope,
            targets: targets.clone(),
            operations: plan.operations.iter().map(Into::into).collect(),
            unchanged: plan.skips,
            conflicts: plan.conflicts.clone(),
            status: None,
        });
        jobs.push((lock_path, targets, lock, sources, plan));
    }
    if local_status {
        return Ok(report.pending_changes);
    }
    if source_lock_command {
        if !cli.dry_run {
            for (path, lock) in prepared_locks {
                lock.write(&path)?;
            }
        }
        return Ok(cli.dry_run && report.pending_changes);
    }
    // All manifests are planned before the first target is changed.
    let conflicts: Vec<_> = jobs
        .iter()
        .flat_map(|job| job.4.conflicts.iter().cloned())
        .collect();
    if !conflicts.is_empty() {
        return Err(SkillSyncError::Config(conflicts.join("\n")).into());
    }
    engine::confirm_bulk_install(
        engine::new_skill_count(jobs.iter().map(|job| &job.4)),
        cli.force,
        cli.dry_run,
    )?;
    if cli.dry_run {
        return Ok(jobs.iter().any(|job| !job.4.operations.is_empty()));
    }
    let mut errors = Vec::new();
    for (index, (lock_path, targets, mut lock, sources, plan)) in jobs.into_iter().enumerate() {
        let skills = sources.skills.as_slice();
        let mut operation_index = 0;
        if let Err(err) = engine::apply_with_observer(
            &plan,
            skills,
            &targets,
            &mut lock,
            &lock_path,
            cli.force,
            |op, outcome, error| {
                let item = &mut report.manifests[index].operations[operation_index];
                item.outcome = outcome;
                item.error = error;
                operation_index += 1;
                if !cli.json {
                    println!(
                        "{outcome:?}: {:?} {} -> {}",
                        op.action,
                        op.skill,
                        op.target.display()
                    );
                }
            },
        ) {
            errors.push(err.to_string());
        }
    }
    report.pending_changes = report
        .manifests
        .iter()
        .flat_map(|job| &job.operations)
        .any(|op| op.outcome != engine::Outcome::Applied);
    if !errors.is_empty() {
        return Err(SkillSyncError::Config(errors.join("\n")).into());
    }
    Ok(report.pending_changes)
}
