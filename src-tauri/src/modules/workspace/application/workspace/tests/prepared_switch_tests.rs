use super::*;
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::workspace::domain::workspace::{
    WorkspaceSwitchOriginSurface, WorkspaceSwitchTarget,
};
use crate::test_utils::{
    insert_test_game, insert_test_mod, insert_test_object, TestGameFixture, TestModFixture,
    TestObjectFixture,
};

#[tokio::test]
async fn duplicate_resolutions_have_distinct_scoped_rename_plans() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let target = root.join("Alice/DISABLED Blue");
    let sibling = root.join("Alice/Red");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_str().unwrap(),
            mods_path: root.to_str(),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "alice",
            game_id: "game",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    for (id, path, status) in [
        ("blue", "Alice/DISABLED Blue", ItemStatus::Disabled),
        ("red", "Alice/Red", ItemStatus::Enabled),
    ] {
        insert_test_mod(
            &pool,
            &TestModFixture {
                id,
                game_id: "game",
                object_id: Some("alice"),
                actual_name: id,
                folder_path: path,
                status,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: root.to_str(),
            },
        )
        .await
        .unwrap();
    }
    let config = ConfigService::new_for_test_async(pool.clone()).await;
    let mut input = WorkspaceSwitchInput {
        game_id: "game".into(),
        target: WorkspaceSwitchTarget {
            kind: WorkspaceSwitchTargetKind::ModPath,
            value: target.to_string_lossy().into_owned(),
        },
        desired_enabled: true,
        resolution: WorkspaceSwitchResolution::Normal,
        enable_disabled_ancestors: false,
        origin_surface: WorkspaceSwitchOriginSurface::FolderGrid,
    };
    let normal = prepare_switch(&input, &config, &pool).await.unwrap();
    assert!(matches!(
        normal.immediate_result().unwrap().status,
        WorkspaceSwitchStatus::RequiresDuplicateResolution
    ));
    assert!(normal.journal_steps().is_empty());

    input.resolution = WorkspaceSwitchResolution::ForceEnable;
    let forced = prepare_switch(&input, &config, &pool).await.unwrap();
    let forced_steps = forced.journal_steps();
    assert_eq!(
        forced_steps.len(),
        1,
        "Force Enable must not disable a sibling"
    );
    assert_eq!(forced_steps[0].1, target.canonicalize().unwrap());
    let PreparedWorkspaceSwitch::Mod(prepared) = forced else {
        panic!("mod plan")
    };
    assert_eq!(
        prepared.target_path,
        forced_steps[0].1.to_string_lossy(),
        "the result path lookup must use the same canonical representation as rename steps"
    );

    input.resolution = WorkspaceSwitchResolution::EnableOnlyThis;
    let exclusive = prepare_switch(&input, &config, &pool).await.unwrap();
    let exclusive_steps = exclusive.journal_steps();
    assert_eq!(exclusive_steps.len(), 2);
    assert_eq!(exclusive_steps[0].1, sibling.canonicalize().unwrap());
    assert_eq!(exclusive_steps[1].1, target.canonicalize().unwrap());
    crate::modules::workspace::adapters::sqlite::conflict::ignore_object_conflict(
        &pool,
        "game",
        "alice",
        &["blue".into(), "red".into()],
    )
    .await
    .unwrap();
    let ignored_exclusive = prepare_switch(&input, &config, &pool).await.unwrap();
    assert_eq!(
        ignored_exclusive.journal_steps().len(),
        2,
        "ignoring a duplicate warning must not change an explicit Enable Only This action"
    );
    input.resolution = WorkspaceSwitchResolution::Normal;
    assert!(
        prepare_switch(&input, &config, &pool)
            .await
            .unwrap()
            .immediate_result()
            .is_none(),
        "normal enable still respects ignored warnings"
    );
    #[cfg(windows)]
    {
        input.resolution = WorkspaceSwitchResolution::EnableOnlyThis;
        for spelling in [
            sibling.with_file_name("rED").to_string_lossy().into_owned(),
            sibling.to_string_lossy().replace("Mods", "mODS"),
            sibling
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ] {
            input.target.value = spelling;
            assert!(
                prepare_switch(&input, &config, &pool)
                    .await
                    .unwrap()
                    .journal_steps()
                    .is_empty(),
                "an enabled target must not disable itself through alternate Windows spelling: {}",
                input.target.value
            );
        }
    }
    assert!(
        target.exists() && exclusive_steps[0].1.exists(),
        "preparation must not mutate disk"
    );
    let status: i64 = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'blue'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        status,
        ItemStatus::Disabled as i64,
        "preparation must not mutate the DB"
    );
}

#[tokio::test]
async fn mod_switch_describes_and_sequences_every_disabled_parent() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let child = root.join("DISABLED Group/DISABLED Alice/Blue");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::create_dir_all(root.join("DISABLED Group/DISABLED Alice/DISABLED Red")).unwrap();
    std::fs::create_dir_all(root.join("DISABLED Group/Other")).unwrap();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_str().unwrap(),
            mods_path: root.to_str(),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "alice",
            game_id: "game",
            name: "Alice",
            folder_path: "DISABLED Group/DISABLED Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    for (id, actual_name, folder_path, status) in [
        (
            "blue",
            "Blue",
            "DISABLED Group/DISABLED Alice/Blue",
            ItemStatus::Disabled,
        ),
        (
            "red",
            "Red",
            "DISABLED Group/DISABLED Alice/DISABLED Red",
            ItemStatus::Disabled,
        ),
        (
            "other",
            "Other",
            "DISABLED Group/Other",
            ItemStatus::Disabled,
        ),
    ] {
        insert_test_mod(
            &pool,
            &TestModFixture {
                id,
                game_id: "game",
                object_id: Some("alice"),
                actual_name,
                folder_path,
                status,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: root.to_str(),
            },
        )
        .await
        .unwrap();
    }
    let config = ConfigService::new_for_test_async(pool.clone()).await;
    let mut input = WorkspaceSwitchInput {
        game_id: "game".into(),
        target: WorkspaceSwitchTarget {
            kind: WorkspaceSwitchTargetKind::ModPath,
            value: child.to_string_lossy().into_owned(),
        },
        desired_enabled: true,
        resolution: WorkspaceSwitchResolution::ForceEnable,
        enable_disabled_ancestors: false,
        origin_surface: WorkspaceSwitchOriginSurface::FolderGrid,
    };

    let requirement = prepare_switch(&input, &config, &pool)
        .await
        .unwrap()
        .immediate_result()
        .expect("a descendant switch must require parent activation")
        .parent_enable_requirement
        .expect("the requirement includes the parent and impact");

    assert_eq!(
        requirement
            .parents
            .iter()
            .map(|parent| parent.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Group", "Alice"]
    );
    assert_eq!(
        requirement
            .will_activate
            .iter()
            .map(|impact| impact.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Blue", "Other"]
    );
    assert_eq!(
        requirement
            .stay_disabled
            .iter()
            .map(|impact| impact.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Red"]
    );
    assert!(child.exists(), "the requirement must not rename folders");

    input.enable_disabled_ancestors = true;
    let confirmed = prepare_switch(&input, &config, &pool).await.unwrap();
    let canonical_root = root.canonicalize().unwrap();
    assert_eq!(
        confirmed.journal_steps(),
        vec![
            (
                0,
                canonical_root.join("DISABLED Group"),
                canonical_root.join("Group"),
            ),
            (
                1,
                canonical_root.join("Group/DISABLED Alice"),
                canonical_root.join("Group/Alice"),
            ),
        ],
        "parents are renamed outer-to-inner before the requested child becomes effective",
    );
}
