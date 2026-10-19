use super::refresh_catalog_metadata;
use crate::modules::workspace::application::scanner::master_db::{
    asset_pack::CatalogPack,
    test_support::{catalog_pool, write_catalog},
};

#[tokio::test]
async fn invalid_pack_does_not_clear_thumbnail_references() {
    let root = tempfile::tempdir().unwrap();
    let pool = catalog_pool().await;
    write_catalog(&CatalogPack::root(root.path()), "fixture");
    std::fs::write(
        CatalogPack::root(root.path()).join("catalog/gimi.json"),
        b"corrupted",
    )
    .unwrap();
    assert!(refresh_catalog_metadata(root.path(), &pool).await.is_err());
    let paths: Vec<String> = sqlx::query_scalar("SELECT thumbnail_path FROM objects ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(paths, ["old-catalog.png", "user.png"]);
}

#[tokio::test]
async fn valid_refresh_clears_only_catalog_owned_references() {
    let root = tempfile::tempdir().unwrap();
    let pool = catalog_pool().await;
    write_catalog(&CatalogPack::root(root.path()), "fixture");
    assert_eq!(
        refresh_catalog_metadata(root.path(), &pool)
            .await
            .unwrap()
            .entries,
        1
    );
    let paths: Vec<Option<String>> =
        sqlx::query_scalar("SELECT thumbnail_path FROM objects ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(paths, [None, Some("user.png".to_string())]);
}
