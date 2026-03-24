//! Unit tests for the file generation pipeline.

use std::collections::HashMap;
use tempfile::TempDir;

use crate::services::ini::document::KeyBinding;
use crate::services::keyviewer::generator::{
    atomic_write, clear_status_file, discover_reload_key, generate_keybind_text,
    generate_keyviewer_ini, generate_status_text, write_keybind_files, write_status_file,
    SourceKeyBinding, StatusFields,
};
use crate::services::keyviewer::matcher::{MatchConfidence, MatchResult};

fn make_keybinding(section: &str, key: Option<&str>, back: Option<&str>) -> KeyBinding {
    KeyBinding {
        section_name: section.to_string(),
        key: key.map(|s| s.to_string()),
        back: back.map(|s| s.to_string()),
        key_line_idx: None,
        back_line_idx: None,
    }
}

fn make_match_result(name: &str, sentinels: &[&str]) -> MatchResult {
    MatchResult {
        object_name: name.to_string(),
        object_type: "Character".to_string(),
        score: 50.0,
        matched_hashes: sentinels.iter().map(|s| s.to_string()).collect(),
        sentinel_hashes: sentinels.iter().map(|s| s.to_string()).collect(),
        confidence: MatchConfidence::High,
    }
}

// ─── Atomic Write ────────────────────────────────────────────────────────────

#[test]
fn atomic_write_creates_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    atomic_write(&path, "hello world").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
}

#[test]
fn atomic_write_creates_parent_dirs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sub").join("dir").join("test.txt");
    atomic_write(&path, "nested").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "nested");
}

#[test]
fn atomic_write_overwrites_existing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    atomic_write(&path, "first").unwrap();
    atomic_write(&path, "second").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
}

#[test]
fn atomic_write_no_tmp_leftover() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    atomic_write(&path, "content").unwrap();
    let tmp_path = path.with_extension("tmp");
    assert!(!tmp_path.exists());
}

// ─── Keybind Text Generation ─────────────────────────────────────────────────

#[test]
fn keybind_text_with_key_and_back() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Albedo".to_string(),
        keybinds: vec![make_keybinding("KeyToggleBody", Some("1"), Some("2"))],
    }];
    let text = generate_keybind_text("Albedo", &kbs, "F7");
    assert!(text.contains("Albedo"));
    assert!(text.contains("Key: 1"));
    assert!(text.contains("Back: 2"));
}

#[test]
fn keybind_text_key_only() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Amber".to_string(),
        keybinds: vec![make_keybinding("KeyToggleBody", Some("3"), None)],
    }];
    let text = generate_keybind_text("Amber", &kbs, "F7");
    assert!(text.contains("Key: 3"));
    assert!(!text.contains("Back:"));
}

#[test]
fn keybind_text_no_keybinds() {
    let text = generate_keybind_text("Empty", &[], "F7");
    assert!(text.contains("No keybinds found"));
}

#[test]
fn keybind_text_multiple_keybinds() {
    let kbs = vec![SourceKeyBinding {
        mod_name: "Test".to_string(),
        keybinds: vec![
            make_keybinding("KeyToggleBody", Some("1"), None),
            make_keybinding("KeyToggleHead", Some("2"), None),
        ],
    }];
    let text = generate_keybind_text("Test", &kbs, "F7");
    assert!(text.contains("[KeyToggleBody]"));
    assert!(text.contains("[KeyToggleHead]"));
}

// ─── Write Keybind Files ─────────────────────────────────────────────────────

#[test]
fn write_keybind_files_creates_per_sentinel() {
    let dir = TempDir::new().unwrap();
    let matches = vec![make_match_result("Albedo", &["aabb1111", "aabb2222"])];
    let mut keybinds = HashMap::new();
    keybinds.insert(
        "Albedo".to_string(),
        vec![SourceKeyBinding {
            mod_name: "Albedo".to_string(),
            keybinds: vec![make_keybinding("KeyToggle", Some("1"), None)],
        }],
    );

    let written =
        write_keybind_files(dir.path(), &matches, &keybinds, false, "F7".to_string()).unwrap();

    // 2 sentinel files + 1 fallback = 3
    assert_eq!(written.len(), 3);
    assert!(dir.path().join("aabb1111.txt").exists());
    assert!(dir.path().join("aabb2222.txt").exists());
    assert!(dir.path().join("_fallback.txt").exists());

    let content = std::fs::read_to_string(dir.path().join("aabb1111.txt")).unwrap();
    assert!(content.contains("Albedo"));
}

#[test]
fn write_keybind_files_fallback_shows_safe_mode() {
    let dir = TempDir::new().unwrap();
    write_keybind_files(dir.path(), &[], &HashMap::new(), true, "F7".to_string()).unwrap();
    let content = std::fs::read_to_string(dir.path().join("_fallback.txt")).unwrap();
    assert!(content.contains("Safe Mode: ON"));
}

// ─── Status Banner ───────────────────────────────────────────────────────────

#[test]
fn status_text_safe_mode_on() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Default".to_string()),
        conflict_count: Some(0),
        ..Default::default()
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.contains("Safe: ON"));
    assert!(text.contains("Preset: Default"));
}

#[test]
fn status_text_safe_mode_off_with_folder() {
    let fields = StatusFields {
        safe_mode: false,
        preset_name: Some("Main".to_string()),
        folder_name: Some("Cape".to_string()),
        scope_name: Some("Albedo".to_string()),
        conflict_count: Some(0),
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.contains("Safe: OFF"));
    assert!(text.contains("Folder: Cape"));
    assert!(text.contains("Scope: Albedo"));
}

#[test]
fn status_text_within_limits() {
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Very Long Preset Name That Could Be Anything".to_string()),
        folder_name: Some("SomeFolderName".to_string()),
        scope_name: Some("SomeScope".to_string()),
        conflict_count: Some(0),
    };
    let text = generate_status_text(&fields, &crate::services::hotkeys::HotkeyConfig::default());
    assert!(text.lines().count() <= 10);
    assert!(text.len() <= 4096);
}

#[test]
fn write_status_file_atomic() {
    let dir = TempDir::new().unwrap();
    let fields = StatusFields {
        safe_mode: true,
        preset_name: Some("Test".to_string()),
        conflict_count: Some(0),
        ..Default::default()
    };
    let path = write_status_file(
        dir.path(),
        &fields,
        &crate::services::hotkeys::HotkeyConfig::default(),
    )
    .unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Safe: ON"));
}

#[test]
fn clear_status_file_empties() {
    let dir = TempDir::new().unwrap();
    let fields = StatusFields {
        safe_mode: false,
        conflict_count: Some(0),
        ..Default::default()
    };
    write_status_file(
        dir.path(),
        &fields,
        &crate::services::hotkeys::HotkeyConfig::default(),
    )
    .unwrap();
    clear_status_file(dir.path()).unwrap();
    let content = std::fs::read_to_string(dir.path().join("runtime_status.txt")).unwrap();
    assert!(content.is_empty());
}

// ─── Reload Key Discovery ───────────────────────────────────────────────────

#[test]
fn discovers_reload_fixes_key() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        r#"
[Constants]
global $active = 0

[KeyReload]
type = reload_fixes
key = F10
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx);
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(!config.is_fallback);
}

#[test]
fn discovers_reload_key_case_insensitive() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        r#"
[KeyReloadMods]
Type = Reload_Fixes
Key = F9
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx);
    assert_eq!(config.reload_fixes_key, "F9");
    assert!(!config.is_fallback);
}

#[test]
fn falls_back_to_f10_when_no_reload_section() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        r#"
[Constants]
global $active = 0
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx);
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(config.is_fallback);
}

#[test]
fn falls_back_to_f10_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("nonexistent.ini");
    let config = discover_reload_key(&d3dx);
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(config.is_fallback);
}

#[test]
fn ignores_reload_config_type() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        r#"
[KeyReload]
type = reload_config
key = F11
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx);
    // reload_config is NOT reload_fixes → fallback
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(config.is_fallback);
}

// ─── KeyViewer.ini Generation ────────────────────────────────────────────────

#[test]
fn keyviewer_ini_contains_header() {
    let ini = generate_keyviewer_ini(&[], "F7", ".emmm_data/keybinds/active");
    assert!(ini.contains(";   KeyViewer.ini — Auto-generated by EMMM"));
}

#[test]
fn keyviewer_ini_contains_toggle_key() {
    let ini = generate_keyviewer_ini(&[], "F7", ".emmm_data/keybinds/active");
    // New key section uses cycle type
    assert!(ini.contains("[KeyEMMM_Toggle]"));
    assert!(ini.contains("key = F7"));
    assert!(ini.contains("type = cycle"));
    assert!(ini.contains("$kv_active = 0, 1"));
}

#[test]
fn keyviewer_ini_contains_sentinel_sections() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");
    // New prefix is TextureOverride_EMM_ with hyphen/space → underscore
    assert!(ini.contains("[TextureOverride_EMM_Albedo_S0]"));
    assert!(ini.contains("hash = aabb1111"));
    assert!(ini.contains("$kv_has_active = 1"));
    assert!(ini.contains("$kv_active_code = 0xaabb1111"));
    assert!(ini.contains("$kv_last_seen = time"));
}

#[test]
fn keyviewer_ini_contains_present_section() {
    let ini = generate_keyviewer_ini(&[], "F7", ".emmm_data/keybinds/active");
    assert!(ini.contains("[Present]"));
    // Reset flag via post
    assert!(ini.contains("post $kv_has_active = 0"));
    // Stale detection at 1.5s (not 0.5s)
    assert!(ini.contains("if time - $kv_last_seen > 1.5"));
    // Delegate to render command list
    assert!(ini.contains("run = CommandList_EMM_Render"));
}

#[test]
fn keyviewer_ini_uses_help_ini_pipeline() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");
    // Must use help.ini notification pipeline — NOT ps-t100
    assert!(!ini.contains("ps-t100"));
    assert!(ini.contains(
        r"pre Resource\ShaderFixes\help.ini\Notification = ref ResourceKeyViewer_aabb1111"
    ));
    assert!(ini.contains(r"pre Resource\ShaderFixes\help.ini\NotificationParams = ref ResourceBox"));
    assert!(ini.contains(r"pre run = CustomShader\ShaderFixes\help.ini\FormatText"));
    assert!(ini.contains("notification_timeout = 1000000000.0"));
    // OFF path must null the notification
    assert!(ini.contains(r"Resource\ShaderFixes\help.ini\Notification = null"));
}

#[test]
fn keyviewer_ini_single_decision_tree_no_dual_draw() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");
    // Status banner shown in else branch, NOT in separate draw call
    assert!(ini.contains("ref ResourceStatus"));
    // There must be NO parallel dual-draw command lists
    assert!(!ini.contains("KV_DrawStatus"));
    assert!(!ini.contains("KV_DrawKeybinds"));
}

#[test]
fn keyviewer_ini_contains_resource_sections() {
    let matches = vec![make_match_result("Albedo", &["aabb1111"])];
    let ini = generate_keyviewer_ini(&matches, "F7", ".emmm_data/keybinds/active");
    // New resource name is ResourceKeyViewer_HASH
    assert!(ini.contains("[ResourceKeyViewer_aabb1111]"));
    assert!(ini.contains("filename = .emmm_data/keybinds/active/aabb1111.txt"));
    // Correct type/format casing
    assert!(ini.contains("type = buffer"));
    assert!(ini.contains("format = R8_UINT"));
    // ResourceStatus section is present
    assert!(ini.contains("[ResourceStatus]"));
    assert!(ini.contains("filename = .emmm_data/status/runtime_status.txt"));
    // ResourceBox section is present
    assert!(ini.contains("[ResourceBox]"));
    assert!(ini.contains("type = StructuredBuffer"));
}

#[test]
fn keyviewer_ini_has_fallback_resource() {
    let ini = generate_keyviewer_ini(&[], "F7", "keybinds");
    assert!(ini.contains("[ResourceKeyViewer_Fallback]"));
    assert!(ini.contains("filename = keybinds/_fallback.txt"));
}

#[test]
fn keyviewer_ini_handles_spaces_in_names() {
    let matches = vec![make_match_result("Arataki Itto", &["cc112233"])];
    let ini = generate_keyviewer_ini(&matches, "F7", "keybinds");
    // Spaces (and hyphens) replaced with underscores in section name
    assert!(ini.contains("[TextureOverride_EMM_Arataki_Itto_S0]"));
}

#[test]
fn keyviewer_ini_global_variables() {
    let ini = generate_keyviewer_ini(&[], "F7", "keybinds");
    // New global variable names
    assert!(ini.contains("global $kv_active = 0"));
    assert!(ini.contains("global $kv_has_active = 0"));
    assert!(ini.contains("global $kv_active_code = 0"));
    assert!(ini.contains("global $kv_last_seen = 0"));
    // Old names must NOT appear
    assert!(!ini.contains("$kv_visible"));
    assert!(!ini.contains("$kv_detection_frame"));
}
