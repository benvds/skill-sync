use skills_sync::{
    agents,
    deployment::DeploymentLock,
    engine::{self, Action, PreparedSkill},
    manifest::{self, Scope},
    metadata::{LockFile, SkillMeta},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn fixture(root: &Path, key: &str, name: &str, sha: &str, body: &str) -> PreparedSkill {
    let path = root.join(key.replace('/', "-"));
    std::fs::create_dir_all(path.join("scripts")).unwrap();
    std::fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Test skill\n---\n{body}\n"),
    )
    .unwrap();
    std::fs::write(path.join("scripts/run.sh"), "#!/bin/sh\necho test\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            path.join("scripts/run.sh"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    PreparedSkill::from_directory(key.into(), sha.into(), path).unwrap()
}

fn apply(skills: &[PreparedSkill], targets: &[PathBuf], lock: &mut DeploymentLock, path: &Path) {
    let plan = engine::plan(skills, targets, lock);
    assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
    engine::apply(&plan, skills, targets, lock, path, true).unwrap();
}

#[test]
fn copies_are_independent_and_tracked_per_skill_and_target() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("codex"), tmp.path().join("claude")];
    let lock_path = tmp.path().join("skills.lock");
    let sources = vec![
        fixture(tmp.path(), "org/one", "one", "v1", "Original"),
        fixture(tmp.path(), "org/two", "two", "v1", "Original"),
    ];
    let mut lock = DeploymentLock::default();
    apply(&sources, &targets, &mut lock, &lock_path);
    assert_eq!(lock.installations.len(), 4);
    assert!(lock
        .installations
        .iter()
        .all(|i| i.content_hash.as_ref().is_some_and(|h| h.len() == 64)));
    assert_eq!(DeploymentLock::read(&lock_path, &targets[0]).unwrap(), lock);
    let before = std::fs::read(&lock_path).unwrap();
    let plan = engine::plan(&sources, &targets, &lock);
    assert_eq!(plan.skips, 4);
    assert!(plan.operations.is_empty());
    engine::apply(&plan, &sources, &targets, &mut lock, &lock_path, true).unwrap();
    assert_eq!(std::fs::read(&lock_path).unwrap(), before);
    assert!(!targets[0]
        .join("one")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    let source = sources[0].path.join("SKILL.md");
    std::fs::write(source, "Changed source").unwrap();
    assert!(std::fs::read_to_string(targets[0].join("one/SKILL.md"))
        .unwrap()
        .contains("Original"));
    assert_eq!(
        std::fs::read(targets[0].join("one/scripts/run.sh")).unwrap(),
        std::fs::read(targets[1].join("one/scripts/run.sh")).unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(targets[0].join("one/scripts/run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0o111
        );
    }
}

#[test]
fn missing_copy_and_new_target_do_not_reset_other_installations() {
    let tmp = TempDir::new().unwrap();
    let mut targets = vec![tmp.path().join("a"), tmp.path().join("b")];
    let path = tmp.path().join("skills.lock");
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "old")];
    let mut lock = DeploymentLock::default();
    apply(&sources, &targets, &mut lock, &path);
    let first = lock.installations[0].clone();
    std::fs::remove_dir_all(targets[1].join("skill")).unwrap();
    targets.push(tmp.path().join("c"));
    let plan = engine::plan(&sources, &targets, &lock);
    assert_eq!(plan.skips, 1);
    assert_eq!(plan.operations.len(), 2);
    assert!(plan
        .operations
        .iter()
        .all(|op| op.action == Action::Install));
    engine::apply(&plan, &sources, &targets, &mut lock, &path, true).unwrap();
    assert!(lock.installations.contains(&first));
    assert_eq!(lock.installations.len(), 3);
}

#[test]
fn updates_and_removed_targets_preserve_unmanaged_skills() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("a"), tmp.path().join("b")];
    let path = tmp.path().join("skills.lock");
    let mut lock = DeploymentLock::default();
    let sources = vec![fixture(tmp.path(), "org/repo", "old-name", "v1", "old")];
    apply(&sources, &targets, &mut lock, &path);
    let installed_at = lock.installations[0].installed_at;
    std::fs::create_dir(targets[1].join("unmanaged")).unwrap();
    let sources = vec![fixture(tmp.path(), "org/repo", "new-name", "v2", "new")];
    apply(&sources, &targets[..1], &mut lock, &path);
    assert_eq!(lock.installations.len(), 1);
    assert_eq!(lock.installations[0].sha, "v2");
    assert_eq!(lock.installations[0].installed_at, installed_at);
    assert!(targets[0].join("new-name/SKILL.md").is_file());
    assert!(!targets[0].join("old-name").exists());
    assert!(!targets[1].join("old-name").exists());
    assert!(targets[1].join("unmanaged").is_dir());
    apply(&[], &targets[..1], &mut lock, &path);
    assert!(lock.installations.is_empty());
}

#[test]
fn local_edits_and_unmanaged_collisions_block_all_changes_even_with_force() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("a"), tmp.path().join("b")];
    let path = tmp.path().join("skills.lock");
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "old")];
    let mut lock = DeploymentLock::default();
    apply(&sources, &targets, &mut lock, &path);
    let before = std::fs::read(&path).unwrap();
    std::fs::write(targets[0].join("skill/scripts/run.sh"), "My changes").unwrap();
    let plan = engine::plan(&sources, &targets, &lock);
    assert_eq!(plan.conflicts.len(), 1);
    assert!(engine::apply(&plan, &sources, &targets, &mut lock, &path, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let removal = engine::plan(&[], &targets, &lock);
    assert!(!removal.conflicts.is_empty());
    assert!(engine::apply(&removal, &[], &targets, &mut lock, &path, true).is_err());
    assert!(targets[1].join("skill/SKILL.md").is_file());

    let fresh = vec![tmp.path().join("fresh"), tmp.path().join("collision")];
    std::fs::create_dir_all(fresh[1].join("skill")).unwrap();
    let mut empty = DeploymentLock::default();
    let plan = engine::plan(&sources, &fresh, &empty);
    assert!(!plan.conflicts.is_empty());
    assert!(engine::apply(
        &plan,
        &sources,
        &fresh,
        &mut empty,
        &tmp.path().join("fresh.lock"),
        true
    )
    .is_err());
    assert!(!fresh[0].exists());
}

#[test]
fn a_failed_target_can_be_retried_without_reinstalling_successful_copies() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("a-good"), tmp.path().join("z-bad")];
    std::fs::write(&targets[1], "blocks directory").unwrap();
    let path = tmp.path().join("skills.lock");
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "old")];
    let mut lock = DeploymentLock::default();
    let plan = engine::plan(&sources, &targets, &lock);
    assert!(engine::apply(&plan, &sources, &targets, &mut lock, &path, true).is_err());
    assert_eq!(lock.installations.len(), 1);
    assert_eq!(
        DeploymentLock::read(&path, &targets[0])
            .unwrap()
            .installations
            .len(),
        1
    );
    std::fs::remove_file(&targets[1]).unwrap();
    let retry = engine::plan(&sources, &targets, &lock);
    assert_eq!(retry.skips, 1);
    assert_eq!(retry.operations.len(), 1);
    engine::apply(&retry, &sources, &targets, &mut lock, &path, true).unwrap();
    assert_eq!(lock.installations.len(), 2);
}

#[test]
fn lock_write_failures_roll_back_install_update_and_removal() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("target")];
    let path = tmp.path().join("skills.lock");
    let invalid_lock = tmp.path().join("a-directory");
    std::fs::create_dir(&invalid_lock).unwrap();
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "old")];
    let mut lock = DeploymentLock::default();
    let plan = engine::plan(&sources, &targets, &lock);
    assert!(engine::apply(&plan, &sources, &targets, &mut lock, &invalid_lock, true).is_err());
    assert!(!targets[0].join("skill").exists());
    assert!(lock.installations.is_empty());
    apply(&sources, &targets, &mut lock, &path);
    let before = lock.clone();
    let updated = vec![fixture(tmp.path(), "org/repo", "renamed", "v2", "new")];
    let plan = engine::plan(&updated, &targets, &lock);
    assert!(engine::apply(&plan, &updated, &targets, &mut lock, &invalid_lock, true).is_err());
    assert!(targets[0].join("skill/SKILL.md").is_file());
    assert!(!targets[0].join("renamed").exists());
    assert_eq!(lock, before);
    let removal = engine::plan(&[], &targets, &lock);
    assert!(engine::apply(&removal, &[], &targets, &mut lock, &invalid_lock, true).is_err());
    assert!(targets[0].join("skill/SKILL.md").is_file());
    assert_eq!(lock, before);
}

#[test]
fn legacy_lock_imports_only_its_original_target_and_preserves_unverified_edits() {
    let tmp = TempDir::new().unwrap();
    let targets = vec![tmp.path().join("a"), tmp.path().join("b")];
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "original")];
    let path = tmp.path().join("skills.lock");
    let mut original = LockFile::new();
    let mut meta = SkillMeta::new("org/repo", "v1");
    meta.directory = Some("skill".into());
    original.skills.insert("org/repo".into(), meta);
    skills_sync::metadata::write_lock_file_atomic(&path, &original).unwrap();
    let mut migrated = DeploymentLock::read(&path, &targets[0]).unwrap();
    assert_eq!(migrated.installations.len(), 1);
    assert_eq!(migrated.installations[0].target, targets[0]);
    // Missing copies are safely installed, and subsequent locks use schema 2.
    apply(&sources, &targets, &mut migrated, &path);
    assert_eq!(migrated.schema_version, 2);
    assert_eq!(migrated.installations.len(), 2);
    skills_sync::metadata::write_lock_file_atomic(&path, &original).unwrap();
    let migrated = DeploymentLock::read(&path, &targets[0]).unwrap();
    assert!(engine::plan(&sources, &targets, &migrated)
        .conflicts
        .iter()
        .any(|s| s.contains("collision")));
    let plan = engine::plan(&sources, &targets[..1], &migrated);
    assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
    std::fs::write(targets[0].join("skill/SKILL.md"), "My edits").unwrap();
    assert!(!engine::plan(&sources, &targets[..1], &migrated)
        .conflicts
        .is_empty());
}

#[test]
fn duplicate_names_and_unsafe_lock_records_are_rejected() {
    let tmp = TempDir::new().unwrap();
    let sources = vec![
        fixture(tmp.path(), "a/repo", "same", "v1", "a"),
        fixture(tmp.path(), "b/repo", "same", "v1", "b"),
    ];
    assert!(!engine::plan(
        &sources,
        &[tmp.path().join("target")],
        &DeploymentLock::default()
    )
    .conflicts
    .is_empty());
    let path = tmp.path().join("lock");
    for text in ["schema_version = 99", "schema_version = 2\n[[installations]]\nskill = 'a/b'\ntarget = '/tmp'\ndirectory = '../unsafe'\nsha = 'a'\ninstalled_at = '2026-10-25T00:00:00Z'\nupdated_at = '2026-10-25T00:00:00Z'"] {
        std::fs::write(&path, text).unwrap();
        assert!(DeploymentLock::read(&path, tmp.path()).is_err());
    }
}

#[test]
fn project_locks_rebase_to_another_checkout_without_touching_the_original() {
    let tmp = TempDir::new().unwrap();
    let old = tmp.path().join("old-project");
    let new = tmp.path().join("new-project");
    let targets = vec![old.join(".agents/skills")];
    let sources = vec![fixture(tmp.path(), "org/repo", "skill", "v1", "original")];
    let mut lock = DeploymentLock::default();
    lock.bind_scope(Scope::Project, &old).unwrap();
    apply(
        &sources,
        &targets,
        &mut lock,
        &tmp.path().join("project.lock"),
    );
    lock.bind_scope(Scope::Project, &new).unwrap();
    assert_eq!(lock.installations[0].target, new.join(".agents/skills"));
    let plan = engine::plan(&sources, &[new.join(".agents/skills")], &lock);
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].action, Action::Install);
    assert!(targets[0].join("skill/SKILL.md").is_file());
    assert!(lock.bind_scope(Scope::User, &new).is_err());
}

#[test]
fn agent_paths_deduplicate_shared_targets_and_honor_custom_paths() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let home = root.join("home");
    let config = root.join("xdg");
    let content = "[sync]\nscope='user'\nagents=['codex','goose','universal','claude-code','cursor','gemini-cli','copilot','opencode','kiro']\n[skills]\nrepos=[]";
    let manifest = manifest::parse_and_validate(content, &root.join("skills.toml")).unwrap();
    let targets = agents::target_directories(
        &manifest,
        &root.join("skills.toml"),
        Scope::User,
        &[],
        &home,
        &config,
    )
    .unwrap();
    assert_eq!(targets.len(), 7);
    assert!(targets.contains(&config.join("opencode/skills")));
    let custom = manifest::parse_and_validate("[sync]\nagents=['codex']\n[agents.codex]\nuser_dir='~/.codex/skills'\nproject_dir='.team/skills'\n[skills]\nrepos=[]", &root.join("skills.toml")).unwrap();
    assert_eq!(
        agents::target_directories(
            &custom,
            &root.join("skills.toml"),
            Scope::User,
            &[],
            &home,
            &config
        )
        .unwrap(),
        vec![home.join(".codex/skills")]
    );
    assert_eq!(
        agents::target_directories(
            &custom,
            &root.join("skills.toml"),
            Scope::Project,
            &[],
            &home,
            &config
        )
        .unwrap(),
        vec![root.join(".team/skills")]
    );
    let invalid = manifest::parse_and_validate(
        "[sync]\nagents=['codex']\n[agents.codex]\nproject_dir='../outside'\n[skills]\nrepos=[]",
        &root.join("skills.toml"),
    )
    .unwrap();
    assert!(agents::target_directories(
        &invalid,
        &root.join("skills.toml"),
        Scope::Project,
        &[],
        &home,
        &config
    )
    .is_err());
}

#[test]
fn unknown_agents_modes_and_traversal_repos_fail_before_installation() {
    for content in [
        "[sync]\nagents=['typo']\n[skills]\nrepos=[]",
        "[sync]\nmode='symlink'\n[skills]\nrepos=[]",
        "[skills]\nrepos=['owner/repo/../escape']",
    ] {
        assert!(manifest::parse_and_validate(content, Path::new("test.toml")).is_err());
    }
}

fn cli(root: &Path, cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_skills-sync"))
        .current_dir(cwd)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        .env("GITHUB_TOKEN", "test-no-network")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_combines_central_and_nearest_project_manifests_and_limits_scopes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let project = root.join("project");
    std::fs::create_dir_all(project.join("src/nested")).unwrap();
    std::fs::write(
        project.join(".skills.toml"),
        "[sync]\nscope='project'\nagents=['codex','claude-code']\n[skills]\nrepos=[]",
    )
    .unwrap();
    assert!(cli(&root, &project, &["--global", "--init"])
        .status
        .success());
    let dry = cli(
        &root,
        &project.join("src/nested"),
        &["--verbose", "--dry-run"],
    );
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let output = String::from_utf8_lossy(&dry.stderr);
    assert!(output.contains("scope: User"));
    assert!(output.contains("scope: Project"));
    assert!(!root.join("home/.agents").exists());
    assert!(!project.join(".agents").exists());
    assert!(cli(&root, &project, &["--global"]).status.success());
    assert!(root.join("home/.agents/skills").is_dir());
    assert!(root.join("home/.claude/skills").is_dir());
    assert!(!project.join(".agents").exists());
    assert!(cli(&root, &project.join("src/nested"), &["--local"])
        .status
        .success());
    assert!(project.join(".agents/skills").is_dir());
    assert!(project.join(".claude/skills").is_dir());
    let override_run = cli(
        &root,
        &project,
        &["--local", "--agent", "cursor", "--verbose"],
    );
    assert!(override_run.status.success());
    assert!(project.join(".cursor/skills").is_dir());
    assert!(!cli(&root, &project, &["--target", "other"])
        .status
        .success());
    assert!(
        !cli(&root, &project, &["--local", "--target", "../outside"])
            .status
            .success()
    );
    assert!(!root.join("outside").exists());
}

#[cfg(unix)]
#[test]
fn target_aliases_deduplicate_and_symlinked_skill_resources_are_rejected() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("shared")).unwrap();
    std::os::unix::fs::symlink(root.join("shared"), root.join("alias")).unwrap();
    let config = manifest::parse_and_validate("[sync]\nagents=['codex','goose']\n[agents.codex]\nproject_dir='shared'\n[agents.goose]\nproject_dir='alias'\n[skills]\nrepos=[]", &root.join("skills.toml")).unwrap();
    assert_eq!(
        agents::target_directories(
            &config,
            &root.join("skills.toml"),
            Scope::Project,
            &[],
            &root,
            &root
        )
        .unwrap(),
        vec![root.join("shared")]
    );
    let skill = fixture(&root, "org/repo", "skill", "v1", "test");
    std::os::unix::fs::symlink("/etc/passwd", skill.path.join("external")).unwrap();
    assert!(engine::content_hash(&skill.path).is_err());
}

#[test]
fn resolves_multiple_pins_once_and_deploys_both_skills_to_all_targets() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("bundle.git");
    fixture(&repo.join("skills"), "org/one", "one", "v1", "one");
    fixture(&repo.join("skills"), "org/two", "two", "v1", "two");
    for args in [
        vec!["init", "-b", "main"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "fixtures",
        ],
    ] {
        let output = Command::new("git")
            .current_dir(&repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "prepared_sources_worker", "--nocapture"])
        .env("SKILLS_SYNC_SOURCE_FIXTURE", tmp.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}/.insteadOf", tmp.path().display()),
        )
        .env("GIT_CONFIG_VALUE_0", "https://github.com/fixture/")
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .env("GIT_TRACE2_EVENT", tmp.path().join("git-events.jsonl"))
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let events = std::fs::read_to_string(tmp.path().join("git-events.jsonl")).unwrap();
    let clones = events
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|event| {
            event["event"] == "start"
                && event["argv"]
                    .as_array()
                    .is_some_and(|args| args.get(1).is_some_and(|value| value == "clone"))
        })
        .count();
    assert_eq!(
        clones, 1,
        "each source repo is cloned only once for all pins and targets"
    );
}

#[test]
fn prepared_sources_worker() {
    use skills_sync::github::{AuthMethod, GitHubClient};
    use std::io::{Read, Write};
    let Some(root) = std::env::var_os("SKILLS_SYNC_SOURCE_FIXTURE") else {
        return;
    };
    let root = PathBuf::from(root);
    let sha = Command::new("git")
        .current_dir(root.join("bundle.git"))
        .args(["rev-parse", "HEAD^{tree}"])
        .output()
        .unwrap();
    let sha = String::from_utf8(sha.stdout).unwrap().trim().to_owned();
    let tree = serde_json::json!({"sha": "cached-api-tree", "truncated": true, "tree": [
        {"path": "skills/org-one/SKILL.md", "type": "blob", "sha": "unused"},
        {"path": "skills/org-two/SKILL.md", "type": "blob", "sha": "unused"}
    ]})
    .to_string();
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", server.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 4096];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            if index == 0 {
                assert!(request.starts_with("GET /repos/fixture/bundle HTTP"));
            } else {
                assert!(request
                    .starts_with("GET /repos/fixture/bundle/git/trees/main?recursive=1 HTTP"));
            }
            let body = if index == 0 {
                "{\"default_branch\":\"main\"}"
            } else {
                &tree
            };
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    let manifest = manifest::parse_and_validate(
        "[skills]\nrepos=['fixture/bundle/skills/org-one','fixture/bundle/skills/org-two']",
        Path::new("skills.toml"),
    )
    .unwrap();
    let client = GitHubClient::with_base_url(None, AuthMethod::Unauthenticated, &endpoint).unwrap();
    let sources = engine::PreparedSources::fetch(&manifest, &client, None).unwrap();
    worker.join().unwrap();
    assert_eq!(sources.skills.len(), 2);
    assert!(
        sources.skills.iter().all(|skill| skill.sha == sha),
        "lock versions must describe the actual checkout"
    );
    let mut lock = DeploymentLock::default();
    let targets = vec![
        root.join("home/.agents/skills"),
        root.join("home/.claude/skills"),
    ];
    apply(
        &sources.skills,
        &targets,
        &mut lock,
        &root.join("skills.lock"),
    );
    assert_eq!(lock.installations.len(), 4);
    assert!(targets.iter().all(
        |target| target.join("one/SKILL.md").is_file() && target.join("two/SKILL.md").is_file()
    ));
    assert!(!targets[0].join("one/.git").exists());
}
