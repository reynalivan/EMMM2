use crate::modules::mutation::journal::{MutationStepKind, PlannedStep};
use crate::shared::errors::AppError;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Freeze replay ownership in the namespace that exists before any step runs.
/// A later step can name a child through an earlier ancestor's destination.
pub fn original_file_lock_scope(steps: &[PlannedStep]) -> Result<Vec<(PathBuf, String)>, AppError> {
    let mut ordered = steps.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|step| step.sequence);
    let mut prior = Vec::<(&PathBuf, &PathBuf)>::new();
    let mut originals = BTreeMap::<PathBuf, String>::new();
    for step in ordered {
        if !matches!(
            step.kind,
            MutationStepKind::Rename | MutationStepKind::Quarantine
        ) {
            continue;
        }
        let invalid = || {
            AppError::Validation("Transactional file-lock recovery lacks frozen source ownership; refresh before retrying".into())
        };
        let old = step.old_path.as_ref().ok_or_else(invalid)?;
        let new = step.new_path.as_ref().ok_or_else(invalid)?;
        if old == new {
            continue;
        }
        let identity = step
            .expected_identity
            .as_ref()
            .filter(|identity| !identity.is_empty())
            .ok_or_else(invalid)?;
        let mut original = old.clone();
        for (before, after) in prior.iter().rev() {
            if let Ok(suffix) = original.strip_prefix(after) {
                original = before.join(suffix);
            }
        }
        if let Some(known) = originals.get(&original) {
            if known != identity {
                return Err(AppError::Validation(
                    "Transactional retry has conflicting original filesystem identities".into(),
                ));
            }
        } else {
            originals.insert(original, identity.clone());
        }
        prior.push((old, new));
    }
    Ok(originals.into_iter().collect())
}

pub fn original_file_lock_scope_with_selection(
    steps: &[PlannedStep],
    selected_sources: &[(PathBuf, String)],
) -> Result<Vec<(PathBuf, String)>, AppError> {
    let mut originals = original_file_lock_scope(steps)?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    for (path, identity) in selected_sources {
        if identity.is_empty() || originals.get(path).is_some_and(|known| known != identity) {
            return Err(AppError::Validation(
                "Selected retry source has missing or conflicting original filesystem ownership"
                    .into(),
            ));
        }
        originals
            .entry(path.clone())
            .or_insert_with(|| identity.clone());
    }
    Ok(originals.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rename(sequence: u32, old: PathBuf, new: PathBuf, identity: &str) -> PlannedStep {
        PlannedStep::rename(sequence, old, new).with_expected_identity(Some(identity.into()))
    }

    #[test]
    fn nested_blocked_child_returns_to_original_scope_after_ancestor_rollback() {
        let root = std::env::temp_dir().join("original-lock-scope");
        let steps = [
            rename(0, root.join("DISABLED Alice"), root.join("Alice"), "parent"),
            rename(
                1,
                root.join("Alice/DISABLED Group"),
                root.join("Alice/Group"),
                "group",
            ),
            rename(
                2,
                root.join("Alice/Group/DISABLED Skin"),
                root.join("Alice/Group/Skin"),
                "child",
            ),
        ];
        let scope = original_file_lock_scope(&steps).unwrap();
        assert!(scope.contains(&(root.join("DISABLED Alice"), "parent".into())));
        assert!(scope.contains(&(root.join("DISABLED Alice/DISABLED Group"), "group".into())));
        assert!(scope.contains(&(
            root.join("DISABLED Alice/DISABLED Group/DISABLED Skin"),
            "child".into()
        )));
    }

    #[test]
    fn staged_publication_deduplicates_one_original_physical_source() {
        let root = std::env::temp_dir().join("original-lock-scope");
        let steps = [
            rename(0, root.join("A"), root.join("stage"), "same"),
            rename(1, root.join("stage"), root.join("B"), "same"),
        ];
        assert_eq!(
            original_file_lock_scope(&steps).unwrap(),
            vec![(root.join("A"), "same".into())]
        );
    }

    #[test]
    fn move_scope_preserves_selected_noop_a_while_b_needs_mutation() {
        let root = std::env::temp_dir().join("original-lock-scope");
        let selection = [
            (root.join("A"), "noop-a-original".into()),
            (root.join("B"), "blocked-b-original".into()),
        ];
        let steps = [rename(
            0,
            root.join("B"),
            root.join("Target/B"),
            "blocked-b-original",
        )];
        let scope = original_file_lock_scope_with_selection(&steps, &selection).unwrap();
        assert_eq!(scope, selection);
        assert!(original_file_lock_scope_with_selection(
            &steps,
            &[(root.join("B"), "replacement-b".into())]
        )
        .is_err());
    }

    #[test]
    fn missing_or_conflicting_proof_cannot_grant_replay_scope() {
        let root = std::env::temp_dir().join("original-lock-scope");
        assert!(original_file_lock_scope(&[PlannedStep::rename(
            0,
            root.join("A"),
            root.join("B")
        )])
        .is_err());
        assert!(original_file_lock_scope(&[
            rename(0, root.join("A"), root.join("B"), "first"),
            rename(1, root.join("B"), root.join("C"), "replacement")
        ])
        .is_err());
        assert!(original_file_lock_scope(&[PlannedStep::rename(
            0,
            root.join("A"),
            root.join("A")
        )])
        .unwrap()
        .is_empty());
    }

    #[test]
    fn transactional_scope_keeps_a_original_identity_when_b_is_blocked_and_a_is_replaced() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("A");
        let blocked = root.path().join("B");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&blocked).unwrap();
        let first_identity = crate::platform::fs::file_utils::filesystem_identity(&first).unwrap();
        let blocked_identity =
            crate::platform::fs::file_utils::filesystem_identity(&blocked).unwrap();
        let steps = [
            rename(
                0,
                first.clone(),
                root.path().join("MovedA"),
                &first_identity,
            ),
            rename(
                1,
                blocked.clone(),
                root.path().join("MovedB"),
                &blocked_identity,
            ),
        ];
        let frozen = original_file_lock_scope(&steps).unwrap();
        std::fs::rename(&first, root.path().join("MovedA")).unwrap();
        std::fs::rename(root.path().join("MovedA"), &first).unwrap();
        std::fs::rename(&first, root.path().join("original-A")).unwrap();
        std::fs::create_dir(&first).unwrap();
        assert_eq!(
            original_file_lock_scope(&steps).unwrap(),
            frozen,
            "retry authority must not recapture replacement A"
        );
        assert_ne!(
            crate::platform::fs::file_utils::filesystem_identity(&first),
            Some(first_identity)
        );
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(&blocked),
            Some(blocked_identity)
        );
    }
}
