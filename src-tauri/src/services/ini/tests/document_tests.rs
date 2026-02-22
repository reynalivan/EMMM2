use super::*;
use std::fs;
use tempfile::TempDir;

// Covers: NC-6.3-02 (Missing INI File)
#[test]
fn test_list_ini_files_filters_non_ini_noise() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    fs::write(mod_dir.join("desktop.ini"), "[.ShellClassInfo]").unwrap();
    fs::write(mod_dir.join("d3dx.ini"), "[Constants]\n$swapvar = 1").unwrap();
    fs::write(mod_dir.join("config.INI"), "[KeySwap]\nkey = v").unwrap();
    fs::write(mod_dir.join("readme.txt"), "notes").unwrap();

    let files = list_ini_files(&mod_dir).unwrap();
    let names: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    assert_eq!(names, vec!["config.INI", "d3dx.ini"]);
}

// Covers: TC-6.3-01 (Parse Variables)
#[test]
fn test_read_ini_document_parses_variable_lines() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("config.ini");
    fs::write(&ini_path, "[Constants]\n$swapvar = 1\n").unwrap();

    let doc = read_ini_document(&ini_path).unwrap();

    assert_eq!(doc.mode, IniReadMode::Structured);
    assert_eq!(doc.variables.len(), 1);
    assert_eq!(doc.variables[0].name, "$swapvar");
    assert_eq!(doc.variables[0].value, "1");
}

// Covers: TC-6.3-01 (Keybindings)
#[test]
fn test_read_ini_document_parses_keybinding_section() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("config.ini");
    fs::write(&ini_path, "[KeyChangeDress]\nkey = v\nback = b\n").unwrap();

    let doc = read_ini_document(&ini_path).unwrap();

    assert_eq!(doc.key_bindings.len(), 1);
    assert_eq!(doc.key_bindings[0].section_name, "KeyChangeDress");
    assert_eq!(doc.key_bindings[0].key.as_deref(), Some("v"));
    assert_eq!(doc.key_bindings[0].back.as_deref(), Some("b"));
}

// Covers: TC-6.3-03 (BOM Handling)
#[test]
fn test_read_ini_document_detects_and_strips_bom_in_memory() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("bom.ini");

    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"[Constants]\n$var = 1\n");
    fs::write(&ini_path, bytes).unwrap();

    let doc = read_ini_document(&ini_path).unwrap();

    assert!(doc.had_bom);
    assert!(!doc.raw_lines[0].starts_with('\u{FEFF}'));
    assert_eq!(doc.mode, IniReadMode::Structured);
}

// Covers: NC-6.3-01 (Malformed Syntax)
#[test]
fn test_read_ini_document_malformed_section_falls_back_to_raw_mode() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("broken.ini");
    fs::write(&ini_path, "[Section\n$var = 1\n").unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.mode, IniReadMode::RawFallback);
}

// Covers: EC-6.01 (Shift-JIS / GBK INI)
#[test]
fn test_read_ini_document_non_utf8_falls_back_to_raw_mode() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("encoded.ini");
    fs::write(&ini_path, [0x82_u8, 0xA0_u8, 0x82_u8]).unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.mode, IniReadMode::RawFallback);
}

// Covers: DI-6.02 (BOM Preservation metadata), EC-6.05
#[test]
fn test_read_ini_document_detects_crlf_newline_style() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("newline.ini");
    fs::write(&ini_path, "[Constants]\r\n$var = 1\r\n").unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.newline_style, NewlineStyle::CrLf);
}
