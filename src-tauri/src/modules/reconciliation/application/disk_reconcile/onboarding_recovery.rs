//! Durable, minimal recovery queue for onboarding games that were not yet
//! indexed when the application last exited.

use std::collections::BTreeSet;

use sqlx::SqlitePool;

use crate::shared::errors::AppError;

const PENDING_GAME_IDS_KEY: &str = "onboarding_pending_index_game_ids";

pub async fn load_pending_game_ids(pool: &SqlitePool) -> Result<Vec<String>, AppError> {
    let Some(serialized) =
        crate::modules::system::adapters::sqlite::settings::get_setting(pool, PENDING_GAME_IDS_KEY)
            .await?
    else {
        return Ok(Vec::new());
    };
    let game_ids = serde_json::from_str::<Vec<String>>(&serialized)?;
    validate_game_ids(&game_ids)?;
    Ok(game_ids)
}

pub async fn replace_pending_game_ids(
    pool: &SqlitePool,
    game_ids: &[String],
) -> Result<(), AppError> {
    validate_game_ids(game_ids)?;
    if game_ids.is_empty() {
        crate::modules::system::adapters::sqlite::settings::delete_setting(
            pool,
            PENDING_GAME_IDS_KEY,
        )
        .await?;
        return Ok(());
    }
    let serialized = serde_json::to_string(game_ids)?;
    crate::modules::system::adapters::sqlite::settings::set_setting(
        pool,
        PENDING_GAME_IDS_KEY,
        &serialized,
    )
    .await?;
    Ok(())
}

pub async fn remove_pending_game_id(pool: &SqlitePool, game_id: &str) -> Result<(), AppError> {
    let mut pending_game_ids = load_pending_game_ids(pool).await?;
    pending_game_ids.retain(|pending_game_id| pending_game_id != game_id);
    replace_pending_game_ids(pool, &pending_game_ids).await
}

fn validate_game_ids(game_ids: &[String]) -> Result<(), AppError> {
    let unique_ids = game_ids.iter().collect::<BTreeSet<_>>();
    if unique_ids.len() != game_ids.len() || game_ids.iter().any(|game_id| game_id.is_empty()) {
        return Err(AppError::Validation(
            "Pending onboarding game IDs must be unique and non-empty".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        load_pending_game_ids, remove_pending_game_id, replace_pending_game_ids, validate_game_ids,
    };

    #[test]
    fn pending_game_ids_must_be_unique_and_non_empty() {
        assert!(validate_game_ids(&["first".to_string(), "second".to_string()]).is_ok());
        assert!(validate_game_ids(&["first".to_string(), "first".to_string()]).is_err());
        assert!(validate_game_ids(&[String::new()]).is_err());
    }

    #[tokio::test]
    async fn pending_game_ids_survive_restart_storage_and_remove_only_completed_game() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(temp.path().join("app.db"))
                    .create_if_missing(true),
            )
            .await
            .expect("test database should be created");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("test database should be migrated");

        let pending = vec!["first".to_string(), "later".to_string()];
        replace_pending_game_ids(&pool, &pending)
            .await
            .expect("pending games should persist");
        assert_eq!(
            load_pending_game_ids(&pool)
                .await
                .expect("pending games should reload"),
            pending
        );

        remove_pending_game_id(&pool, "first")
            .await
            .expect("completed game should be removed");
        assert_eq!(
            load_pending_game_ids(&pool)
                .await
                .expect("remaining game should reload"),
            vec!["later".to_string()]
        );
    }
}
