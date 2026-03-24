use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let tests = vec!["", " ", "\n", "null", "\"\"", "{}", "[]"];
    for t in tests {
        let (res,): (i64,) = sqlx::query_as("SELECT json_valid(?)")
            .bind(t)
            .fetch_one(&pool)
            .await
            .unwrap();
        println!("json_valid({:?}) = {}", t, res);
    }
}
