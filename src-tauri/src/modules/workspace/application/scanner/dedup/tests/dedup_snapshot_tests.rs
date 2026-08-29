use super::read_ini_signals;
use std::fs;
use tempfile::TempDir;

#[test]
fn ini_signals_scan_the_complete_document_without_comment_headers() {
    let temp = TempDir::new().unwrap();
    let ini_path = temp.path().join("merged.ini");
    let mut content = (0..250)
        .map(|index| format!("; generated source {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    content.push_str("\n[TextureOverrideBody]\nhash = 12345678\n");
    fs::write(&ini_path, content).unwrap();

    let (headers, _bindings, hashes) = read_ini_signals(&ini_path);

    assert!(headers.contains("[textureoverridebody]"));
    assert!(hashes.contains("texture:12345678"));
    assert!(headers.iter().all(|header| !header.starts_with(';')));
}

#[test]
fn ini_signals_reject_invalid_or_untyped_hash_values() {
    let temp = TempDir::new().unwrap();
    let ini_path = temp.path().join("typed.ini");
    fs::write(
        &ini_path,
        "[ResourceData]\nhash = 12345678\n\
         [TextureOverrideBad]\nhash = not-a-hash\n\
         [ShaderOverrideGood]\nhash = 0123456789abcdef\n",
    )
    .unwrap();

    let (_, _, hashes) = read_ini_signals(&ini_path);

    assert_eq!(
        hashes,
        std::collections::BTreeSet::from(["shader:0123456789abcdef".to_string()])
    );
}
