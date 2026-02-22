use super::*;
use std::fs;
use tempfile::TempDir;

// Covers: Task5 command bridge smoke
#[test]
fn details_command_bridge_smoke() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    fs::write(mod_dir.join("desktop.ini"), "[.ShellClassInfo]").unwrap();
    fs::write(mod_dir.join("config.ini"), "[Constants]\n$swapvar = 0\n").unwrap();
    fs::write(mod_dir.join("preview_custom.png"), "img").unwrap();

    let list = list_mod_ini_files_inner(&mod_dir).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].filename, "config.ini");

    let read_doc = read_mod_ini_inner(&mod_dir, "config.ini").unwrap();
    assert_eq!(read_doc.variables.len(), 1);

    write_mod_ini_inner(
        &mod_dir,
        "config.ini",
        vec![IniLineUpdate {
            line_idx: 1,
            content: "$swapvar = 1".to_string(),
        }],
    )
    .unwrap();

    let updated = fs::read_to_string(mod_dir.join("config.ini")).unwrap();
    assert!(updated.contains("$swapvar = 1"));
    assert!(mod_dir.join("config.ini.bak").exists());

    let images = list_mod_preview_images_inner(&mod_dir).unwrap();
    assert_eq!(images.len(), 1);
    assert!(images[0].ends_with("preview_custom.png"));

    let saved = save_mod_preview_image_inner(&mod_dir, "Keqing", b"not-an-image");
    assert!(saved.is_err());
}

// Covers: Task5 command invalid path rejection
#[test]
fn details_command_rejects_path_escape() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    let outside = tmp.path().join("outside.ini");
    fs::write(&outside, "[Constants]\n$x = 1\n").unwrap();

    let err = read_mod_ini_inner(&mod_dir, "..\\outside.ini").unwrap_err();
    assert!(err.contains("Invalid INI filename path"));

    let err2 = read_mod_ini_inner(&mod_dir, "desktop.ini").unwrap_err();
    assert!(err2.contains("desktop.ini"));

    let err3 = read_mod_ini_inner(&mod_dir, "notes.txt").unwrap_err();
    assert!(err3.contains("Only .ini files"));
}

// Covers: EC-6.06 (Operation lock for INI writes)
#[tokio::test]
async fn details_command_write_respects_operation_lock() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("config.ini"), "[Constants]\n$swapvar = 0\n").unwrap();

    let op_lock = OperationLock::new();
    let _guard = op_lock.acquire().await.unwrap();

    let result = write_mod_ini_locked_inner(
        &op_lock,
        &mod_dir,
        "config.ini",
        vec![IniLineUpdate {
            line_idx: 1,
            content: "$swapvar = 1".to_string(),
        }],
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Operation in progress"));
}

#[test]
fn details_command_remove_and_clear_preview_images() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("preview_custom.png"), "img").unwrap();
    fs::write(mod_dir.join("preview_custom_1.png"), "img").unwrap();

    let target = mod_dir.join("preview_custom.png");
    remove_mod_preview_image_inner(&mod_dir, &target.to_string_lossy()).unwrap();
    assert!(!target.exists());

    let cleared = clear_mod_preview_images_inner(&mod_dir).unwrap();
    assert_eq!(cleared.len(), 1);
    assert!(!mod_dir.join("preview_custom_1.png").exists());
}
