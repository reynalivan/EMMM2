use super::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn ambiguous_master_db() -> MasterDb {
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

    MasterDb::from_json(&db_json).unwrap()
}

fn simple_master_db() -> MasterDb {
    let db_json = json!([
        {
            "name": "Raiden Shogun",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {"Default": ["d94c8962"]}
        }
    ])
    .to_string();

    MasterDb::from_json(&db_json).unwrap()
}

// Covers: TC-2.3-Review-01 (NeedsReview must stay pending in auto-organize)
#[test]
fn test_organize_mod_needs_review_keeps_folder_unmoved() {
    let temp_dir = TempDir::new().unwrap();
    let target_root = temp_dir.path().join("Mods");
    fs::create_dir(&target_root).unwrap();

    let mod_dir = temp_dir.path().join("Sunset Pack");
    fs::create_dir(&mod_dir).unwrap();

    let db = ambiguous_master_db();
    let result = organize_mod(&mod_dir, &target_root, &db).unwrap();

    assert_eq!(result.new_path, mod_dir);
    assert!(mod_dir.exists());
}

// Covers: TC-2.3-Review-02 (NoMatch must stay pending in auto-organize)
#[test]
fn test_organize_mod_no_match_keeps_folder_unmoved() {
    let temp_dir = TempDir::new().unwrap();
    let target_root = temp_dir.path().join("Mods");
    fs::create_dir(&target_root).unwrap();

    let mod_dir = temp_dir.path().join("totally unknown package");
    fs::create_dir(&mod_dir).unwrap();

    let db = simple_master_db();
    let result = organize_mod(&mod_dir, &target_root, &db).unwrap();

    assert_eq!(result.new_path, mod_dir);
    assert!(mod_dir.exists());
}
