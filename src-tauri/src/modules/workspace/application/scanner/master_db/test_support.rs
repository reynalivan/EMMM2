use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Write},
    path::Path,
};

pub(crate) fn catalog_archive(id: &str) -> Vec<u8> {
    let catalog = serde_json::to_vec(&serde_json::json!({
        "entries": [{ "name": "Amber", "object_type": "Character" }]
    }))
    .unwrap();
    let manifest = serde_json::to_vec(&serde_json::json!({
        "id": id, "version": "1.0.0", "author": "Test", "source": "https://example.invalid/catalog",
        "catalogs": { "gimi": { "path": "catalog/gimi.json", "sha256": format!("{:x}", Sha256::digest(&catalog)) } }
    })).unwrap();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [("manifest.json", manifest), ("catalog/gimi.json", catalog)] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

pub(crate) fn write_catalog(root: &Path, id: &str) {
    let archive = catalog_archive(id);
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).unwrap();
    zip.extract(root).unwrap();
}

pub(crate) async fn catalog_pool() -> sqlx::SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE TABLE catalog_pack_provenance (
        singleton INTEGER PRIMARY KEY, source_kind TEXT, source_url TEXT, repository TEXT,
        release_tag TEXT, asset_name TEXT, asset_digest TEXT, installed_at TEXT);
        CREATE TABLE objects (id TEXT PRIMARY KEY, thumbnail_path TEXT, thumbnail_source TEXT);
        INSERT INTO objects VALUES ('catalog', 'old-catalog.png', 'asset_pack'),
            ('user', 'user.png', 'user');",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}
