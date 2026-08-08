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
    let tmp_path = path.with_extension("tmp");
    assert!(!tmp_path.exists());
}
