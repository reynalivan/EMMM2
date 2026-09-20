use super::*;

#[test]
fn keyviewer_ini_contains_stable_namespace_and_persistent_toggle() {
    let ini = generate_keyviewer_ini(&[], "F7", ".emmm_data/keybinds/active");

    assert!(ini.contains("namespace = EMMMv1"));
    assert!(ini.contains("[KeyEMMMv1_ToggleOverlay]"));
    assert!(ini.contains("key = F7"));
    assert!(ini.contains("type = cycle"));
    assert!(ini.contains("global persist $emmm_kv_active = 0"));
    assert!(ini.contains("$emmm_kv_active = 0, 1"));
}

#[test]
fn keyviewer_ini_marks_each_sentinel_with_a_per_frame_flag() {
    let matches = vec![make_match_result("Albedo", &["aabb1111", "aabb2222"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    assert!(ini.contains("[TextureOverride_EMMMv1_Albedo_0_S0]"));
    assert!(ini.contains("[TextureOverride_EMMMv1_Albedo_0_S1]"));
    assert!(ini.contains("hash = aabb1111"));
    assert!(ini.contains("match_priority = -100"));
    assert!(ini.contains("$emmm_kv_detect_000 = 1"));
    assert!(!ini.contains("allow_duplicate_hash"));
}

#[test]
fn keyviewer_ini_preserves_index_draw_context_on_the_observer() {
    use crate::modules::automation::application::keyviewer::matcher::{
        RuntimeSentinel, RuntimeSentinelSource,
    };
    use crate::modules::matching::application::deep_matcher::models::types::RuntimeResourceKind;

    let mut result = make_match_result("Indexed", &["e811d2a1"]);
    result.sentinels = vec![RuntimeSentinel {
        hash: "e811d2a1".to_string(),
        resource_kind: RuntimeResourceKind::IndexBuffer,
        callback_slot: "ib".to_string(),
        match_first_index: Some(40179),
        source: RuntimeSentinelSource::Harvest {
            section_name: "TextureOverrideIndexed".to_string(),
            file_path: std::path::PathBuf::from("fixture.ini"),
        },
    }];

    let ini = generate_keyviewer_ini(&[result], "F7", ".emmm_data/keybinds/active");
    assert!(ini.contains("hash = e811d2a1\nmatch_first_index = 40179\nmatch_priority = -100"));
}

#[test]
fn keyviewer_ini_renders_before_resetting_detection_flags() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    let render = ini.find("run = CommandList_EMMMv1_Render").unwrap();
    let reset = ini.find("post $emmm_kv_detect_000 = 0").unwrap();
    assert!(render < reset);
    assert!(!ini.contains("notification_timeout"));
    assert!(!ini.contains("$kv_active_code"));
}

#[test]
fn keyviewer_ini_draws_status_and_detected_character_panels_directly() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    assert!(ini.contains(r"Resource\GIMIv8\Text = ref ResourceEMMM_Status"));
    assert!(ini.contains(r"Resource\GIMIv8\Text = ref ResourceEMMM_KeyViewer_000"));
    assert!(ini.contains(r"Resource\GIMIv8\TextParams = ref ResourceEMMM_StatusBox"));
    assert!(ini.contains(r"run = CommandList\GIMIv8\PrintText"));
    assert!(ini.contains(r"Resource\GIMIv8\TextParams = ref ResourceEMMM_KeyViewerBox_0"));
    assert!(!ini.contains("$emmm_kv_panel_index"));
    assert!(!ini.contains("ShaderFixes\\help.ini"));
}

#[test]
fn keyviewer_ini_uses_compact_native_scale_and_left_alignment() {
    let ini = generate_keyviewer_ini(
        &[make_match_result("Albedo", &["aabb1111"])],
        "F7",
        ".emmm_data/keybinds/active",
    );

    assert!(ini.contains(
        "data = R32_FLOAT  -0.96 0.36 -0.30 0.24  1 1 1 1  0 0 0 0.92  0.02 0.02  0 3  0  1.00"
    ));
    assert!(ini.contains(
        "data = R32_FLOAT  -0.96 -0.24 -0.56 -0.92  1 1 1 1  0 0 0 0.92  0.02 0.02  0 3  0  0.92"
    ));
}

#[test]
fn keyviewer_ini_uses_the_same_viewport_geometry_for_every_match() {
    let matches = (0..4)
        .map(|index| make_match_result(&format!("Character{index}"), &["aabb1111"]))
        .collect::<Vec<_>>();
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    let character_geometry =
        "data = R32_FLOAT  -0.96 -0.24 -0.56 -0.92  1 1 1 1  0 0 0 0.92  0.02 0.02  0 3  0  0.92";
    assert_eq!(ini.matches(character_geometry).count(), matches.len());
}

#[test]
fn keyviewer_ini_uses_one_text_file_per_character() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    assert!(ini.contains("[ResourceEMMM_KeyViewer_000]"));
    assert!(ini.contains("filename = keybinds/active/character_000.txt"));
    assert!(ini.contains("[ResourceEMMM_Status]"));
    assert!(ini.contains("filename = status/runtime_status.txt"));
    assert!(ini.contains("[ResourceEMMM_KeyViewerBox_0]"));
}

#[test]
fn keyviewer_ini_allocates_a_panel_for_every_match() {
    let matches = (0..9)
        .map(|index| make_match_result(&format!("Character{index}"), &["aabb1111"]))
        .collect::<Vec<_>>();
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");

    assert!(ini.contains("[ResourceEMMM_KeyViewerBox_8]"));
    assert!(ini.contains(r"Resource\GIMIv8\TextParams = ref ResourceEMMM_KeyViewerBox_8"));
    assert!(!ini.contains("else if $emmm_kv_panel_index"));
}

#[test]
fn keyviewer_ini_uses_one_stable_resource_directory() {
    use crate::modules::automation::application::keyviewer::generator::generate_keyviewer_ini_for_resources;
    use crate::modules::games::domain::models::GameType;

    let ini = generate_keyviewer_ini_for_resources(
        &[make_match_result("Albedo", &["aabb1111"])],
        "F7",
        GameType::GIMI,
        "generations",
    )
    .unwrap();

    assert!(ini.contains("filename = generations/status/runtime_status.txt"));
    assert!(ini.contains("filename = generations/keybinds/active/character_000.txt"));
    assert!(!ini.contains("generations/20260914-1"));
}

#[test]
fn keyviewer_ini_uses_the_selected_package_namespace() {
    use crate::modules::games::domain::models::GameType;

    for (game_type, namespace) in [
        (GameType::GIMI, "GIMIv8"),
        (GameType::WWMI, "WWMIv1"),
        (GameType::SRMI, "SRMIv1"),
        (GameType::ZZMI, "ZZMIv1"),
    ] {
        let ini = generate_keyviewer_ini_for_game(&[], "F7", game_type).unwrap();
        assert!(ini.contains(&format!("Resource\\{namespace}\\Text")));
        assert!(ini.contains(&format!("CommandList\\{namespace}\\PrintText")));
    }
}

#[test]
fn keyviewer_ini_rejects_unsupported_profile_and_invalid_sentinel() {
    use crate::modules::games::domain::models::GameType;

    assert!(generate_keyviewer_ini_for_game(&[], "F7", GameType::EFMI).is_err());
    let invalid = vec![make_match_result("Bad", &["not-a-hash"])];
    assert!(generate_keyviewer_ini_for_game(&invalid, "F7", GameType::GIMI).is_err());
    assert!(generate_keyviewer_ini_for_game(&[], "F7\n[Constants]", GameType::GIMI).is_err());
}
