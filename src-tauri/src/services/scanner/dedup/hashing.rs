//! BLAKE3 hashes of the files that identify a mod.

use crate::domain::errors::ScannerError;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::snapshot::ModSnapshot;

const KEY_EXTS: &[&str] = &["ini", "dds", "buf", "ib", "vb"];
/// Textures carry the colour, meshes carry the shape. The two are hashed into
/// separate buckets because a pair that shares its meshes but not its textures
/// is a recolor, and that scores differently from an unrelated pair.
const TEXTURE_EXT: &str = "dds";
const MESH_EXTS: &[&str] = &["ib", "buf"];
const PARTIAL_HASH_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct HashProfile {
    pub key_file_hashes: BTreeMap<String, String>,
    pub texture_samples: BTreeMap<String, String>,
    pub mesh_hashes: BTreeMap<String, String>,
}

pub(crate) fn hash_snapshot(snapshot: &ModSnapshot) -> HashProfile {
    let mut profile = HashProfile::default();
    for file in &snapshot.files {
        if !KEY_EXTS.contains(&file.extension.as_str()) {
            continue;
        }
        let is_texture = file.extension == TEXTURE_EXT;
        let hash = if is_texture && file.size_bytes > PARTIAL_HASH_THRESHOLD_BYTES {
            partial_blake3_hash(&file.abs_path)
        } else {
            full_blake3_hash(&file.abs_path)
        };
        if let Ok(value) = hash {
            profile
                .key_file_hashes
                .insert(file.rel_path.clone(), value.clone());
            if is_texture {
                profile.texture_samples.insert(file.rel_path.clone(), value);
            } else if MESH_EXTS.contains(&file.extension.as_str()) {
                profile.mesh_hashes.insert(file.rel_path.clone(), value);
            }
        }
    }
    profile
}

fn full_blake3_hash(path: &Path) -> Result<String, ScannerError> {
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
