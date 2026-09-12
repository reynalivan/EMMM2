use crate::modules::matching::application::deep_matcher::analysis::content::FolderSignals;
use crate::modules::matching::application::deep_matcher::analysis::gamebanana::{
    detect_gamebanana_ids, fetch_gamebanana_metadata, GameBananaConfig, GameBananaGame,
    GameBananaRef,
};

#[test]
fn test_detect_gamebanana_ids_from_text() {
    let mut signals = FolderSignals::default();

    // Test URL in folder_tokens
    signals
        .folder_tokens
        .push("https://gamebanana.com/mods/528562".to_string());
    // Test URL in ini content
    signals
        .ini_content_tokens
        .push("gamebanana.com/skins/123456".to_string());

    let refs = detect_gamebanana_ids(&signals);

    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&GameBananaRef {
        item_type: "Mod".to_string(),
        item_id: 528562
    }));
    assert!(refs.contains(&GameBananaRef {
        item_type: "Skin".to_string(),
        item_id: 123456
    }));
}

#[test]
fn test_game_enum_resolution() {
    assert_eq!(GameBananaGame::Genshin.game_id(), 8552);
    assert_eq!(GameBananaGame::StarRail.game_id(), 18366);
    assert_eq!(GameBananaGame::ZenlessZoneZero.game_id(), 19567);
    assert_eq!(GameBananaGame::WutheringWaves.game_id(), 20357);
    assert_eq!(GameBananaGame::ArknightsEndfield.game_id(), 21842);
}

// The public API is intentionally outside the normal test suite: its availability,
// rate limits, and the referenced community item can change independently of EMMM.
// Run it explicitly with `cargo test test_live_api_fetch_and_validate -- --ignored`.
#[test]
#[ignore = "requires the live GameBanana API and a mutable community item"]
fn test_live_api_fetch_and_validate() {
    // A known Genshin Impact mod (from your request or recent subfeed)
    // Mod ID: 654298 -> "❤️Zibai❤️ Lunar Qilin"
    let gb_ref = GameBananaRef {
        item_type: "Mod".to_string(),
        item_id: 654298,
    };

    // Config with Genshin verification
    let config = GameBananaConfig {
        enabled: true,
        game: Some(GameBananaGame::Genshin),
    };

    let result = fetch_gamebanana_metadata(std::slice::from_ref(&gb_ref), &config);

    // Verify it fetched the name
    assert!(result.mod_name.is_some(), "Expected mod name from API");
    println!("Fetched Mod Name: {}", result.mod_name.as_ref().unwrap());

    // Verify it parsed file stems
    assert!(
        !result.file_stems.is_empty(),
        "Expected file stems from _aFiles"
    );
    println!("Fetched File Stems: {:?}", result.file_stems);

    // Now test game validation rejection by passing the WRONG game (e.g., Star Rail for a Genshin mod)
    let wrong_config = GameBananaConfig {
        enabled: true,
        game: Some(GameBananaGame::StarRail),
    };

    let wrong_result = fetch_gamebanana_metadata(&[gb_ref], &wrong_config);

    // Since it's a Genshin mod, asking for StarRail should fail validation and return empty
    assert!(
        wrong_result.mod_name.is_none(),
        "Expected API to reject wrong game"
    );
    assert!(
        wrong_result.file_stems.is_empty(),
        "Expected API to reject wrong game"
    );
    println!("Validation correctly rejected wrong game!");
}
