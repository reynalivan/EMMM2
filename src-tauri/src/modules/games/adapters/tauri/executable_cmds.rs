use crate::modules::games::{
    application::{executable_discovery::*, executable_profiles},
    domain::executable::*,
};
use crate::shared::errors::AppError;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Instant,
};
use tokio::sync::Semaphore;

#[cfg(test)]
#[path = "executable_cmds_tests.rs"]
mod tests;

fn worker() -> &'static Arc<Semaphore> {
    static WORKER: OnceLock<Arc<Semaphore>> = OnceLock::new();
    WORKER.get_or_init(|| Arc::new(Semaphore::new(1)))
}

#[tauri::command]
#[specta::specta]
pub async fn discover_game_executables(
    request: DiscoverGameExecutablesRequest,
) -> Result<GameExecutableDiscoveryResult, AppError> {
    let permit = worker().clone().try_acquire_owned().map_err(|_| {
        AppError::Validation(
            "Executable detection is already running. Try again when it finishes.".into(),
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        validate_request(&request)?;
        let mut context = DiscoveryContext::new(request.game_types, Instant::now());
        #[cfg(windows)]
        crate::modules::games::adapters::executable_registry::collect_hoyoplay(
            &mut context,
            &crate::modules::games::adapters::executable_registry::WindowsRegistry,
        );
        discover_configuration(
            &mut context,
            DiscoveryLocations {
                roaming: std::env::var_os("APPDATA").map(PathBuf::from),
                xxmi_roots: request.xxmi_roots.into_iter().map(PathBuf::from).collect(),
            },
        );
        Ok(context.finish())
    })
    .await?
}

#[tauri::command]
#[specta::specta]
pub async fn validate_game_executable(
    request: ValidateGameExecutableRequest,
) -> Result<GameExecutableCandidate, AppError> {
    // Share capacity with discovery, including after a caller drops its future.
    let permit = worker().clone().try_acquire_owned().map_err(|_| {
        AppError::Validation(
            "Executable detection is already running. Try again when it finishes.".into(),
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        executable_profiles::validate_game_executable(request.game_type, &request.executable_path)
    })
    .await?
}
