//! BLAKE3 hashes of the files that identify a mod.

use crate::domain::errors::ScannerError;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::snapshot::ModSnapshot;

/// Textures carry the colour, meshes carry the shape. The two are hashed into
/// separate buckets because a pair that shares its meshes but not its textures
/// is a recolor, and that scores differently from an unrelated pair.
const TEXTURE_EXT: &str = "dds";
const MESH_EXTS: &[&str] = &["ib", "buf"];
const PARTIAL_HASH_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct HashProfile {
    pub file_hashes: BTreeMap<String, String>,
    pub texture_samples: BTreeMap<String, String>,
    pub mesh_hashes: BTreeMap<String, String>,
}

pub(crate) fn hash_snapshot(snapshot: &ModSnapshot) -> HashProfile {
    hash_snapshot_with_identity_mode(snapshot, false)
}

pub(crate) fn hash_snapshot_full(snapshot: &ModSnapshot) -> HashProfile {
    hash_snapshot_with_identity_mode(snapshot, true)
}

fn hash_snapshot_with_identity_mode(
    snapshot: &ModSnapshot,
    require_full_identity: bool,
) -> HashProfile {
    let mut profile = HashProfile::default();
    for file in &snapshot.files {
        let is_texture = file.extension == TEXTURE_EXT;
        let use_sample =
            is_texture && file.size_bytes > PARTIAL_HASH_THRESHOLD_BYTES && !require_full_identity;
        let identity_hash = if use_sample {
            partial_blake3_hash(&file.abs_path)
        } else {
            full_blake3_hash(&file.abs_path)
        };
        let Ok(identity_hash) = identity_hash else {
            continue;
        };

        profile
            .file_hashes
            .insert(file.rel_path.clone(), identity_hash.clone());

        if is_texture {
            let sample_hash = if file.size_bytes > PARTIAL_HASH_THRESHOLD_BYTES {
                partial_blake3_hash(&file.abs_path).unwrap_or_else(|_| identity_hash.clone())
            } else {
                identity_hash.clone()
            };
            profile
                .texture_samples
                .insert(file.rel_path.clone(), sample_hash);
        } else if MESH_EXTS.contains(&file.extension.as_str()) {
            profile
                .mesh_hashes
                .insert(file.rel_path.clone(), identity_hash);
        }
    }
    profile
}

pub(super) fn full_blake3_hash(path: &Path) -> Result<String, ScannerError> {
    let file = File::open(path)?;
    // blake3's own reader does the buffering; an 8 KiB hand-rolled loop is
    // below the 16 KiB the multi-threaded fast path needs.
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file)?;
    Ok(hasher.finalize().to_string())
}

fn partial_blake3_hash(path: &Path) -> Result<String, ScannerError> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    let mut head = [0_u8; 1024];
    let head_len = file.read(&mut head)?;
    hasher.update(&head[..head_len]);

    if size > 1024 {
        file.seek(SeekFrom::End(-1024))?;
        let mut tail = [0_u8; 1024];
        let tail_len = file.read(&mut tail)?;
        hasher.update(&tail[..tail_len]);
    }

    Ok(hasher.finalize().to_string())
}
