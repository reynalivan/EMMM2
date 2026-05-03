use std::path::{Path, PathBuf};

use crate::services::disk_reconcile::helpers::{is_disabled_runtime_name, normalize_runtime_name};
use crate::services::explorer::classifier::{classify_folder, NodeType};

#[derive(Debug, Clone)]
pub struct DiskObjectEntry {
    pub folder_path: String,
    pub folder_path_key: String,
    pub name: String,
    pub is_disabled: bool,
}

#[derive(Debug, Clone)]
pub struct DiskModEntry {
    pub folder_path: String,
    pub folder_path_key: String,
    pub object_folder_path_key: String,
    pub object_disabled: bool,
    pub raw_name: String,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct DiskProjection {
    pub objects: Vec<DiskObjectEntry>,
    pub mods: Vec<DiskModEntry>,
}

fn list_runtime_dirs(path: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = std::fs::read_dir(path)
        .map_err(|error| format!("Failed to read directory '{}': {error}", path.display()))?;
    let mut result = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to read directory entry in '{}': {error}",
                path.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to read file type for '{}' in '{}': {error}",
                entry.file_name().to_string_lossy(),
                path.display()
            )
        })?;
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        result.push((name, entry.path()));
    }

    Ok(result)
}

fn relative_path_string(mods_path: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(mods_path).map_err(|error| {
        format!(
            "Failed to compute relative path for '{}' under '{}': {error}",
            path.display(),
            mods_path.display()
        )
    })?;
    Ok(relative.to_string_lossy().to_string())
}

fn collect_terminal_mods(
    projection: &mut DiskProjection,
    mods_path: &Path,
    object_folder_path_key: &str,
    object_disabled: bool,
    path: &Path,
) -> Result<(), String> {
    let (node_type, _reasons, _warnings) = classify_folder(path);
    match node_type {
        NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
            let folder_path = relative_path_string(mods_path, path)?;
            let raw_name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .ok_or_else(|| {
                    format!("Disk Reconcile mod path has no name: {}", path.display())
                })?;
            projection.mods.push(DiskModEntry {
                folder_path: folder_path.clone(),
                folder_path_key: crate::services::path_key::folder_path_key(&folder_path, None),
                object_folder_path_key: object_folder_path_key.to_string(),
                object_disabled,
                raw_name,
                absolute_path: path.to_path_buf(),
            });
            Ok(())
        }
        NodeType::InternalAssets => Ok(()),
        NodeType::ContainerFolder => {
            for (_child_name, child_path) in list_runtime_dirs(path)? {
                collect_terminal_mods(
                    projection,
                    mods_path,
                    object_folder_path_key,
                    object_disabled,
                    &child_path,
                )?;
            }
            Ok(())
        }
    }
}

pub fn collect_disk_projection(
    mods_path: &Path,
    changed_roots: &[String],
    scoped: bool,
) -> Result<DiskProjection, String> {
    if !mods_path.exists() || !mods_path.is_dir() {
        return Err(format!(
            "Disk Reconcile mods path is unavailable: {}",
            mods_path.display()
        ));
    }

    let mut projection = DiskProjection::default();
    let target_roots = if scoped {
        changed_roots
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|root| (root.clone(), mods_path.join(&root)))
            .collect::<Vec<_>>()
    } else {
        list_runtime_dirs(mods_path)?
    };

    for (root_name, root_path) in target_roots {
        if !root_path.exists() || !root_path.is_dir() {
            continue;
        }

        let object_entry = DiskObjectEntry {
            folder_path: root_name.clone(),
            folder_path_key: crate::services::path_key::folder_path_key(&root_name, None),
            name: normalize_runtime_name(&root_name),
            is_disabled: is_disabled_runtime_name(&root_name),
        };
        let object_folder_path_key = object_entry.folder_path_key.clone();
        let object_disabled = object_entry.is_disabled;
        projection.objects.push(object_entry);

        for (_mod_name, mod_path) in list_runtime_dirs(&root_path)? {
            collect_terminal_mods(
                &mut projection,
                mods_path,
                &object_folder_path_key,
                object_disabled,
                &mod_path,
            )?;
        }
    }

    Ok(projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_snapshot_skips_container_only_folders_and_indexes_terminal_mod_roots() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let object_root = temp.path().join("Alice");
        let container = object_root.join("Nested");
        let terminal = container.join("Blue Dress");
        let empty_container = object_root.join("Empty Container");

        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::create_dir_all(&empty_container).expect("empty container should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash = abc\n",
        )
        .expect("ini should be written");
        std::fs::write(terminal.join("mesh.buf"), "mesh").expect("asset should be written");

        let projection =
            collect_disk_projection(temp.path(), &[], false).expect("snapshot should succeed");

        assert_eq!(projection.objects.len(), 1);
        assert_eq!(projection.objects[0].folder_path, "Alice");
        assert_eq!(projection.mods.len(), 1);
        assert_eq!(
            projection.mods[0].folder_path,
            PathBuf::from("Alice")
                .join("Nested")
                .join("Blue Dress")
                .to_string_lossy()
                .to_string()
        );
    }
}
