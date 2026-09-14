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

/// Refresh catalog state from a manually managed pack.
/// Catalog-owned thumbnail references are cleared only for this explicit
/// refresh path; importing a reviewed pack never changes object metadata.
pub(crate) async fn refresh_catalog_pack_service(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult,
    AppError,
> {
    sqlx::query(
        "UPDATE objects
         SET thumbnail_path = NULL, thumbnail_source = NULL
         WHERE thumbnail_source = 'asset_pack'",
    )
    .execute(pool)
    .await?;
    activate_catalog_pack_service(app).await
}

/// Expose a newly activated pack to the matcher without reconciling the game
/// library or replacing any object metadata or thumbnails.
pub(crate) async fn activate_catalog_pack_service(
    app: &tauri::AppHandle,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult,
    AppError,
> {
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()?;
    let pack =
        crate::modules::workspace::application::scanner::master_db::asset_pack::CatalogPack::load(
            &app_data_dir,
        )?;
    let status = pack.status()?;
    crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(app)
        .await;
    Ok(
        crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult {
            state: status.state,
            entries: status.entries,
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
) -> Result<crate::modules::workspace::application::scanner::master_db::CatalogUpdateCheck, AppError>
{
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    let update = crate::modules::workspace::application::scanner::master_db::catalog_update::check(
        &app_data_dir,
    )
    .await?;
    Ok(update)
}

/// Download, verify, validate, and atomically activate the latest signed
/// catalog release. The active catalog is left untouched on every failure.
#[tauri::command]
#[specta::specta]
pub async fn install_catalog_update(
    app: tauri::AppHandle,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
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
    activate_catalog_pack_service(&app).await?;
    Ok(installed)
}

/// Stage and inspect a public GitHub release asset. The returned token is the
/// reviewed archive identity; activation requires a separate confirmation.
#[tauri::command]
#[specta::specta]
pub async fn preview_catalog_github_import(
    app: tauri::AppHandle,
    import_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogImportState,
    >,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
    source_url: String,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogImportPreview,
    AppError,
> {
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    crate::modules::workspace::application::scanner::master_db::catalog_import::preview_github(
        import_state.inner(),
        &app_data_dir,
        &source_url,
    )
    .await
    .map_err(Into::into)
}

/// Show the native picker and stage a local catalog ZIP for the same review
/// and activation flow used by GitHub releases. Selecting a file never moves
/// or deletes it.
#[tauri::command]
#[specta::specta]
pub async fn preview_catalog_local_import(
    app: tauri::AppHandle,
    import_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogImportState,
    >,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
) -> Result<
    Option<crate::modules::workspace::application::scanner::master_db::CatalogImportPreview>,
    AppError,
> {
    use tauri_plugin_dialog::DialogExt;
    let selected = app
        .dialog()
        .file()
        .add_filter("Catalog Pack", &["zip"])
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| {
        AppError::Validation("Selected catalog ZIP is not available as a local file".to_string())
    })?;
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    crate::modules::workspace::application::scanner::master_db::catalog_import::preview_local_archive(
        import_state.inner(),
        &app_data_dir,
        &path,
    )
    .await
    .map(Some)
    .map_err(Into::into)
}

/// Atomically activate the exact reviewed catalog archive, no matter which
/// manual source produced its staging token.
#[tauri::command]
#[specta::specta]
pub async fn install_catalog_import(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    import_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogImportState,
    >,
    update_state: tauri::State<
        '_,
        crate::modules::workspace::application::scanner::master_db::CatalogUpdateState,
    >,
    staging_token: String,
) -> Result<
    crate::modules::workspace::application::scanner::master_db::CatalogPackRefreshResult,
    AppError,
> {
    use tauri::Manager;
    let _operation = update_state.0.lock().await;
    let app_data_dir = app.path().app_data_dir()?;
    let provenance =
        crate::modules::workspace::application::scanner::master_db::catalog_import::install(
            import_state.inner(),
            &app_data_dir,
            &staging_token,
        )
        .await?;
    crate::modules::workspace::application::scanner::master_db::catalog_import::record_provenance(
        pool.inner(),
        &provenance,
    )
    .await?;
    activate_catalog_pack_service(&app).await
}

/// Read the latest cached identity-suggestion state for one game. This command
/// never starts disk inspection; an explicit user action owns that lifecycle.
#[tauri::command]
#[specta::specta]
pub async fn get_object_identity_suggestion_status(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<
    crate::modules::catalog::application::objects::identity_suggestions::ObjectIdentitySuggestionStatus,
    AppError,
>{
    crate::modules::catalog::application::objects::identity_suggestions::status(
        &app,
        pool.inner(),
        &game_id,
    )
    .await
}

/// Page review candidates without sending the whole result set to Dashboard.
#[tauri::command]
#[specta::specta]
pub async fn list_object_identity_suggestions(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    offset: u32,
    limit: u32,
) -> Result<
    crate::modules::catalog::application::objects::identity_suggestions::ObjectIdentitySuggestionPage,
    AppError,
>{
    crate::modules::catalog::application::objects::identity_suggestions::list(
        &app,
        pool.inner(),
        &game_id,
        offset,
        limit,
    )
    .await
}

/// Start a user-requested local catalog inspection for a game or object root.
#[tauri::command]
#[specta::specta]
pub async fn retry_object_identity_suggestions(
    app: tauri::AppHandle,
    game_id: String,
    changed_roots: Option<Vec<String>>,
) -> Result<(), AppError> {
    crate::modules::catalog::application::objects::identity_suggestions::schedule(
        app,
        game_id,
        changed_roots,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_object_identity_suggestion(
    app: tauri::AppHandle,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<(), AppError> {
    crate::modules::catalog::application::objects::identity_suggestions::dismiss(
        &app,
        pool.inner(),
        &game_id,
        &object_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn reset_object_identity_suggestion_dismissals(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<(), AppError> {
    crate::modules::catalog::application::objects::identity_suggestions::reset_dismissals(
        pool.inner(),
        &game_id,
    )
    .await
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
    let db =
        crate::modules::workspace::application::scanner::master_db::get_cached(&app, game_type)
            .await?;

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
