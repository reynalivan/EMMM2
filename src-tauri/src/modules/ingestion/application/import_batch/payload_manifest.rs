//! Full-content manifests used by Mod Inbox duplicate and collision checks.
//!
//! A manifest covers every regular file below the selected mod root.  It does
//! not sample large textures: a matching manifest is evidence that the two
//! payload trees are byte-for-byte equivalent, including INI and preview files.

use crate::shared::errors::AppError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

pub const PAYLOAD_MANIFEST_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadManifestFile {
    pub relative_path: String,
    pub size_bytes: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadManifest {
    pub version: u8,
    pub file_count: u32,
    pub total_size_bytes: String,
    pub content_sha256: String,
    pub files: Vec<PayloadManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadManifestMetadata {
    pub file_count: u32,
    pub total_size_bytes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestComparisonKind {
    Exact,
    TargetHasAdditionalFiles,
    Different,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestComparison {
    pub kind: ManifestComparisonKind,
    pub same_files: u32,
    pub changed_files: u32,
    pub missing_files: u32,
    pub additional_files: u32,
}

pub fn sha256_file(path: &Path, cancel_token: Option<&AtomicBool>) -> Result<String, AppError> {
    let file = File::open(path)?;
    sha256_reader(BufReader::new(file), cancel_token)
}

pub fn validate_import_payload_tree(
    root: &Path,
    cancel_token: Option<&AtomicBool>,
) -> Result<(), AppError> {
    let root = root.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "import payload: selected folder '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "import payload: selected path is not a folder: {}",
            root.display()
        )));
    }

    for entry in WalkDir::new(&root).follow_links(false).sort_by_file_name() {
        check_cancelled(cancel_token)?;
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "import payload: could not read '{}': {error}",
                root.display()
            ))
        })?;
        if entry.path() == root || entry.file_type().is_dir() {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(AppError::Security(format!(
                "import payload: symbolic link is not allowed: {}",
                entry.path().display()
            )));
        }
        if !entry.file_type().is_file() {
            return Err(AppError::Validation(format!(
                "import payload: unsupported filesystem entry: {}",
                entry.path().display()
            )));
        }
        let relative = entry.path().strip_prefix(&root).map_err(|error| {
            AppError::Security(format!(
                "import payload: entry escaped its root '{}': {error}",
                entry.path().display()
            ))
        })?;
        let relative_path = normalized_relative_path(relative)?;
        validate_import_payload_file(&relative_path)?;
    }
    Ok(())
}

pub fn build_payload_manifest(
    root: &Path,
    cancel_token: Option<&AtomicBool>,
) -> Result<PayloadManifest, AppError> {
    build_payload_manifest_inner(root, cancel_token, false)
}

/// Builds the source manifest used by import analysis while enforcing the same
/// payload restrictions applied before staging. Keeping those checks in this
/// traversal avoids a separate metadata walk before every full-content hash.
pub fn build_validated_import_payload_manifest(
    root: &Path,
    cancel_token: Option<&AtomicBool>,
) -> Result<PayloadManifest, AppError> {
    build_payload_manifest_inner(root, cancel_token, true)
}

fn build_payload_manifest_inner(
    root: &Path,
    cancel_token: Option<&AtomicBool>,
    validate_import_payload: bool,
) -> Result<PayloadManifest, AppError> {
    let root = root.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "payload_manifest: selected root '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "payload_manifest: selected root is not a folder: {}",
            root.display()
        )));
    }

    let mut files = Vec::new();
    let mut total_size_bytes = 0_u64;
    for entry in WalkDir::new(&root).follow_links(false).sort_by_file_name() {
        check_cancelled(cancel_token)?;
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "payload_manifest: could not read '{}': {error}",
                root.display()
            ))
        })?;
        if entry.path() == root {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(AppError::Security(format!(
                "payload_manifest: symbolic link is not allowed: {}",
                entry.path().display()
            )));
        }
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(AppError::Validation(format!(
                "payload_manifest: unsupported filesystem entry: {}",
                entry.path().display()
            )));
        }
        let relative = entry.path().strip_prefix(&root).map_err(|error| {
            AppError::Security(format!(
                "payload_manifest: entry escaped its root '{}': {error}",
                entry.path().display()
            ))
        })?;
        let relative_path = normalized_relative_path(relative)?;
        if validate_import_payload {
            validate_import_payload_file(&relative_path)?;
        }
        let size_bytes = entry
            .metadata()
            .map_err(|error| {
                AppError::Io(format!(
                    "payload_manifest: could not read metadata for '{}': {error}",
                    entry.path().display()
                ))
            })?
            .len();
        total_size_bytes = total_size_bytes.checked_add(size_bytes).ok_or_else(|| {
            AppError::Validation("payload_manifest: total size overflow".to_string())
        })?;
        files.push(PayloadManifestFile {
            relative_path,
            size_bytes: size_bytes.to_string(),
            sha256: sha256_file(entry.path(), cancel_token)?,
        });
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let content_sha256 = manifest_hash(&files);
    let file_count = u32::try_from(files.len()).map_err(|_| {
        AppError::Validation("payload_manifest: file count exceeds supported limit".to_string())
    })?;
    Ok(PayloadManifest {
        version: PAYLOAD_MANIFEST_VERSION,
        file_count,
        total_size_bytes: total_size_bytes.to_string(),
        content_sha256,
        files,
    })
}

fn validate_import_payload_file(relative_path: &str) -> Result<(), AppError> {
    if let Some(extension) =
        crate::shared::payload_security::blocked_mod_payload_extension(Path::new(relative_path))
    {
        return Err(AppError::Security(format!(
            "Mod folders cannot contain executable or script files (.{}): {}",
            extension, relative_path
        )));
    }
    Ok(())
}

pub fn payload_manifest_metadata(
    root: &Path,
    cancel_token: Option<&AtomicBool>,
) -> Result<PayloadManifestMetadata, AppError> {
    let root = root.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "payload_manifest: selected root '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(AppError::Validation(format!(
            "payload_manifest: selected root is not a folder: {}",
            root.display()
        )));
    }
    let mut file_count = 0_u32;
    let mut total_size_bytes = 0_u64;
    for entry in WalkDir::new(&root).follow_links(false).sort_by_file_name() {
        check_cancelled(cancel_token)?;
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "payload_manifest: could not read '{}': {error}",
                root.display()
            ))
        })?;
        if entry.path() == root {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(AppError::Security(format!(
                "payload_manifest: symbolic link is not allowed: {}",
                entry.path().display()
            )));
        }
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(AppError::Validation(format!(
                "payload_manifest: unsupported filesystem entry: {}",
                entry.path().display()
            )));
        }
        file_count = file_count.checked_add(1).ok_or_else(|| {
            AppError::Validation("payload_manifest: file count exceeds supported limit".to_string())
        })?;
        total_size_bytes = total_size_bytes
            .checked_add(
                entry
                    .metadata()
                    .map_err(|error| {
                        AppError::Io(format!(
                            "payload_manifest: could not read metadata for '{}': {error}",
                            entry.path().display()
                        ))
                    })?
                    .len(),
            )
            .ok_or_else(|| {
                AppError::Validation("payload_manifest: total size overflow".to_string())
            })?;
    }
    Ok(PayloadManifestMetadata {
        file_count,
        total_size_bytes: total_size_bytes.to_string(),
    })
}

pub fn compare_payload_manifests(
    source: &PayloadManifest,
    target: &PayloadManifest,
) -> ManifestComparison {
    let source_files = source
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let target_files = target
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut same_files = 0_u32;
    let mut changed_files = 0_u32;
    let mut missing_files = 0_u32;
    let mut additional_files = 0_u32;

    for (path, source_file) in &source_files {
        match target_files.get(path) {
            Some(target_file)
                if source_file.size_bytes == target_file.size_bytes
                    && source_file.sha256 == target_file.sha256 =>
            {
                same_files += 1;
            }
            Some(_) => changed_files += 1,
            None => missing_files += 1,
        }
    }
    for path in target_files.keys() {
        if !source_files.contains_key(path) {
            additional_files += 1;
        }
    }

    let kind = if changed_files == 0 && missing_files == 0 && additional_files == 0 {
        ManifestComparisonKind::Exact
    } else if changed_files == 0 && missing_files == 0 {
        ManifestComparisonKind::TargetHasAdditionalFiles
    } else {
        ManifestComparisonKind::Different
    };
    ManifestComparison {
        kind,
        same_files,
        changed_files,
        missing_files,
        additional_files,
    }
}

fn sha256_reader<R: Read>(
    mut reader: R,
    cancel_token: Option<&AtomicBool>,
) -> Result<String, AppError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancelled(cancel_token)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn normalized_relative_path(path: &Path) -> Result<String, AppError> {
    let mut parts = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(AppError::Security(format!(
                "payload_manifest: invalid relative path component: {}",
                path.display()
            )));
        };
        let name = name.to_string_lossy();
        if name.is_empty() {
            return Err(AppError::Security(
                "payload_manifest: empty path component".to_string(),
            ));
        }
        parts.push(name.into_owned());
    }
    if parts.is_empty() {
        return Err(AppError::Security(
            "payload_manifest: empty relative path".to_string(),
        ));
    }
    Ok(parts.join("/"))
}

fn manifest_hash(files: &[PayloadManifestFile]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"EMMM2-payload-manifest-v1\0");
    for file in files {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.size_bytes.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.sha256.as_bytes());
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

fn check_cancelled(cancel_token: Option<&AtomicBool>) -> Result<(), AppError> {
    if cancel_token.is_some_and(|token| token.load(Ordering::SeqCst)) {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_payload_manifest, build_validated_import_payload_manifest, compare_payload_manifests,
        validate_import_payload_tree, ManifestComparisonKind,
    };
    use crate::shared::errors::AppError;

    #[test]
    fn manifests_include_every_file_and_explain_target_extras() {
        let workspace = tempfile::tempdir().unwrap();
        let source = workspace.path().join("source");
        let target = workspace.path().join("target");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(source.join("merged.ini"), b"[TextureOverride]\nhash = aa").unwrap();
        std::fs::write(source.join("nested/preview.png"), b"preview").unwrap();
        std::fs::write(target.join("merged.ini"), b"[TextureOverride]\nhash = aa").unwrap();
        std::fs::create_dir_all(target.join("nested")).unwrap();
        std::fs::write(target.join("nested/preview.png"), b"preview").unwrap();
        std::fs::write(target.join("author-notes.txt"), b"keep me").unwrap();

        let source_manifest = build_payload_manifest(&source, None).unwrap();
        let validated_source_manifest = build_validated_import_payload_manifest(&source, None)
            .expect("validated import manifest");
        let target_manifest = build_payload_manifest(&target, None).unwrap();
        assert_eq!(source_manifest.file_count, 2);
        assert_eq!(source_manifest.files[1].relative_path, "nested/preview.png");
        assert_eq!(validated_source_manifest, source_manifest);
        let comparison = compare_payload_manifests(&source_manifest, &target_manifest);
        assert_eq!(
            comparison.kind,
            ManifestComparisonKind::TargetHasAdditionalFiles
        );
        assert_eq!(comparison.same_files, 2);
        assert_eq!(comparison.additional_files, 1);
    }

    #[test]
    fn import_payload_validation_rejects_executable_files() {
        let workspace = tempfile::tempdir().unwrap();
        let source = workspace.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("merged.ini"), b"[TextureOverride]\nhash = aa").unwrap();
        std::fs::write(source.join("loader.dll"), b"binary").unwrap();

        let error = validate_import_payload_tree(&source, None).unwrap_err();

        assert!(matches!(error, AppError::Security(message) if message.contains("loader.dll")));

        let error = build_validated_import_payload_manifest(&source, None).unwrap_err();

        assert!(matches!(error, AppError::Security(message) if message.contains("loader.dll")));
    }
}
