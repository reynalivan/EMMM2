use sqlx::SqliteConnection;

use super::path_transition::rewrite_descendant_path;
use crate::modules::collections::adapters::sqlite::{self as storage, safe_mode};
use crate::modules::collections::domain::collection::{
    CollectionPathRewrite, ProjectedCollectionState,
};
use crate::shared::errors::CollectionError;
use crate::shared::path_key::{folder_path_key, strip_path_prefix_preserve_display};

pub(crate) async fn rewrite_safe_mode_references_tx(
    conn: &mut SqliteConnection,
    game_id: &str,
    rewrites: &[CollectionPathRewrite],
) -> Result<(), CollectionError> {
    if rewrites.is_empty() {
        return Ok(());
    }
    let mods_path: Option<String> = sqlx::query_scalar("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(&mut *conn)
        .await?;
    let objects = storage::get_live_objects_tx(conn, game_id).await?;
    let mut ordered = rewrites.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|rewrite| std::cmp::Reverse(rewrite.from.len()));
    let rewrite = |state: &mut ProjectedCollectionState| -> Result<(), CollectionError> {
        for root in &mut state.active_roots {
            let Some(path) = rewrite_path(&root.source_path, &ordered, mods_path.as_deref()) else {
                continue;
            };
            root.source_path = path;
            root.root_key = folder_path_key(&root.source_path, mods_path.as_deref());
            if let Some(hint) = root.thumbnail_hint.as_mut() {
                if let Some(path) = rewrite_path(hint, &ordered, mods_path.as_deref()) {
                    *hint = path;
                }
            }
            if let Some(object) = objects
                .iter()
                .filter(|object| {
                    object.path_key.as_deref().is_some_and(|path| {
                        strip_path_prefix_preserve_display(
                            &root.source_path,
                            path,
                            mods_path.as_deref(),
                        )
                        .is_some()
                    })
                })
                .max_by_key(|object| object.path_key.as_ref().map_or(0, String::len))
            {
                root.object_id.clone_from(&object.object_id);
            }
        }
        for object in &mut state.object_states {
            if let Some(path) = rewrite_path(&object.path_key, &ordered, mods_path.as_deref()) {
                object.path_key = folder_path_key(&path, mods_path.as_deref());
            }
        }
        let keys = state
            .active_roots
            .iter()
            .map(|root| root.root_key.as_str())
            .collect::<std::collections::HashSet<_>>();
        if keys.len() != state.active_roots.len() {
            return Err(CollectionError::Validation(
                "Safe Mode snapshot paths collide after a move".to_string(),
            ));
        }
        Ok(())
    };
    if let Some(mut state) = safe_mode::get_snapshot_tx(conn, game_id).await? {
        rewrite(&mut state)?;
        safe_mode::set_snapshot_tx(conn, game_id, Some(&state)).await?;
    }
    for (id, mut intent) in safe_mode::get_game_intents_tx(conn, game_id).await? {
        rewrite(&mut intent.target)?;
        rewrite(&mut intent.rollback)?;
        if let Some(state) = intent.previous_restore.as_mut() {
            rewrite(state)?;
        }
        safe_mode::update_intent_snapshots_tx(conn, &id, &intent).await?;
    }
    Ok(())
}

fn rewrite_path(
    path: &str,
    rewrites: &[&CollectionPathRewrite],
    mods_path: Option<&str>,
) -> Option<String> {
    rewrites.iter().find_map(|rewrite| {
        rewrite_descendant_path(path, &rewrite.from, &rewrite.to).or_else(|| {
            let root = std::path::Path::new(mods_path?);
            rewrite_descendant_path(
                path,
                &root.join(&rewrite.from).to_string_lossy(),
                &root.join(&rewrite.to).to_string_lossy(),
            )
        })
    })
}
