use skills_sync::config::target_dir_for_manifest;
use skills_sync::metadata::{read_lock_file, write_lock_file_atomic, LockFile, SkillMeta};
use skills_sync::resolver::{resolve, RemoteSkill, RepoSkills, SyncAction};
use skills_sync::sync::{execute_install, execute_sync_plan, execute_update, read_skill_name};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn installs_portable_skills_in_all_supported_layouts() {
    let tmp = TempDir::new().unwrap();
    let fixtures = tmp.path().join("remotes");
    for (repo, path, name) in [
        ("single.git", "", "portable-skill"),
        ("bundle.git", "skills/tool", "portable-tool"),
    ] {
        let root = fixtures.join(repo);
        let skill = root.join(path);
        std::fs::create_dir_all(skill.join("references")).unwrap();
        std::fs::write(skill.join("SKILL.md"), format!("---\nname: {name}\ndescription: >\n  A portable test skill.\n---\n# Instructions\nRead references/guide.md.\n")).unwrap();
        std::fs::write(skill.join("references/guide.md"), "Supporting instructions").unwrap();
        git(&root, &["init", "-b", "main"]);
        git(&root, &["add", "."]);
        git(
            &root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "fixture",
            ],
        );
    }
    // Isolate Git configuration in a subprocess; the suite never contacts GitHub.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "local_git_sync_worker", "--nocapture"])
        .env("SKILLS_SYNC_TEST_ROOT", tmp.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}/.insteadOf", fixtures.display()),
        )
        .env("GIT_CONFIG_VALUE_0", "https://github.com/fixture/")
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_git_sync_worker() {
    let Some(root) = std::env::var_os("SKILLS_SYNC_TEST_ROOT") else {
        return;
    };
    let root = Path::new(&root).canonicalize().unwrap();
    for layout in [
        "project",
        ".agents",
        "home/.agents/skills",
        "home/.codex/skills",
        "skills",
    ] {
        let base = root.join(layout);
        std::fs::create_dir_all(&base).unwrap();
        if layout == "project" || layout == "skills" {
            // Even a skills collection can itself be version controlled.
            std::fs::write(base.join(".git"), "gitdir: elsewhere").unwrap();
        }
        let manifest = base.join("skills.toml");
        std::fs::write(&manifest, "[skills]\nrepos = []\n").unwrap();
        let lock_path = manifest.with_extension("lock");
        let target = target_dir_for_manifest(&manifest);
        let mut lock = LockFile::new();
        for (repo, subdir, name) in [
            ("fixture/single", None, "portable-skill"),
            (
                "fixture/bundle",
                Some("skills/tool".to_string()),
                "portable-tool",
            ),
        ] {
            execute_install(
                repo, &subdir, "sha", "main", &target, None, &lock_path, &mut lock,
            )
            .unwrap();
            let installed = target.join(name);
            assert_eq!(read_skill_name(&installed).unwrap(), name);
            assert!(installed.join("references/guide.md").is_file());
            assert!(!base.join(".claude").exists());
            assert!(!target.join("skills").exists());
            let state = RepoSkills {
                repo: repo.into(),
                skills: vec![RemoteSkill {
                    subdirectory: subdir.clone(),
                    tree_sha: "sha".into(),
                }],
                commit_sha: "sha".into(),
            };
            let local = lock
                .skills
                .iter()
                .filter(|(_, meta)| meta.source == repo)
                .map(|(key, meta)| (key.clone(), meta.clone()))
                .collect();
            assert!(matches!(
                resolve(&state, &local).as_slice(),
                [SyncAction::Skip { .. }]
            ));
        }
        let old_lock = lock.clone();
        assert!(execute_install(
            "fixture/single",
            &None,
            "sha",
            "main",
            &target,
            None,
            &lock_path,
            &mut lock
        )
        .is_err());
        assert_eq!(lock, old_lock, "name collision must preserve the lock");
        // Failed updates must restore the previous installation and state.
        assert!(execute_update(
            "fixture/single",
            &None,
            "new",
            "missing-branch",
            &target,
            None,
            &lock_path,
            &mut lock
        )
        .is_err());
        assert!(target.join("portable-skill/SKILL.md").is_file());
        assert_eq!(read_lock_file(&lock_path).unwrap(), old_lock);
        execute_update(
            "fixture/single",
            &None,
            "new",
            "main",
            &target,
            None,
            &lock_path,
            &mut lock,
        )
        .unwrap();
        assert_eq!(
            lock.skills["fixture/single"].installed_at,
            old_lock.skills["fixture/single"].installed_at
        );

        // Unmanaged skills, manifest and lock survive orphan removal.
        std::fs::create_dir_all(target.join("unmanaged")).unwrap();
        execute_sync_plan(
            &[],
            &target,
            None,
            &HashMap::new(),
            &HashSet::new(),
            &lock_path,
            true,
            false,
        )
        .unwrap();
        assert!(!target.join("portable-skill").exists());
        assert!(!target.join("portable-tool").exists());
        assert!(target.join("unmanaged").is_dir());
        assert!(manifest.is_file());
        assert!(read_lock_file(&lock_path).unwrap().skills.is_empty());
    }
    // Legacy locks still point to their old directory and migrate safely on update.
    let target = root.join("legacy/skills");
    let old_path = target.join("fixture--single");
    std::fs::create_dir_all(&old_path).unwrap();
    std::fs::write(old_path.join("SKILL.md"), "old skill").unwrap();
    let lock_path = target.join(".skills.lock");
    let mut lock = LockFile::new();
    lock.skills.insert(
        "fixture/single".into(),
        SkillMeta::new("fixture/single", "sha"),
    );
    write_lock_file_atomic(&lock_path, &lock).unwrap();
    execute_update(
        "fixture/single",
        &None,
        "sha",
        "main",
        &target,
        None,
        &lock_path,
        &mut lock,
    )
    .unwrap();
    assert!(!old_path.exists());
    assert!(target.join("portable-skill/SKILL.md").is_file());
    assert_eq!(
        lock.skills["fixture/single"].directory.as_deref(),
        Some("portable-skill")
    );
}

#[test]
fn rejects_invalid_skill_metadata() {
    let dir = TempDir::new().unwrap();
    for yaml in [
        "name: ../escape\ndescription: test",
        "name: Bad-Name\ndescription: test",
        "name: bad--name\ndescription: test",
        "name: -bad\ndescription: test",
        "name: bad-\ndescription: test",
        "name: valid\ndescription: ''",
        "description: missing name",
        "name: missing-description",
    ] {
        std::fs::write(dir.path().join("SKILL.md"), format!("---\n{yaml}\n---\n")).unwrap();
        assert!(read_skill_name(dir.path()).is_err(), "{yaml}");
    }
    for text in ["# no frontmatter", "---\nname: test\ndescription: test"] {
        std::fs::write(dir.path().join("SKILL.md"), text).unwrap();
        assert!(read_skill_name(dir.path()).is_err());
    }
}

#[test]
fn cli_uses_config_location_and_respects_init_paths() {
    let tmp = TempDir::new().unwrap();
    for (folder, expected) in [
        ("project", "project/.agents/skills"),
        (".agents", ".agents/skills"),
        (".codex/skills", ".codex/skills"),
        ("skills", "skills"),
    ] {
        let base = tmp.path().join(folder);
        let manifest = base.join("skills.toml");
        let output = Command::new(env!("CARGO_BIN_EXE_skills-sync"))
            .args(["--init", "--config"])
            .arg(&manifest)
            .output()
            .unwrap();
        assert!(output.status.success());
        if folder == "project" {
            std::fs::write(base.join("package.json"), "{}").unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_skills-sync"))
            .current_dir(tmp.path())
            .env("GITHUB_TOKEN", "test-no-network")
            .args(["--config"])
            .arg(&manifest)
            .arg("--verbose")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(tmp.path().join(expected).is_dir());
        assert!(!tmp.path().join(".claude").exists());
        let automatic = Command::new(env!("CARGO_BIN_EXE_skills-sync"))
            .current_dir(&base)
            .env("GITHUB_TOKEN", "test-no-network")
            .args(["--verbose", "--dry-run"])
            .output()
            .unwrap();
        assert!(automatic.status.success());
        assert!(
            String::from_utf8_lossy(&automatic.stderr).contains(&format!(
                "target: {}",
                tmp.path().join(expected).canonicalize().unwrap().display()
            ))
        );
    }
}

#[test]
fn cli_global_target_override_and_dry_run() {
    let tmp = TempDir::new().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_skills-sync"))
            .current_dir(tmp.path())
            .env("HOME", tmp.path())
            .env("GITHUB_TOKEN", "test-no-network")
            .args(args)
            .output()
            .unwrap()
    };
    assert!(run(&["--global", "--init"]).status.success());
    let global = tmp.path().join(".agents/skills/.skills.toml");
    assert!(global.is_file());
    // A local manifest must not override the global one.
    std::fs::write(tmp.path().join(".skills.toml"), "invalid toml [").unwrap();
    assert!(run(&["--global"]).status.success());
    std::fs::rename(&global, global.with_file_name("skills.toml")).unwrap();
    assert!(run(&["--global"]).status.success());
    assert!(run(&["--init", "--config", "config/skills.toml"])
        .status
        .success());
    assert!(run(&[
        "--config",
        "config/skills.toml",
        "--target",
        "custom/skills",
        "--dry-run"
    ])
    .status
    .success());
    assert!(
        !tmp.path().join("custom").exists(),
        "dry-run must not create directories"
    );
    assert!(run(&[
        "--config",
        "config/skills.toml",
        "--target",
        "custom/skills"
    ])
    .status
    .success());
    assert!(tmp.path().join("custom/skills").is_dir());
    std::fs::write(tmp.path().join("not-a-directory"), "file").unwrap();
    assert!(!run(&[
        "--config",
        "config/skills.toml",
        "--target",
        "not-a-directory"
    ])
    .status
    .success());
    assert_eq!(
        run(&["--global", "--target", "custom/skills"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(run(&["--global", "--local"]).status.code(), Some(2));
}
