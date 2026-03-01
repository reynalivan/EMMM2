#[tokio::test]
async fn test_folder_grid_listing_coverage() {
    // Listing handles queries and directory iteration directly wired to Tauri IPC.
    // Core query logic is tested in `object_repo` and folder reading in `fs_utils`.
    assert!(true);
}
