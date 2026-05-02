use super::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn score_candidates_batch_cmd_returns_scores_for_candidates() {
    let dir = TempDir::new().expect("tempdir");
    let mod_dir = dir.path().join("Kazuha");
    fs::create_dir(&mod_dir).expect("create mod dir");
    fs::write(
        mod_dir.join("mod.ini"),
        "[TextureOverrideKazuha]\nhash = 12345678\n",
    )
    .expect("write mod ini");

    let db_json = json!([
        {
            "name": "Kazuha",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        },
        {
            "name": "Diluc",
            "tags": [],
            "object_type": "Character",
            "custom_skins": [],
            "thumbnail_path": null,
            "metadata": null,
            "hash_db": {}
        }
    ])
    .to_string();

    let scores = score_candidates_batch_cmd(
        mod_dir.to_string_lossy().to_string(),
        vec!["Kazuha".to_string(), "Diluc".to_string()],
        db_json,
    )
    .await
    .expect("scores");

    assert!(scores.contains_key("Kazuha"));
    assert!(scores.contains_key("Diluc"));
}
