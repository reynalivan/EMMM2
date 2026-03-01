use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_find_unique_name() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path();

    // 1st duplication
    let name1 = find_unique_name(parent, "TestMod", false);
    assert_eq!(name1, "TestMod (dup)");

    // Simulate creation of first dup
    fs::create_dir(parent.join(&name1)).unwrap();

    // 2nd duplication
    let name2 = find_unique_name(parent, "TestMod", false);
    assert_eq!(name2, "TestMod (dup 2)");

    // Disabled duplication
    let name_disabled = find_unique_name(parent, "TestMod", true);
    assert_eq!(
        name_disabled,
        format!("{}TestMod (dup)", crate::DISABLED_PREFIX)
    );
}

#[test]
fn test_resolve_conflict_keep_enabled() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path();
    let watcher = WatcherState::new();

    let enabled_mod = parent.join("ConflictMod");
    let disabled_mod = parent.join(format!("{}ConflictMod", crate::DISABLED_PREFIX));

    fs::create_dir(&enabled_mod).unwrap();
    fs::create_dir(&disabled_mod).unwrap();

    let res = resolve_conflict_inner(
        &watcher,
        &enabled_mod.to_string_lossy(),
        &disabled_mod.to_string_lossy(),
        "keep_enabled",
    )
    .unwrap();

    // Strategy was keep_enabled, so disabled_mod was renamed to DISABLED ConflictMod (dup)
    assert_eq!(
        res,
        parent
            .join(format!("{}ConflictMod (dup)", crate::DISABLED_PREFIX))
            .to_string_lossy()
    );

    // Verify rename happened
    assert!(!disabled_mod.exists());
    assert!(Path::new(&res).exists());
    assert!(enabled_mod.exists()); // keep didn't move
}

#[test]
fn test_resolve_conflict_separate() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path();
    let watcher = WatcherState::new();

    let enabled_mod = parent.join("ConflictMod");
    let disabled_mod = parent.join(format!("{}ConflictMod", crate::DISABLED_PREFIX));

    fs::create_dir(&enabled_mod).unwrap();
    fs::create_dir(&disabled_mod).unwrap();

    let res = resolve_conflict_inner(
        &watcher,
        &enabled_mod.to_string_lossy(),
        &disabled_mod.to_string_lossy(),
        "separate",
    )
    .unwrap();

    // Strategy was separate, so disabled duplicate was renamed to base (copy)
    assert_eq!(
        res,
        parent
            .join(format!(
                "{}ConflictMod (copy) (dup)",
                crate::DISABLED_PREFIX
            ))
            .to_string_lossy()
    );

    assert!(!disabled_mod.exists());
    assert!(Path::new(&res).exists());
    assert!(enabled_mod.exists());
}
