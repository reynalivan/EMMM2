use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct TempDirGuard {
    path: PathBuf,
    committed: bool,
}

impl TempDirGuard {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    pub(super) fn commit(&mut self) {
        self.committed = true;
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        fs::remove_dir_all(&self.path).ok();
        cleanup_temp_extract_parent(&self.path);
    }
}

pub(super) fn cleanup_temp_extract_parent(temp_dir: &Path) {
    let Some(parent) = temp_dir.parent() else {
        return;
    };

    if !parent
        .file_name()
        .map(|name| name == ".temp_extract")
        .unwrap_or(false)
    {
        return;
    }

    if let Ok(mut entries) = fs::read_dir(parent) {
        if entries.next().is_none() {
            fs::remove_dir(parent).ok();
        }
    }
}
