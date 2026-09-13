use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::modules::reconciliation::application::disk_reconcile::types::{
    IndexingRootWork, OnboardingIndexingWorkPlan,
};
use crate::modules::settings::application::config::GameConfig;
use crate::shared::errors::AppError;

const FILE_METADATA_WORK_BYTES: u64 = 64 * 1024;
const MINIMUM_ROOT_WORK_UNITS: u64 = 1;

pub fn plan_onboarding_indexing_work(
    games: &[GameConfig],
) -> Result<Vec<OnboardingIndexingWorkPlan>, AppError> {
    games.iter().map(plan_game_work).collect()
}

fn plan_game_work(game: &GameConfig) -> Result<OnboardingIndexingWorkPlan, AppError> {
    let roots = list_mod_roots(&game.mod_path)?
        .into_iter()
        .map(|(root_name, root_path)| measure_root_work(root_name, root_path))
        .collect::<Result<Vec<_>, _>>()?;
    let file_count = roots.iter().map(|root| root.file_count).sum();
    let total_bytes = roots.iter().map(|root| root.total_bytes).sum();
    let work_units = roots.iter().map(|root| root.work_units).sum();

    Ok(OnboardingIndexingWorkPlan {
        game_id: game.id.clone(),
        file_count,
        total_bytes,
        work_units,
        roots,
    })
}

fn list_mod_roots(mods_path: &Path) -> Result<Vec<(String, PathBuf)>, AppError> {
    if !mods_path.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(mods_path).map_err(|error| {
        AppError::Io(format!(
            "Could not read mods directory '{}': {error}",
            mods_path.display()
        ))
    })?;
    let mut roots = entries
        .map(|entry| {
            let entry = entry.map_err(|error| {
                AppError::Io(format!(
                    "Could not read an entry in '{}': {error}",
                    mods_path.display()
                ))
            })?;
            let file_type = entry.file_type().map_err(|error| {
                AppError::Io(format!(
                    "Could not inspect '{}': {error}",
                    entry.path().display()
                ))
            })?;
            Ok(file_type.is_dir().then(|| {
                (
                    entry.file_name().to_string_lossy().to_string(),
                    entry.path(),
                )
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(roots)
}

fn measure_root_work(root_name: String, root_path: PathBuf) -> Result<IndexingRootWork, AppError> {
    let mut file_count = 0_u64;
    let mut total_bytes = 0_u64;
    for entry in WalkDir::new(&root_path).follow_links(false) {
        let entry = entry.map_err(|error| {
            AppError::Io(format!("Could not scan '{}': {error}", root_path.display()))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| {
            AppError::Io(format!(
                "Could not inspect '{}': {error}",
                entry.path().display()
            ))
        })?;
        file_count = file_count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(metadata.len());
    }
    let work_units = total_bytes
        .saturating_add(file_count.saturating_mul(FILE_METADATA_WORK_BYTES))
        .max(MINIMUM_ROOT_WORK_UNITS);

    Ok(IndexingRootWork {
        root_name,
        file_count,
        total_bytes,
        work_units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::games::domain::models::{GameType, LaunchMode};

    #[test]
    fn work_plan_combines_file_count_and_file_sizes() {
        let temp = tempfile::tempdir().unwrap();
        let mods_path = temp.path().join("Mods");
        let root_path = mods_path.join("UI");
        fs::create_dir_all(&root_path).unwrap();
        fs::write(root_path.join("config.ini"), b"abc").unwrap();
        fs::write(root_path.join("mesh.buf"), vec![0_u8; 100]).unwrap();
        let game = GameConfig {
            id: "game-1".to_string(),
            name: "Test Game".to_string(),
            game_type: GameType::GIMI,
            instance_path: temp.path().to_path_buf(),
            mod_path: mods_path,
            ready_to_move_path: None,
            launch_mode: LaunchMode::Standalone,
            game_exe: None,
            loader_exe: None,
            xxmi_launcher_exe: None,
            launch_args: None,
            warnings: Vec::new(),
        };

        let plan = plan_onboarding_indexing_work(&[game]).unwrap();

        assert_eq!(plan[0].file_count, 2);
        assert_eq!(plan[0].total_bytes, 103);
        assert_eq!(plan[0].roots[0].root_name, "UI");
        assert_eq!(
            plan[0].roots[0].work_units,
            2 * FILE_METADATA_WORK_BYTES + 103
        );
    }
}
