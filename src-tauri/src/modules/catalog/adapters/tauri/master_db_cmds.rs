use crate::modules::games::application::game::schema_loader;
use crate::shared::errors::AppError;
use tauri::Manager;

/// The bundled-resources directory, or a typed error.
///
/// Five commands re-derived this, and `search_master_db` did it twice in one
/// body so a cache hit still paid a second lookup.
fn resource_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .resource_dir()
        .map_err(|error| AppError::Internal(format!("Failed to get resource dir: {error}")))
}

/// Get the game schema (categories + filters) for a specific game type.
/// Falls back to default [Character, Weapon, UI, Other] if schema.json is missing/corrupt.
///
/// Covers: NC-3.4-02 (Schema Load Failure → fallback)
#[tauri::command]
#[specta::specta]
pub async fn get_game_schema(
    app: tauri::AppHandle,
    game_type: i32,
) -> Result<schema_loader::GameSchema, AppError> {
    let resource_dir = resource_dir(&app)?;

    log::info!("get_game_schema: resource_dir = {}", resource_dir.display());

    let schema = schema_loader::load_schema(&resource_dir, game_type);
    Ok(schema)
}

/// Get a single object by ID (full details including metadata).
#[tauri::command]
#[specta::specta]
pub async fn get_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
) -> Result<
    Option<crate::modules::workspace::application::scanner::core::types::GameObject>,
    AppError,
> {
    let row =
        crate::modules::catalog::application::objects::query::get_object_by_id_service(&pool, &id)
            .await?;
    Ok(row)
}

/// Get the user-installed MasterDB catalog for a specific game type.
/// Returns DbEntry array directly, eliminating manual String parsing.
/// When hash_db is present in source, merges hashes into matching entries.
#[tauri::command]
#[specta::specta]
pub async fn get_master_db(
    app: tauri::AppHandle,
    game_type: i32,
) -> Result<Vec<crate::modules::matching::application::deep_matcher::DbEntry>, AppError> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    Ok(
        crate::modules::workspace::application::scanner::master_db::load_master_db_entries(
            &app_data_dir,
            game_type,
        )?,
    )
}

/// Report whether a user-installed, data-only catalog pack is available.
#[tauri::command]
#[specta::specta]
pub fn get_catalog_pack_status(
    app: tauri::AppHandle,
) -> crate::modules::workspace::application::scanner::master_db::CatalogPackStatus {
    crate::modules::workspace::application::scanner::master_db::asset_pack::status(&app)
}

/// Open the user-writable folder where a manually extracted catalog pack belongs.
#[tauri::command]
#[specta::specta]
pub fn open_catalog_pack_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Manager;
    let root = app.path().app_data_dir()?.join("asset-pack");
    std::fs::create_dir_all(&root)?;
    crate::platform::process::reveal_in_file_manager(&root)
}

/// Validate the candidate pack before replacing cached catalog data.
#[tauri::command]
#[specta::specta]
pub async fn refresh_catalog_pack(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult,
    AppError,
> {
    refresh_catalog_pack_service(&app, pool.inner()).await
}

/// Rebuild catalog-derived thumbnail mappings after a validated pack change.
/// Kept outside the IPC wrapper so the signed updater and manual refresh use
/// exactly the same ownership-preserving rule.
pub(crate) async fn refresh_catalog_pack_service(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult,
    AppError,
> {
    use sqlx::Row;
    use std::collections::HashMap;
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    let asset_pack_root =
        crate::modules::workspace::application::scanner::master_db::asset_pack::CatalogPack::root(
            &app_data_dir,
        );
    let pack =
        crate::modules::workspace::application::scanner::master_db::asset_pack::CatalogPack::load(
            &app_data_dir,
        )?;
    let status = pack.status()?;
    let mut thumbnails = HashMap::new();
    for game_type in 0..=4 {
        for entry in pack.entries_for(game_type)? {
            if let Some(path) = entry.thumbnail_path {
                thumbnails.insert(
                    (game_type, crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(&entry.name)),
                    path,
                );
            }
        }
    }
    let rows = sqlx::query(
        "SELECT o.id, o.matched_entry_key, o.thumbnail_path, o.thumbnail_source, g.game_type
         FROM objects o JOIN games g ON g.id = o.game_id
         WHERE o.matched_entry_key IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    let mut thumbnails_applied = 0;
    for row in rows {
        let id: String = row.try_get("id")?;
        let key: String = row.try_get("matched_entry_key")?;
        let game_type: i32 = row.try_get("game_type")?;
        let current: Option<String> = row.try_get("thumbnail_path")?;
        let source: Option<String> = row.try_get("thumbnail_source")?;
        if source.as_deref() == Some("user") {
            continue;
        }
        let current_is_pack_asset = current
            .as_deref()
            .is_some_and(|path| std::path::Path::new(path).starts_with(&asset_pack_root));
        let replacement = thumbnails.get(&(game_type, key));
        if replacement.is_none()
            && source.as_deref() != Some("asset_pack")
            && !current_is_pack_asset
        {
            continue;
        }
        let next = replacement.cloned();
        if current == next {
            continue;
        }
        sqlx::query("UPDATE objects SET thumbnail_path = ?, thumbnail_source = ? WHERE id = ?")
            .bind(&next)
            .bind(next.as_ref().map(|_| "asset_pack"))
            .bind(&id)
            .execute(pool)
            .await?;
        if next.is_some() {
            thumbnails_applied += 1;
        }
    }
    crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(app)
        .await;
    Ok(
        crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult {
            state: status.state,
            entries: status.entries,
            thumbnails_applied,
            missing_assets: status.missing_assets,
            skipped_invalid_files: 0,
        },
    )
}

/// Check the fixed public catalog release channel. This is read-only and uses
/// no GitHub credentials from the user or the application.
#[tauri::command]
#[specta::specta]
pub async fn check_catalog_update(
    app: tauri::AppHandle,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<crate::modules::workspace::application::scanner::master_db::CatalogUpdateCheck, AppError>
{
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    let update = crate::modules::workspace::application::scanner::master_db::catalog_update::check(
        &app_data_dir,
    )
    .await?;
    config.record_catalog_update_check(chrono::Utc::now().timestamp())?;
    Ok(update)
}

/// Download, verify, validate, and atomically activate the latest signed
/// catalog release. The active catalog is left untouched on every failure.
#[tauri::command]
#[specta::specta]
pub async fn install_catalog_update(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogUpdateInstallResult,
    AppError,
> {
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    let installed =
        crate::modules::workspace::application::scanner::master_db::catalog_update::install(
            &app_data_dir,
        )
        .await?;
    refresh_catalog_pack_service(&app, pool.inner()).await?;
    config.record_catalog_update_check(chrono::Utc::now().timestamp())?;
    Ok(installed)
}

#[tauri::command]
#[specta::specta]
pub fn set_catalog_auto_install(
    enabled: bool,
    config: tauri::State<'_, crate::modules::settings::application::config::ConfigService>,
) -> Result<crate::modules::settings::application::config::AppSettings, AppError> {
    config.set_catalog_auto_install(enabled)
}

/// Start the weekly background check after application state has finished
/// initializing. Failures are logged only; browsing the local catalog never
/// depends on release-channel availability.
pub(crate) fn schedule_catalog_auto_update(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri::Manager;
        let config = app.state::<crate::modules::settings::application::config::ConfigService>();
        let preferences = config.get_settings().catalog_updates;
        let now = chrono::Utc::now().timestamp();
        if !preferences.auto_check
            || !crate::modules::workspace::application::scanner::master_db::catalog_update::update_due(
                preferences.last_successful_check_unix_seconds,
                now,
            )
        {
            return;
        }

        let update_state = app.state::<
            crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
        >();
        let _operation = update_state.0.lock().await;
        let app_data_dir = match app.path().app_data_dir() {
            Ok(path) => path,
            Err(error) => {
                let telemetry_error = AppError::Internal(error.to_string());
                crate::modules::system::application::telemetry::record_background_failure(
                    &app,
                    &telemetry_error,
                )
                .await;
                log::warn!("Catalog update check skipped: app-data path unavailable: {error}");
                return;
            }
        };
        let update =
            match crate::modules::workspace::application::scanner::master_db::catalog_update::check(
                &app_data_dir,
            )
            .await
            {
                Ok(update) => update,
                Err(error) => {
                    crate::modules::system::application::telemetry::record_background_failure(
                        &app,
                        &AppError::Scanner(error.clone()),
                    )
                    .await;
                    log::warn!("Catalog update check failed: {error}");
                    return;
                }
            };
        if let Err(error) = config.record_catalog_update_check(now) {
            crate::modules::system::application::telemetry::record_background_failure(&app, &error)
                .await;
            log::warn!("Catalog update was checked but its timestamp could not be saved: {error}");
        }
        if !preferences.auto_install || update.state != "update_available" {
            return;
        }
        match crate::modules::workspace::application::scanner::master_db::catalog_update::install(
            &app_data_dir,
        )
        .await
        {
            Ok(_) => {
                let pool = app.state::<sqlx::SqlitePool>();
                if let Err(error) = refresh_catalog_pack_service(&app, pool.inner()).await {
                    crate::modules::system::application::telemetry::record_background_failure(
                        &app, &error,
                    )
                    .await;
                    log::warn!(
                        "Catalog update installed but local mappings could not refresh: {error}"
                    );
                }
            }
            Err(error) => {
                let telemetry_error = AppError::Scanner(error.clone());
                crate::modules::system::application::telemetry::record_background_failure(
                    &app,
                    &telemetry_error,
                )
                .await;
                log::warn!("Catalog auto-install failed: {error}");
            }
        }
    });
}

/// Pin or unpin an object in the database.
#[tauri::command]
#[specta::specta]
pub async fn pin_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    id: String,
    pin: bool,
) -> Result<(), AppError> {
    crate::modules::catalog::application::objects::mutate::toggle_pin_object(pool.inner(), &id, pin)
        .await
}

/// Search Master DB from Rust to offload fuzzy matching from the JS thread.
/// Finds the top results matching `query`, optionally filtering by `object_type`.
#[tauri::command]
#[specta::specta]
pub async fn search_master_db(
    app: tauri::AppHandle,
    game_type: i32,
    query: String,
    object_type: Option<String>,
) -> Result<
    Vec<crate::modules::workspace::application::scanner::master_db::SearchResultEntry>,
    AppError,
> {
    let Some(db) =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?
    else {
        return Ok(Vec::new());
    };

    let resource_dir = resource_dir(&app)?;

    Ok(
        crate::modules::workspace::application::scanner::master_db::search_master_db_service(
            &db,
            &resource_dir,
            &query,
            object_type.as_deref(),
        ),
    )
}
