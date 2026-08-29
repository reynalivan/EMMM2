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

#[test]
fn test_list_ini_files_recurses_deterministically_and_skips_disabled_subtrees() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir_all(mod_dir.join("Nested/Deep")).unwrap();
    fs::create_dir_all(mod_dir.join("DISABLEDHidden")).unwrap();
    fs::write(mod_dir.join("z.ini"), "[Constants]").unwrap();
    fs::write(mod_dir.join("Nested/a.INI"), "[Constants]").unwrap();
    fs::write(mod_dir.join("Nested/Deep/b.ini"), "[Constants]").unwrap();
    fs::write(mod_dir.join("DISABLEDHidden/ignored.ini"), "[Constants]").unwrap();

    let files = list_ini_files(&mod_dir).unwrap();
    let relative: Vec<String> = files
        .iter()
        .map(|path| {
            path.strip_prefix(&mod_dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert_eq!(relative, vec!["Nested/a.INI", "Nested/Deep/b.ini", "z.ini"]);
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
    assert_eq!(doc.variables[0].qualifier, None);
}

#[test]
fn test_read_ini_document_parses_qualified_variables() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("qualified.ini");
    fs::write(
        &ini_path,
        "[Constants]\nglobal $active = 1\npersist $mode = 2\nlocal $temp = 3\n",
    )
    .unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.variables.len(), 3);
    assert_eq!(doc.variables[0].qualifier.as_deref(), Some("global"));
    assert_eq!(doc.variables[1].qualifier.as_deref(), Some("persist"));
    assert_eq!(doc.variables[2].qualifier.as_deref(), Some("local"));
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

#[test]
fn test_read_ini_document_keeps_repeated_bindings_and_section_identity() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("keys.ini");
    fs::write(
        &ini_path,
        "[KeySwap]\nkey = v\nback = b\nkey = x\n[KEYSWAP]\nkey = y\nback = z\n",
    )
    .unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.key_bindings.len(), 3);
    assert_eq!(doc.key_bindings[0].key.as_deref(), Some("v"));
    assert_eq!(doc.key_bindings[0].back.as_deref(), Some("b"));
    assert_eq!(doc.key_bindings[1].key.as_deref(), Some("x"));
    assert_eq!(doc.key_bindings[2].key.as_deref(), Some("y"));
    assert_eq!(doc.key_bindings[2].back.as_deref(), Some("z"));
    assert!(doc
        .key_bindings
        .iter()
        .all(|binding| binding.section_name == "KeySwap"));
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
fn test_read_ini_decodes_shift_jis_gracefully() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("encoded.ini");

    // Encode "[Section]\nKey=Value" in Shift-JIS
    let (cow, _, _) = encoding_rs::SHIFT_JIS.encode("[Section]\nKey=Value");
    fs::write(&ini_path, cow).unwrap();

    // The implementation should use encoding_rs to detect invalid UTF-8 and try Shift-JIS
    let doc = read_ini_document(&ini_path).unwrap();
    // If it properly decodes, it will parse as Structured
    assert_eq!(doc.mode, IniReadMode::Structured);
    assert_eq!(doc.raw_lines[0], "[Section]");
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

#[test]
fn test_read_ini_document_tracks_mixed_and_final_terminators() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("mixed.ini");
    fs::write(&ini_path, b"one\r\ntwo\nthree\rfour").unwrap();

    let doc = read_ini_document(&ini_path).unwrap();
    assert_eq!(doc.raw_lines, vec!["one", "two", "three", "four"]);
    assert_eq!(
        doc.line_terminators,
        vec![
            LineTerminator::CrLf,
            LineTerminator::Lf,
            LineTerminator::Cr,
            LineTerminator::None,
        ]
    );
    assert_eq!(
        doc.source_hash,
        blake3::hash(b"one\r\ntwo\nthree\rfour")
            .to_hex()
            .to_string()
    );
}

// Covers: TC-18 (Over 2MB INI aborts)
#[test]
fn test_read_ini_aborts_over_2mb() {
    let tmp = TempDir::new().unwrap();
    let ini_path = tmp.path().join("huge.ini");

    // Create a file slightly larger than 2MB
    let huge_data = vec![b'A'; 2 * 1024 * 1024 + 1024];
    fs::write(&ini_path, huge_data).unwrap();

    let result = read_ini_document(&ini_path);
    assert!(result.is_err(), "Reading INI files > 2MB should be aborted");
    if let Err(e) = result {
        assert!(e.to_string().contains("too large"));
    }
}
