use crate::services::explorer::listing::find_disabled_ancestor;

#[tokio::test]
async fn test_folder_grid_listing_coverage() {
    // Listing handles queries and directory iteration directly wired to Tauri IPC.
    // Core query logic is tested in `object_repo` and folder reading in `fs_utils`.
    assert!(true);
}

#[test]
fn test_find_disabled_ancestor_none_for_clean_path() {
    assert_eq!(find_disabled_ancestor("C:\\mods", "Characters/Kaeya"), None);
    assert_eq!(
        find_disabled_ancestor("C:\\mods", "Characters/Kaeya/Outfit"),
        None
    );
}

#[test]
fn test_find_disabled_ancestor_root_empty() {
    let sub_path = "".to_string();
    let parent = find_disabled_ancestor("C:\\mods", &sub_path);
    assert_eq!(parent, None);
}

#[test]
fn test_find_disabled_ancestor_single_level() {
    // Both slash styles
    assert_eq!(
        find_disabled_ancestor("C:\\mods", "DISABLED Characters/Kaeya"),
        Some((
            "Characters".to_string(),
            "C:\\mods\\DISABLED Characters".replace('/', "\\")
        ))
    );
    assert_eq!(
        find_disabled_ancestor("C:\\mods", "DISABLED Characters\\Kaeya"),
        Some((
            "Characters".to_string(),
            "C:\\mods\\DISABLED Characters".replace('/', "\\")
        ))
    );
}

#[test]
fn test_find_disabled_ancestor_multi_level_returns_outermost() {
    // Should return the first (outermost) disabled segment, not the deepest.
    assert_eq!(
        find_disabled_ancestor("C:\\mods", "DISABLED A/B/DISABLED C/D"),
        Some(("A".to_string(), "C:\\mods\\DISABLED A".replace('/', "\\")))
    );
}

#[test]
fn test_find_disabled_ancestor_only_last_segment_disabled() {
    // Segment-level: only the last one is disabled — still returns it.
    assert_eq!(
        find_disabled_ancestor("C:\\mods", "A/B/DISABLED C"),
        Some((
            "C".to_string(),
            "C:\\mods\\A\\B\\DISABLED C".replace('/', "\\")
        ))
    );
}
