use serde_json::Value;
use skill_sync::{diff, edit, manifest, sources};
use std::{
    path::Path,
    process::{Command, Output},
};

fn skill(root: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Fixture\n---\n{body}\n"),
    )
    .unwrap();
}
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skill-sync"))
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(args)
        .output()
        .unwrap()
}
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
}
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn local_onboarding_frozen_diff_edit_protection_and_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    skill(&root.join("private/team/one"), "one", "original");
    skill(&root.join("private/team/deep/two"), "two", "original");
    std::fs::write(root.join(".skills.toml"), "# keep me\n[sync]\nscope='project'\nagents=['codex','claude-code']\n[skills]\nrepos=['local:private/team'] # selections\n").unwrap();
    let preview = cli(root, &["--local", "--dry-run", "--json"]);
    assert_eq!(preview.status.code(), Some(2));
    assert_eq!(
        json(&preview)["manifests"][0]["operations"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(!root.join(".agents").exists());
    assert!(cli(root, &["lock", "--local", "--json"]).status.success());
    let portable = std::fs::read(root.join(".skills.sources.lock")).unwrap();
    assert!(!String::from_utf8_lossy(&portable).contains(root.to_str().unwrap()));
    assert!(cli(root, &["--local", "--frozen", "--json"])
        .status
        .success());
    let ledger = std::fs::read(root.join(".skills.lock")).unwrap();
    assert!(cli(root, &["--local", "--frozen", "--json"])
        .status
        .success());
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), ledger);
    assert!(cli(root, &["status", "--local", "--json"]).status.success());
    skill(&root.join("private/team/one"), "one", "upstream");
    assert_eq!(
        cli(root, &["--local", "--frozen", "--json"]).status.code(),
        Some(1)
    );
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), ledger);
    let preview = cli(root, &["--local", "--diff", "--json"]);
    assert_eq!(preview.status.code(), Some(2));
    assert!(json(&preview)["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["comparison"] == "upstream"
            && d["files"][0]["patch"]
                .as_str()
                .unwrap()
                .contains("+upstream")));
    assert!(
        std::fs::read_to_string(root.join(".agents/skills/one/SKILL.md"))
            .unwrap()
            .contains("original")
    );
    skill(&root.join(".agents/skills/one"), "one", "local edit");
    let preview = cli(root, &["--local", "--diff", "--json"]);
    assert_eq!(preview.status.code(), Some(1));
    assert!(json(&preview)["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["comparison"] == "local"));
    assert_eq!(
        cli(root, &["--local", "--force", "--json"]).status.code(),
        Some(1)
    );
    std::fs::rename(root.join(".agents/skills/one"), root.join("saved-edit")).unwrap();
    assert!(cli(root, &["update", "--local", "--json"]).status.success());
    assert!(cli(root, &["--local", "--frozen", "--json"])
        .status
        .success());
    assert!(std::fs::read_to_string(root.join("saved-edit/SKILL.md"))
        .unwrap()
        .contains("local edit"));
    let before = std::fs::read(root.join(".skills.toml")).unwrap();
    let preview = cli(
        root,
        &[
            "remove",
            "local:private/team",
            "--local",
            "--dry-run",
            "--json",
        ],
    );
    assert_eq!(preview.status.code(), Some(2));
    assert_eq!(std::fs::read(root.join(".skills.toml")).unwrap(), before);
    assert!(
        cli(root, &["remove", "local:private/team", "--local", "--json"])
            .status
            .success()
    );
    assert!(root.join(".agents/skills/one").exists());
    assert!(cli(root, &["--local", "--force", "--json"])
        .status
        .success());
    assert!(!root.join(".agents/skills/one").exists());
}

#[test]
fn comment_preserving_edits_are_idempotent_and_detect_external_changes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("skills.toml");
    let before = "# header\n[skills]\nrepos = [\n # existing\n 'org/repo',\n] # tail\n[unrelated]\nanswer = 42 # retain\n";
    std::fs::write(&path, before).unwrap();
    let edit = edit::preview(&path, "local:../private", false, &[]).unwrap();
    for comment in ["# header", "# existing", "# tail", "answer = 42 # retain"] {
        assert!(edit.after.contains(comment));
    }
    edit::apply(&edit).unwrap();
    let duplicate = edit::preview(&path, "local:../private", false, &[]).unwrap();
    assert_eq!(duplicate.before, duplicate.after);
    let remove = edit::preview(&path, "local:../private", true, &[]).unwrap();
    edit::apply(&remove).unwrap();
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .contains("# existing"));
    std::fs::write(&path, "external edit").unwrap();
    assert!(edit::apply(&edit).is_err());
}

#[test]
fn git_cache_frozen_pins_repair_eviction_and_portability() {
    let Some(fixture) = std::env::var_os("SKILL_SYNC_GIT_CACHE_FIXTURE") else {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repository");
        skill(&repo.join("skills/one"), "one", "old");
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "old"]);
        let old = git(&repo, &["rev-parse", "HEAD"]);
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "git_cache_frozen_pins_repair_eviction_and_portability",
                "--nocapture",
            ])
            .env("SKILL_SYNC_GIT_CACHE_FIXTURE", temp.path())
            .env("SKILL_SYNC_OLD_SHA", old)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_COUNT", "4")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.file://{}.insteadOf", repo.display()),
            )
            .env("GIT_CONFIG_VALUE_0", "https://example.test/team/repo.git")
            .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
            .env("GIT_CONFIG_VALUE_1", "always")
            .env(
                "GIT_CONFIG_KEY_2",
                format!("url.file://{}.insteadOf", repo.display()),
            )
            .env("GIT_CONFIG_VALUE_2", "ssh://git@example.test/team/repo.git")
            .env(
                "GIT_CONFIG_KEY_3",
                format!("url.file://{}.insteadOf", repo.display()),
            )
            .env(
                "GIT_CONFIG_VALUE_3",
                "https://github.com/fixture/bundle.git",
            )
            .env("GITHUB_TOKEN", "fixture-only-token")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };
    let root = Path::new(&fixture);
    let path = root.join(".skills.toml");
    let home = root.join("home");
    let text = "[skills]\nrepos=['git+https://example.test/team/repo.git//skills#main']";
    std::fs::write(&path, text).unwrap();
    let manifest = manifest::parse_and_validate(text, &path).unwrap();
    let prepared = sources::prepare(&manifest, &path, &home, false, 512 * 1024 * 1024).unwrap();
    prepared.lock.write(&sources::lock_path(&path)).unwrap();
    assert_eq!(
        prepared.skills[0].sha,
        std::env::var("SKILL_SYNC_OLD_SHA").unwrap()
    );
    skill(&root.join("repository/skills/one"), "one", "new");
    git(&root.join("repository"), &["add", "."]);
    git(&root.join("repository"), &["commit", "-m", "new"]);
    let updated = sources::prepare(&manifest, &path, &home, false, 512 * 1024 * 1024).unwrap();
    assert_ne!(updated.skills[0].sha, prepared.skills[0].sha);
    std::fs::rename(root.join("repository"), root.join("offline")).unwrap();
    let frozen = sources::prepare(&manifest, &path, &home, true, 512 * 1024 * 1024).unwrap();
    assert_eq!(frozen.skills[0].sha, prepared.skills[0].sha);
    std::fs::rename(root.join("offline"), root.join("repository")).unwrap();
    let bare = std::fs::read_dir(home.join(".cache/skill-sync/repos"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::remove_file(bare.join("HEAD")).unwrap();
    assert!(sources::prepare(&manifest, &path, &home, false, 0).is_ok());
    assert_eq!(
        std::fs::read_dir(home.join(".cache/skill-sync/repos"))
            .unwrap()
            .count(),
        0
    );
    let ssh = manifest::parse_and_validate(
        "[skills]\nrepos=['git+ssh://git@example.test/team/repo.git//skills#main']",
        &path,
    )
    .unwrap();
    assert_eq!(
        sources::prepare(&ssh, &path, &home, false, 0)
            .unwrap()
            .skills[0]
            .sha,
        updated.skills[0].sha
    );
    let github =
        manifest::parse_and_validate("[skills]\nrepos=['fixture/bundle/skills']", &path).unwrap();
    assert_eq!(
        sources::prepare(&github, &path, &home, false, 0)
            .unwrap()
            .skills[0]
            .key,
        "fixture/bundle/skills/one"
    );
    // The portable lock can be used in another checkout with an empty cache.
    let other = root.join("other");
    std::fs::create_dir(&other).unwrap();
    let other_path = other.join(".skills.toml");
    std::fs::write(&other_path, text).unwrap();
    prepared
        .lock
        .write(&sources::lock_path(&other_path))
        .unwrap();
    assert_eq!(
        sources::prepare(&manifest, &other_path, &other.join("home"), true, 0)
            .unwrap()
            .skills[0]
            .sha,
        prepared.skills[0].sha
    );
}

#[test]
fn source_validation_and_custom_targets() {
    for source in [
        "local:",
        "git+http://host/repo",
        "git+file:///tmp/repo",
        "git+https://token@host/repo",
        "git+https://host/repo//../outside",
        "git+ssh://git@host/repo#--upload-pack=evil",
        "git+https://host/repo#",
    ] {
        assert!(sources::parse(source).is_err(), "{source}");
    }
    assert!(sources::parse("git+ssh://git@host/team/repo.git//skills#release/v1").is_ok());
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    skill(&root.join("source"), "one", "fixture");
    std::fs::write(root.join(".skills.toml"), "[sync]\nscope='project'\nagents=['custom']\n[agents.custom]\nproject_dir='tool/skills'\n[skills]\nrepos=['local:source']").unwrap();
    assert!(cli(root, &["--local", "--json"]).status.success());
    assert!(root.join("tool/skills/one/SKILL.md").is_file());
    std::fs::remove_dir_all(root.join("source")).unwrap();
    assert!(cli(root, &["status", "--local", "--json"]).status.success());
    assert_eq!(cli(root, &["--local", "--json"]).status.code(), Some(1));
    assert!(root.join("tool/skills/one").exists());
}

#[cfg(unix)]
#[test]
fn binary_mode_and_symlink_diffs() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    skill(&a, "one", "old");
    skill(&b, "one", "new\u{1b}[31m");
    std::fs::write(a.join("binary"), [0, 1]).unwrap();
    std::fs::write(b.join("binary"), [0, 2]).unwrap();
    std::fs::write(a.join("script"), "same").unwrap();
    std::fs::write(b.join("script"), "same").unwrap();
    std::fs::set_permissions(b.join("script"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let diffs = diff::compare(Some(&a), Some(&b)).unwrap();
    assert!(diffs.iter().any(|d| d.binary));
    assert!(diffs.iter().any(|d| d.mode_changed));
    assert!(diffs.iter().all(|d| !d.patch.contains('\u{1b}')));
    symlink(&a, b.join("escape")).unwrap();
    assert!(diff::compare(Some(&a), Some(&b)).is_err());
}

#[test]
fn source_lock_commands_ignore_deployment_state_and_reject_frozen_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    skill(&root.join("private"), "one", "original");
    std::fs::write(
        root.join(".skills.toml"),
        "[skills]\nrepos=['local:private']",
    )
    .unwrap();
    std::fs::write(root.join(".skills.lock"), "broken deployment state").unwrap();
    assert!(cli(root, &["lock", "--local", "--json"]).status.success());
    let before = std::fs::read(root.join(".skills.sources.lock")).unwrap();
    skill(&root.join("private"), "one", "changed");
    assert_eq!(
        cli(root, &["lock", "--local", "--json"]).status.code(),
        Some(1)
    );
    assert_eq!(
        std::fs::read(root.join(".skills.sources.lock")).unwrap(),
        before
    );
    assert_eq!(
        cli(root, &["update", "--local", "--dry-run", "--json"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        std::fs::read(root.join(".skills.sources.lock")).unwrap(),
        before
    );
    assert!(cli(root, &["update", "--local", "--json"]).status.success());
    std::fs::write(root.join(".skills.toml"), "[skills]\nrepos=[]").unwrap();
    assert_eq!(
        cli(root, &["lock", "--local", "--json"]).status.code(),
        Some(1)
    );
}

#[test]
fn large_diffs_detect_changes_after_the_preview_limit() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    skill(&a, "one", "same");
    skill(&b, "one", "same");
    let mut bytes = vec![b'a'; 65536];
    std::fs::write(a.join("large.txt"), &bytes).unwrap();
    bytes.push(b'b');
    std::fs::write(b.join("large.txt"), &bytes).unwrap();
    let diffs = diff::compare(Some(&a), Some(&b)).unwrap();
    assert_eq!(diffs.len(), 1);
    assert!(diffs[0].patch.contains("65536 -> 65537 bytes"));
}

#[test]
fn collections_preserve_existing_nested_discovery_rules() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    skill(&root.join("source/parent"), "parent", "parent");
    skill(&root.join("source/parent/child"), "child", "child");
    let path = root.join(".skills.toml");
    let manifest = manifest::parse_and_validate("[skills]\nrepos=['local:source']", &path).unwrap();
    assert_eq!(
        sources::prepare(&manifest, &path, &root.join("home"), false, 0)
            .unwrap()
            .skills
            .len(),
        2
    );
    let selected =
        manifest::parse_and_validate("[skills]\nrepos=['local:source/parent']", &path).unwrap();
    assert_eq!(
        sources::prepare(&selected, &path, &root.join("home"), false, 0)
            .unwrap()
            .skills
            .len(),
        1
    );
}
