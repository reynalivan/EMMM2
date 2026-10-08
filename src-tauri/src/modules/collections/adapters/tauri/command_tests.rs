use super::collection_apply_changed_disk;
use crate::modules::collections::domain::collection::ApplyResult;
use crate::modules::reconciliation::application::disk_reconcile::types::{
    FolderNameConflictCandidate, FolderNameConflictGroup,
};

#[test]
fn object_only_collection_rename_requires_terminal_disk_authority() {
    let result = ApplyResult {
        mods_enabled: 0,
        mods_disabled: 0,
        warnings: Vec::new(),
        final_state_name: None,
        partial_apply: false,
        skipped_missing_paths: Vec::new(),
        runtime_path_rewrites: vec![
            crate::modules::workspace::domain::workspace::WorkspacePathRewrite {
                old_path: "DISABLED Object".to_string(),
                new_path: "Object".to_string(),
            },
        ],
        sync_warning: None,
    };
    assert!(collection_apply_changed_disk(&result));
}

#[test]
fn normal_apply_command_has_no_fallible_reconcile_after_service_commit() {
    let source = command_sources();
    let start = source
        .find("pub async fn apply_collection(")
        .expect("apply command source");
    let remainder = &source[start..];
    let end = remainder[1..]
        .find("#[tauri::command]")
        .map(|offset| offset + 1)
        .expect("next command boundary");
    let apply_command = &remainder[..end];

    assert!(
        !apply_command.contains("run_full_internal_disk_reconcile"),
        "a committed apply must not be converted to Err by an outer reconcile"
    );
    let durable_apply = apply_command
        .find("apply_collection_durable")
        .expect("durable collection apply");
    let runtime_queue = apply_command
        .find("enqueue_runtime_sync_for_rewrites")
        .expect("async runtime queue");
    assert!(
        durable_apply < runtime_queue,
        "runtime work must be queued only after the durable apply returns"
    );
}

#[test]
fn current_runtime_snapshot_preflight_blocks_only_active_conflict_scopes() {
    let inactive_conflicts = vec![FolderNameConflictGroup {
        group_id: "blue".to_string(),
        identity: "ainoz/blue".to_string(),
        display_name: "Blue".to_string(),
        candidates: vec![FolderNameConflictCandidate {
            path: "E:/Mods/AINOZ/DISABLED Blue".to_string(),
            folder_name: "DISABLED Blue".to_string(),
            base_name: "Blue".to_string(),
            is_enabled: false,
        }],
    }];

    let active_paths = vec!["E:/Mods/AINOZ/Active".to_string()];
    assert!(!super::current_runtime_snapshot_conflicts_block(
        &inactive_conflicts,
        &active_paths,
    ));
    assert!(!super::current_runtime_snapshot_conflicts_block(
        &[],
        &active_paths,
    ));

    let active_conflicts = vec![FolderNameConflictGroup {
        group_id: "active-blue".to_string(),
        identity: "ainoz/blue".to_string(),
        display_name: "Blue".to_string(),
        candidates: vec![FolderNameConflictCandidate {
            path: "E:/Mods/AINOZ/Active".to_string(),
            folder_name: "Active".to_string(),
            base_name: "Active".to_string(),
            is_enabled: true,
        }],
    }];
    assert!(super::current_runtime_snapshot_conflicts_block(
        &active_conflicts,
        &active_paths,
    ));
}

#[test]
fn current_runtime_snapshot_commands_share_projection_barrier() {
    let source = command_sources();
    for command in [
        "pub async fn create_collection(",
        "pub async fn save_current_runtime_as_collection(",
        "pub async fn replace_collection_with_current_state(",
        "pub async fn save_collection_changes(",
    ] {
        let start = source.find(command).expect("collection command");
        let remainder = &source[start..];
        let end = remainder[1..]
            .find("#[tauri::command]")
            .map(|offset| offset + 1)
            .unwrap_or(remainder.len());
        assert!(
            remainder[..end].contains("acquire_current_snapshot_guard"),
            "{command} must reconcile pending disk changes before capturing current state"
        );
    }
}

#[test]
fn current_snapshot_retains_game_and_operation_lease_during_capture() {
    let source = command_sources();
    let start = source
        .find("async fn acquire_current_snapshot_guard(")
        .unwrap();
    let end = source[start..]
        .find("fn current_runtime_snapshot_conflicts_block(")
        .unwrap()
        + start;
    let guard = &source[start..end];
    assert!(guard.contains("acquire_nested_mutation_lease(game_id, coordinator)"));
    assert!(!guard.contains("acquire_exempt("));
    assert!(guard.contains("DiskMutationLease"));
}

#[test]
fn immutable_collection_admission_precedes_projection_and_storage_waits() {
    let source = command_sources();
    let helper_start = source
        .find("async fn acquire_current_snapshot_guard(")
        .unwrap();
    let helper_end = source[helper_start..]
        .find("struct CurrentSnapshotGuard")
        .unwrap()
        + helper_start;
    let helper = &source[helper_start..helper_end];
    assert!(
        helper.find("admit_immutable_mutation").unwrap()
            < helper
                .find("ensure_current_runtime_snapshot_preflight")
                .unwrap()
    );
    for command in [
        "pub async fn apply_collection(",
        "pub async fn restore_last_changes(",
    ] {
        let start = source.find(command).unwrap();
        let remainder = &source[start..];
        let end = remainder[1..]
            .find("#[tauri::command]")
            .map(|offset| offset + 1)
            .unwrap_or(remainder.len());
        let command_source = &remainder[..end];
        assert!(
            command_source.find("admit_immutable_mutation").unwrap()
                < command_source
                    .find("acquire_nested_mutation_lease")
                    .unwrap()
        );
    }
}

#[test]
fn restore_queues_runtime_sync_without_a_second_full_reconcile() {
    let source = command_sources();
    let start = source
        .find("pub async fn restore_last_changes(")
        .expect("restore command source");
    let remainder = &source[start..];
    let end = remainder[1..]
        .find("#[tauri::command]")
        .map(|offset| offset + 1)
        .expect("next command boundary");
    let restore_command = &remainder[..end];

    assert!(
        !restore_command.contains("run_full_internal_disk_reconcile"),
        "the durable pipeline already reconciled the affected roots"
    );
    assert!(restore_command.contains("ensure_mutation_preflight_for_paths"));
    assert!(!restore_command.contains("ensure_mutation_preflight("));
    assert!(restore_command.contains("enqueue_runtime_sync_for_rewrites"));
}

fn command_sources() -> &'static str {
    concat!(
        include_str!("tauri.rs"),
        include_str!("runtime_commands.rs"),
        include_str!("capture_commands.rs"),
        include_str!("apply_commands.rs"),
        include_str!("preview_commands.rs"),
        include_str!("recovery_commands.rs"),
        include_str!("save_changes.rs")
    )
}
