use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::gamebanana::{
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
    assert_eq!(
        GameBananaGame::from_key("gimi"),
        Some(GameBananaGame::Genshin)
    );
    assert_eq!(
        GameBananaGame::from_key("srmi"),
        Some(GameBananaGame::StarRail)
    );
    assert_eq!(
        GameBananaGame::from_key("zzmi"),
        Some(GameBananaGame::ZenlessZoneZero)
    );
    assert_eq!(
        GameBananaGame::from_key("wwmi"),
        Some(GameBananaGame::WutheringWaves)
    );
    assert_eq!(
        GameBananaGame::from_key("efmi"),
        Some(GameBananaGame::ArknightsEndfield)
    );
    assert_eq!(GameBananaGame::from_key("unknown"), None);

    assert_eq!(GameBananaGame::Genshin.game_id(), 8552);
    assert_eq!(GameBananaGame::StarRail.game_id(), 18366);
    assert_eq!(GameBananaGame::ZenlessZoneZero.game_id(), 19567);
    assert_eq!(GameBananaGame::WutheringWaves.game_id(), 20357);
    assert_eq!(GameBananaGame::ArknightsEndfield.game_id(), 21842);
}

// Live API test (we can run this with cargo test -- --ignored to prevent CI flakiness,
// but we'll run it normally for manual testing here)
#[test]
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

    let result = fetch_gamebanana_metadata(&[gb_ref.clone()], &config);

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
