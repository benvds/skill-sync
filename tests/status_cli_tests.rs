use serde_json::Value;
use skill_sync::{
    deployment::{DeploymentLock, Installation},
    engine,
    manifest::Scope,
    run_lock::RunLocks,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

fn root() -> TempDir {
    TempDir::new().unwrap()
}
fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_skill-sync"));
    command
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("xdg"))
        // Status must succeed with no Git or gh executables available.
        .env("PATH", root.join("no-executables"))
        .env_remove("GITHUB_TOKEN");
    command
}
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
fn manifest(root: &Path, repos: &str) {
    std::fs::write(
        root.join(".skills.toml"),
        format!("[sync]\nscope='project'\nagents=['codex']\n[skills]\nrepos={repos}\n"),
    )
    .unwrap();
}
fn installed(target: &Path, name: &str) -> Installation {
    let directory = target.join(name);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Fixture\n---\nOriginal\n"),
    )
    .unwrap();
    let now = chrono::Utc::now();
    Installation {
        skill: format!("org/repo/{name}"),
        target: target.to_owned(),
        directory: name.into(),
        sha: "v1".into(),
        content_hash: Some(engine::content_hash(&directory).unwrap()),
        installed_at: now,
        updated_at: now,
    }
}
fn save(root: &Path, installations: Vec<Installation>) {
    DeploymentLock {
        schema_version: 2,
        scope: Some(Scope::Project),
        project_root: Some(root.to_owned()),
        installations,
    }
    .write(&root.join(".skills.lock"))
    .unwrap();
}

#[test]
fn offline_status_reports_drift_new_targets_and_unmanaged_skills_without_writes() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "['org/repo']");
    let target = root.join(".agents/skills");
    let healthy = installed(&target, "healthy");
    let edited = installed(&target, "edited");
    let missing = installed(&target, "missing");
    installed(&target, "unmanaged");
    std::fs::write(target.join("edited/SKILL.md"), "local edit").unwrap();
    std::fs::remove_dir_all(target.join("missing")).unwrap();
    save(&root, vec![healthy, edited, missing]);
    let before = std::fs::read(root.join(".skills.lock")).unwrap();
    let output = command(&root)
        .args(["status", "--local", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output);
    assert_eq!(report["success"], true);
    let status = &report["manifests"][0]["status"];
    assert_eq!(status["upstream_checked"], false);
    let states: Vec<_> = status["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["directory"].as_str().unwrap(),
                row["state"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            ("edited", "modified"),
            ("healthy", "unchanged"),
            ("missing", "missing"),
            ("unmanaged", "unmanaged")
        ]
    );
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), before);
    assert!(!root.join("home").exists());
    assert!(!root.join("xdg").exists());
    let output = command(&root)
        .args(["doctor", "--local", "--agent", "claude-code", "--json"])
        .output()
        .unwrap();
    let report = json(&output);
    assert!(report["manifests"][0]["status"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| {
            row["state"] == "missing" && row["selected"] == true && row["managed"] == false
        }));
    assert!(!root.join(".claude").exists());
}

#[test]
fn status_distinguishes_unknown_sources_legacy_baselines_and_local_health() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "['org/repo@moving-tag','org/unknown']");
    let target = root.join(".agents/skills");
    let mut item = installed(&target, "healthy");
    item.content_hash = None;
    save(&root, vec![item]);
    let output = command(&root).args(["status", "--json"]).output().unwrap();
    let report = json(&output);
    assert_eq!(
        report["manifests"][0]["status"]["unresolved_sources"],
        serde_json::json!(["org/unknown"])
    );
    assert_eq!(
        report["manifests"][0]["status"]["entries"][0]["state"],
        "unverified"
    );
    manifest(&root, "['org/repo@moving-tag']");
    save(&root, vec![installed(&target, "healthy")]);
    let output = command(&root).args(["status", "--json"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        json(&output)["manifests"][0]["status"]["upstream_checked"],
        false
    );
}

#[test]
fn json_plan_apply_and_conflict_results_agree_with_the_filesystem() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "[]");
    let target = root.join(".agents/skills");
    save(&root, vec![installed(&target, "old")]);
    let before = std::fs::read(root.join(".skills.lock")).unwrap();
    let output = command(&root)
        .args(["--local", "--dry-run", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output);
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["manifests"][0]["operations"][0]["action"], "remove");
    assert_eq!(
        report["manifests"][0]["operations"][0]["outcome"],
        "planned"
    );
    assert!(!root.join("home").exists());
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), before);
    std::fs::write(target.join("old/SKILL.md"), "edit").unwrap();
    let output = command(&root)
        .args(["--local", "--force", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["success"], false);
    assert!(!report["manifests"][0]["conflicts"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), before);
    save(&root, vec![installed(&target, "old")]);
    let output = command(&root)
        .args(["--local", "--force", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let report = json(&output);
    assert_eq!(
        report["manifests"][0]["operations"][0]["outcome"],
        "applied"
    );
    assert_eq!(report["pending_changes"], false);
    assert!(!target.join("old").exists());
    assert!(DeploymentLock::read(&root.join(".skills.lock"), &target)
        .unwrap()
        .installations
        .is_empty());
}

#[test]
fn json_reports_a_declined_removal_as_skipped() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "[]");
    let target = root.join(".agents/skills");
    save(&root, vec![installed(&target, "old")]);
    let before = std::fs::read(root.join(".skills.lock")).unwrap();
    let output = command(&root)
        .args(["--local", "--json"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output);
    assert_eq!(report["success"], true);
    assert_eq!(report["pending_changes"], true);
    assert_eq!(
        report["manifests"][0]["operations"][0]["outcome"],
        "skipped"
    );
    assert!(target.join("old").exists());
    assert_eq!(std::fs::read(root.join(".skills.lock")).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn status_reports_replaced_directories_and_unowned_selected_names() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "['org/repo']");
    let target = root.join(".agents/skills");
    save(&root, vec![installed(&target, "one")]);
    std::fs::rename(target.join("one"), root.join("backup")).unwrap();
    std::os::unix::fs::symlink(root.join("backup"), target.join("one")).unwrap();
    let other = root.join(".claude/skills");
    installed(&other, "one");
    let output = command(&root)
        .args(["status", "--json", "--agent", "codex,claude-code"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output);
    let entries = report["manifests"][0]["status"]["entries"]
        .as_array()
        .unwrap();
    assert!(entries
        .iter()
        .any(|row| row["state"] == "unreadable" && row["managed"] == true));
    assert!(entries
        .iter()
        .any(|row| row["state"] == "unmanaged" && row["selected"] == true));
    assert!(target.join("one").is_symlink());
}

#[test]
fn json_handles_argument_errors_config_errors_and_init() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    let output = command(&root)
        .args(["--json", "--unknown"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json(&output)["command"], "arguments");
    let output = command(&root).args(["--json", "--local"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(json(&output)["success"], false);
    let output = command(&root).args(["--json", "--init"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(json(&output)["command"], "init");
    let output = command(&root)
        .args(["status", "--json", "--force"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(json(&output)["success"], false);
}

#[test]
fn manifest_target_and_deselected_target_locks_block_sync_before_changes() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "[]");
    let mut locks = RunLocks::new(&root.join("home"));
    locks
        .acquire("manifest", &root.join(".skills.lock"))
        .unwrap();
    let output = command(&root).args(["--local", "--json"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(json(&output)["errors"][0]
        .as_str()
        .unwrap()
        .contains("another skill-sync"));
    assert!(!root.join(".agents").exists());
    drop(locks);
    let stale = root.join("old-target");
    save(&root, vec![installed(&stale, "old")]);
    let mut locks = RunLocks::new(&root.join("home"));
    locks.acquire("target", &stale).unwrap();
    let output = command(&root)
        .args(["--local", "--force", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stale.join("old").exists());
    assert!(!root.join(".agents").exists());
    drop(locks);
    assert!(command(&root)
        .args(["--local", "--force", "--json"])
        .output()
        .unwrap()
        .status
        .success());
    assert!(!stale.join("old").exists());
}

#[cfg(unix)]
#[test]
fn canonical_target_aliases_share_a_lock() {
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("target")).unwrap();
    std::os::unix::fs::symlink(root.join("target"), root.join("alias")).unwrap();
    let mut one = RunLocks::new(&root);
    let mut two = RunLocks::new(&root);
    one.acquire("target", &root.join("target")).unwrap();
    two.acquire("target", &root.join("independent")).unwrap();
    assert!(two.acquire("target", &root.join("alias")).is_err());
    drop(one);
    two.acquire("target", &root.join("alias")).unwrap();
}

#[test]
fn lock_worker() {
    use std::io::Write;
    let Some(root) = std::env::var_os("SKILL_SYNC_LOCK_WORKER") else {
        return;
    };
    let root = PathBuf::from(root);
    let mut locks = RunLocks::new(&root.join("home"));
    locks
        .acquire("target", &root.join(".agents/skills"))
        .unwrap();
    println!("LOCK_READY");
    std::io::stdout().flush().unwrap();
    std::thread::park();
}

#[test]
fn killing_a_lock_holder_does_not_leave_a_stale_lock() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "[]");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_worker", "--nocapture"])
        .env("SKILL_SYNC_LOCK_WORKER", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line.unwrap().contains("LOCK_READY") {
                let _ = sender.send(());
                break;
            }
        }
    });
    let ready = receiver.recv_timeout(std::time::Duration::from_secs(10));
    if ready.is_err() {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("lock worker did not become ready");
    }
    let blocked = command(&root).args(["--local", "--json"]).output().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    assert_eq!(blocked.status.code(), Some(1));
    assert!(command(&root)
        .args(["--local", "--json"])
        .output()
        .unwrap()
        .status
        .success());
}

#[cfg(unix)]
#[test]
fn json_keeps_successful_operations_when_another_target_fails() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = root();
    let root = tmp.path().canonicalize().unwrap();
    manifest(&root, "[]");
    let good = root.join("a-good");
    let bad = root.join("b-readonly");
    save(&root, vec![installed(&good, "old"), installed(&bad, "old")]);
    std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o555)).unwrap();
    let output = command(&root)
        .args(["--local", "--force", "--json"])
        .output()
        .unwrap();
    std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(
        report["manifests"][0]["operations"][0]["outcome"],
        "applied"
    );
    assert_eq!(report["manifests"][0]["operations"][1]["outcome"], "failed");
    assert!(!good.join("old").exists());
    assert!(bad.join("old").exists());
    let lock = DeploymentLock::read(&root.join(".skills.lock"), &good).unwrap();
    assert_eq!(lock.installations.len(), 1);
    assert_eq!(lock.installations[0].target, bad);
}
