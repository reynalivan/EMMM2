use super::{harvest_active_keybindings_for_mods, ActiveKeyBinding};
use crate::modules::system::domain::mod_path::ModFolderPath;
use sqlx::sqlite::SqliteConnectOptions;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type KeybindingRow = (String, ModFolderPath, Option<String>, Option<String>);

fn measure(root: &Path, rows: &[KeybindingRow]) -> (Duration, Vec<ActiveKeyBinding>) {
    let started = Instant::now();
    let bindings = harvest_active_keybindings_for_mods(root, rows.to_vec()).expect("harvest");
    (started.elapsed(), bindings)
}

#[tokio::test]
#[ignore = "manual read-only harvest comparison; requires audit DB and Mods root"]
async fn benchmark_effective_keybinding_filter_readonly() {
    let root = PathBuf::from(std::env::var_os("EMMM_AUDIT_MODS_PATH").expect("Mods root"));
    let database = PathBuf::from(std::env::var_os("EMMM_AUDIT_DATABASE_PATH").expect("audit DB"));
    assert!(root.is_absolute() && root.is_dir());
    assert!(database.is_absolute() && database.is_file());
    assert!(
        !database.starts_with(root.parent().expect("Mods root parent")),
        "audit database must be outside the protected library"
    );
    let pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(database)
            .read_only(true)
            .create_if_missing(false),
    )
    .await
    .expect("open existing audit DB read-only");
    let game_id: String = sqlx::query_scalar("SELECT id FROM games WHERE mods_path = ?")
        .bind(root.to_string_lossy().as_ref())
        .fetch_one(&pool)
        .await
        .expect("exact audit root binding");
    let rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT m.actual_name, m.folder_path, COALESCE(m.object_type, o.object_type), o.matched_alias_name FROM mods m LEFT JOIN objects o ON o.id=m.object_id WHERE m.game_id=? AND m.status=1 ORDER BY m.folder_path_key",
    ).bind(&game_id).fetch_all(&pool).await.expect("original candidates");
    let rows: Vec<KeybindingRow> = rows
        .into_iter()
        .map(|(name, path, kind, alias)| (name, ModFolderPath::from_stored(path), kind, alias))
        .collect();
    for (_, folder_path, _, _) in &rows {
        let resolved = folder_path.resolve(&root);
        assert!(resolved.starts_with(&root), "candidate outside Mods root");
        assert!(
            !resolved
                .components()
                .any(|component| component == std::path::Component::ParentDir),
            "candidate contains parent traversal"
        );
    }
    let filtered = rows
        .iter()
        .filter(|(_, path, _, _)| {
            crate::modules::workspace::api::is_effectively_enabled_path(path.as_stored())
        })
        .cloned()
        .collect::<Vec<_>>();
    let selected =
        crate::modules::dashboard::adapters::sqlite::dashboard::fetch_enabled_mods_for_keybindings(
            &pool, &game_id,
        )
        .await
        .expect("production candidate filter");
    let expected_rows = filtered
        .iter()
        .map(|(name, path, kind, alias)| {
            (
                name.clone(),
                path.as_stored().to_string(),
                kind.clone(),
                alias.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(selected, expected_rows);
    pool.close().await;

    let (_, baseline) = measure(&root, &rows);
    let expected = baseline
        .into_iter()
        .filter(|binding| {
            let relative = Path::new(&binding.folder_path)
                .strip_prefix(&root)
                .expect("binding under root");
            crate::modules::workspace::api::is_effectively_enabled_path(&relative.to_string_lossy())
        })
        .collect::<Vec<_>>();
    let expected = serde_json::to_value(expected).expect("binding signature");
    for trial in 0..5 {
        let (all_time, filtered_time, bindings) = if trial % 2 == 0 {
            let (all_time, _) = measure(&root, &rows);
            let (filtered_time, bindings) = measure(&root, &filtered);
            (all_time, filtered_time, bindings)
        } else {
            let (filtered_time, bindings) = measure(&root, &filtered);
            let (all_time, _) = measure(&root, &rows);
            (all_time, filtered_time, bindings)
        };
        assert_eq!(serde_json::to_value(&bindings).expect("bindings"), expected);
        eprintln!("keybinding_filter_readonly trial={} original_roots={} filtered_roots={} original_us={} filtered_us={} retained_bindings={}",
            trial + 1, rows.len(), filtered.len(), all_time.as_micros(), filtered_time.as_micros(), bindings.len());
    }
}
