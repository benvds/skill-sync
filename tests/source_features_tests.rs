use skill_sync::{
    deployment::DeploymentLock,
    engine::{self, PreparedSources},
    github::{AuthMethod, GitHubClient},
    manifest,
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Test",
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn write_skill(root: &Path, folder: &str, name: &str, text: &str) {
    let path = root.join(folder);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Fixture\n---\n{text}\n"),
    )
    .unwrap();
}

#[test]
fn source_features_use_real_historical_checkouts() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("bundle.git");
    write_skill(&repo, "skills/team/one", "one", "original");
    write_skill(&repo, "skills/team/deep/two", "two", "original");
    write_skill(&repo, "skills/teammate/three", "three", "original");
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "original"]);
    git(&repo, &["tag", "-a", "v1.0.0", "-m", "annotated release"]);
    git(&repo, &["tag", "1.0.0"]);
    git(&repo, &["tag", "release/v1"]);
    let old = git(&repo, &["rev-parse", "HEAD"]);
    write_skill(&repo, "skills/team/one", "one", "newer");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "newer"]);
    let new = git(&repo, &["rev-parse", "HEAD"]);

    // Isolate Git URL rewriting and proxy configuration from other tests.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "source_features_worker", "--nocapture"])
        .env("SKILLS_SYNC_FEATURE_FIXTURE", tmp.path())
        .env("SKILLS_SYNC_OLD_COMMIT", &old)
        .env("SKILLS_SYNC_NEW_COMMIT", &new)
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
}

struct ApiFixture {
    client: GitHubClient,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl ApiFixture {
    fn new(old: &str, new: &str) -> Self {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        server.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", server.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let old = old.to_owned();
        let new = new.to_owned();
        let worker = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let (mut stream, _) = match server.accept() {
                    Ok(stream) => stream,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    Err(err) => panic!("{err}"),
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0; 4096];
                let length = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..length]);
                let path = request.split_whitespace().nth(1).unwrap().to_owned();
                recorded.lock().unwrap().push(path.clone());
                let (status, body) = if path == "/repos/fixture/bundle" {
                    (200, "{\"default_branch\":\"main\"}".to_owned())
                } else if let Some(reference) = path.strip_prefix("/repos/fixture/bundle/commits/")
                {
                    match reference {
                        "missing" => (404, "{}".to_owned()),
                        "main" => (200, serde_json::json!({"sha": new}).to_string()),
                        reference
                            if ["v1.0.0", "1.0.0", "release%2Fv1", &old, &old[..7]]
                                .contains(&reference) =>
                        {
                            (200, serde_json::json!({"sha": old}).to_string())
                        }
                        _ => panic!("unexpected ref {reference}"),
                    }
                } else if path.starts_with("/repos/fixture/bundle/git/trees/") {
                    // Force discovery to inspect the actual checkout.
                    (
                        200,
                        "{\"sha\":\"cached\",\"truncated\":true,\"tree\":[]}".to_owned(),
                    )
                } else {
                    panic!("unexpected API request {path}");
                };
                write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        Self {
            client: GitHubClient::with_base_url(None, AuthMethod::Unauthenticated, &endpoint)
                .unwrap(),
            requests,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for ApiFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn fetch(api: &ApiFixture, entries: &[&str]) -> skill_sync::error::Result<PreparedSources> {
    let content = format!("[skills]\nrepos={entries:?}");
    let manifest = manifest::parse_and_validate(&content, Path::new("skills.toml"))?;
    PreparedSources::fetch(&manifest, &api.client, None)
}

#[test]
fn source_features_worker() {
    let Some(root) = std::env::var_os("SKILLS_SYNC_FEATURE_FIXTURE") else {
        return;
    };
    let root = PathBuf::from(root);
    let old = std::env::var("SKILLS_SYNC_OLD_COMMIT").unwrap();
    let new = std::env::var("SKILLS_SYNC_NEW_COMMIT").unwrap();
    let api = ApiFixture::new(&old, &new);

    assert_eq!(fetch(&api, &["fixture/bundle"]).unwrap().skills.len(), 3);
    let folder = fetch(&api, &["fixture/bundle/skills/team"]).unwrap();
    assert_eq!(folder.skills.len(), 2);
    assert!(folder
        .skills
        .iter()
        .all(|skill| skill.key.starts_with("fixture/bundle/skills/team/")));
    assert_eq!(
        fetch(&api, &["fixture/bundle/skills/team/one"])
            .unwrap()
            .skills
            .len(),
        1
    );
    assert!(fetch(&api, &["fixture/bundle/missing"]).is_err());

    // Annotated tags, exact version tags, slash tags and old abbreviated/full commits.
    for revision in ["v1.0.0", "1.0.0", "release/v1", &old[..7], &old] {
        let source = format!("fixture/bundle/skills/team@{revision}");
        let pinned = fetch(&api, &[&source]).unwrap();
        assert_eq!(pinned.skills.len(), 2);
        for skill in &pinned.skills {
            assert_eq!(skill.sha, old);
            assert!(std::fs::read_to_string(skill.path.join("SKILL.md"))
                .unwrap()
                .contains("original"));
        }
    }
    assert_eq!(
        fetch(&api, &["fixture/bundle@v1.0.0"])
            .unwrap()
            .skills
            .len(),
        3
    );
    assert_eq!(
        fetch(&api, &["fixture/bundle", "fixture/bundle@main"])
            .unwrap()
            .skills
            .len(),
        3
    );
    // The API resolved the old tag before it moved: fetch must use that SHA.
    git(
        &root.join("bundle.git"),
        &["tag", "-f", "release/v1", "main"],
    );
    let snapshot = fetch(&api, &["fixture/bundle/skills/team/one@release/v1"]).unwrap();
    assert_eq!(snapshot.skills[0].sha, old);
    assert!(
        std::fs::read_to_string(snapshot.skills[0].path.join("SKILL.md"))
            .unwrap()
            .contains("original")
    );

    // Different paths can select different commits of the same repository.
    let mixed = fetch(
        &api,
        &[
            "fixture/bundle/skills/team/one@v1.0.0",
            "fixture/bundle/skills/team/deep/two@main",
        ],
    )
    .unwrap();
    assert_eq!(
        mixed
            .skills
            .iter()
            .find(|s| s.directory == "one")
            .unwrap()
            .sha,
        old
    );
    assert_eq!(
        mixed
            .skills
            .iter()
            .find(|s| s.directory == "two")
            .unwrap()
            .sha,
        new
    );
    let conflict = fetch(
        &api,
        &[
            "fixture/bundle/skills/team@v1.0.0",
            "fixture/bundle/skills/team/one@main",
        ],
    );
    assert!(conflict
        .err()
        .unwrap()
        .to_string()
        .contains("conflicting revisions"));
    assert!(fetch(&api, &["fixture/bundle@missing"])
        .err()
        .unwrap()
        .to_string()
        .contains("revision `missing` not found"));

    api.requests.lock().unwrap().clear();
    let pinned = fetch(
        &api,
        &[
            "fixture/bundle/skills/team@v1.0.0",
            "fixture/bundle/skills/team/one@v1.0.0",
        ],
    )
    .unwrap();
    assert_eq!(
        pinned.skills.len(),
        2,
        "overlapping selections must deduplicate"
    );
    assert_eq!(
        api.requests.lock().unwrap().len(),
        2,
        "one revision resolution and tree per repo/ref"
    );
    let targets = vec![root.join("target-one"), root.join("target-two")];
    let mut lock = DeploymentLock::default();
    let lock_path = root.join("skills.lock");
    let plan = engine::plan(&pinned.skills, &targets, &lock);
    assert_eq!(engine::new_skill_count([&plan]), 2);
    engine::apply(&plan, &pinned.skills, &targets, &mut lock, &lock_path, true).unwrap();
    assert!(lock.installations.iter().all(|i| i.sha == old));
    assert_eq!(engine::plan(&pinned.skills, &targets, &lock).skips, 4);
    // Changing a pin updates the same skill identity instead of remove + install.
    let newer = fetch(&api, &["fixture/bundle/skills/team@main"]).unwrap();
    let update = engine::plan(&newer.skills, &targets, &lock);
    assert_eq!(update.operations.len(), 4);
    assert!(update
        .operations
        .iter()
        .all(|op| op.action == engine::Action::Update));
}

#[test]
fn bulk_count_deduplicates_targets_and_checks_the_whole_run() {
    let tmp = tempfile::tempdir().unwrap();
    let mut sources = Vec::new();
    for index in 0..10 {
        let name = format!("skill-{index}");
        write_skill(tmp.path(), &name, &name, "fixture");
        sources.push(
            engine::PreparedSkill::from_directory(
                format!("org/repo/{name}"),
                "sha".into(),
                tmp.path().join(&name),
            )
            .unwrap(),
        );
    }
    let targets = vec![tmp.path().join("a"), tmp.path().join("b")];
    let lock = DeploymentLock::default();
    let nine = engine::plan(&sources[..9], &targets, &lock);
    assert_eq!(nine.operations.len(), 18);
    assert_eq!(engine::new_skill_count([&nine]), 9);
    let ten = engine::plan(&sources, &targets, &lock);
    assert_eq!(engine::new_skill_count([&ten]), 10);
    let first = engine::plan(&sources[..5], &targets, &lock);
    let second = engine::plan(&sources[5..], &targets, &lock);
    assert_eq!(engine::new_skill_count([&first, &second]), 10);
    assert!(engine::confirm_bulk_install(10, false, true).is_ok());
    assert!(engine::confirm_bulk_install(10, true, false).is_ok());
    assert!(targets.iter().all(|target| !target.exists()));
}
