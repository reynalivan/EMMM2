use crate::modules::dashboard::adapters::sqlite::dashboard;
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

/// Create a fully-schemed in-memory pool for dashboard tests.
async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

async fn seed_game(pool: &SqlitePool, id: &str, name: &str) {
    crate::test_utils::insert_test_game(
        pool,
        &crate::test_utils::TestGameFixture {
            id,
            name,
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: &format!("/dummy/{}", id),
            mods_path: Some(&format!("/dummy/mods/{}", id)),
        },
    )
    .await
    .unwrap();

    // Create a dummy object for this game so mods can link to it (NOT NULL constraint)
    crate::test_utils::insert_test_object(
        pool,
        &crate::test_utils::TestObjectFixture {
            id: &format!("obj_{}", id),
            game_id: id,
            name: "Default Object",
            folder_path: &format!("/dummy/obj/{}", id),
            object_type: "Other",
        },
    )
    .await
    .unwrap();
}

struct SeedMod<'a> {
    id: &'a str,
    game_id: &'a str,
    name: &'a str,
    status: &'a str,
    is_safe: bool,
    size_bytes: i64,
    object_type: Option<&'a str>,
}

/// The unremarkable mod. Each test then spells out only what it actually
/// varies, so the assertion's inputs are visible instead of buried in seven
/// fields of boilerplate repeated per seed.
impl Default for SeedMod<'_> {
    fn default() -> Self {
        Self {
            id: "m1",
            game_id: "g1",
            name: "Mod1",
            status: "ENABLED",
            is_safe: true,
            size_bytes: 100,
            object_type: Some("Character"),
        }
    }
}

async fn seed_mod(pool: &SqlitePool, seed: SeedMod<'_>) {
    let SeedMod {
        id,
        game_id,
        name,
        status,
        is_safe,
        size_bytes,
        object_type,
    } = seed;
    crate::test_utils::insert_test_mod(
        pool,
        &crate::test_utils::TestModFixture {
            id,
            game_id,
            object_id: Some(&format!("obj_{}", game_id)),
            actual_name: name,
            folder_path: &format!("/dummy/mod/{}", id),
            status: crate::modules::games::domain::models::ItemStatus::from_str(status).unwrap(),
            is_safe,
            object_type,
            mods_path: Some(&format!("/dummy/mods/{}", game_id)),
        },
    )
    .await
    .unwrap();

    // Update size_bytes since insert_test_mod might default to 0
    sqlx::query("UPDATE mods SET size_bytes = ? WHERE id = ?")
        .bind(size_bytes)
        .bind(id)
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
        SeedMod {
            id: "m1",
            game_id: "g1",
            name: "Mod1",
            size_bytes: 1000,
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m2",
            game_id: "g1",
            name: "Mod2",
            status: "DISABLED",
            size_bytes: 2000,
            object_type: Some("Weapon"),
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m3",
            game_id: "g1",
            name: "Mod3",
            is_safe: false,
            size_bytes: 500,
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m4",
            game_id: "g2",
            name: "Mod4",
            size_bytes: 3000,
            object_type: Some("UI"),
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m5",
            game_id: "g2",
            name: "Mod5",
            status: "DISABLED",
            size_bytes: 1500,
            object_type: None,
            ..Default::default()
        },
    )
    .await;

    let stats = dashboard::fetch_global_stats(&pool).await.unwrap();
    assert_eq!(stats.total_mods, 5);
    assert_eq!(stats.enabled_mods, 3);
    assert_eq!(stats.disabled_mods, 2);
    assert_eq!(stats.total_games, 2);
}

// ── TC-13.4-01: Safety classification filter ────────────────────────────

#[tokio::test]
async fn test_dashboard_includes_all_safety_classifications() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;

    seed_mod(
        &pool,
        SeedMod {
            id: "m1",
            game_id: "g1",
            name: "SafeMod",
            size_bytes: 1000,
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m2",
            game_id: "g1",
            name: "UnsafeMod",
            is_safe: false,
            size_bytes: 2000,
            ..Default::default()
        },
    )
    .await;

    let stats = dashboard::fetch_global_stats(&pool).await.unwrap();
    assert_eq!(stats.total_mods, 2);
    assert_eq!(stats.enabled_mods, 2);
}

// ── NC-13.1-02: Zero Data (Empty DB) ────────────────────────────────────

#[tokio::test]
async fn test_zero_data_empty_db() {
    let pool = setup_pool().await;

    let stats = dashboard::fetch_global_stats(&pool).await.unwrap();
    assert_eq!(stats.total_mods, 0);
    assert_eq!(stats.enabled_mods, 0);
    assert_eq!(stats.disabled_mods, 0);
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
        SeedMod {
            id: "m1",
            game_id: "g1",
            name: "Mod1",
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m2",
            game_id: "g1",
            name: "Mod2",
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m3",
            game_id: "g1",
            name: "Mod3",
            object_type: Some("Weapon"),
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m4",
            game_id: "g1",
            name: "Mod4",
            object_type: None,
            ..Default::default()
        },
    )
    .await;

    let dist = dashboard::fetch_category_distribution(&pool).await.unwrap();
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
        SeedMod {
            id: "m1",
            game_id: "g1",
            name: "Mod1",
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m2",
            game_id: "g1",
            name: "Mod2",
            ..Default::default()
        },
    )
    .await;
    seed_mod(
        &pool,
        SeedMod {
            id: "m3",
            game_id: "g2",
            name: "Mod3",
            object_type: Some("Weapon"),
            ..Default::default()
        },
    )
    .await;

    let dist = dashboard::fetch_game_distribution(&pool).await.unwrap();
    assert_eq!(dist.len(), 2);

    let genshin = dist.iter().find(|d| d.game_name == "Genshin").unwrap();
    assert_eq!(genshin.count, 2);
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
            SeedMod {
                id: &id,
                game_id: "g1",
                name: &name,
                ..Default::default()
            },
        )
        .await;
    }

    let recents = dashboard::fetch_recent_mods(&pool, 5).await.unwrap();
    assert_eq!(recents.len(), 5, "Should return at most 5 recent mods");
    assert!(recents.iter().all(|recent| recent.game_id == "g1"));
    assert!(recents
        .iter()
        .all(|recent| recent.folder_path.starts_with("/dummy/mod/")));
}

#[tokio::test]
async fn test_recent_mods_resolve_relative_folder_paths() {
    let pool = setup_pool().await;
    seed_game(&pool, "g1", "Genshin").await;
    seed_mod(&pool, SeedMod::default()).await;
    sqlx::query("UPDATE mods SET folder_path = ? WHERE id = ?")
        .bind("Character/Mod1")
        .bind("m1")
        .execute(&pool)
        .await
        .unwrap();

    let recents = dashboard::fetch_recent_mods(&pool, 5).await.unwrap();

    assert_eq!(
        recents[0].folder_path,
        Path::new("/dummy/mods/g1")
            .join("Character")
            .join("Mod1")
            .to_string_lossy()
    );
}
