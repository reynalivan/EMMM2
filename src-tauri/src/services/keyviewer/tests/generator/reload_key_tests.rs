use super::*;

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

[Hunting]
reload_fixes = no_modifiers VK_F10
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
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
[hUnTiNg]
Reload_Fixes = shift VK_F9
"#,
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_fixes_key, "Shift+F9");
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

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(config.is_fallback);
}

#[test]
fn falls_back_to_f10_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("nonexistent.ini");
    let config = discover_reload_key(&d3dx).unwrap();
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

    let config = discover_reload_key(&d3dx).unwrap();
    // reload_config is NOT reload_fixes → fallback
    assert_eq!(config.reload_fixes_key, "F10");
    assert!(config.is_fallback);
}

#[test]
fn normalizes_space_separated_modifiers_and_inline_comment() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        "[Hunting]\nreload_fixes = no_alt ctrl shift VK_F5 ; user binding\n",
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_fixes_key, "Ctrl+Shift+F5");
    assert!(!config.is_fallback);
}

#[test]
fn rejects_ambiguous_or_controller_only_reload_bindings() {
    let dir = TempDir::new().unwrap();
    let ambiguous = dir.path().join("ambiguous.ini");
    std::fs::write(&ambiguous, "[Hunting]\nreload_fixes = F9 F10\n").unwrap();
    assert!(discover_reload_key(&ambiguous).is_err());

    let controller = dir.path().join("controller.ini");
    std::fs::write(&controller, "[Hunting]\nreload_fixes = XB_A\n").unwrap();
    assert!(discover_reload_key(&controller).is_err());
}

#[test]
fn resolves_package_config_before_executable_fallback() {
    use crate::domain::models::GameType;
    use crate::services::config::GameConfig;

    let dir = TempDir::new().unwrap();
    let package_root = dir.path().join("Package");
    let game_root = dir.path().join("Game");
    std::fs::create_dir_all(package_root.join("Mods")).unwrap();
    std::fs::create_dir_all(&game_root).unwrap();
    std::fs::write(package_root.join("d3dx.ini"), "[Hunting]").unwrap();
    std::fs::write(game_root.join("d3dx.ini"), "[Hunting]").unwrap();

    let game = GameConfig {
        id: "g1".into(),
        name: "Game".into(),
        game_type: GameType::GIMI,
        mod_path: package_root.join("Mods"),
        game_exe: game_root.join("game.exe"),
        loader_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    };

    assert_eq!(
        resolve_d3dx_ini_path(&game),
        Some(package_root.join("d3dx.ini"))
    );
}
