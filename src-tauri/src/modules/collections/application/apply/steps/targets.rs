use crate::modules::collections::application::apply::apply_pipeline::ApplyContext;
use crate::modules::mutation::application::workspace_mutation::engine::RuntimeToggleTarget;
use crate::shared::errors::CollectionError;
use std::collections::{HashMap, HashSet};
pub(super) async fn load_targets_by_key(
    ctx: &ApplyContext,
) -> Result<HashMap<String, RuntimeToggleTarget>, CollectionError> {
    let root_keys = ctx
        .to_enable
        .iter()
        .chain(&ctx.to_disable)
        .map(|key| key.to_lowercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut conn = ctx.pool.acquire().await?;
    let rows = crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile_scope(
        &mut conn,
        &ctx.game_id,
        &root_keys,
        &[],
    )
    .await?;
    drop(conn);
    let mods_path = ctx.mods_path.to_string_lossy().to_string();
    let mut by_key = HashMap::with_capacity(rows.len() * 2);
    for row in rows {
        let target = RuntimeToggleTarget {
            id: row.id,
            folder_path: row.folder_path.clone(),
        };
        by_key.insert(
            normalized_enabled_key(&row.folder_path, Some(&mods_path)),
            target.clone(),
        );
        by_key.insert(row.folder_path_key.to_lowercase(), target);
    }
    Ok(by_key)
}

pub(super) fn pick_targets(
    by_key: &HashMap<String, RuntimeToggleTarget>,
    keys: &[String],
) -> Vec<RuntimeToggleTarget> {
    let mut seen = HashSet::with_capacity(keys.len());
    keys.iter()
        .filter_map(|key| by_key.get(&key.to_lowercase()))
        .filter(|target| seen.insert(target.id.clone()))
        .cloned()
        .collect()
}

fn normalized_enabled_key(path: &str, mods_path: Option<&str>) -> String {
    let clean_path = path
        .split(['/', '\\'])
        .map(|segment| {
            crate::modules::library::application::mods::core_ops::standardize_prefix(segment, true)
        })
        .collect::<Vec<_>>()
        .join("/");
    crate::shared::path_key::folder_path_key(&clean_path, mods_path).to_lowercase()
}
