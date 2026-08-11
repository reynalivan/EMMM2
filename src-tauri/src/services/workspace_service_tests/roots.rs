use super::*;

async fn setup_root_only_workspace() -> (sqlx::SqlitePool, TempDir, String) {
    let ctx = init_test_db().await;
    let mods_root = TempDir::new().expect("tempdir");
    let mods_path = mods_root.path().join("Mods");
    fs::create_dir_all(&mods_path).expect("mods root");

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game_roots",
            name: "Root Test Game",
            game_type: GameType::GIMI,
            path: mods_root.path().to_string_lossy().as_ref(),
            mods_path: Some(mods_path.to_string_lossy().as_ref()),
        },
    )
    .await
    .expect("insert game");

    (ctx.pool, mods_root, mods_path.to_string_lossy().to_string())
}

#[tokio::test]
async fn workspace_lists_visible_root_folders_without_database_objects() {
    let (pool, _mods_root, mods_path) = setup_root_only_workspace().await;
    fs::create_dir_all(std::path::Path::new(&mods_path).join("Aether")).expect("aether root");
    fs::create_dir_all(std::path::Path::new(&mods_path).join("DISABLED Amber"))
        .expect("disabled amber root");
    fs::create_dir_all(std::path::Path::new(&mods_path).join(".temp_extract"))
        .expect("hidden root");
    let private_root = std::path::Path::new(&mods_path).join("Private");
    fs::create_dir_all(&private_root).expect("private root");
    write_file(
        &private_root.join("info.json"),
        r#"{"actual_name":"Private","is_safe":false}"#,
    );
    write_file(
        &std::path::Path::new(&mods_path).join("readme.txt"),
        "not a folder",
    );

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_roots")
            },
            selected_object_folder_path: None,
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    let names = view_model
        .objects
        .iter()
        .map(|root| root.object.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["Aether", "Amber"]);
    assert_eq!(view_model.objects[1].object.folder_path, "DISABLED Amber");
    assert!(view_model.objects.iter().all(|root| !root.is_registered));
    assert!(view_model.objects.iter().all(|root| {
        !root.capabilities.can_toggle
            && !root.capabilities.can_edit_metadata
            && !root.capabilities.can_delete
    }));
    assert_eq!(view_model.explorer.children.len(), 2);
}

#[tokio::test]
async fn workspace_enriches_registered_roots_without_duplicates() {
    let (pool, _mods_root, mods_path) = setup_root_only_workspace().await;
    fs::create_dir_all(std::path::Path::new(&mods_path).join("Amber")).expect("amber root");
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "obj_amber",
            game_id: "game_roots",
            name: "Amber Metadata",
            folder_path: "Amber",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_roots")
            },
            selected_object_folder_path: None,
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(view_model.objects.len(), 1);
    assert_eq!(view_model.objects[0].object.id, "obj_amber");
    assert_eq!(view_model.objects[0].object.name, "Amber Metadata");
    assert!(view_model.objects[0].is_registered);
}

#[tokio::test]
async fn workspace_excludes_unregistered_roots_from_metadata_filters() {
    let (pool, _mods_root, mods_path) = setup_root_only_workspace().await;
    fs::create_dir_all(std::path::Path::new(&mods_path).join("Aether")).expect("aether root");
    fs::create_dir_all(std::path::Path::new(&mods_path).join("Amber")).expect("amber root");
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "obj_amber",
            game_id: "game_roots",
            name: "Amber",
            folder_path: "Amber",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                game_id: "game_roots".to_string(),
                object_type: Some("Character".to_string()),
                safe_mode: true,
                ..Default::default()
            },
            selected_object_folder_path: None,
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(view_model.objects.len(), 1);
    assert_eq!(view_model.objects[0].object.name, "Amber");
    assert!(view_model.objects[0].is_registered);
}

#[tokio::test]
async fn workspace_grid_keeps_mods_and_plain_folders_visible() {
    let (pool, _mods_root, mods_path) = setup_root_only_workspace().await;
    let aether_root = std::path::Path::new(&mods_path).join("Aether");
    let mod_folder = aether_root.join("Traveler Mod");
    let plain_folder = aether_root.join("Variants");
    fs::create_dir_all(&mod_folder).expect("mod folder");
    fs::create_dir_all(&plain_folder).expect("plain folder");
    write_file(&mod_folder.join("mod.ini"), "[TextureOverrideTraveler]\n");

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_roots")
            },
            selected_object_folder_path: Some("Aether".to_string()),
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(view_model.explorer.children.len(), 2);
    let traveler = view_model
        .explorer
        .children
        .iter()
        .find(|folder| folder.display_name == "Traveler Mod")
        .expect("traveler mod");
    let variants = view_model
        .explorer
        .children
        .iter()
        .find(|folder| folder.display_name == "Variants")
        .expect("variants folder");
    assert_eq!(traveler.node_kind, WorkspaceNodeKind::TerminalMod);
    assert!(!traveler.can_navigate);
    assert_eq!(variants.node_kind, WorkspaceNodeKind::Container);
    assert!(variants.can_navigate);
}
