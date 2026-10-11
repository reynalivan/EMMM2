use std::path::{Path, PathBuf};

use super::asset_pack::CatalogPack;
use crate::shared::errors::ScannerError;

/// Keep the previous pack until database publication succeeds. Dropping an
/// unfinished swap (including a cancelled future) restores the reviewed staging.
pub(super) struct PackSwap {
    active: PathBuf,
    staging: PathBuf,
    backup: Option<PathBuf>,
    finished: bool,
}

impl PackSwap {
    pub(super) fn begin(app_data_dir: &Path, staging: &Path) -> Result<Self, ScannerError> {
        let active = CatalogPack::root(app_data_dir);
        let backup = active
            .exists()
            .then(|| app_data_dir.join(format!(".catalog-pack-backup-{}", uuid::Uuid::new_v4())));
        if let Some(backup) = &backup {
            std::fs::rename(&active, backup)?;
        }
        if let Err(error) = std::fs::rename(staging, &active) {
            if let Some(backup) = &backup {
                std::fs::rename(backup, &active).map_err(|restore| ScannerError::Io(format!(
                    "Catalog activation failed ({error}); restore failed ({restore}). Previous pack retained at {}",
                    backup.display()
                )))?;
            }
            return Err(error.into());
        }
        Ok(Self {
            active,
            staging: staging.to_path_buf(),
            backup,
            finished: false,
        })
    }

    pub(super) fn rollback(&mut self) -> Result<(), ScannerError> {
        if self.finished {
            return Ok(());
        }
        if self.active.exists() && !self.staging.exists() {
            std::fs::rename(&self.active, &self.staging)?;
        }
        if let Some(backup) = &self.backup {
            std::fs::rename(backup, &self.active).map_err(|error| {
                ScannerError::Io(format!(
                    "Could not restore catalog: {error}. Previous pack retained at {}",
                    backup.display()
                ))
            })?;
        }
        self.finished = true;
        Ok(())
    }

    pub(super) fn commit(mut self) {
        self.finished = true;
        if let Some(backup) = &self.backup {
            if let Err(error) = std::fs::remove_dir_all(backup) {
                // Publication succeeded; backup cleanup must not turn it into a failed install.
                log::warn!(
                    "Installed catalog; backup cleanup failed at {}: {error}",
                    backup.display()
                );
            }
        }
    }
}

impl Drop for PackSwap {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            log::error!("Catalog swap rollback failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::write_catalog;
    use super::*;

    #[test]
    fn dropped_swap_restores_previous_pack_and_reviewed_staging() {
        let root = tempfile::tempdir().unwrap();
        let active = CatalogPack::root(root.path());
        let staging = root.path().join("staging");
        write_catalog(&active, "old");
        write_catalog(&staging, "new");
        let swap = PackSwap::begin(root.path(), &staging).unwrap();
        assert_eq!(
            CatalogPack::load(root.path())
                .unwrap()
                .review()
                .unwrap()
                .pack_id,
            "new"
        );
        drop(swap);
        assert_eq!(
            CatalogPack::load(root.path())
                .unwrap()
                .review()
                .unwrap()
                .pack_id,
            "old"
        );
        assert_eq!(
            CatalogPack::load_from_root(staging)
                .unwrap()
                .review()
                .unwrap()
                .pack_id,
            "new"
        );
    }

    #[test]
    fn activation_failure_restores_previous_pack() {
        let root = tempfile::tempdir().unwrap();
        write_catalog(&CatalogPack::root(root.path()), "old");
        assert!(PackSwap::begin(root.path(), &root.path().join("missing")).is_err());
        assert_eq!(
            CatalogPack::load(root.path())
                .unwrap()
                .review()
                .unwrap()
                .pack_id,
            "old"
        );
    }

    #[test]
    fn rollback_of_first_install_leaves_no_active_pack() {
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("staging");
        write_catalog(&staging, "new");
        drop(PackSwap::begin(root.path(), &staging).unwrap());
        assert!(!CatalogPack::root(root.path()).exists());
        assert!(staging.exists());
    }
}
