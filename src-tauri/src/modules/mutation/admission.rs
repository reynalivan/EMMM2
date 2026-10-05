use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::shared::errors::AppError;

// Includes the running transaction: at most three immutable successors wait.
const MAX_IMMUTABLE_OPERATIONS_PER_GAME: usize = 4;

#[derive(Clone, Copy, Debug)]
pub enum ImmutableMutationKind {
    CollectionCapture,
    CollectionApply,
    FolderConflictFix,
    SafeMode,
    PresetApply,
    RandomizerApply,
    StructuralMutation,
    Recovery,
}

impl ImmutableMutationKind {
    fn label(self) -> &'static str {
        match self {
            Self::CollectionCapture => "collection capture",
            Self::CollectionApply => "collection apply",
            Self::FolderConflictFix => "folder conflict fix",
            Self::SafeMode => "Safe Mode switch",
            Self::PresetApply => "preset apply",
            Self::RandomizerApply => "randomizer apply",
            Self::StructuralMutation => "folder mutation",
            Self::Recovery => "operation recovery",
        }
    }
}

#[derive(Default)]
struct ImmutableAdmission {
    active_by_game: Arc<Mutex<HashMap<String, usize>>>,
}

/// Admission bounds queued immutable snapshots before callers await mutation locks.
/// It does not acquire a storage lock or alter latest-wins toggle admission.
pub struct ImmutableMutationPermit {
    game_id: String,
    active_by_game: Arc<Mutex<HashMap<String, usize>>>,
}

impl ImmutableAdmission {
    fn try_admit(
        &self,
        game_id: &str,
        kind: ImmutableMutationKind,
    ) -> Result<ImmutableMutationPermit, AppError> {
        if game_id.trim().is_empty() {
            return Err(AppError::Validation(
                "A game is required for mutation admission".to_string(),
            ));
        }
        let mut active = self.active_by_game.lock().map_err(|_| {
            AppError::Internal(
                "Immutable operation admission is unavailable; restart the app".to_string(),
            )
        })?;
        let count = active.entry(game_id.to_string()).or_default();
        if *count >= MAX_IMMUTABLE_OPERATIONS_PER_GAME {
            return Err(AppError::Validation(format!(
                "Too many pending operations for this game; wait for the current operation to finish, then retry {}",
                kind.label(),
            )));
        }
        *count += 1;
        Ok(ImmutableMutationPermit {
            game_id: game_id.to_string(),
            active_by_game: self.active_by_game.clone(),
        })
    }
}

impl Drop for ImmutableMutationPermit {
    fn drop(&mut self) {
        let mut active = match self.active_by_game.lock() {
            Ok(active) => active,
            Err(poisoned) => {
                log::error!("Immutable operation admission was poisoned while releasing a permit");
                poisoned.into_inner()
            }
        };
        if let Some(count) = active.get_mut(&self.game_id) {
            *count -= 1;
            if *count == 0 {
                active.remove(&self.game_id);
            }
        }
    }
}

pub fn admit_immutable_mutation(
    game_id: &str,
    kind: ImmutableMutationKind,
) -> Result<ImmutableMutationPermit, AppError> {
    static ADMISSION: LazyLock<ImmutableAdmission> = LazyLock::new(ImmutableAdmission::default);
    ADMISSION.try_admit(game_id, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_exhaustion_and_releases_capacity_on_drop() {
        let admission = ImmutableAdmission::default();
        let mut permits = (0..MAX_IMMUTABLE_OPERATIONS_PER_GAME)
            .map(|_| {
                admission
                    .try_admit("game-a", ImmutableMutationKind::CollectionApply)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let error = admission
            .try_admit("game-a", ImmutableMutationKind::FolderConflictFix)
            .err()
            .unwrap();
        assert!(error.to_string().contains("retry folder conflict fix"));
        drop(permits.pop());
        assert!(admission
            .try_admit("game-a", ImmutableMutationKind::FolderConflictFix)
            .is_ok());
        drop(permits);
        assert!(admission.active_by_game.lock().unwrap().is_empty());
    }

    #[test]
    fn games_have_independent_capacity_and_empty_ids_are_rejected() {
        let admission = ImmutableAdmission::default();
        let _permits = (0..MAX_IMMUTABLE_OPERATIONS_PER_GAME)
            .map(|_| {
                admission
                    .try_admit("game-a", ImmutableMutationKind::CollectionCapture)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(admission
            .try_admit("game-b", ImmutableMutationKind::SafeMode)
            .is_ok());
        assert!(admission
            .try_admit(" ", ImmutableMutationKind::CollectionApply)
            .is_err());
    }

    #[test]
    fn cancelled_future_releases_its_admission() {
        let admission = ImmutableAdmission::default();
        let permit = admission
            .try_admit("game-a", ImmutableMutationKind::PresetApply)
            .unwrap();
        let future = async move {
            let _permit = permit;
            std::future::pending::<()>().await;
        };
        drop(future);
        assert!(admission.active_by_game.lock().unwrap().is_empty());
    }
}
