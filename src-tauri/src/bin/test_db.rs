use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = "sqlite://../db/emmm.db".to_string();

    let pool = SqlitePoolOptions::new().connect(&db_url).await?;

    let rows = sqlx::query("SELECT id, name, folder_path, object_type, name_key, folder_path_key FROM objects WHERE folder_path LIKE '%amber%' OR name LIKE '%amber%'")
        .fetch_all(&pool)
        .await?;

    println!("Found {} objects.", rows.len());
    for row in rows {
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let folder_path: String = row.try_get("folder_path")?;
        let object_type: String = row.try_get("object_type")?;
        let name_key: String = row.try_get("name_key")?;
        let folder_path_key: String = row.try_get("folder_path_key")?;

        println!("ID: {}", id);
        println!("Name: {}", name);
        println!("Folder Path: {}", folder_path);
        println!("Object Type: {}", object_type);
        println!("Name Key: {}", name_key);
        println!("Folder Path Key: {}", folder_path_key);
        println!("---------------------------------");
    }

    Ok(())
}
