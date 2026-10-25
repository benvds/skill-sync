//! OS advisory locks survive neither normal exit nor a crash. The files stay in
//! place: unlinking a lock file can let another process lock a different inode.
use crate::{agents, error::SkillSyncError};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
};

pub struct RunLocks {
    directory: PathBuf,
    held: BTreeMap<String, File>,
}

impl RunLocks {
    /// Use one registry per home, independent of the chosen manifest or XDG config.
    pub fn new(home: &Path) -> Self {
        Self {
            directory: home.join(".cache/skill-sync/run-locks"),
            held: BTreeMap::new(),
        }
    }

    pub fn acquire(&mut self, kind: &str, path: &Path) -> crate::error::Result<()> {
        let path =
            agents::absolute_path(path, &std::env::current_dir().map_err(SkillSyncError::Io)?)?;
        let mut hash = Sha256::new();
        hash.update(kind.as_bytes());
        hash.update([0]);
        hash.update(path.as_os_str().as_encoded_bytes());
        let key = format!("{:x}", hash.finalize());
        if self.held.contains_key(&key) {
            return Ok(());
        }
        std::fs::create_dir_all(&self.directory).map_err(SkillSyncError::Io)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join(format!("{key}.lock")))
            .map_err(SkillSyncError::Io)?;
        match file.try_lock() {
            Ok(()) => {
                self.held.insert(key, file);
                Ok(())
            }
            Err(TryLockError::WouldBlock) => Err(SkillSyncError::Config(format!(
                "another skill-sync is using {kind} {}; retry when it finishes",
                path.display()
            ))
            .into()),
            Err(TryLockError::Error(error)) => Err(SkillSyncError::Io(error).into()),
        }
    }
}
