//! Bounded, terminal-safe whole-file patches; binary files are summarized.
use crate::{
    engine::{self, PreparedSkill},
    error::{Result, SkillSyncError},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct FileDiff {
    pub file: String,
    pub binary: bool,
    pub mode_changed: bool,
    pub patch: String,
}
#[derive(Serialize)]
pub struct SkillDiff {
    pub skill: String,
    pub target: PathBuf,
    pub comparison: &'static str,
    pub files: Vec<FileDiff>,
}

pub fn baseline_root(lock: &Path) -> PathBuf {
    lock.with_extension("baselines")
}
pub fn retain_baseline(lock: &Path, skill: &PreparedSkill) -> Result<()> {
    let root = baseline_root(lock);
    std::fs::create_dir_all(&root).map_err(SkillSyncError::Io)?;
    let destination = root.join(&skill.hash);
    if destination.exists() {
        if engine::content_hash(&destination)? != skill.hash {
            return Err(SkillSyncError::Config(
                "corrupt comparison baseline; remove the baseline directory and retry".into(),
            )
            .into());
        }
        return Ok(());
    }
    let staging = tempfile::tempdir_in(&root).map_err(SkillSyncError::Io)?;
    engine::copy_directory(&skill.path, &staging.path().join("copy"))?;
    if engine::content_hash(&staging.path().join("copy"))? != skill.hash {
        return Err(SkillSyncError::Config("source changed while saving baseline".into()).into());
    }
    std::fs::rename(staging.path().join("copy"), destination).map_err(SkillSyncError::Io)?;
    Ok(())
}

fn files(root: Option<&Path>) -> Result<BTreeMap<String, PathBuf>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<String, PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(SkillSyncError::Io)? {
            let entry = entry.map_err(SkillSyncError::Io)?;
            if entry.file_name() == ".git" {
                continue;
            }
            let kind = entry.file_type().map_err(SkillSyncError::Io)?;
            if kind.is_dir() {
                visit(root, &entry.path(), result)?;
            } else if kind.is_file() {
                result.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    entry.path(),
                );
            } else {
                return Err(
                    SkillSyncError::Config("cannot diff symlinks or special files".into()).into(),
                );
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    if let Some(root) = root.filter(|r| r.exists()) {
        engine::content_hash(root)?;
        visit(root, root, &mut result)?;
    }
    Ok(result)
}
fn mode(path: Option<&PathBuf>) -> Result<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.map(|p| std::fs::metadata(p).map(|m| m.permissions().mode() & 0o111))
            .transpose()
            .map(|v| v.unwrap_or(0))
            .map_err(|e| SkillSyncError::Io(e).into())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(0)
    }
}
fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn fingerprint(path: Option<&PathBuf>) -> Result<(u64, String)> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let Some(path) = path else {
        return Ok((0, String::new()));
    };
    let mut file = std::fs::File::open(path).map_err(SkillSyncError::Io)?;
    let mut hash = Sha256::new();
    let mut total = 0;
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(SkillSyncError::Io)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        total += count as u64;
    }
    Ok((total, format!("{:x}", hash.finalize())))
}

pub fn compare(before: Option<&Path>, after: Option<&Path>) -> Result<Vec<FileDiff>> {
    let old = files(before)?;
    let new = files(after)?;
    let names: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut output = Vec::new();
    let mut budget: usize = 128 * 1024;
    for name in names {
        let left = old.get(name);
        let right = new.get(name);
        let read = |p: Option<&PathBuf>| -> Result<Vec<u8>> {
            p.map(|path| {
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(32 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                Ok::<_, std::io::Error>(bytes)
            })
            .transpose()
            .map(|v| v.unwrap_or_default())
            .map_err(|e| SkillSyncError::Io(e).into())
        };
        let a = read(left)?;
        let b = read(right)?;
        let mode_changed = mode(left)? != mode(right)?;
        let old_fingerprint = fingerprint(left)?;
        let new_fingerprint = fingerprint(right)?;
        if old_fingerprint == new_fingerprint && left.is_some() == right.is_some() && !mode_changed
        {
            continue;
        }
        let binary = a.contains(&0)
            || b.contains(&0)
            || std::str::from_utf8(&a).is_err()
            || std::str::from_utf8(&b).is_err();
        let patch = if binary || a.len() + b.len() > 32 * 1024 || budget == 0 {
            format!(
                "{} -> {} bytes (content omitted)",
                old_fingerprint.0, new_fingerprint.0
            )
        } else {
            let mut patch = format!(
                "--- {}\n+++ {}\n",
                if left.is_some() {
                    safe(name)
                } else {
                    "/dev/null".into()
                },
                if right.is_some() {
                    safe(name)
                } else {
                    "/dev/null".into()
                }
            );
            let a = std::str::from_utf8(&a).unwrap();
            let b = std::str::from_utf8(&b).unwrap();
            patch.push_str(&format!(
                "@@ -1,{} +1,{} @@\n",
                a.lines().count(),
                b.lines().count()
            ));
            for (prefix, text) in [("-", a), ("+", b)] {
                for line in text.lines() {
                    patch.push_str(prefix);
                    patch.push_str(&safe(line));
                    patch.push('\n');
                }
            }
            budget = budget.saturating_sub(patch.len());
            patch
        };
        output.push(FileDiff {
            file: safe(name),
            binary,
            mode_changed,
            patch,
        });
    }
    Ok(output)
}
