use futures_util::StreamExt;
use uuid::Uuid;

use crate::modules::catalog::domain::objects::{
    CreateObjectInput, CreateObjectThumbnail, UpdateObjectInput,
};
use crate::shared::errors::AppError;

const REMOTE_THUMBNAIL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

fn invalid_remote_thumbnail_url(message: impl Into<String>) -> AppError {
    AppError::Validation(format!("Invalid thumbnail URL: {}", message.into()))
}

fn is_disallowed_thumbnail_address(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(ip) => {
            ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
        }
        std::net::IpAddr::V6(ip) => ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local(),
    }
}

fn validate_remote_thumbnail_url(value: &str) -> Result<reqwest::Url, AppError> {
    let url = reqwest::Url::parse(value.trim())
        .map_err(|_| invalid_remote_thumbnail_url("enter a valid HTTP(S) URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(invalid_remote_thumbnail_url(
            "only HTTP(S) URLs are allowed",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid_remote_thumbnail_url("credentials are not allowed"));
    }
    let host = url
        .host_str()
        .ok_or_else(|| invalid_remote_thumbnail_url("a host is required"))?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err(invalid_remote_thumbnail_url("local hosts are not allowed"));
    }
    let normalized_host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(address) = normalized_host.parse::<std::net::IpAddr>() {
        if is_disallowed_thumbnail_address(address) {
            return Err(invalid_remote_thumbnail_url(
                "local network addresses are not allowed",
            ));
        }
    }
    Ok(url)
}

async fn ensure_remote_thumbnail_host_is_public(url: &reqwest::Url) -> Result<(), AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| invalid_remote_thumbnail_url("a host is required"))?;
    let normalized_host = host.trim_start_matches('[').trim_end_matches(']');
    if normalized_host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let mut addresses = tokio::net::lookup_host((normalized_host, port))
        .await
        .map_err(|error| invalid_remote_thumbnail_url(format!("host lookup failed: {error}")))?;
    if addresses.any(|address| is_disallowed_thumbnail_address(address.ip())) {
        return Err(invalid_remote_thumbnail_url(
            "local network addresses are not allowed",
        ));
    }
    Ok(())
}

async fn fetch_remote_thumbnail(url: &str) -> Result<Vec<u8>, AppError> {
    let url = validate_remote_thumbnail_url(url)?;
    ensure_remote_thumbnail_host_is_public(&url).await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REMOTE_THUMBNAIL_TIMEOUT)
        .build()
        .map_err(|error| AppError::Io(format!("Failed to prepare thumbnail download: {error}")))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AppError::Io(format!("Failed to download thumbnail: {error}")))?;
    if !response.status().is_success() {
        return Err(AppError::Validation(format!(
            "Thumbnail URL returned HTTP {}",
            response.status()
        )));
    }

    if response
        .content_length()
        .is_some_and(|length| length > 10 * 1024 * 1024)
    {
        return Err(AppError::Validation(
            "Thumbnail image is larger than 10MB".to_string(),
        ));
    }

    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|error| AppError::Io(format!("Failed to download thumbnail: {error}")))?;
        if bytes.len().saturating_add(chunk.len()) > 10 * 1024 * 1024 {
            return Err(AppError::Validation(
                "Thumbnail image is larger than 10MB".to_string(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn materialize_thumbnail_source(source: &CreateObjectThumbnail) -> Result<Vec<u8>, AppError> {
    let bytes = match source {
        CreateObjectThumbnail::File { source_path } => {
            std::fs::read(source_path).map_err(|error| {
                AppError::Io(format!(
                    "Failed to read thumbnail file '{source_path}': {error}"
                ))
            })?
        }
        CreateObjectThumbnail::Clipboard { image_data } => image_data.clone(),
        CreateObjectThumbnail::Url { url } => fetch_remote_thumbnail(url).await?,
    };
    crate::modules::library::application::mods::preview_ops::normalize_thumbnail_png(&bytes)
}

fn normalize_object_category(category: &str) -> Result<&str, AppError> {
    let category = category.trim();
    if matches!(category, "Character" | "Weapon" | "UI" | "Other") {
        Ok(category)
    } else {
        Err(AppError::Validation(
            "Category must be Character, Weapon, UI, or Other".to_string(),
        ))
    }
}

pub struct PreparedObjectCreate {
    stage: std::path::PathBuf,
    target: std::path::PathBuf,
}

impl PreparedObjectCreate {
    pub fn prepare(&self) -> Result<(), AppError> {
        std::fs::create_dir(&self.stage).map_err(AppError::from)
    }

    pub fn journal_step(&self) -> crate::modules::mutation::journal::PlannedStep {
        crate::modules::mutation::journal::PlannedStep::rename(
            0,
            self.stage.clone(),
            self.target.clone(),
        )
    }

    pub fn promote(&self) -> Result<(), AppError> {
        std::fs::rename(&self.stage, &self.target).map_err(AppError::from)
    }

    pub fn rollback(&self) -> Result<(), AppError> {
        if self.target.exists() && !self.stage.exists() {
            std::fs::rename(&self.target, &self.stage)?;
        }
        if self.stage.exists() {
            std::fs::remove_dir_all(&self.stage)?;
        }
        Ok(())
    }
}

pub async fn prepare_object_create(
    pool: &sqlx::SqlitePool,
    input: &CreateObjectInput,
) -> Result<PreparedObjectCreate, AppError> {
    normalize_object_category(&input.object_type)?;
    let folder_path = input.folder_path.as_deref().unwrap_or(&input.name);
    validate_relative_object_folder(folder_path)?;
    let mods_path = crate::modules::games::adapters::sqlite::game::get_configured_mods_path(
        pool,
        &input.game_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Game mods path not configured".to_string()))?;
    let target = std::path::Path::new(&mods_path).join(folder_path);
    if target.exists() {
        return Err(AppError::Validation(format!(
            "Object folder already exists: {}",
            target.display()
        )));
    }
    let stage = target.with_file_name(format!(".emmm-object-create-{}", Uuid::new_v4().simple()));
    Ok(PreparedObjectCreate { stage, target })
}

pub struct PreparedObjectDelete {
    source: std::path::PathBuf,
    quarantine: std::path::PathBuf,
}

impl PreparedObjectDelete {
    pub fn journal_step(&self) -> crate::modules::mutation::journal::PlannedStep {
        crate::modules::mutation::journal::PlannedStep::quarantine(
            0,
            self.source.clone(),
            self.quarantine.clone(),
        )
    }

    pub fn execute(
        &self,
        watcher: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    ) -> Result<(), AppError> {
        let _guard = watcher
            .suppressor
            .suppress_paths([self.source.as_path(), self.quarantine.as_path()]);
        std::fs::rename(&self.source, &self.quarantine).map_err(AppError::from)
    }

    pub fn finalize(&self) -> Result<(), AppError> {
        if self.quarantine.exists() {
            crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&self.quarantine)
                .map_err(|error| AppError::Io(error.to_string()))?;
        }
        Ok(())
    }
}

pub async fn prepare_object_delete(
    pool: &sqlx::SqlitePool,
    id: &str,
    force: bool,
) -> Result<Option<PreparedObjectDelete>, AppError> {
    let (game_id, folder_path) =
        crate::modules::catalog::adapters::sqlite::object::get_game_id_and_folder_path(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {id}")))?;
    let count =
        crate::modules::catalog::adapters::sqlite::object::get_mod_count_for_object(pool, id)
            .await?;
    if count > 0 && !force {
        return Err(AppError::ObjectHasMods(count as i32));
    }
    let Some(folder_path) = folder_path else {
        return Ok(None);
    };
    let Some(mods_path) =
        crate::modules::games::adapters::sqlite::game::get_configured_mods_path(pool, &game_id)
            .await?
    else {
        return Ok(None);
    };
    let source = std::path::Path::new(&mods_path).join(folder_path);
    if !source.exists() {
        return Ok(None);
    }
    let quarantine =
        source.with_file_name(format!(".emmm-object-delete-{}", Uuid::new_v4().simple()));
    Ok(Some(PreparedObjectDelete { source, quarantine }))
}

pub async fn create_object_cmd_inner(
    pool: &sqlx::SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    input: CreateObjectInput,
) -> Result<String, AppError> {
    let object_type = normalize_object_category(&input.object_type)?;
    let id = Uuid::new_v4().to_string();
    let metadata_str = input
        .metadata
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let folder_path = input.folder_path.unwrap_or_else(|| input.name.clone());
    validate_relative_object_folder(&folder_path)?;

    let mut thumbnail_abs_path: Option<String> = None;
    let mut pending_thumbnail = None;
    let mut previous_thumbnail = None;

    let mods_path = crate::modules::games::adapters::sqlite::game::get_configured_mods_path(
        pool,
        &input.game_id,
    )
    .await
    .map_err(|e| AppError::Db(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Game mods path not configured".to_string()))?;
    let target_dir = std::path::Path::new(&mods_path).join(&folder_path);
    if let Some((attempted_path, existing_path, base_name)) = find_new_path_identity_conflict(
        std::path::Path::new(&mods_path),
        std::path::Path::new(&folder_path),
    ) {
        return Err(
            crate::modules::library::application::mods::core_ops::rename_conflict_error(
                &attempted_path,
                &existing_path,
                &base_name,
            ),
        );
    }

    if let Some(source) = input.thumbnail.as_ref() {
        let destination = target_dir.join("preview_custom.png");
        thumbnail_abs_path = Some(destination.to_string_lossy().to_string());
        pending_thumbnail = Some((destination, materialize_thumbnail_source(source).await?));
    } else if let (Some(thumb), Some(app)) = (&input.thumbnail_url, app_handle) {
        use tauri::Manager;
        if let Ok(app_data_dir) = app.path().app_data_dir() {
            let asset_root = app_data_dir.join("asset-pack");
            let source_thumb = std::path::PathBuf::from(thumb);
            if source_thumb.is_absolute()
                && source_thumb.is_file()
                && source_thumb.starts_with(&asset_root)
            {
                let ext = source_thumb.extension().unwrap_or_default();
                let dest_thumb = target_dir.join(format!("preview.{}", ext.to_string_lossy()));

                let thumbnail_bytes = std::fs::read(&source_thumb).map_err(|error| {
                    AppError::Io(format!(
                        "Failed to read object thumbnail '{}': {error}",
                        source_thumb.display()
                    ))
                })?;
                thumbnail_abs_path = Some(dest_thumb.to_string_lossy().to_string());
                pending_thumbnail = Some((dest_thumb, thumbnail_bytes));
            }
        }
    }

    let created_folder = !target_dir.exists();
    std::fs::create_dir_all(&target_dir).map_err(|error| {
        AppError::Io(format!(
            "Failed to create object folder '{}': {error}",
            target_dir.display()
        ))
    })?;

    if !target_dir.is_dir() {
        return Err(AppError::Io(format!(
            "Failed to create object folder '{}': target is not a directory",
            target_dir.display()
        )));
    }

    if let Some((dest, thumbnail_bytes)) = &pending_thumbnail {
        previous_thumbnail = Some(match std::fs::read(dest) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                cleanup_created_object_folder(&target_dir, created_folder);
                return Err(error.into());
            }
        });
        crate::platform::fs::atomic_file::atomic_write(dest, &thumbnail_bytes).map_err(
            |error| {
                cleanup_created_object_folder(&target_dir, created_folder);
                AppError::Io(format!(
                    "Failed to save object thumbnail to '{}': {error}",
                    dest.display()
                ))
            },
        )?;
        crate::platform::images::thumbnail_cache::ThumbnailCache::invalidate(dest);
    }

    let res = crate::modules::catalog::adapters::sqlite::object::create_object(
        pool,
        &id,
        &input.game_id,
        &input.name,
        &folder_path,
        object_type,
        input.sub_category.as_ref(),
        input.status,
        &metadata_str,
        thumbnail_abs_path.as_ref(),
        None,
        None,
    )
    .await;

    match res {
        Ok(_) => {
            crate::modules::workspace::adapters::sqlite::runtime_projection::refresh_object_projection(
                pool,
                &input.game_id,
                &id,
            )
            .await
            .map_err(|e| AppError::Db(e.to_string()))?;

            Ok(id)
        }
        Err(e) => {
            if let Some((destination, _)) = pending_thumbnail.as_ref() {
                let rollback = match previous_thumbnail.as_ref() {
                    Some(Some(bytes)) => {
                        crate::platform::fs::atomic_file::atomic_write(destination, bytes)
                    }
                    Some(None) if destination.exists() => {
                        std::fs::remove_file(destination).map_err(AppError::from)
                    }
                    _ => Ok(()),
                };
                if let Err(rollback_error) = rollback {
                    cleanup_created_object_folder(&target_dir, created_folder);
                    return Err(AppError::Io(format!(
                        "Object database insert failed ({e}); thumbnail rollback failed: {rollback_error}"
                    )));
                }
            }
            cleanup_created_object_folder(&target_dir, created_folder);
            if is_object_name_conflict(&e) {
                Err(AppError::Db(format!(
                    "An object named '{}' already exists for this game.",
                    input.name.trim()
                )))
            } else {
                Err(e.into())
            }
        }
    }
}

/// SQLite names the objects(game_id, name) unique index differently across
/// versions; both spellings mean the same collision.
fn find_new_path_identity_conflict(
    root: &std::path::Path,
    relative_path: &std::path::Path,
) -> Option<(std::path::PathBuf, std::path::PathBuf, String)> {
    let mut parent = root.to_path_buf();
    for component in relative_path.components() {
        let std::path::Component::Normal(name) = component else {
            continue;
        };
        let target_name = name.to_string_lossy();
        let attempted_path = parent.join(name);
        let Some(existing_path) =
            crate::modules::library::application::mods::core_ops::find_sibling_identity_collision(
                &parent,
                &target_name,
                None,
            )
        else {
            parent = attempted_path;
            continue;
        };
        let existing_name = existing_path.file_name()?.to_string_lossy();
        if existing_name.eq_ignore_ascii_case(&target_name) {
            parent = existing_path;
            continue;
        }
        return Some((
            attempted_path,
            existing_path,
            crate::modules::workspace::domain::normalizer::normalize_display_name(&target_name)
                .into_owned(),
        ));
    }
    None
}

fn is_object_name_conflict(error: &sqlx::Error) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("unique constraint failed") || message.contains("idx_objects_game_name")
}

fn validate_relative_object_folder(folder_path: &str) -> Result<(), AppError> {
    let trimmed = folder_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Object folder path cannot be empty".to_string(),
        ));
    }

    let path = std::path::Path::new(trimmed);
    if path.is_absolute() {
        return Err(AppError::Validation(
            "Object folder path must be relative".to_string(),
        ));
    }

    // Anything but plain names — `..`, a root, a drive prefix — could escape the
    // mods tree once joined onto it.
    if !path
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return Err(AppError::Validation(
            "Object folder path contains invalid components".to_string(),
        ));
    }

    Ok(())
}

fn cleanup_created_object_folder(path: &std::path::Path, created_folder: bool) {
    if !created_folder {
        return;
    }

    if let Err(error) = std::fs::remove_dir(path) {
        log::warn!(
            "Failed to remove object folder '{}' after create failure: {}",
            path.display(),
            error
        );
    }
}

#[cfg(test)]
mod thumbnail_url_tests {
    use super::validate_remote_thumbnail_url;

    #[test]
    fn remote_thumbnail_urls_reject_unsafe_origins() {
        for url in [
            "file:///C:/thumbnail.png",
            "http://localhost/thumbnail.png",
            "http://127.0.0.1/thumbnail.png",
            "http://192.168.1.5/thumbnail.png",
            "http://[::1]/thumbnail.png",
        ] {
            assert!(validate_remote_thumbnail_url(url).is_err(), "{url}");
        }
    }

    #[test]
    fn remote_thumbnail_urls_allow_public_http_and_https() {
        assert!(validate_remote_thumbnail_url("https://example.com/thumbnail.png").is_ok());
        assert!(validate_remote_thumbnail_url("http://example.com/thumbnail.png").is_ok());
    }
}

/// Toggle the pinned state of an object.
pub async fn toggle_pin_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    pin: bool,
) -> Result<(), AppError> {
    Ok(crate::modules::catalog::adapters::sqlite::object::set_is_pinned(pool, id, pin).await?)
}

/// Update an object, returning a user-friendly error on unique-name conflicts.
pub async fn update_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    updates: &UpdateObjectInput,
) -> Result<(), AppError> {
    let mut normalized_updates = updates.clone();
    if let Some(category) = updates.object_type.as_deref() {
        normalized_updates.object_type = Some(normalize_object_category(category)?.to_string());
    }
    let mut tx = pool.begin().await?;
    let object_game_id =
        crate::modules::catalog::adapters::sqlite::object::get_game_id_conn(&mut tx, id).await?;
    let update_result = async {
        crate::modules::catalog::adapters::sqlite::object::update_object(
            &mut *tx,
            id,
            &normalized_updates,
        )
        .await?;
        if let Some(game_id) = object_game_id.as_deref() {
            if let Some(category) = normalized_updates.object_type.as_deref() {
                crate::modules::library::adapters::sqlite::mods::set_object_type_for_object(
                    &mut *tx,
                    game_id,
                    id,
                    category,
                )
                .await?;
            }
            crate::modules::workspace::adapters::sqlite::runtime_projection::refresh_projection_for_object_ids_tx(
                &mut tx,
                game_id,
                [id.to_string()],
            )
            .await?;
        }
        tx.commit().await
    }
    .await;

    match update_result {
        Ok(()) => Ok(()),
        Err(e) if is_object_name_conflict(&e) => Err(AppError::Db(
            "An object with that name already exists.".to_string(),
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn set_object_and_mods_category(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
    category: &str,
) -> Result<usize, AppError> {
    let category = normalize_object_category(category)?;

    let mut tx = pool.begin().await?;
    let object_updated =
        crate::modules::catalog::adapters::sqlite::object::update_object_type_for_game(
            &mut *tx, game_id, object_id, category,
        )
        .await?;
    if object_updated == 0 {
        return Err(AppError::NotFound(format!(
            "Object '{object_id}' was not found for game '{game_id}'"
        )));
    }

    let child_updated =
        crate::modules::library::adapters::sqlite::mods::set_object_type_for_object(
            &mut *tx, game_id, object_id, category,
        )
        .await?;
    crate::modules::workspace::adapters::sqlite::runtime_projection::refresh_projection_for_object_ids_tx(
        &mut tx,
        game_id,
        [object_id.to_string()],
    )
    .await?;
    tx.commit().await?;
    Ok(child_updated as usize)
}
/// Delete an object on disk. The command's trailing full reconcile is the
/// single writer that removes object/mod projections and records collection
/// members as missing before those runtime rows disappear.
pub async fn delete_object(
    pool: &sqlx::SqlitePool,
    id: &str,
    force: bool,
    watcher_state: &crate::modules::workspace::application::scanner::watcher::WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<(), AppError> {
    let _guard = crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(
        &watcher_state.suppressor,
    );
    // 1. Fetch object from DB to get game_id and folder_path
    let (obj_game_id, obj_folder_path) =
        crate::modules::catalog::adapters::sqlite::object::get_game_id_and_folder_path(pool, id)
            .await
            .map_err(|e| AppError::Db(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {}", id)))?;

    let mut target_dir_opt: Option<std::path::PathBuf> = None;

    let mods_path =
        crate::modules::games::adapters::sqlite::game::get_configured_mods_path(pool, &obj_game_id)
            .await
            .map_err(|e| AppError::Db(e.to_string()))?;

    if let (Some(mods_path), Some(folder_path)) = (mods_path, obj_folder_path.as_ref()) {
        target_dir_opt = Some(std::path::Path::new(&mods_path).join(folder_path));
    }

    // 1.5. Safety Guard: Check if the object has any mods
    let count =
        crate::modules::catalog::adapters::sqlite::object::get_mod_count_for_object(pool, id)
            .await?;
    if count > 0 && !force {
        return Err(AppError::ObjectHasMods(count as i32));
    }

    // 2. Move folder to trash (if it exists on disk)
    if let Some(target_dir) = target_dir_opt {
        if target_dir.exists() {
            log::info!("delete_object: moving {:?} to trash", target_dir);
            crate::modules::library::application::mods::trash::move_to_trash(&target_dir).map_err(
                |e| {
                    log::error!("delete_object: trash move failed: {}", e);
                    AppError::Io(format!(
                        "Failed to move folder '{}' to trash. {}",
                        target_dir.display(),
                        e
                    ))
                },
            )?;
            log::info!("delete_object: successfully trashed {:?}", target_dir);
        } else {
            log::info!(
                "delete_object: dir {:?} does not exist, skipping trash",
                target_dir
            );
        }
    } else {
        log::warn!(
            "delete_object: could not resolve folder path for object id={}",
            id
        );
    }

    Ok(())
}
