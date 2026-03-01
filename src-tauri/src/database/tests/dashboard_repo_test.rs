use crate::database::dashboard_repo;
use sqlx::SqlitePool;

/// Create a fully-schemed in-memory pool for dashboard tests.
async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

async fn seed_game(pool: &SqlitePool, id: &str, name: &str) {
    sqlx::query("INSERT INTO games (id, name, game_type, path) VALUES (?, ?, 'GIMI', ?)")
        .bind(id)
        .bind(name)
        .bind(format!("/dummy/{}", id))
        .execute(pool)
        .await
        .unwrap();
}

async fn seed_mod(
    pool: &SqlitePool,
    id: &str,
    game_id: &str,
    name: &str,
    status: &str,
    is_safe: bool,
    size_bytes: i64,
    object_type: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO mods (id, game_id, actual_name, folder_path, status, is_safe, size_bytes, object_type, indexed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(id)
    .bind(game_id)
    .bind(name)
    .bind(format!("/dummy/mod/{}", id))
    .bind(status)
    .bind(is_safe)
    .bind(size_bytes)
    .bind(object_type)
    .execute(pool)
    .await
    .unwrap();
}

// ── TC-13.1-01: Stats Accuracy ──────────────────────────────────────────

#[tokio::test]
async fn test_stats_accuracy() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;
    seed_game(&pool, "g2", "StarRail").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "Mod1",
        "ENABLED",
        true,
        1000,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m2",
        "g1",
        "Mod2",
        "DISABLED",
        true,
        2000,
        Some("Weapon"),
    )
    .await;
    seed_mod(
        &pool,
        "m3",
        "g1",
        "Mod3",
        "ENABLED",
        false,
        500,
        Some("Character"),
    )
    .await;
    seed_mod(&pool, "m4", "g2", "Mod4", "ENABLED", true, 3000, Some("UI")).await;
    seed_mod(&pool, "m5", "g2", "Mod5", "DISABLED", true, 1500, None).await;

    let stats = dashboard_repo::fetch_global_stats(&pool, false)
        .await
        .unwrap();
    assert_eq!(stats.total_mods, 5);
    assert_eq!(stats.enabled_mods, 3);
    assert_eq!(stats.disabled_mods, 2);
    assert_eq!(stats.total_size_bytes, 8000);
    assert_eq!(stats.total_games, 2);
}

// ── TC-13.4-01: Safe Mode Filter ────────────────────────────────────────

#[tokio::test]
async fn test_safe_mode_filter() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "SafeMod",
        "ENABLED",
        true,
        1000,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m2",
        "g1",
        "UnsafeMod",
        "ENABLED",
        false,
        2000,
        Some("Character"),
    )
    .await;

    let stats = dashboard_repo::fetch_global_stats(&pool, true)
        .await
        .unwrap();
    assert_eq!(
        stats.total_mods, 1,
        "Safe mode should filter out unsafe mods"
    );
    assert_eq!(stats.enabled_mods, 1);
    assert_eq!(stats.total_size_bytes, 1000);
}

// ── NC-13.1-02: Zero Data (Empty DB) ────────────────────────────────────

#[tokio::test]
async fn test_zero_data_empty_db() {
    let pool = setup_pool().await;

    let stats = dashboard_repo::fetch_global_stats(&pool, false)
        .await
        .unwrap();
    assert_eq!(stats.total_mods, 0);
    assert_eq!(stats.enabled_mods, 0);
    assert_eq!(stats.disabled_mods, 0);
    assert_eq!(stats.total_size_bytes, 0);
    assert_eq!(stats.total_games, 0);
    assert_eq!(stats.total_collections, 0);
}

// ── Category Distribution ───────────────────────────────────────────────

#[tokio::test]
async fn test_category_distribution() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "Mod1",
        "ENABLED",
        true,
        100,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m2",
        "g1",
        "Mod2",
        "ENABLED",
        true,
        100,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m3",
        "g1",
        "Mod3",
        "ENABLED",
        true,
        100,
        Some("Weapon"),
    )
    .await;
    seed_mod(&pool, "m4", "g1", "Mod4", "ENABLED", true, 100, None).await;

    let dist = dashboard_repo::fetch_category_distribution(&pool, false)
        .await
        .unwrap();
    assert_eq!(
        dist.len(),
        3,
        "Should have Character, Weapon, Uncategorized"
    );

    let char_count = dist.iter().find(|d| d.category == "Character").unwrap();
    assert_eq!(char_count.count, 2);
}

// ── Game Distribution ───────────────────────────────────────────────────

#[tokio::test]
async fn test_game_distribution() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;
    seed_game(&pool, "g2", "StarRail").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "Mod1",
        "ENABLED",
        true,
        100,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m2",
        "g1",
        "Mod2",
        "ENABLED",
        true,
        100,
        Some("Character"),
    )
    .await;
    seed_mod(
        &pool,
        "m3",
        "g2",
        "Mod3",
        "ENABLED",
        true,
        100,
        Some("Weapon"),
    )
    .await;

    let dist = dashboard_repo::fetch_game_distribution(&pool, false)
        .await
        .unwrap();
    assert_eq!(dist.len(), 2);

    let genshin = dist.iter().find(|d| d.game_name == "Genshin").unwrap();
    assert_eq!(genshin.count, 2);
}

// ── EC-13.01: Negative size_bytes clamped ───────────────────────────────

#[tokio::test]
async fn test_negative_size_clamped() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(
        &pool,
        "m1",
        "g1",
        "BadMod",
        "ENABLED",
        true,
        -500,
        Some("Character"),
    )
    .await;

    let stats = dashboard_repo::fetch_global_stats(&pool, false)
        .await
        .unwrap();
    assert_eq!(
        stats.total_size_bytes, 0,
        "Negative size should be clamped to 0"
    );
}

// ── Recent Mods (LIMIT) ─────────────────────────────────────────────────

#[tokio::test]
async fn test_recent_mods_limit() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    for i in 0..10 {
        let id = format!("m{i}");
        let name = format!("Mod{i}");
        seed_mod(
            &pool,
            &id,
            "g1",
            &name,
            "ENABLED",
            true,
            100,
            Some("Character"),
        )
        .await;
    }

    let recents = dashboard_repo::fetch_recent_mods(&pool, false, 5)
        .await
        .unwrap();
    assert_eq!(recents.len(), 5, "Should return at most 5 recent mods");
}
