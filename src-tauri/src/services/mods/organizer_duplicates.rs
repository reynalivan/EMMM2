use crate::domain::errors::AppError;
use crate::services::mods::core_ops::standardize_prefix;
use std::path::Path;

pub async fn disable_target_duplicates(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    target_object_id: &str,
    new_rel: &str,
    base_path: &Path,
    target_obj_path: &Path,
    path_rewrites: &mut Vec<crate::domain::workspace::WorkspacePathRewrite>,
) -> Result<(), AppError> {
    use crate::common::normalizer::is_disabled_folder;

    let siblings =
        crate::repo::mod_repo::get_enabled_duplicates(pool, target_object_id, game_id, new_rel)
            .await?;
    for (_id, sibling_rel, _name) in siblings {
        let sibling_path = sibling_rel.resolve(base_path);
        let Some(sibling_name) = sibling_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if is_disabled_folder(sibling_name) || sibling_name.starts_with('.') {
            continue;
        }

        let sibling_disabled_path = sibling_path
            .parent()
            .unwrap_or(target_obj_path)
            .join(standardize_prefix(sibling_name, false));
        if sibling_disabled_path.exists()
            || std::fs::rename(&sibling_path, &sibling_disabled_path).is_err()
        {
            continue;
        }

        let sibling_new_rel = sibling_disabled_path
            .strip_prefix(base_path)
            .unwrap_or(&sibling_disabled_path)
            .to_string_lossy()
            .to_string();
        // Path-only: status derives from the DISABLED prefix via the caller's
        // scoped reconcile (single writer).
        crate::repo::mod_repo::update_mod_path_by_old_path_in_game(
            pool,
            game_id,
            sibling_rel.as_stored(),
            &sibling_new_rel,
        )
        .await?;
        path_rewrites.push(crate::domain::workspace::WorkspacePathRewrite {
            old_path: sibling_rel.into_stored(),
            new_path: sibling_new_rel,
        });
    }

    Ok(())
}
