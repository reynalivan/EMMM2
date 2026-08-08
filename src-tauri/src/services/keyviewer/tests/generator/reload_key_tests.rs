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
