use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::services::runtime_mutation_engine::{
    toggle_mods_mixed, RuntimeToggleBatchRequest, RuntimeToggleOperation, RuntimeToggleTarget,
};

/// Step 6: Batch rename mod folders and persist DB projection.
pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let _guard = crate::services::scanner::watcher::SuppressionGuard::new(&ctx.suppressor);
    let to_enable = load_targets_for_keys(ctx, &ctx.to_enable).await?;
    let to_disable = load_targets_for_keys(ctx, &ctx.to_disable).await?;
    let enable_count = to_enable.len();
    let disable_count = to_disable.len();
    let mut operations = Vec::with_capacity(enable_count + disable_count);
    operations.extend(to_enable.into_iter().map(|target| RuntimeToggleOperation {
        id: target.id,
        folder_path: target.folder_path,
        target_enabled: true,
        disabled_reason: None,
    }));
    operations.extend(to_disable.into_iter().map(|target| RuntimeToggleOperation {
        id: target.id,
        folder_path: target.folder_path,
        target_enabled: false,
        disabled_reason: Some("COLLECTION".to_string()),
    }));

    let result = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: ctx.game_id.clone(),
            mods_path: ctx.mods_path.clone(),
            operations,
        },
    )
    .await
    .map_err(CollectionError::Io)?;

    ctx.mods_enabled = result.enabled_count;
    ctx.mods_disabled = result.disabled_count;
    ctx.warnings.extend(result.warnings);

    log::info!(
        "apply_pipeline[batch_rename]: {} enabled, {} disabled",
        ctx.mods_enabled,
        ctx.mods_disabled
    );

    Ok(())
}

async fn load_targets_for_keys(
    ctx: &ApplyContext,
    keys: &[String],
) -> Result<Vec<RuntimeToggleTarget>, CollectionError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query(
        r#"
        SELECT id, folder_path, folder_path_key
        FROM mods
        WHERE game_id = ? AND is_safe = ?
        "#,
    )
    .bind(&ctx.game_id)
    .bind(if ctx.is_safe { 1i32 } else { 0i32 })
    .fetch_all(&ctx.pool)
    .await?;
    let desired: std::collections::HashSet<String> =
        keys.iter().map(|key| key.to_lowercase()).collect();
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let mut targets = Vec::new();

    for row in rows {
        use sqlx::Row;
        let id: String = row.get("id");
        let folder_path: String = row.get("folder_path");
        let folder_path_key: Option<String> = row.try_get("folder_path_key").ok();
        let normalized_key = normalized_enabled_key(&folder_path, Some(&mods_path));

        let matches = folder_path_key
            .as_deref()
            .is_some_and(|key| desired.contains(&key.to_lowercase()))
            || desired.contains(&normalized_key);

        if matches {
            targets.push(RuntimeToggleTarget { id, folder_path });
        }
    }

    Ok(targets)
}

fn normalized_enabled_key(path: &str, mods_path: Option<&str>) -> String {
    let clean_path = path
        .split(['/', '\\'])
        .map(|segment| crate::services::mods::core_ops::standardize_prefix(segment, true))
        .collect::<Vec<_>>()
        .join("/");
    crate::services::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}
