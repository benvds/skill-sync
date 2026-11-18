use skill_sync::{deployment::DeploymentLock, sources::SourceLock};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn skill(root: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Pin fixture\n---\n{body}\n"),
    )
    .unwrap();
}

fn cli(root: &Path, repo: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_skill-sync"))
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("home/.config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!("url.file://{}/.insteadOf", repo.display()),
        )
        .env("GIT_CONFIG_VALUE_0", "https://example.test/team/repo.git")
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn installed_shas(root: &Path) -> BTreeMap<String, String> {
    let lock =
        DeploymentLock::read(&root.join(".skills.lock"), &root.join(".agents/skills")).unwrap();
    lock.installations
        .into_iter()
        .map(|installation| (installation.directory, installation.sha))
        .collect()
}

fn installed_body(root: &Path, name: &str) -> String {
    std::fs::read_to_string(root.join(".agents/skills").join(name).join("SKILL.md"))
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .to_owned()
}

fn source_lock(root: &Path) -> SourceLock {
    SourceLock::read(&root.join(".skills.sources.lock")).unwrap()
}

#[test]
fn branch_and_tag_pins_record_commit_and_frozen_keeps_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let repo: PathBuf = root.join("repository");
    skill(
        &repo.join("skills/branch-skill"),
        "branch-skill",
        "branch original",
    );
    skill(&repo.join("skills/tag-skill"), "tag-skill", "tag original");
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "original"]);
    let old = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["tag", "v1"]);

    std::fs::write(
        root.join(".skills.toml"),
        "[sync]\nscope = \"project\"\nagents = [\"codex\"]\n\n[skills]\nrepos = [\n  \
         \"git+https://example.test/team/repo.git//skills/branch-skill#main\",\n  \
         \"git+https://example.test/team/repo.git//skills/tag-skill#v1\",\n]\n",
    )
    .unwrap();

    assert_success(&cli(root, &repo, &["--local", "--json"]));
    assert_success(&cli(root, &repo, &["lock", "--local", "--json"]));
    let initial_lock = source_lock(root);
    let initial_sources = initial_lock
        .sources
        .iter()
        .map(|source| source.source.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        initial_sources,
        [
            "git+https://example.test/team/repo.git//skills/branch-skill#main",
            "git+https://example.test/team/repo.git//skills/tag-skill#v1",
        ]
    );
    assert!(initial_lock
        .sources
        .iter()
        .all(|source| source.commit.as_deref() == Some(old.as_str())));
    let initial = installed_shas(root);
    assert_eq!(initial.get("branch-skill"), Some(&old));
    assert_eq!(initial.get("tag-skill"), Some(&old));
    assert_eq!(installed_body(root, "branch-skill"), "branch original");
    assert_eq!(installed_body(root, "tag-skill"), "tag original");
    println!("normal sync before upstream commit: branch-skill={old}, tag-skill={old}");

    skill(
        &repo.join("skills/branch-skill"),
        "branch-skill",
        "branch updated",
    );
    skill(&repo.join("skills/tag-skill"), "tag-skill", "tag updated");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "updated"]);
    let new = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["tag", "-f", "v1", &new]);

    // An existing source lock is verified and reused; lock alone does not move it.
    let before_lock = std::fs::read(root.join(".skills.sources.lock")).unwrap();
    assert_success(&cli(root, &repo, &["lock", "--local", "--json"]));
    assert_eq!(
        std::fs::read(root.join(".skills.sources.lock")).unwrap(),
        before_lock
    );

    // Ordinary sync resolves the current branch and moved tag again.
    assert_success(&cli(root, &repo, &["--local", "--json"]));
    let updated = installed_shas(root);
    assert_eq!(updated.get("branch-skill"), Some(&new));
    assert_eq!(updated.get("tag-skill"), Some(&new));
    assert_eq!(installed_body(root, "branch-skill"), "branch updated");
    assert_eq!(installed_body(root, "tag-skill"), "tag updated");
    assert!(source_lock(root)
        .sources
        .iter()
        .all(|source| source.commit.as_deref() == Some(old.as_str())));
    println!("normal sync after upstream commit: branch-skill={new}, tag-skill={new}");

    // Frozen sync ignores both the branch head and the re-pointed tag.
    assert_success(&cli(root, &repo, &["--local", "--frozen", "--json"]));
    let frozen = installed_shas(root);
    assert_eq!(frozen.get("branch-skill"), Some(&old));
    assert_eq!(frozen.get("tag-skill"), Some(&old));
    assert_eq!(installed_body(root, "branch-skill"), "branch original");
    assert_eq!(installed_body(root, "tag-skill"), "tag original");
    println!("frozen sync after upstream commit: branch-skill={old}, tag-skill={old}");

    // update deliberately refreshes the portable source lock, so frozen moves too.
    assert_success(&cli(root, &repo, &["update", "--local", "--json"]));
    assert!(source_lock(root)
        .sources
        .iter()
        .all(|source| source.commit.as_deref() == Some(new.as_str())));
    assert_success(&cli(root, &repo, &["--local", "--frozen", "--json"]));
    let refreshed = installed_shas(root);
    assert_eq!(refreshed.get("branch-skill"), Some(&new));
    assert_eq!(refreshed.get("tag-skill"), Some(&new));
    println!("frozen sync after source-lock update: branch-skill={new}, tag-skill={new}");
}
