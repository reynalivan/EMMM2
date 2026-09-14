use super::*;

#[test]
fn discovers_reload_config_without_requiring_reload_fixes() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        "[Hunting]\nreload_fixes = no_modifiers VK_F9\nreload_config = Ctrl+F10\n",
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_config_key, "Ctrl+F10");
}

#[test]
fn discovers_reload_config_case_insensitively() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        "[hUnTiNg]\nReload_Fixes = shift VK_F9\nRELOAD_CONFIG = VK_F10\n",
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_config_key, "F10");
}

#[test]
fn requires_manual_reload_without_a_reload_config_binding() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(&d3dx, "[Hunting]\nreload_fixes = F9\n").unwrap();

    let error = discover_reload_key(&d3dx).unwrap_err();
    assert!(error.to_string().contains("NeedsManualReload"));
}

#[test]
fn requires_manual_reload_when_the_config_file_is_missing() {
    let dir = TempDir::new().unwrap();
    let error = discover_reload_key(&dir.path().join("missing.ini")).unwrap_err();
    assert!(error.to_string().contains("NeedsManualReload"));
}

#[test]
fn does_not_treat_a_key_section_type_as_a_reload_binding() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(&d3dx, "[KeyReload]\ntype = reload_config\nkey = F11\n").unwrap();

    let error = discover_reload_key(&d3dx).unwrap_err();
    assert!(error.to_string().contains("reload_config is not bound"));
}

#[test]
fn normalizes_modifiers_and_ignores_negative_modifier_tokens() {
    let dir = TempDir::new().unwrap();
    let d3dx = dir.path().join("d3dx.ini");
    std::fs::write(
        &d3dx,
        "[Hunting]\nreload_fixes = no_alt ctrl shift VK_F5 ; user binding\nreload_config = no_modifiers F10\n",
    )
    .unwrap();

    let config = discover_reload_key(&d3dx).unwrap();
    assert_eq!(config.reload_config_key, "F10");
}

#[test]
fn rejects_ambiguous_or_controller_only_reload_bindings() {
    let dir = TempDir::new().unwrap();
    let ambiguous = dir.path().join("ambiguous.ini");
    std::fs::write(&ambiguous, "[Hunting]\nreload_config = F9 F10\n").unwrap();
    assert!(discover_reload_key(&ambiguous).is_err());

    let controller = dir.path().join("controller.ini");
    std::fs::write(&controller, "[Hunting]\nreload_config = XB_A\n").unwrap();
    assert!(discover_reload_key(&controller).is_err());
}

#[test]
fn resolves_an_existing_instance_config_before_mods_parent() {
    use crate::modules::games::domain::models::GameType;
    use crate::modules::settings::application::config::GameConfig;

    let dir = TempDir::new().unwrap();
    let package_root = dir.path().join("Package");
    std::fs::create_dir_all(package_root.join("Mods")).unwrap();
    std::fs::write(package_root.join("d3dx.ini"), "[Hunting]").unwrap();

    let game = GameConfig {
        id: "g1".into(),
        name: "Game".into(),
        game_type: GameType::GIMI,
        instance_path: package_root.clone(),
        mod_path: package_root.join("Mods"),
        ready_to_move_path: None,
        launch_mode: crate::modules::games::domain::models::LaunchMode::Standalone,
        game_exe: None,
        loader_exe: None,
        xxmi_launcher_exe: None,
        launch_args: None,
        warnings: Vec::new(),
    };

    assert_eq!(
        resolve_d3dx_ini_path(&game),
        Some(package_root.join("d3dx.ini"))
    );
}
