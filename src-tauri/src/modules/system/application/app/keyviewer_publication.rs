use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::modules::automation::application::keyviewer::{generator, models::KeyViewerPanel};
use crate::modules::games::domain::models::GameType;
use crate::shared::errors::AppError;

pub(super) const KEYVIEWER_MANIFEST_FILE: &str = "manifest.json";
pub(super) const KEYVIEWER_RESOURCE_ROOT: &str = "generations";
pub(super) const KEYVIEWER_MANIFEST_VERSION: u8 = 2;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct KeyViewerManifest {
    pub(super) version: u8,
    pub(super) fingerprint: String,
    pub(super) generation_id: String,
}

pub(super) fn manifest_path(emmm_data_dir: &Path) -> PathBuf {
    emmm_data_dir.join(KEYVIEWER_MANIFEST_FILE)
}

pub(super) fn read_manifest(emmm_data_dir: &Path) -> Option<KeyViewerManifest> {
    let content = std::fs::read_to_string(manifest_path(emmm_data_dir)).ok()?;
    serde_json::from_str(&content).ok()
}

pub(super) fn entrypoint_uses_generation(content: &[u8], generation_id: &str) -> bool {
    let content = String::from_utf8_lossy(content);
    let prefix = format!("filename = {KEYVIEWER_RESOURCE_ROOT}/{generation_id}/");
    content.contains(&format!("{prefix}status/")) || content.contains(&format!("{prefix}keybinds/"))
}

pub(super) struct ExpectedKeyViewerResources<'a> {
    pub(super) panels: &'a [KeyViewerPanel],
    pub(super) toggle_key: &'a str,
    pub(super) keyviewer_enabled: bool,
    pub(super) status_text: Option<String>,
    pub(super) game_type: GameType,
}

impl ExpectedKeyViewerResources<'_> {
    pub(super) fn entrypoint_text(&self, generation_id: &str) -> Result<String, AppError> {
        generator::generate_keyviewer_ini_for_resources_with_options(
            self.panels,
            self.toggle_key,
            self.game_type,
            &format!("{KEYVIEWER_RESOURCE_ROOT}/{generation_id}"),
            self.keyviewer_enabled,
            self.status_text.is_some(),
        )
    }
}

pub(super) fn generation_resources_match(
    generation: &Path,
    resources: &ExpectedKeyViewerResources<'_>,
) -> bool {
    if !generation.is_dir() {
        return false;
    }
    if resources.keyviewer_enabled {
        for (index, panel) in resources.panels.iter().enumerate() {
            let expected = generator::generate_keybind_text(
                &panel.match_result.object_name,
                &panel.sources,
                resources.toggle_key,
            );
            let path = generation
                .join("keybinds/active")
                .join(generator::keybind_file_name(index));
            if !std::fs::read(path).is_ok_and(|bytes| bytes == expected.as_bytes()) {
                return false;
            }
        }
    }
    resources.status_text.as_ref().is_none_or(|expected| {
        std::fs::read(generation.join("status/runtime_status.txt"))
            .is_ok_and(|bytes| bytes == expected.as_bytes())
    })
}

fn valid_generation_id(id: &str) -> bool {
    is_content_generation_id(id) || is_legacy_generation_id(id)
}

pub(super) fn select_generation_id(
    emmm_data_dir: &Path,
    fingerprint: &str,
    resources: &ExpectedKeyViewerResources<'_>,
) -> String {
    let generations = emmm_data_dir.join(KEYVIEWER_RESOURCE_ROOT);
    if let Some(manifest) = read_manifest(emmm_data_dir).filter(|manifest| {
        manifest.version == KEYVIEWER_MANIFEST_VERSION
            && manifest.fingerprint == fingerprint
            && valid_generation_id(&manifest.generation_id)
            && generation_resources_match(&generations.join(&manifest.generation_id), resources)
    }) {
        return manifest.generation_id;
    }
    let id = format!("g{fingerprint}");
    let generation = generations.join(&id);
    if !generation.exists() || generation_resources_match(&generation, resources) {
        return id;
    }
    // A running game can still hold this tree. Repair publishes another immutable
    // generation and leaves the old one for the existing reload-gated cleanup.
    let repair_identity = format!("{fingerprint}:repair:{}", uuid::Uuid::new_v4());
    format!("g{:x}", Sha256::digest(repair_identity.as_bytes()))
}

pub(super) fn published_artifact_matches(
    emmm_data_dir: &Path,
    entrypoint: &Path,
    fingerprint: &str,
    resources: &ExpectedKeyViewerResources<'_>,
) -> bool {
    let Some(manifest) = read_manifest(emmm_data_dir) else {
        return false;
    };
    if manifest.version != KEYVIEWER_MANIFEST_VERSION
        || manifest.fingerprint != fingerprint
        || !valid_generation_id(&manifest.generation_id)
    {
        return false;
    }
    let Ok(content) = std::fs::read(entrypoint) else {
        return false;
    };
    if !is_current_emmm_entrypoint(&content) {
        return false;
    }
    let Ok(expected_ini) = resources.entrypoint_text(&manifest.generation_id) else {
        return false;
    };
    if content != expected_ini.as_bytes() {
        return false;
    }
    generation_resources_match(
        &emmm_data_dir
            .join(KEYVIEWER_RESOURCE_ROOT)
            .join(&manifest.generation_id),
        resources,
    )
}

pub(super) fn is_content_generation_id(name: &str) -> bool {
    name.len() == 65
        && name.starts_with('g')
        && name[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

pub(super) fn is_legacy_generation_id(name: &str) -> bool {
    let Some(value) = name.strip_prefix('g') else {
        return false;
    };
    let mut parts = value.split('-');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(milliseconds), Some(process_id), Some(sequence), None)
            if milliseconds.parse::<u128>().is_ok()
                && process_id.parse::<u32>().is_ok()
                && sequence.parse::<u64>().is_ok()
    )
}

pub(super) fn is_current_emmm_entrypoint(content: &[u8]) -> bool {
    let content = String::from_utf8_lossy(content).to_ascii_lowercase();
    content.contains("namespace = emmmv1") && content.contains("emmm-artifact: keyviewer v1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_keyviewer_publication_is_reusable_without_resource_references() {
        let root = tempfile::tempdir().unwrap();
        let resources = ExpectedKeyViewerResources {
            panels: &[],
            toggle_key: "F7",
            keyviewer_enabled: true,
            status_text: None,
            game_type: GameType::Gimi,
        };
        let fingerprint = "a".repeat(64);
        let generation_id = format!("g{fingerprint}");
        let generation = root
            .path()
            .join(KEYVIEWER_RESOURCE_ROOT)
            .join(&generation_id);
        std::fs::create_dir_all(&generation).unwrap();
        let manifest = KeyViewerManifest {
            version: KEYVIEWER_MANIFEST_VERSION,
            fingerprint: fingerprint.clone(),
            generation_id: generation_id.clone(),
        };
        std::fs::write(
            manifest_path(root.path()),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let content = resources.entrypoint_text(&generation_id).unwrap();
        assert!(!entrypoint_uses_generation(
            content.as_bytes(),
            &generation_id
        ));
        let entrypoint = root.path().join("KeyViewer.ini");
        std::fs::write(&entrypoint, &content).unwrap();

        assert!(published_artifact_matches(
            root.path(),
            &entrypoint,
            &fingerprint,
            &resources,
        ));

        std::fs::write(&entrypoint, format!("{content}\n; modified")).unwrap();
        assert!(!published_artifact_matches(
            root.path(),
            &entrypoint,
            &fingerprint,
            &resources,
        ));
        std::fs::write(&entrypoint, &content).unwrap();
        std::fs::remove_dir(&generation).unwrap();
        assert!(!published_artifact_matches(
            root.path(),
            &entrypoint,
            &fingerprint,
            &resources,
        ));
    }
}
