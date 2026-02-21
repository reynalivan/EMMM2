use crate::services::scanner::deep_matcher::MatchMode;
use crate::services::scanner::walker;

fn write_ini(path: &std::path::Path, hash: &str, section_token: &str, path_token: &str) {
    let text = format!(
        "[TextureOverride{section_token}]\nhash = {hash}\nfilename = Characters/{path_token}/body_diffuse.dds\n"
    );
    std::fs::write(path, text).expect("write ini");
}

#[test]
fn test_collect_deep_signals_quick_budget_enforces_two_root_ini_sorted() {
    // Covers: TC-2.2-Task8-01
    let temp = tempfile::TempDir::new().expect("temp dir");
    let folder = temp.path().join("quick_budget_mod");
    std::fs::create_dir_all(&folder).expect("create folder");
    std::fs::create_dir_all(folder.join("nested")).expect("create nested");

    write_ini(&folder.join("c.ini"), "33333333", "Yoimiya", "yoimiya");
    write_ini(&folder.join("a.ini"), "11111111", "Raiden", "raiden");
    write_ini(&folder.join("b.ini"), "22222222", "Ayaka", "ayaka");
    write_ini(
        &folder.join("nested").join("z.ini"),
        "44444444",
        "Diluc",
        "diluc",
    );

    let content = walker::scan_folder_content(&folder, 3);
    let signals = collect_deep_signals(
        &folder,
        &content,
        MatchMode::Quick,
        &IniTokenizationConfig::default(),
    );

    assert_eq!(signals.scanned_ini_files, QUICK_MAX_INI_FILES);
    assert_eq!(signals.ini_hashes, vec!["11111111", "22222222"]);
    assert!(signals.ini_section_tokens.contains(&"raiden".to_string()));
    assert!(signals.ini_section_tokens.contains(&"ayaka".to_string()));
    assert!(!signals.ini_hashes.contains(&"33333333".to_string()));
    assert!(!signals.ini_hashes.contains(&"44444444".to_string()));
}

#[test]
fn test_collect_deep_signals_full_budget_caps_total_bytes() {
    // Covers: TC-2.2-Task8-02
    let temp = tempfile::TempDir::new().expect("temp dir");
    let folder = temp.path().join("full_budget_mod");
    std::fs::create_dir_all(&folder).expect("create folder");

    let payload = "a".repeat(220 * 1024);
    for idx in 1..=6 {
        let hash = format!("{idx}{idx}{idx}{idx}{idx}{idx}{idx}{idx}");
        let filename = format!("{idx:02}.ini");
        let ini_text = format!(
            "[TextureOverrideSignal{idx}]\nhash = {hash}\nfilename = Characters/Entry{idx}/body_diffuse.dds\n;{payload}\n"
        );
        std::fs::write(folder.join(filename), ini_text).expect("write ini");
    }

    let content = walker::scan_folder_content(&folder, 3);
    let signals = collect_deep_signals(
        &folder,
        &content,
        MatchMode::FullScoring,
        &IniTokenizationConfig::default(),
    );

    assert_eq!(signals.scanned_ini_bytes, FULL_MAX_TOTAL_INI_BYTES);
    assert_eq!(signals.scanned_ini_files, 5);
    assert!(!signals.ini_hashes.contains(&"66666666".to_string()));
}
