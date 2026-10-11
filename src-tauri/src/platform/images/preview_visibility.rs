use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

const VISIBILITY_FILE: &str = ".emmm-preview-visibility.json";
const MAX_VISIBILITY_BYTES: u64 = 1024 * 1024;

#[derive(Default, Deserialize, Serialize)]
struct VisibilityState {
    hidden_paths: BTreeSet<String>,
    #[serde(default)]
    managed_paths: BTreeSet<String>,
}

fn load_state(root: &Path) -> Result<VisibilityState, AppError> {
    let state_path = root.join(VISIBILITY_FILE);
    let metadata = match std::fs::symlink_metadata(&state_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(VisibilityState::default())
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_VISIBILITY_BYTES
    {
        return Err(AppError::Validation(
            "Preview visibility state must be a regular bounded file".to_string(),
        ));
    }
    let state: VisibilityState =
        serde_json::from_slice(&std::fs::read(&state_path)?).map_err(|error| {
            AppError::Validation(format!("Invalid preview visibility state: {error}"))
        })?;
    if state
        .hidden_paths
        .iter()
        .chain(&state.managed_paths)
        .any(|value| {
            let path = Path::new(value);
            value.is_empty()
                || path.is_absolute()
                || !path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        super::IMAGE_EXTENSIONS
                            .iter()
                            .any(|allowed| extension.eq_ignore_ascii_case(allowed))
                    })
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
        })
    {
        return Err(AppError::Validation(
            "Preview visibility contains an invalid relative image path".to_string(),
        ));
    }
    Ok(state)
}

pub(crate) struct PreviewVisibilityEntry {
    pub path: PathBuf,
    pub is_hidden: bool,
}

pub(crate) fn apply_visibility(
    root: &Path,
    mut discovered: Vec<PathBuf>,
) -> Result<Vec<PreviewVisibilityEntry>, AppError> {
    let state = load_state(root)?;
    let canonical_root = root.canonicalize()?;
    for key in state.managed_paths.union(&state.hidden_paths) {
        let candidate = canonical_root.join(key);
        if candidate.is_file() {
            discovered.push(candidate);
        }
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for path in discovered {
        let key = image_key(&canonical_root, &path)?;
        if seen.insert(key.clone()) {
            entries.push(PreviewVisibilityEntry {
                path,
                is_hidden: state.hidden_paths.contains(&key),
            });
        }
    }
    Ok(entries)
}

fn relative_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}

pub(crate) fn image_key(root: &Path, image: &Path) -> Result<String, AppError> {
    let root = root.canonicalize()?;
    let image = image.canonicalize()?;
    if !image.is_file() {
        return Err(AppError::Validation(
            "Preview image must be a regular file".to_string(),
        ));
    }
    if !image
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            super::IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| extension.eq_ignore_ascii_case(allowed))
        })
    {
        return Err(AppError::Validation(
            "Preview visibility requires an image file".to_string(),
        ));
    }
    let relative = image.strip_prefix(&root).map_err(|_| {
        AppError::Security("Preview image escapes the selected mod folder".to_string())
    })?;
    Ok(relative_key(relative))
}

pub(crate) fn set_hidden(root: &Path, images: &[PathBuf], hide: bool) -> Result<(), AppError> {
    let mut state = load_state(root)?;
    let previous = (state.hidden_paths.clone(), state.managed_paths.clone());
    for image in images {
        let key = image_key(root, image)?;
        if hide {
            state.managed_paths.insert(key.clone());
            state.hidden_paths.insert(key);
        } else {
            if state.hidden_paths.remove(&key) {
                state.managed_paths.insert(key);
            }
        }
    }
    if state.hidden_paths == previous.0 && state.managed_paths == previous.1 {
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(&state).map_err(|error| {
        AppError::Internal(format!("Could not encode preview visibility: {error}"))
    })?;
    if bytes.len() as u64 > MAX_VISIBILITY_BYTES {
        return Err(AppError::Validation(
            "Too many hidden preview images".to_string(),
        ));
    }
    crate::platform::fs::atomic_file::atomic_write(&root.join(VISIBILITY_FILE), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_round_trips_without_modifying_images_and_survives_toggle() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mod");
        std::fs::create_dir(&root).unwrap();
        let image = root.join("preview.png");
        std::fs::write(&image, b"original").unwrap();
        set_hidden(&root, std::slice::from_ref(&image), true).unwrap();
        assert!(apply_visibility(&root, vec![image.clone()]).unwrap()[0].is_hidden);
        assert_eq!(std::fs::read(&image).unwrap(), b"original");
        let disabled = temp.path().join("DISABLED Mod");
        std::fs::rename(&root, &disabled).unwrap();
        let image = disabled.join("preview.png");
        assert!(apply_visibility(&disabled, vec![image.clone()]).unwrap()[0].is_hidden);
        set_hidden(&disabled, &[image], false).unwrap();
        assert!(load_state(&disabled).unwrap().hidden_paths.is_empty());
    }

    #[test]
    fn outside_images_cannot_modify_visibility() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mod");
        std::fs::create_dir(&root).unwrap();
        let outside = temp.path().join("asset.png");
        std::fs::write(&outside, b"asset").unwrap();
        assert!(set_hidden(&root, &[outside], true).is_err());
        assert!(!root.join(VISIBILITY_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn internal_junction_aliases_share_the_canonical_visibility_key() {
        use std::os::windows::process::CommandExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mod");
        let textures = root.join("Textures");
        let alias = root.join("Alias");
        std::fs::create_dir_all(&textures).unwrap();
        let image = textures.join("asset.png");
        std::fs::write(&image, b"asset").unwrap();
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "New-Item -ItemType Junction -Path $env:PREVIEW_ALIAS -Target $env:PREVIEW_TARGET | Out-Null",
            ])
            .env("PREVIEW_ALIAS", &alias)
            .env("PREVIEW_TARGET", &textures)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let alias_image = alias.join("asset.png");
        set_hidden(&root, std::slice::from_ref(&alias_image), true).unwrap();
        let entries = apply_visibility(&root, vec![alias_image, image.clone()]).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_hidden);
        set_hidden(&root, std::slice::from_ref(&image), false).unwrap();
        assert!(!apply_visibility(&root, vec![image.clone()]).unwrap()[0].is_hidden);
        assert_eq!(std::fs::read(&image).unwrap(), b"asset");
    }
}
