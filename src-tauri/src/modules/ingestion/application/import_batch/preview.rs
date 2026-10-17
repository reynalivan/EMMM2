use super::types::{
    ImportItem, ImportItemStatus, ImportSourcePreview, ImportSourcePreviewEntry,
    ImportSourcePreviewEntryKind,
};
use crate::modules::ingestion::adapters::sqlite::import_batch;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MAX_VISIBLE_ENTRIES: usize = 24;
const MAX_SCANNED_ENTRIES: usize = 2_048;
const MAX_PREVIEW_IMAGES: usize = 4;
const MAX_PREVIEW_DEPTH: usize = 4;

pub async fn get_import_source_preview(
    db: &SqlitePool,
    item_id: &str,
) -> Result<ImportSourcePreview, AppError> {
    let item = import_batch::get_item(db, item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let item_id = item.id.clone();
    let candidates = preview_path_candidates(&item);

    tokio::task::spawn_blocking(move || {
        let source = first_existing_path(candidates)?;
        build_preview(item_id, source)
    })
    .await
    .map_err(|error| AppError::Internal(format!("Source preview task failed: {error}")))?
}

pub async fn reveal_import_source(db: &SqlitePool, item_id: &str) -> Result<(), AppError> {
    let item = import_batch::get_item(db, item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let source = first_existing_path(preview_path_candidates(&item))?;
    let canonical = canonical_existing_path(&source, "Import source")?;
    crate::platform::process::reveal_in_file_manager(&canonical)
}

pub async fn reveal_import_destination(db: &SqlitePool, item_id: &str) -> Result<(), AppError> {
    let item = import_batch::get_item(db, item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))?;
    let destination = item.destination_path.as_deref().ok_or_else(|| {
        AppError::Validation("Import item has no selected destination".to_string())
    })?;
    let batch = import_batch::get_batch(db, &item.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(db, &batch.game_id)
        .await?
        .ok_or_else(|| AppError::Validation("Game has no configured mods path".to_string()))?;
    let canonical_root = canonical_existing_path(Path::new(&mods_root), "Mods root")?;
    let destination_path = Path::new(destination);
    let reveal_target = if destination_path.exists() {
        destination_path.to_path_buf()
    } else {
        destination_path
            .parent()
            .ok_or_else(|| AppError::Validation("Destination has no parent folder".to_string()))?
            .to_path_buf()
    };
    let canonical_target = canonical_existing_path(&reveal_target, "Import destination")?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(AppError::Security(
            "Import destination escaped the configured mods root".to_string(),
        ));
    }
    crate::platform::process::reveal_in_file_manager(&canonical_target)
}

fn build_preview(item_id: String, source: PathBuf) -> Result<ImportSourcePreview, AppError> {
    let canonical = canonical_existing_path(&source, "Import source")?;
    if canonical.is_file() {
        return Ok(ImportSourcePreview {
            item_id,
            thumbnail_path: None,
            image_thumbnails: Vec::new(),
            entries: vec![ImportSourcePreviewEntry {
                relative_path: canonical
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: ImportSourcePreviewEntryKind::File,
                depth: 0,
            }],
            folder_count: 0,
            file_count: 1,
            total_size_bytes: canonical.metadata()?.len(),
            truncated: false,
        });
    }

    let thumbnail_path =
        crate::modules::workspace::application::scanner::core::thumbnail::find_thumbnail(
            &canonical,
        )
        .map(|path| path.to_string_lossy().into_owned());
    let image_thumbnails =
        crate::modules::workspace::application::scanner::core::thumbnail::list_preview_images(
            &canonical,
        )
        .into_iter()
        .take(MAX_PREVIEW_IMAGES)
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let mut entries = Vec::new();
    let mut folder_count = 0_u32;
    let mut file_count = 0_u32;
    let mut total_size_bytes = 0_u64;
    let mut truncated = false;

    for (index, entry) in WalkDir::new(&canonical)
        .max_depth(MAX_PREVIEW_DEPTH)
        .follow_links(false)
        .into_iter()
        .enumerate()
    {
        if index > MAX_SCANNED_ENTRIES {
            truncated = true;
            break;
        }
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "Could not read import preview '{}': {error}",
                canonical.display()
            ))
        })?;
        if entry.depth() == 0 {
            continue;
        }
        if entry.file_type().is_dir() {
            folder_count = folder_count.saturating_add(1);
            if entry.depth() == MAX_PREVIEW_DEPTH {
                let mut children = std::fs::read_dir(entry.path())?;
                if let Some(child) = children.next() {
                    child?;
                    truncated = true;
                }
            }
        } else if entry.file_type().is_file() {
            file_count = file_count.saturating_add(1);
            let metadata = entry.metadata().map_err(|error| {
                AppError::Io(format!(
                    "Could not read import preview metadata '{}': {error}",
                    entry.path().display()
                ))
            })?;
            total_size_bytes = total_size_bytes.saturating_add(metadata.len());
        }
        if entries.len() < MAX_VISIBLE_ENTRIES {
            entries.push(ImportSourcePreviewEntry {
                relative_path: entry
                    .path()
                    .strip_prefix(&canonical)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .into_owned(),
                kind: if entry.file_type().is_dir() {
                    ImportSourcePreviewEntryKind::Folder
                } else {
                    ImportSourcePreviewEntryKind::File
                },
                depth: entry.depth().saturating_sub(1).min(u8::MAX as usize) as u8,
            });
        } else {
            truncated = true;
        }
    }

    Ok(ImportSourcePreview {
        item_id,
        thumbnail_path,
        image_thumbnails,
        entries,
        folder_count,
        file_count,
        total_size_bytes,
        truncated,
    })
}

fn preview_path_candidates(item: &ImportItem) -> Vec<PathBuf> {
    let mut candidates = Vec::with_capacity(3);
    let destination_first = matches!(
        item.status,
        ImportItemStatus::Committing
            | ImportItemStatus::Reconciling
            | ImportItemStatus::FinalizingMetadata
            | ImportItemStatus::Partial
            | ImportItemStatus::MetadataPending
            | ImportItemStatus::Done
    );

    if destination_first {
        push_path(&mut candidates, item.destination_path.as_deref());
    }
    push_path(&mut candidates, item.staging_path.as_deref());
    push_path(&mut candidates, Some(&item.source_path));
    if !destination_first {
        push_path(&mut candidates, item.destination_path.as_deref());
    }
    candidates
}

fn push_path(candidates: &mut Vec<PathBuf>, path: Option<&str>) {
    let Some(path) = path.filter(|path| !path.is_empty()) else {
        return;
    };
    let path = PathBuf::from(path);
    if !candidates.iter().any(|candidate| candidate == &path) {
        candidates.push(path);
    }
}

fn first_existing_path(candidates: Vec<PathBuf>) -> Result<PathBuf, AppError> {
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| AppError::Validation("Import source is unavailable".to_string()))
}

fn canonical_existing_path(path: &Path, label: &str) -> Result<PathBuf, AppError> {
    path.canonicalize()
        .map_err(|error| AppError::Validation(format!("{label} is unavailable: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_uses_placed_folder_when_staging_was_moved() {
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("staging").join("Robin");
        let source = root.path().join("Robin.zip");
        let placed = root.path().join("Mods").join("Robin");
        std::fs::write(&source, b"archive").unwrap();
        std::fs::create_dir_all(&placed).unwrap();

        let selected = first_existing_path(vec![staging, placed.clone(), source]).unwrap();

        assert_eq!(selected, placed);
    }

    #[test]
    fn preview_marks_descendants_beyond_its_depth_bound_as_partial() {
        let root = tempfile::tempdir().unwrap();
        let hidden = root.path().join("A/B/C/D/E");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("payload.dat"), b"payload").unwrap();

        let preview = build_preview("item".to_string(), root.path().to_path_buf()).unwrap();

        assert!(preview.truncated);
        assert_eq!(preview.file_count, 0);
        assert_eq!(preview.total_size_bytes, 0);
    }

    #[test]
    fn preview_with_an_empty_directory_at_the_depth_bound_is_complete() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("A/B/C/D")).unwrap();

        let preview = build_preview("item".to_string(), root.path().to_path_buf()).unwrap();

        assert!(!preview.truncated);
        assert_eq!(preview.folder_count, 4);
        assert_eq!(preview.file_count, 0);
    }

    #[test]
    fn shallow_preview_retains_complete_file_counts_and_size() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("payload.dat"), b"payload").unwrap();

        let preview = build_preview("item".to_string(), root.path().to_path_buf()).unwrap();

        assert!(!preview.truncated);
        assert_eq!(preview.file_count, 1);
        assert_eq!(preview.total_size_bytes, 7);
    }

    #[test]
    fn preview_preserves_its_scan_and_visible_entry_limits() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..MAX_SCANNED_ENTRIES + 2 {
            std::fs::write(root.path().join(format!("payload-{index}.dat")), b"x").unwrap();
        }

        let preview = build_preview("item".to_string(), root.path().to_path_buf()).unwrap();

        assert!(preview.truncated);
        assert_eq!(preview.entries.len(), MAX_VISIBLE_ENTRIES);
        assert_eq!(preview.file_count as usize, MAX_SCANNED_ENTRIES);
        assert_eq!(preview.total_size_bytes as usize, MAX_SCANNED_ENTRIES);
    }
}
