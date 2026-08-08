//! Phase 1: heuristic linking of disk entries to existing DB rows.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::services::scanner::sync::types::ConfirmedScanItem;

use super::request::DbModRow;

fn clean_folder(name: &str) -> String {
    crate::common::normalizer::normalize_display_name(name).into_owned()
}

fn get_parent_and_name(path_str: &str) -> (String, String) {
    let p = Path::new(path_str);
    let parent = p
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let name = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    (parent, name)
}
/// Maps disk entry index -> DB row index using exact, toggle and 1:1 rename passes.
///
/// `disk_entries` carry absolute paths — they address real folders — while
/// `mods.folder_path` is stored relative to the mods root. Every pass below
/// compares the two, so the disk side is converted once up front rather than
/// each side being compared in whichever form it happens to arrive in.
pub(super) fn link_disk_to_db(
    disk_entries: &[(ConfirmedScanItem, String)],
    db_mods: &[DbModRow],
    mods_root: &Path,
) -> HashMap<usize, usize> {
    let mut disk_to_db: HashMap<usize, usize> = HashMap::new();
    let mut db_matched: HashSet<usize> = HashSet::new();

    let disk_relative: Vec<String> = disk_entries
        .iter()
        .map(|(_, path)| crate::common::path_key::relative_to_root(path, mods_root))
        .collect();

    // Phase 1: Heuristic Linking
    // Pass A: Exact Match (folder_path == folder_path)
    for (disk_idx, disk_path) in disk_relative.iter().enumerate() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        for (db_idx, db_mod) in db_mods.iter().enumerate() {
            if db_matched.contains(&db_idx) {
                continue;
            }
            if db_mod.1 == *disk_path {
                disk_to_db.insert(disk_idx, db_idx);
                db_matched.insert(db_idx);
                break;
            }
        }
    }

    // Pass B: Toggle Match (ignore "DISABLED " prefix)
    for (disk_idx, disk_path) in disk_relative.iter().enumerate() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        let (disk_parent, disk_name) = get_parent_and_name(disk_path);
        let disk_clean = clean_folder(&disk_name);

        for (db_idx, db_mod) in db_mods.iter().enumerate() {
            if db_matched.contains(&db_idx) {
                continue;
            }
            let (db_parent, db_name) = get_parent_and_name(&db_mod.1);
            let db_clean = clean_folder(&db_name);

            if disk_parent == db_parent && disk_clean == db_clean {
                disk_to_db.insert(disk_idx, db_idx);
                db_matched.insert(db_idx);
                break;
            }
        }
    }

    // Pass C: 1:1 Rename Match (isolated unmatched item in same parent directory)
    let mut unmatched_disk_by_parent: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (disk_idx, disk_path) in disk_relative.iter().enumerate() {
        if disk_to_db.contains_key(&disk_idx) {
            continue;
        }
        let (parent, _) = get_parent_and_name(disk_path);
        unmatched_disk_by_parent
            .entry(parent)
            .or_default()
            .push(disk_idx);
    }

    let mut unmatched_db_by_parent: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (db_idx, db_mod) in db_mods.iter().enumerate() {
        if db_matched.contains(&db_idx) {
            continue;
        }
        let (parent, _) = get_parent_and_name(&db_mod.1);
        // Ensure this DB mod isn't physically on disk anymore before considering
        // it renamed. Joining is what makes the check mean anything: a stored
        // path is relative, so testing it directly resolves against the process
        // working directory and reports every row as missing.
        if !mods_root.join(&db_mod.1).exists() {
            unmatched_db_by_parent
                .entry(parent)
                .or_default()
                .push(db_idx);
        }
    }

    for (parent, disk_indices) in unmatched_disk_by_parent.iter() {
        if disk_indices.len() == 1 {
            if let Some(db_indices) = unmatched_db_by_parent.get(parent) {
                if db_indices.len() == 1 {
                    let disk_idx = disk_indices[0];
                    let db_idx = db_indices[0];
                    disk_to_db.insert(disk_idx, db_idx);
                    db_matched.insert(db_idx);
                }
            }
        }
    }
    disk_to_db
}
