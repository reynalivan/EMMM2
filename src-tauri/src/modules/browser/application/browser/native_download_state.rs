use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::download_handler::{
    MAX_CONCURRENT_DOWNLOADS, MAX_PENDING_CONFIRMATIONS, MAX_QUEUED_DOWNLOADS,
    PENDING_CONFIRMATION_TTL,
};
use crate::shared::errors::BrowserError;

pub(super) struct NativeDownloadFile {
    staging: tempfile::TempPath,
    destination: PathBuf,
}

pub(super) struct NativeDownloadPublication {
    file: std::fs::File,
    parent: crate::platform::fs::rename::PinnedDirectory,
}

impl NativeDownloadFile {
    pub(super) fn new(destination: &Path) -> Result<Self, BrowserError> {
        let parent = destination.parent().ok_or_else(|| {
            BrowserError::Download("Download destination has no parent directory".into())
        })?;
        std::fs::create_dir_all(parent)?;
        let staging = tempfile::Builder::new()
            .prefix(super::download_service::NATIVE_DOWNLOAD_TEMP_PREFIX)
            .suffix(".part")
            .tempfile_in(parent)?
            .into_temp_path();
        Ok(Self {
            staging,
            destination: destination.to_path_buf(),
        })
    }

    pub(super) fn staging_path(&self) -> &Path {
        &self.staging
    }

    pub(super) fn destination(&self) -> &Path {
        &self.destination
    }

    pub(super) fn publish(&self) -> Result<NativeDownloadPublication, BrowserError> {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        };
        let parent_path = self.destination.parent().ok_or_else(|| {
            BrowserError::Download("Download destination has no parent directory".into())
        })?;
        let parent = crate::platform::fs::rename::PinnedDirectory::open(parent_path)?;
        let file = std::fs::OpenOptions::new()
            .access_mode(DELETE | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&self.staging)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(BrowserError::Download(
                "Native payload is not a regular file".into(),
            ));
        }
        let name = self
            .destination
            .file_name()
            .ok_or_else(|| BrowserError::Download("Download destination has no filename".into()))?;
        crate::platform::fs::rename::rename_handle_into(&file, &parent, name)?;
        Ok(NativeDownloadPublication { file, parent })
    }

    pub(super) fn restore_publication(
        &mut self,
        publication: &NativeDownloadPublication,
    ) -> Result<(), BrowserError> {
        self.staging.disable_cleanup(true);
        let name = self
            .staging
            .file_name()
            .ok_or_else(|| BrowserError::Download("Native staging path has no filename".into()))?;
        crate::platform::fs::rename::rename_handle_into(&publication.file, &publication.parent, name)
            .map_err(|error| BrowserError::Download(format!(
                "Could not restore native publication; preserved payload at {}, staging at {}: {error}",
                self.destination.display(), self.staging.display()
            )))
    }
}

#[derive(Default)]
pub(super) struct NativeDownloadAdmission {
    reservations: HashMap<String, Reservation>,
}

struct Reservation {
    destination: PathBuf,
    requested_at: Option<Instant>,
}

impl NativeDownloadAdmission {
    pub(super) fn is_available(&self, path: &Path) -> bool {
        !self
            .reservations
            .values()
            .any(|entry| entry.destination == path)
    }

    pub(super) fn reserve(
        &mut self,
        id: &str,
        destination: PathBuf,
        now: Instant,
    ) -> Result<(), BrowserError> {
        let pending = self
            .reservations
            .values()
            .filter(|entry| entry.requested_at.is_some())
            .count();
        if pending >= MAX_PENDING_CONFIRMATIONS
            || self.reservations.len() >= MAX_CONCURRENT_DOWNLOADS + MAX_QUEUED_DOWNLOADS
        {
            return Err(BrowserError::QueueFull);
        }
        if self.reservations.contains_key(id) || !self.is_available(&destination) {
            return Err(BrowserError::DownloadAlreadyActive);
        }
        self.reservations.insert(
            id.to_string(),
            Reservation {
                destination,
                requested_at: Some(now),
            },
        );
        Ok(())
    }

    pub(super) fn confirm(&mut self, id: &str, now: Instant) -> Result<(), BrowserError> {
        let entry = self
            .reservations
            .get_mut(id)
            .ok_or(BrowserError::DownloadConfirmationUnavailable)?;
        let requested_at = entry
            .requested_at
            .ok_or(BrowserError::DownloadAlreadyActive)?;
        if now.duration_since(requested_at) > PENDING_CONFIRMATION_TTL {
            return Err(BrowserError::DownloadConfirmationUnavailable);
        }
        entry.requested_at = None;
        Ok(())
    }

    pub(super) fn release(&mut self, id: &str) {
        self.reservations.remove(id);
    }

    pub(super) fn expired_ids(&self, now: Instant) -> Vec<String> {
        self.reservations
            .iter()
            .filter_map(|(id, entry)| {
                entry
                    .requested_at
                    .filter(|at| now.duration_since(*at) > PENDING_CONFIRMATION_TTL)
                    .map(|_| id.clone())
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/native_download_state_tests.rs"]
mod tests;
