use super::*;
use crate::domain::models::{GameType, ItemStatus};
use crate::repo::game_repo::{upsert_game, GameRow};

#[test]
fn effective_enabled_path_checks_every_runtime_component() {
    assert!(is_effectively_enabled_path("Amber/Blue Dress"));
    assert!(!is_effectively_enabled_path("DISABLED Amber/Blue Dress"));
    assert!(!is_effectively_enabled_path("Amber/DISABLED_Blue Dress"));
    assert!(!is_effectively_enabled_path("Amber/DISABLEDBlueDress"));
    assert!(!is_effectively_enabled_path("amber/disabled-blue"));
}

#[tokio::test]
async fn enabled_queries_exclude_rows_under_disabled_ancestors() {
    let context = crate::test_utils::init_test_db().await;
    let pool = context.pool;
    upsert_game(
        &pool,
        &GameRow {
            id: "g_effective".into(),
            name: "Effective".into(),
            game_type: GameType::GIMI,
            path: "C:/Game".into(),
            mods_path: Some("C:/Package/Mods".into()),
            game_exe: Some("C:/Game/game.exe".into()),
            launcher_path: None,
            loader_exe: None,
            launch_args: None,
        },
    )
    .await
    .unwrap();

    for (id, name, path, status) in [
        ("active", "Active", "Amber/BlueDress", ItemStatus::Enabled),
        (
            "ancestor_disabled",
            "Hidden",
            "DISABLEDAmber/RedDress",
            ItemStatus::Enabled,
        ),
        (
            "row_disabled",
            "Row Disabled",
            "Amber/GreenDress",
            ItemStatus::Disabled,
        ),
    ] {
        crate::test_utils::insert_test_mod(
            &pool,
            &crate::test_utils::TestModFixture {
                id,
                game_id: "g_effective",
                object_id: None,
                actual_name: name,
                folder_path: path,
                status,
                is_safe: true,
                object_type: None,
                mods_path: Some("C:/Package/Mods"),
            },
        )
        .await
        .unwrap();
    }

    let paths = get_enabled_mods_paths(&pool, "g_effective").await.unwrap();
    assert_eq!(
        paths
            .into_iter()
            .map(ModFolderPath::into_stored)
            .collect::<Vec<_>>(),
        vec!["Amber/BlueDress"]
    );

    let names = get_enabled_mods_names_and_paths(&pool, "g_effective")
        .await
        .unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].0, "Active");
}
