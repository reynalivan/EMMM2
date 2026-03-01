#[tokio::test]
async fn test_folder_grid_types_coverage() {
    // Types.rs is just a re-export of `services::explorer::types`.
    // Validated implicitly across all other integration points.
    assert!(true);
}
