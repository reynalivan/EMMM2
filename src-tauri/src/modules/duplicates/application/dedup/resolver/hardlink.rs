use crate::shared::errors::ScannerError;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug)]
pub(super) struct PreparedHardlinkReplacement {
    pub keeper: std::path::PathBuf,
    pub target: std::path::PathBuf,
    pub backup: std::path::PathBuf,
    pub keeper_identity: String,
    pub sequence: u32,
}

impl PreparedHardlinkReplacement {
    pub fn journal_step(&self) -> crate::modules::mutation::journal::PlannedStep {
        crate::modules::mutation::journal::PlannedStep::hardlink_replace(
            self.sequence,
            self.keeper.clone(),
            self.target.clone(),
            self.backup.clone(),
            self.keeper_identity.clone(),
        )
    }

    pub fn execute(&self) -> Result<(), ScannerError> {
        replace_file_with_hardlink_at(&self.keeper, &self.target, &self.backup)
    }

    pub fn finalize(&self) -> Result<(), ScannerError> {
        if !self.backup.exists() {
            return Ok(());
        }
        crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&self.backup)
            .map_err(|error| ScannerError::Io(error.to_string()))
    }
}

pub(super) fn prepare_hardlinks(
    keep_folder: &str,
    target_folder: &str,
    next_sequence: &mut u32,
) -> Result<Vec<PreparedHardlinkReplacement>, ScannerError> {
    let keep_path = Path::new(keep_folder);
    let target_path = Path::new(target_folder);
    if !keep_path.exists() || !target_path.exists() {
        return Err(ScannerError::Validation(
            "One or both folders do not exist for hardlinking".to_string(),
        ));
    }

    let mut replacements = Vec::new();
    let entries = walkdir::WalkDir::new(target_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !entry.file_name().to_string_lossy().starts_with('.')
        });
    for entry in entries {
        let entry = entry.map_err(|error| ScannerError::Io(error.to_string()))?;
        if !entry.file_type().is_file()
            || super::super::snapshot::is_ignored_file_name(entry.file_name())
        {
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(target_path)
            .map_err(|error| ScannerError::Io(error.to_string()))?;
        let keeper = keep_path.join(relative_path);
        if !keeper.is_file() {
            continue;
        }
        if fs::metadata(&keeper)?.len() != fs::metadata(entry.path())?.len()
            || super::super::hashing::full_blake3_hash(&keeper)?
                != super::super::hashing::full_blake3_hash(entry.path())?
        {
            return Err(ScannerError::Validation(format!(
                "File changed after duplicate verification: {}",
                entry.path().display()
            )));
        }
        let keeper_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&keeper)
            .ok_or_else(|| ScannerError::Io(format!("Unable to read file identity: {}", keeper.display())))?;
        let target = entry.path().to_path_buf();
        let backup = backup_path(&target)?;
        replacements.push(PreparedHardlinkReplacement {
            keeper,
            target,
            backup,
            keeper_identity,
            sequence: *next_sequence,
        });
        *next_sequence += 1;
    }
    Ok(replacements)
}

pub(super) fn apply_hardlinks(keep_folder: &str, target_folder: &str) -> Result<(), ScannerError> {
    let keep_path = Path::new(keep_folder);
    let target_path = Path::new(target_folder);
    if !keep_path.exists() || !target_path.exists() {
        return Err(ScannerError::Validation(
            "One or both folders do not exist for hardlinking".to_string(),
        ));
    }

    let mut success_count = 0;
    let entries = walkdir::WalkDir::new(target_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !entry.file_name().to_string_lossy().starts_with('.')
        });
    for entry in entries {
        let entry = entry.map_err(|error| {
            ScannerError::Io(format!(
                "failed to inspect hardlink target '{}': {error}",
                target_path.display()
            ))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        if super::super::snapshot::is_ignored_file_name(entry.file_name()) {
            continue;
        }
        let Ok(relative_path) = entry.path().strip_prefix(target_path) else {
            continue;
        };
        let source_file = keep_path.join(relative_path);
        if !source_file.is_file() {
            continue;
        }

        let source_size = fs::metadata(&source_file)?.len();
        let target_size = fs::metadata(entry.path())?.len();
        if source_size != target_size
            || super::super::hashing::full_blake3_hash(&source_file)?
                != super::super::hashing::full_blake3_hash(entry.path())?
        {
            return Err(ScannerError::Validation(format!(
                "File changed after duplicate verification: {}",
                entry.path().display()
            )));
        }

        replace_file_with_hardlink(&source_file, entry.path())?;
        success_count += 1;
    }

    log::info!("Created {success_count} hardlinks from {keep_folder} to {target_folder}");
    Ok(())
}

fn replace_file_with_hardlink(source: &Path, target: &Path) -> Result<(), ScannerError> {
    replace_file_with_hardlink_using(source, target, |source, target| {
        fs::hard_link(source, target)
    })
}

fn replace_file_with_hardlink_at(
    source: &Path,
    target: &Path,
    backup: &Path,
) -> Result<(), ScannerError> {
    fs::rename(target, backup).map_err(|error| {
        ScannerError::Io(format!(
            "failed to stage '{}' for hardlink replacement: {error}",
            target.display()
        ))
    })?;
    if let Err(link_error) = fs::hard_link(source, target) {
        return match fs::rename(backup, target) {
            Ok(()) => Err(ScannerError::Io(format!(
                "failed to create hardlink '{}' -> '{}': {link_error}",
                source.display(),
                target.display()
            ))),
            Err(rollback_error) => Err(ScannerError::Io(format!(
                "failed to create hardlink '{}' -> '{}': {link_error}; original file rollback failed: {rollback_error}; preserved backup: {}",
                source.display(),
                target.display(),
                backup.display()
            ))),
        };
    }
    Ok(())
}

fn backup_path(target: &Path) -> Result<std::path::PathBuf, ScannerError> {
    let parent = target.parent().ok_or_else(|| {
        ScannerError::Io(format!(
            "hardlink target has no parent: {}",
            target.display()
        ))
    })?;
    let file_name = target.file_name().ok_or_else(|| {
        ScannerError::Io(format!(
            "hardlink target has no file name: {}",
            target.display()
        ))
    })?;
    Ok(parent.join(format!(
        ".{}.emmm-hardlink-{}",
        file_name.to_string_lossy(),
        Uuid::new_v4().as_simple()
    )))
}

pub(crate) fn replace_file_with_hardlink_using<F>(
    source: &Path,
    target: &Path,
    create_hardlink: F,
) -> Result<(), ScannerError>
where
    F: FnOnce(&Path, &Path) -> std::io::Result<()>,
{
    let backup = backup_path(target)?;

    fs::rename(target, &backup).map_err(|error| {
        ScannerError::Io(format!(
            "failed to stage '{}' for hardlink replacement: {error}",
            target.display()
        ))
    })?;

    if let Err(link_error) = create_hardlink(source, target) {
        return match fs::rename(&backup, target) {
            Ok(()) => Err(ScannerError::Io(format!(
                "failed to create hardlink '{}' -> '{}': {link_error}",
                source.display(),
                target.display()
            ))),
            Err(rollback_error) => Err(ScannerError::Io(format!(
                "failed to create hardlink '{}' -> '{}': {link_error}; original file rollback failed: {rollback_error}; preserved backup: {}",
                source.display(),
                target.display(),
                backup.display()
            ))),
        };
    }

    if let Err(recycle_error) = crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&backup)
    {
        let rollback_result = fs::remove_file(target).and_then(|_| fs::rename(&backup, target));
        return match rollback_result {
            Ok(()) => Err(ScannerError::Io(format!(
                "failed to move replaced file to the Recycle Bin: {recycle_error}"
            ))),
            Err(rollback_error) => Err(ScannerError::Io(format!(
                "failed to move replaced file to the Recycle Bin: {recycle_error}; hardlink rollback failed: {rollback_error}; preserved backup: {}",
                backup.display()
            ))),
        };
    }

    Ok(())
}
