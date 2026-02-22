#[cfg(test)]
mod tests {
    use emmm2_lib::commands::mods::mod_bulk_cmds;
    use emmm2_lib::services::mod_files::info_json;
    use std::fs;
    use tempfile::TempDir;

    use emmm2_lib::services::scanner::watcher::WatcherState;

    #[tokio::test]
    async fn test_bulk_toggle_mods() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let mod1 = root.join("Mod1");
        let mod2 = root.join("Mod2");
        let mod3 = root.join("DISABLED Mod3");

        fs::create_dir(&mod1).unwrap();
        fs::create_dir(&mod2).unwrap();
        fs::create_dir(&mod3).unwrap();

        // Instantiate WatcherState
        let state = WatcherState::new();

        // 1. Bulk Disable [Mod1, Mod2]
        let paths = vec![
            mod1.to_string_lossy().to_string(),
            mod2.to_string_lossy().to_string(),
        ];

        let result = mod_bulk_cmds::bulk_toggle_mods_inner(&state, paths, false)
            .await
            .expect("Bulk disable should succeed");

        assert_eq!(result.success.len(), 2);
        assert!(result.failures.is_empty());

        assert!(root.join("DISABLED Mod1").exists());
        assert!(root.join("DISABLED Mod2").exists());

        // 2. Bulk Enable [DISABLED Mod1, DISABLED Mod3]
        let paths_enable = vec![
            root.join("DISABLED Mod1").to_string_lossy().to_string(),
            mod3.to_string_lossy().to_string(),
        ];

        let result_enable = mod_bulk_cmds::bulk_toggle_mods_inner(&state, paths_enable, true)
            .await
            .expect("Bulk enable should succeed");

        assert_eq!(result_enable.success.len(), 2);
        assert!(root.join("Mod1").exists());
        assert!(root.join("Mod3").exists());
    }

    #[tokio::test]
    async fn test_bulk_update_info() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let mod1 = root.join("ModA");
        let mod2 = root.join("ModB");

        fs::create_dir(&mod1).unwrap();
        fs::create_dir(&mod2).unwrap();

        let paths = vec![
            mod1.to_string_lossy().to_string(),
            mod2.to_string_lossy().to_string(),
        ];

        let update = info_json::ModInfoUpdate {
            tags_add: Some(vec!["Tag1".to_string(), "Tag2".to_string()]),
            is_safe: Some(true),
            ..info_json::ModInfoUpdate::default()
        };

        let result = mod_bulk_cmds::bulk_update_info(paths, update)
            .await
            .expect("Bulk update should succeed");

        assert_eq!(result.success.len(), 2);

        // Verify ModA
        let info1 = info_json::read_info_json(&mod1)
            .expect("Read 1")
            .expect("Info 1");
        assert!(info1.tags.contains(&"Tag1".to_string()));
        assert!(info1.is_safe); // Mapped from safe_mode

        // Verify ModB
        let info2 = info_json::read_info_json(&mod2)
            .expect("Read 2")
            .expect("Info 2");
        assert!(info2.tags.contains(&"Tag2".to_string()));
    }
}
