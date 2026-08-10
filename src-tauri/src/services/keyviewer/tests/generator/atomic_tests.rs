use super::*;

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
    let leftovers = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .count();
    assert_eq!(leftovers, 0);
}

#[test]
fn replace_directory_swaps_complete_artifact_set() {
    let dir = TempDir::new().unwrap();
    let active = dir.path().join("active");
    std::fs::create_dir(&active).unwrap();
    std::fs::write(active.join("old.txt"), "old").unwrap();

    let staging = create_staging_directory(&active).unwrap();
    std::fs::write(staging.join("new.txt"), "new").unwrap();
    replace_directory(&staging, &active).unwrap();

    assert!(!active.join("old.txt").exists());
    assert_eq!(
        std::fs::read_to_string(active.join("new.txt")).unwrap(),
        "new"
    );
    assert!(!staging.exists());
}
