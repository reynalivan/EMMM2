use super::*;
use crate::platform::fs::operation_lock::OperationLock;
use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn write_ini_benchmark_fixture(mod_dir: &std::path::Path, file_count: usize) {
    for index in 0..file_count {
        let variant_dir = mod_dir.join(format!("variant_{:03}", index % 25));
        fs::create_dir_all(&variant_dir).unwrap();
        fs::write(
            variant_dir.join(format!("mod_{index:04}.ini")),
            "[Constants]\n$swapvar = 0\n[KeyToggle]\nkey = F1\n",
        )
        .unwrap();
    }
}

fn median_duration(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

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

    let source_hash = read_doc.source_hash.clone();
    write_mod_ini_inner(
        &mod_dir,
        "config.ini",
        &source_hash,
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

    let err = read_mod_ini_inner(&mod_dir, "..\\outside.ini")
        .unwrap_err()
        .to_string();
    assert!(err.to_string().contains("Invalid INI filename path"));

    let err2 = read_mod_ini_inner(&mod_dir, "desktop.ini").unwrap_err();
    assert!(err2.to_string().contains("desktop.ini"));

    let err3 = read_mod_ini_inner(&mod_dir, "notes.txt").unwrap_err();
    assert!(err3.to_string().contains("Only .ini files"));
}

// Covers: EC-6.06 (Operation lock for INI writes). Writing without the lock
// is a compile error now — `write_mod_ini_locked_inner` takes `&OpGuard` —
// so the contention path lives in `operation_lock_tests` and this covers the
// happy path under the guard.
#[tokio::test]
async fn details_command_write_requires_held_guard() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();
    fs::write(mod_dir.join("config.ini"), "[Constants]\n$swapvar = 0\n").unwrap();

    let op_lock = OperationLock::new();
    let op_guard = op_lock.acquire().await.unwrap();
    let source_hash = read_mod_ini_inner(&mod_dir, "config.ini")
        .unwrap()
        .source_hash;

    write_mod_ini_locked_inner(
        &op_guard,
        &mod_dir,
        "config.ini",
        &source_hash,
        vec![IniLineUpdate {
            line_idx: 1,
            content: "$swapvar = 1".to_string(),
        }],
    )
    .await
    .unwrap();

    let written = fs::read_to_string(mod_dir.join("config.ini")).unwrap();
    assert!(written.contains("$swapvar = 1"));
}

#[test]
fn details_command_reads_and_writes_nested_ini() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir_all(mod_dir.join("variants/red")).unwrap();
    fs::write(
        mod_dir.join("variants/red/config.ini"),
        "[Constants]\n$value = 0\n",
    )
    .unwrap();

    let listed = list_mod_ini_files_inner(&mod_dir).unwrap();
    assert_eq!(listed[0].filename, "variants/red/config.ini");
    let document = read_mod_ini_inner(&mod_dir, &listed[0].filename).unwrap();
    write_mod_ini_inner(
        &mod_dir,
        &listed[0].filename,
        &document.source_hash,
        vec![IniLineUpdate {
            line_idx: 1,
            content: "$value = 1".to_string(),
        }],
    )
    .unwrap();

    let written = fs::read_to_string(mod_dir.join("variants/red/config.ini")).unwrap();
    assert!(written.contains("$value = 1"));
}

#[test]
fn details_command_reads_all_ini_documents_with_relative_filenames() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir_all(mod_dir.join("variants/red")).unwrap();
    fs::write(mod_dir.join("config.ini"), "[Constants]\n$value = 0\n").unwrap();
    fs::write(
        mod_dir.join("variants/red/config.ini"),
        "[KeyToggle]\nkey = F1\n",
    )
    .unwrap();

    let documents = read_mod_ini_documents_inner(&mod_dir).unwrap();

    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].filename, "config.ini");
    assert_eq!(documents[1].filename, "variants/red/config.ini");
    assert_eq!(documents[1].document.key_bindings.len(), 1);
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

// Covers: TC-19-010 (Scan depth to 3 subfolders)
#[test]
fn details_command_list_preview_images_scan_depth() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("ModA");
    fs::create_dir(&mod_dir).unwrap();

    // Depth 0 (Root)
    fs::write(mod_dir.join("root.png"), "img").unwrap();

    // Depth 1
    let d1 = mod_dir.join("variants");
    fs::create_dir(&d1).unwrap();
    fs::write(d1.join("v1.jpg"), "img").unwrap();

    // Depth 2
    let d2 = d1.join("hair_color");
    fs::create_dir(&d2).unwrap();
    fs::write(d2.join("v2.webp"), "img").unwrap();

    // Depth 3 (WalkDir depth 3)
    fs::write(d2.join("v3.png"), "img").unwrap();

    // Depth 4 (WalkDir depth 4 - ignored)
    let d3 = d2.join("blonde");
    fs::create_dir(&d3).unwrap();
    fs::write(d3.join("v4.png"), "img").unwrap();

    let list = list_mod_preview_images_inner(&mod_dir).unwrap();

    let paths: Vec<String> = list;

    let has_d3_image = paths.iter().any(|p| p.ends_with("v3.png"));
    assert!(has_d3_image, "Did not find image at nesting depth 3");

    let has_d4_image = paths.iter().any(|p| p.ends_with("v4.png"));
    assert!(
        !has_d4_image,
        "Found image at nesting depth 4, which should be ignored"
    );
}

/// Manual baseline for preview detail loading. Run with:
/// `cargo test --lib preview_ini_discovery_and_parse_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "manual performance baseline"]
fn preview_ini_discovery_and_parse_benchmark() {
    for file_count in [100, 500] {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("ModA");
        fs::create_dir(&mod_dir).unwrap();
        write_ini_benchmark_fixture(&mod_dir, file_count);

        let mut discovery_samples = Vec::with_capacity(5);
        let mut parse_samples = Vec::with_capacity(5);
        let mut batch_samples = Vec::with_capacity(5);
        for _ in 0..5 {
            let discovery_start = Instant::now();
            let files = list_mod_ini_files_inner(&mod_dir).unwrap();
            discovery_samples.push(discovery_start.elapsed());
            assert_eq!(files.len(), file_count);

            let parse_start = Instant::now();
            for file in &files {
                read_mod_ini_inner(&mod_dir, &file.filename).unwrap();
            }
            parse_samples.push(parse_start.elapsed());

            let batch_start = Instant::now();
            let documents = read_mod_ini_documents_inner(&mod_dir).unwrap();
            batch_samples.push(batch_start.elapsed());
            assert_eq!(documents.len(), file_count);
        }

        println!(
            "{file_count} INI files: discovery median {:?}, serial parse median {:?}, batch median {:?}",
            median_duration(discovery_samples),
            median_duration(parse_samples),
            median_duration(batch_samples),
        );
    }
}
