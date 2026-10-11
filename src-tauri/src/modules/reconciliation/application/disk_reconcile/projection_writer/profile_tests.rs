use super::{reconcile_projection_in_tx, ProjectionWriteRequest};
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::reconciliation::application::disk_reconcile::{
    change_summary::ChangeSummaryBuilder,
    disk_snapshot::{DiskModEntry, DiskObjectEntry, DiskProjection},
};
use sqlx::sqlite::SqliteConnectOptions;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

fn expected_profiler_database(
    workspace: &Path,
    run_id: Option<&str>,
) -> Result<PathBuf, crate::shared::errors::AppError> {
    use crate::shared::errors::AppError;
    if !workspace.is_absolute() {
        return Err(AppError::Validation(
            "Profiler workspace must be absolute".to_string(),
        ));
    }
    match run_id {
        None => Ok(workspace
            .join(".audit-output/gimi-final-stabilization-20261010/projection-benchmark.db")),
        Some(run_id) => {
            let parsed = uuid::Uuid::parse_str(run_id).map_err(|_| {
                AppError::Validation("Profiler test run ID must be a canonical UUID".to_string())
            })?;
            if parsed.hyphenated().to_string() != run_id {
                return Err(AppError::Validation(
                    "Profiler test run ID must be a canonical UUID".to_string(),
                ));
            }
            Ok(workspace
                .join(".audit-output/ignored-tests")
                .join(run_id)
                .join("projection-benchmark.db"))
        }
    }
}

fn reject_profiler_links(path: &Path) -> Result<(), crate::shared::errors::AppError> {
    for ancestor in path.ancestors() {
        let metadata = match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        #[cfg(windows)]
        let reparse = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                != 0
        };
        #[cfg(not(windows))]
        let reparse = false;
        if metadata.file_type().is_symlink() || reparse {
            return Err(crate::shared::errors::AppError::Validation(
                "Projection profiler refuses linked database paths or sidecars".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_profiler_database(
    database: &Path,
    expected: &Path,
    protected: &Path,
) -> Result<PathBuf, crate::shared::errors::AppError> {
    use crate::shared::errors::AppError;
    if !database.is_absolute() || !expected.is_absolute() || !protected.is_absolute() {
        return Err(AppError::Validation(
            "Profiler paths must be absolute".to_string(),
        ));
    }
    reject_profiler_links(database)?;
    reject_profiler_links(expected)?;
    reject_profiler_links(protected)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = database.as_os_str().to_os_string();
        sidecar.push(suffix);
        reject_profiler_links(Path::new(&sidecar))?;
    }
    let database = database.canonicalize()?;
    let expected = expected.canonicalize()?;
    let protected = protected.canonicalize()?;
    if database != expected || !database.is_file() {
        return Err(AppError::Validation(
            "Profiler requires the owned workspace DB clone".to_string(),
        ));
    }
    let protected_key = crate::shared::path_key::exact_location_key_for_path(&protected);
    for candidate in [
        &database,
        expected.parent().expect("absolute database parent"),
    ] {
        let key = crate::shared::path_key::exact_location_key_for_path(candidate);
        if key == protected_key
            || key.starts_with(&format!("{protected_key}/"))
            || protected_key.starts_with(&format!("{key}/"))
        {
            return Err(AppError::Validation(
                "Profiler DB must not overlap the protected library".to_string(),
            ));
        }
    }
    Ok(database)
}

fn contained(root: &Path, stored: &str) -> PathBuf {
    let path = root.join(stored);
    assert!(path.starts_with(root));
    assert!(!path.components().any(|part| part == Component::ParentDir));
    assert!(path.is_dir(), "archived projection folder missing");
    path
}

#[test]
fn profiler_expected_database_preserves_legacy_and_bounds_synthetic_run() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    assert_eq!(
        expected_profiler_database(workspace, None).unwrap(),
        workspace.join(".audit-output/gimi-final-stabilization-20261010/projection-benchmark.db")
    );
    let run_id = "069ac14c-e071-4ec5-adba-fbb336710dab";
    assert_eq!(
        expected_profiler_database(workspace, Some(run_id)).unwrap(),
        workspace.join(format!(
            ".audit-output/ignored-tests/{run_id}/projection-benchmark.db"
        ))
    );
    for invalid in [
        "",
        "..",
        "../clone.db",
        "C:/clone.db",
        "{069ac14c-e071-4ec5-adba-fbb336710dab}",
    ] {
        assert!(expected_profiler_database(workspace, Some(invalid)).is_err());
    }
    assert!(expected_profiler_database(Path::new("relative"), Some(run_id)).is_err());
}

#[test]
fn profiler_database_guard_rejects_an_alternate_owned_database() {
    let temp = tempfile::tempdir().expect("guard fixture");
    let protected = temp.path().join("library");
    let owned = temp.path().join("audit");
    std::fs::create_dir_all(&protected).expect("protected fixture");
    std::fs::create_dir_all(&owned).expect("owned fixture");
    let expected = owned.join("projection-benchmark.db");
    let alternate = owned.join("alternate.db");
    std::fs::write(&expected, []).expect("expected fixture");
    std::fs::write(&alternate, []).expect("alternate fixture");
    assert!(validate_profiler_database(&alternate, &expected, &protected).is_err());
}

#[test]
fn profiler_database_guard_rejects_overlapping_roots_in_both_directions() {
    let temp = tempfile::tempdir().expect("guard fixture");
    let protected = temp.path().join("library");
    let owned = temp.path().join("audit");
    std::fs::create_dir_all(&protected).expect("library fixture");
    std::fs::create_dir_all(&owned).expect("audit fixture");
    let safe = owned.join("clone.db");
    std::fs::write(&safe, []).expect("owned file");
    assert!(validate_profiler_database(&safe, &safe, &protected).is_ok());
    let inside = protected.join("clone.db");
    std::fs::write(&inside, []).expect("protected fixture file");
    assert!(validate_profiler_database(&inside, &inside, &protected).is_err());
    std::fs::create_dir_all(owned.join("nested-library")).expect("nested fixture");
    assert!(validate_profiler_database(&safe, &safe, &owned.join("nested-library")).is_err());
}

#[cfg(windows)]
#[test]
fn profiler_database_guard_rejects_ancestor_junction_before_sqlite_open() {
    let temp = tempfile::tempdir().expect("junction fixture");
    let protected = temp.path().join("library");
    let target = protected.join("profile");
    std::fs::create_dir_all(&target).expect("target fixture");
    let clone = target.join("clone.db");
    std::fs::write(&clone, b"preserve").expect("protected fixture");
    let link = temp.path().join("audit-link");
    let result = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&protected)
        .output()
        .expect("junction command");
    assert!(result.status.success(), "junction fixture creation failed");
    let redirected = link.join("profile/clone.db");
    assert!(validate_profiler_database(&redirected, &redirected, &protected).is_err());
    assert_eq!(
        std::fs::read(&clone).expect("preserved fixture"),
        b"preserve"
    );
}

#[tokio::test]
#[ignore = "manual projection profiling: writes only a workspace DB clone, reads real metadata"]
async fn benchmark_projection_phases_isolated() {
    let root = PathBuf::from(std::env::var_os("EMMM_AUDIT_MODS_PATH").expect("Mods root"));
    let database =
        PathBuf::from(std::env::var_os("EMMM_AUDIT_PROJECTION_DATABASE").expect("DB clone"));
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    let run_id = std::env::var_os("EMMM_AUDIT_TEST_RUN_ID").map(|value| {
        value
            .into_string()
            .expect("Profiler test run ID must be UTF-8")
    });
    let expected = expected_profiler_database(workspace, run_id.as_deref())
        .expect("bounded profiler workspace database");
    let database =
        validate_profiler_database(&database, &expected, root.parent().expect("protected root"))
            .expect("isolated physical DB clone");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(false),
        )
        .await
        .expect("open owned workspace clone");
    let game_id: String = sqlx::query_scalar("SELECT id FROM games WHERE mods_path = ?")
        .bind(root.to_string_lossy().as_ref())
        .fetch_one(&pool)
        .await
        .expect("exact root binding");
    let objects: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT folder_path, name, status, filesystem_identity FROM objects WHERE game_id = ? ORDER BY folder_path_key",
    ).bind(&game_id).fetch_all(&pool).await.expect("objects");
    let mods: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT m.folder_path, o.folder_path, m.filesystem_identity FROM mods m JOIN objects o ON o.id=m.object_id WHERE m.game_id=? ORDER BY m.folder_path_key",
    ).bind(&game_id).fetch_all(&pool).await.expect("mods");
    let key = |path: &str| crate::shared::path_key::folder_path_key(path, None);
    let projection = DiskProjection {
        objects: objects
            .into_iter()
            .map(
                |(folder_path, name, status, filesystem_identity)| DiskObjectEntry {
                    absolute_path: contained(&root, &folder_path),
                    folder_path_key: key(&folder_path),
                    folder_path,
                    name,
                    is_disabled: status == 0,
                    filesystem_identity,
                },
            )
            .collect(),
        mods: mods
            .into_iter()
            .map(
                |(folder_path, object_path, filesystem_identity)| DiskModEntry {
                    absolute_path: contained(&root, &folder_path),
                    folder_path_key: key(&folder_path),
                    raw_name: Path::new(&folder_path)
                        .file_name()
                        .expect("mod name")
                        .to_string_lossy()
                        .into_owned(),
                    object_folder_path_key: key(&object_path),
                    folder_path,
                    filesystem_identity,
                },
            )
            .collect(),
    };
    assert_eq!(projection.objects.len(), 148);
    assert_eq!(projection.mods.len(), 4243);
    let mut connection = pool.acquire().await.expect("single connection");
    sqlx::query("CREATE TEMP TABLE profile_writes (kind TEXT NOT NULL)")
        .execute(&mut *connection)
        .await
        .expect("counter");
    for table in ["objects", "mods", "object_runtime_projection"] {
        for operation in ["INSERT", "UPDATE", "DELETE"] {
            let query = format!("CREATE TEMP TRIGGER profile_{table}_{operation} AFTER {operation} ON main.{table} BEGIN INSERT INTO profile_writes VALUES ('{table}_{operation}'); END");
            sqlx::query(&query)
                .execute(&mut *connection)
                .await
                .expect("counter trigger");
        }
    }
    use sqlx::Acquire;
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('objects')")
        .fetch_all(&mut *connection)
        .await
        .expect("owned object schema");
    let unchanged = columns
        .iter()
        .map(|column| {
            let column = column.replace('"', "\"\"");
            format!("NEW.\"{column}\" IS OLD.\"{column}\"")
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    let suppression = format!(
        "CREATE TEMP TRIGGER profile_suppress_noop BEFORE UPDATE ON main.objects WHEN {unchanged} BEGIN SELECT RAISE(IGNORE); END"
    );
    const BASELINE_ROUNDS: usize = 6;
    const COMPARISON_PAIRS: usize = 7;
    for trial in 0..BASELINE_ROUNDS + COMPARISON_PAIRS * 2 {
        let comparison = trial.saturating_sub(BASELINE_ROUNDS);
        let second = !comparison.is_multiple_of(2);
        let suppress_noops = trial >= BASELINE_ROUNDS
            && if (comparison / 2).is_multiple_of(2) {
                second
            } else {
                !second
            };
        sqlx::query("DROP TRIGGER IF EXISTS temp.profile_suppress_noop")
            .execute(&mut *connection)
            .await
            .expect("remove prototype");
        if suppress_noops {
            sqlx::query(&suppression)
                .execute(&mut *connection)
                .await
                .expect("test-only noop prototype");
        }
        sqlx::query("DELETE FROM profile_writes")
            .execute(&mut *connection)
            .await
            .expect("clear counters");
        let mut updates = Vec::new();
        let mut impact = CollectionReferenceImpact::default();
        let mut summary = ChangeSummaryBuilder::default();
        let started = Instant::now();
        let mut tx = connection.begin().await.expect("transaction");
        let writer = reconcile_projection_in_tx(
            &mut tx,
            ProjectionWriteRequest {
                game_id: &game_id,
                mods_path: &root,
                safe_mode_keywords: &[],
                projection: &projection,
                changed_roots: &[],
                scoped: false,
                force_full: false,
                path_updates: &mut updates,
                collection_reference_impact: &mut impact,
                change_summary: &mut summary,
                protected_object_keys: &HashSet::new(),
                protected_mod_keys: &HashSet::new(),
                on_progress: None,
            },
        )
        .await
        .expect("writer");
        let refresh_started = Instant::now();
        crate::modules::workspace::adapters::sqlite::runtime_projection::rebuild_game_projection_tx(&mut tx, &game_id)
            .await.expect("read-model repair");
        let refresh = refresh_started.elapsed();
        let commit_started = Instant::now();
        tx.commit().await.expect("commit clone");
        let commit = commit_started.elapsed();
        let total = started.elapsed();
        let counters: Vec<(String, i64)> =
            sqlx::query_as("SELECT kind, COUNT(*) FROM profile_writes GROUP BY kind ORDER BY kind")
                .fetch_all(&mut *connection)
                .await
                .expect("write counts");
        if trial > 0 {
            assert!(
                !writer.objects_changed && !writer.folders_changed,
                "warm projection should be settled"
            );
            assert!(updates.is_empty());
        }
        eprintln!("projection_profile trial={trial} suppress_noops={suppress_noops} initial_index_us={} identity_us={} objects_us={} mods_us={} metadata_within_mods_us={} finalization_us={} refresh_us={} commit_us={} total_us={} writes={counters:?}",
            writer.timings.initial_index.as_micros(), writer.timings.identity.as_micros(), writer.timings.objects.as_micros(),
            writer.timings.mods.as_micros(), writer.timings.metadata_within_mods.as_micros(), writer.timings.finalization.as_micros(),
            refresh.as_micros(), commit.as_micros(), total.as_micros());
    }
    drop(connection);
    pool.close().await;
}
