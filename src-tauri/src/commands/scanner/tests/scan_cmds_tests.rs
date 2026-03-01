use super::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_scan_state_cancellation() {
    let state = ScanState::new();
    assert!(!state.is_cancelled());

    state.cancel();
    assert!(state.is_cancelled());

    state.reset();
    assert!(!state.is_cancelled());
}

#[tokio::test]
async fn test_get_scan_result_auto_matched_uses_staged_status_semantics() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("Mystery Mod");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(
        mod_dir.join("config.ini"),
        "[TextureOverrideBody]\nhash = d94c8962\n",
    )
    .unwrap();

    let db_json = json!([
        {
            "name": "Raiden Shogun",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {"Default": ["d94c8962"]}
        },
        {
            "name": "Jean",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        }
    ])
    .to_string();

    let results = get_scan_result(dir.path().to_string_lossy().to_string(), db_json)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let item = &results[0];
    assert_eq!(item.matched_object.as_deref(), Some("Raiden Shogun"));
    assert_eq!(item.match_level, "AutoMatched");
}

#[tokio::test]
async fn test_get_scan_result_needs_review_does_not_auto_assign_object() {
    let dir = TempDir::new().unwrap();
    let mod_dir = dir.path().join("Sunset Pack");
    fs::create_dir(&mod_dir).unwrap();

    let db_json = json!([
        {
            "name": "Amber",
            "tags": ["sunset"],
            "object_type": "Character",
            "custom_skins": [
                {
                    "name": "Sunset",
                    "aliases": ["Sunset"],
                    "thumbnail_skin_path": null,
                    "rarity": null
                }
            ],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        },
        {
            "name": "Lisa",
            "tags": ["sunset"],
            "object_type": "Character",
            "custom_skins": [
                {
                    "name": "Sunset",
                    "aliases": ["Sunset"],
                    "thumbnail_skin_path": null,
                    "rarity": null
                }
            ],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        }
    ])
    .to_string();

    let results = get_scan_result(dir.path().to_string_lossy().to_string(), db_json)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let item = &results[0];
    assert_eq!(item.match_level, "NeedsReview");
    assert_eq!(item.matched_object, None);
}

#[tokio::test]
async fn test_scan_state_halts_iteration() {
    let state = ScanState::new();
    state.cancel();

    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("Mod1")).unwrap();
    fs::create_dir(dir.path().join("Mod2")).unwrap();

    let candidates = crate::services::scanner::core::walker::scan_mod_folders(dir.path()).unwrap();

    let mut processed = 0;
    for _ in candidates.iter() {
        if state.is_cancelled() {
            break;
        }
        processed += 1;
    }

    assert_eq!(processed, 0, "Iteration should halt immediately");
}
