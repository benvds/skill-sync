//! Source resolution is separate from machine-specific deployment state.
use crate::{
    engine::{self, PreparedSkill},
    error::{Result, SkillSyncError},
    manifest::{self, Manifest},
    run_lock::RunLocks,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

fn invalid(message: impl Into<String>) -> miette::Report {
    SkillSyncError::Config(message.into()).into()
}

#[derive(Debug)]
pub enum Source {
    Local(PathBuf),
    Git {
        url: String,
        prefix: String,
        subdir: String,
        revision: String,
    },
}

pub fn parse(entry: &str) -> Result<Source> {
    if let Some(path) = entry.strip_prefix("local:") {
        if path.is_empty() || path.chars().any(char::is_control) {
            return Err(invalid("local: requires a directory path"));
        }
        return Ok(Source::Local(path.into()));
    }
    if let Some(remote) = entry.strip_prefix("git+") {
        let (remote, revision) = remote.rsplit_once('#').unwrap_or((remote, "HEAD"));
        let (scheme, address) = remote
            .split_once("://")
            .ok_or_else(|| invalid("use git+https:// or git+ssh://"))?;
        if !matches!(scheme, "https" | "ssh") {
            return Err(invalid("Git sources allow HTTPS and SSH only"));
        }
        let (address, subdir) = address.split_once("//").unwrap_or((address, ""));
        let url = reqwest::Url::parse(&format!("{scheme}://{address}"))
            .map_err(|_| invalid("invalid Git URL"))?;
        if url.host_str().is_none()
            || url.path().trim_matches('/').is_empty()
            || url.password().is_some()
            || (scheme == "https" && !url.username().is_empty())
            || url.query().is_some()
            || url.fragment().is_some()
            || remote.chars().any(char::is_control)
            || address.contains('%')
        {
            return Err(invalid("Git URLs require a host and repository, without credentials, query or escapes; use Git credential helpers or SSH"));
        }
        if !subdir.is_empty()
            && (!subdir
                .split('/')
                .all(|s| !s.is_empty() && s != "." && s != "..")
                || subdir.contains('\\'))
        {
            return Err(invalid("Git skill paths must stay inside the repository"));
        }
        if !manifest::valid_revision(revision) {
            return Err(invalid("invalid Git revision"));
        }
        return Ok(Source::Git {
            url: format!("{scheme}://{address}"),
            prefix: format!("git+{scheme}://{address}"),
            subdir: subdir.into(),
            revision: revision.into(),
        });
    }
    let (source, revision) = manifest::split_revision(entry);
    if !manifest::valid_github_source(source)
        || revision.is_some_and(|r| !manifest::valid_revision(r))
    {
        return Err(invalid(format!("unsupported source `{entry}`; use owner/repo[/path][@ref], local:path or git+https://host/repo.git[//path][#ref]")));
    }
    let (owner, repo, subdir) = manifest::parse_repo_entry(entry);
    Ok(Source::Git {
        url: format!("https://github.com/{owner}/{repo}.git"),
        prefix: format!("{owner}/{repo}"),
        subdir: subdir.unwrap_or("").into(),
        revision: revision.unwrap_or("HEAD").into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockedSkill {
    pub key: String,
    pub path: String,
    pub name: String,
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockedSource {
    pub source: String,
    pub commit: Option<String>,
    pub skills: Vec<LockedSkill>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceLock {
    pub schema_version: u32,
    pub sources: Vec<LockedSource>,
}

pub fn lock_path(manifest: &Path) -> PathBuf {
    manifest.with_extension("sources.lock")
}
impl SourceLock {
    pub fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            invalid(format!(
                "cannot read {}: {e}; run skill-sync lock first",
                path.display()
            ))
        })?;
        let lock: Self =
            toml::from_str(&text).map_err(|e| invalid(format!("invalid source lock: {e}")))?;
        if lock.schema_version != 1 {
            return Err(invalid("unsupported source lock schema"));
        }
        Ok(lock)
    }
    pub fn write(&self, path: &Path) -> Result<()> {
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
            .map_err(SkillSyncError::Io)?;
        temp.write_all(
            toml::to_string_pretty(self)
                .map_err(|e| invalid(e.to_string()))?
                .as_bytes(),
        )
        .map_err(SkillSyncError::Io)?;
        temp.flush().map_err(SkillSyncError::Io)?;
        temp.persist(path)
            .map_err(|e| SkillSyncError::Io(e.error))?;
        Ok(())
    }
}

pub struct Prepared {
    pub skills: Vec<PreparedSkill>,
    pub lock: SourceLock,
    _checkout: tempfile::TempDir,
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let mut command = Command::new("git");
    command.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "protocol.ext.allow=never",
    ]);
    if args
        .iter()
        .any(|arg| arg.starts_with("https://github.com/"))
    {
        if let (Some(token), _) = crate::github::discover_token() {
            use base64::Engine;
            let auth =
                base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
            command
                .arg("--config-env=http.https://github.com/.extraheader=SKILL_SYNC_AUTH_HEADER")
                .env(
                    "SKILL_SYNC_AUTH_HEADER",
                    format!("AUTHORIZATION: basic {auth}"),
                );
        }
    }
    let output = command
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(SkillSyncError::Io)?;
    if !output.status.success() {
        return Err(invalid(format!(
            "Git {} failed; check repository access, ref, credential helper or SSH agent",
            args.first().unwrap_or(&"operation")
        )));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|_| invalid("Git returned non-UTF-8 output"))
}

// Persistent bare repositories store objects, never credentials or working copies.
// One cache lock coordinates repair and LRU eviction with all cache readers.
fn checkout(
    url: &str,
    revision: &str,
    locked: Option<&str>,
    cache: &Path,
    root: &Path,
) -> Result<String> {
    let key = format!("{:x}", Sha256::digest(url.as_bytes()));
    let repo = cache.join(key);
    if repo.exists()
        && (!matches!(
            git(&repo, &["rev-parse", "--is-bare-repository"]).as_deref(),
            Ok("true")
        ) || git(&repo, &["fsck", "--no-dangling"]).is_err())
    {
        std::fs::remove_dir_all(&repo).map_err(SkillSyncError::Io)?;
    }
    if !repo.exists() {
        std::fs::create_dir(&repo).map_err(SkillSyncError::Io)?;
        git(&repo, &["init", "--bare"])?;
    }
    let commit = if let Some(sha) = locked {
        if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("source lock requires full commit SHAs"));
        }
        if git(&repo, &["cat-file", "-e", &format!("{sha}^{{commit}}")]).is_err() {
            git(&repo, &["fetch", "--force", "--depth=1", "--", url, sha])?;
        }
        sha.to_owned()
    } else {
        git(
            &repo,
            &["fetch", "--force", "--depth=1", "--", url, revision],
        )?;
        git(&repo, &["rev-parse", "FETCH_HEAD^{commit}"])?
    };
    // A ref keeps frozen commits reachable through future Git maintenance.
    git(
        &repo,
        &["update-ref", &format!("refs/skill-sync/{commit}"), &commit],
    )?;
    git(
        root.parent().unwrap(),
        &[
            "clone",
            "--shared",
            "--no-checkout",
            "--",
            repo.to_str()
                .ok_or_else(|| invalid("non-UTF-8 cache path"))?,
            root.to_str()
                .ok_or_else(|| invalid("non-UTF-8 checkout path"))?,
        ],
    )?;
    git(root, &["fetch", "--depth=1", "origin", &commit])?;
    git(root, &["checkout", "--detach", &commit])?;
    std::fs::write(repo.join("last-used"), b"").map_err(SkillSyncError::Io)?;
    Ok(commit)
}

fn size(root: &Path) -> std::io::Result<u64> {
    let meta = std::fs::symlink_metadata(root)?;
    if meta.is_file() {
        return Ok(meta.len());
    }
    if !meta.is_dir() {
        return Ok(0);
    }
    std::fs::read_dir(root)?.try_fold(0, |total, e| Ok(total + size(&e?.path())?))
}

fn trim_cache(cache: &Path, limit: u64) -> Result<()> {
    let mut entries = std::fs::read_dir(cache)
        .map_err(SkillSyncError::Io)?
        .map(|e| {
            let p = e?.path();
            Ok((
                std::fs::metadata(p.join("last-used"))
                    .and_then(|m| m.modified())
                    .ok(),
                size(&p)?,
                p,
            ))
        })
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(SkillSyncError::Io)?;
    entries.sort_by_key(|e| e.0);
    let mut total: u64 = entries.iter().map(|e| e.1).sum();
    for (_, bytes, path) in entries {
        if total <= limit {
            break;
        }
        std::fs::remove_dir_all(path).map_err(SkillSyncError::Io)?;
        total -= bytes;
    }
    Ok(())
}

fn discover(_root: &Path, dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    fn visit(root: &Path, dir: &Path, entries: &mut Vec<crate::github::TreeEntry>) -> Result<()> {
        if !std::fs::symlink_metadata(dir)
            .map_err(SkillSyncError::Io)?
            .is_dir()
        {
            return Err(invalid("source must be a regular directory"));
        }
        for child in std::fs::read_dir(dir).map_err(SkillSyncError::Io)? {
            let child = child.map_err(SkillSyncError::Io)?;
            if child.file_name() == ".git" {
                continue;
            }
            let kind = child.file_type().map_err(SkillSyncError::Io)?;
            if kind.is_dir() {
                visit(root, &child.path(), entries)?;
            } else if !kind.is_file() {
                return Err(invalid(
                    "sources may contain only regular files and directories",
                ));
            } else if child.file_name() == "SKILL.md" {
                entries.push(crate::github::TreeEntry {
                    path: child
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    entry_type: "blob".into(),
                    sha: String::new(),
                });
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    visit(dir, dir, &mut entries)?;
    let tree = crate::github::TreeResponse {
        sha: String::new(),
        tree: entries,
        truncated: false,
    };
    for skill in crate::resolver::detect_skills(&tree) {
        paths.push(
            skill
                .subdirectory
                .map(|relative| dir.join(relative))
                .unwrap_or_else(|| dir.to_owned()),
        );
    }
    Ok(())
}

pub fn prepare(
    manifest: &Manifest,
    path: &Path,
    home: &Path,
    frozen: bool,
    cache_limit: u64,
) -> Result<Prepared> {
    let prior = if frozen {
        Some(SourceLock::read(&lock_path(path))?)
    } else {
        None
    };
    if let Some(lock) = &prior {
        if lock.sources.iter().map(|s| &s.source).collect::<Vec<_>>()
            != manifest.skills.repos.iter().collect::<Vec<_>>()
        {
            return Err(invalid(
                "manifest sources differ from the source lock; run skill-sync update",
            ));
        }
    }
    let temp = tempfile::tempdir().map_err(SkillSyncError::Io)?;
    let cache = home.join(".cache/skill-sync/repos");
    let mut cache_locks = RunLocks::new(home);
    let mut locked_cache = false;
    let mut skills: BTreeMap<String, PreparedSkill> = BTreeMap::new();
    let mut sources = Vec::new();
    let mut checkouts = BTreeMap::new();
    for (index, entry) in manifest.skills.repos.iter().enumerate() {
        let source = parse(entry)?;
        let old = prior.as_ref().map(|p| &p.sources[index]);
        let (root, selected, prefix, commit) = match source {
            Source::Local(local) => {
                if old.is_some_and(|s| s.commit.is_some()) {
                    return Err(invalid("local source lock cannot contain a Git commit"));
                }
                let root = path
                    .parent()
                    .unwrap()
                    .join(local)
                    .canonicalize()
                    .map_err(SkillSyncError::Io)?;
                (root.clone(), root, entry.clone(), None)
            }
            Source::Git {
                url,
                prefix,
                subdir,
                revision,
            } => {
                if !locked_cache {
                    cache_locks.acquire("cache", &cache)?;
                    std::fs::create_dir_all(&cache).map_err(SkillSyncError::Io)?;
                    locked_cache = true;
                }
                let root = temp.path().join(format!("git-{index}"));
                let locked = old
                    .map(|s| {
                        s.commit
                            .as_deref()
                            .ok_or_else(|| invalid("missing Git commit in source lock"))
                    })
                    .transpose()?;
                let checkout_key = (url.clone(), locked.unwrap_or(&revision).to_owned());
                let (root, commit) = if let Some((root, commit)) = checkouts.get(&checkout_key) {
                    (PathBuf::clone(root), String::clone(commit))
                } else {
                    let commit = checkout(&url, &revision, locked, &cache, &root)?;
                    checkouts.insert(checkout_key, (root.clone(), commit.clone()));
                    (root, commit)
                };
                let selected = root.join(&subdir);
                // Resolve existing ancestors to reject symlink escapes before discovery.
                if !selected
                    .canonicalize()
                    .map_err(|e| {
                        invalid(format!(
                            "cannot find selected source {}: {e}",
                            selected.display()
                        ))
                    })?
                    .starts_with(&root.canonicalize().map_err(SkillSyncError::Io)?)
                {
                    return Err(invalid("source path escapes checkout"));
                }
                (root, selected, prefix, Some(commit))
            }
        };
        let mut paths = Vec::new();
        discover(&root, &selected, &mut paths)?;
        if paths.is_empty() {
            return Err(invalid(format!("no SKILL.md found in {entry}")));
        }
        let mut records = Vec::new();
        for (n, dir) in paths.iter().enumerate() {
            let relative = dir
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let separator = if prefix.starts_with("git+") {
                "//"
            } else {
                "/"
            };
            let key = if relative.is_empty() {
                prefix.clone()
            } else {
                format!("{prefix}{separator}{relative}")
            };
            let hash = engine::content_hash(dir)?;
            let staged = temp.path().join(format!("skill-{index}-{n}"));
            engine::copy_directory(dir, &staged)?;
            let skill = PreparedSkill::from_directory(
                key.clone(),
                commit.clone().unwrap_or_else(|| hash.clone()),
                staged,
            )?;
            if skill.hash != hash {
                return Err(invalid("source changed while preparing; retry"));
            }
            records.push(LockedSkill {
                key: key.clone(),
                path: relative,
                name: skill.directory.clone(),
                hash,
            });
            if let Some(previous) = skills.get(&key) {
                if previous.sha != skill.sha || previous.hash != skill.hash {
                    return Err(invalid(
                        "source selects conflicting revisions of the same skill",
                    ));
                }
            } else {
                skills.insert(key, skill);
            }
        }
        let resolved = LockedSource {
            source: entry.clone(),
            commit,
            skills: records,
        };
        if old.is_some_and(|old| old != &resolved) {
            return Err(invalid(format!(
                "frozen source content differs for {entry}; review and run skill-sync update"
            )));
        }
        sources.push(resolved);
    }
    // Shared clones are copied above; eviction can now safely remove bare objects.
    if locked_cache {
        trim_cache(&cache, cache_limit)?;
    }
    Ok(Prepared {
        skills: skills.into_values().collect(),
        lock: SourceLock {
            schema_version: 1,
            sources,
        },
        _checkout: temp,
    })
}
