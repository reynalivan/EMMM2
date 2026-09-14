use serde::{Deserialize, Serialize};
use specta::Type;

use crate::modules::collections::domain::collection::CollectionReferenceImpact;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileReason {
    StartupBoot,
    OnboardingCompleted,
    ModsViewEntered,
    WindowRefocused,
    WatcherBatch,
    ManualRepair,
    GameSwitched,
    InternalMutation,
    StorageSizeBackfill,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcilePathKind {
    Object,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileStatus {
    Applied,
    AppliedWithFolderConflicts,
    SourceUnavailable,
    NeedsRenameConfirmation,
}

/// The filesystem discovery coverage that produced a reconcile result.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileScanScope {
    Full,
    Scoped,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcilePhase {
    DiscoveringRoots,
    ScanningRoots,
    Projecting,
    Finalizing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileProgress {
    pub game_id: String,
    pub run_id: String,
    pub reason: DiskReconcileReason,
    pub phase: DiskReconcilePhase,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub current_root: Option<String>,
    pub elapsed_ms: u64,
    pub eta_ms: Option<u64>,
}

impl DiskReconcileStatus {
    pub fn applied(&self) -> bool {
        matches!(self, Self::Applied | Self::AppliedWithFolderConflicts)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum RenameConfirmationKind {
    Object,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum RenameConfirmationReason {
    MissingIdentity,
    AmbiguousIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct RenameConfirmationGroup {
    pub group_id: String,
    pub kind: RenameConfirmationKind,
    pub reason: RenameConfirmationReason,
    pub scope_key: String,
    pub previous_paths: Vec<String>,
    pub current_paths: Vec<String>,
    #[specta(type = f64)]
    pub previous_path_count: usize,
    #[specta(type = f64)]
    pub current_path_count: usize,
    pub candidates_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum RenameConfirmationResolutionAction {
    Rename,
    Separate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct RenameConfirmationResolution {
    pub group_id: String,
    pub action: RenameConfirmationResolutionAction,
    pub previous_path: Option<String>,
    pub current_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct FolderNameConflictCandidate {
    pub path: String,
    pub folder_name: String,
    pub base_name: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct FolderNameConflictGroup {
    pub group_id: String,
    pub identity: String,
    pub display_name: String,
    pub candidates: Vec<FolderNameConflictCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcilePathUpdate {
    pub from: String,
    pub to: String,
    pub kind: DiskReconcilePathKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileChangeCounts {
    pub added: u32,
    pub removed: u32,
    pub renamed: u32,
    pub modified: u32,
}

impl DiskReconcileChangeCounts {
    pub fn any(&self) -> bool {
        self.added > 0 || self.removed > 0 || self.renamed > 0 || self.modified > 0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileChangeSummary {
    pub object_changes: DiskReconcileChangeCounts,
    pub mod_changes: DiskReconcileChangeCounts,
    pub object_sample_names: Vec<String>,
    pub mod_sample_names: Vec<String>,
    pub has_user_visible_changes: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct PendingRuntimeEffects {
    pub collections_dirty: bool,
    pub overlay_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileWarningKind {
    RuntimeEffectsPending,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileWarning {
    pub kind: DiskReconcileWarningKind,
    pub message: String,
}

/// A filesystem mutation already committed, but its terminal projection sync
/// could not finish. Commands return this as data so callers refresh the
/// successful disk effect without retrying the mutation itself.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum CommittedMutationSyncWarningKind {
    ReconcileFailed,
    ReconcileBlocked,
    CleanupPending,
    RuntimeSyncPending,
    ManualReloadRequired,
    WatcherUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct IndexingRootWork {
    pub root_name: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub work_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct OnboardingIndexingWorkPlan {
    pub game_id: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub work_units: u64,
    pub roots: Vec<IndexingRootWork>,
}

/// Opaque handle for a short-lived onboarding disk snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct OnboardingIndexingSession {
    pub session_id: String,
    pub work_plans: Vec<OnboardingIndexingWorkPlan>,
}

/// Replaces a game's onboarding work estimate after a scoped rescan.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct OnboardingIndexingWorkPlanUpdate {
    pub session_id: String,
    pub work_plan: OnboardingIndexingWorkPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum OnboardingIndexingSnapshotPhase {
    Scanning,
    Ready,
}

/// Progress for the pre-index snapshot phase. This is intentionally separate
/// from projection progress because the DB reconcile has not begun yet.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct OnboardingIndexingSnapshotProgress {
    pub session_id: String,
    pub game_id: String,
    pub phase: OnboardingIndexingSnapshotPhase,
    pub completed_games: u64,
    pub total_games: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CommittedMutationSyncWarning {
    pub kind: CommittedMutationSyncWarningKind,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CommittedMutationResult {
    pub sync_warning: Option<CommittedMutationSyncWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DiskReconcileResult {
    pub game_id: String,
    /// Monotonic per-game revision assigned after this disk observation finishes.
    pub reconcile_revision: u64,
    pub reason: DiskReconcileReason,
    pub status: DiskReconcileStatus,
    pub scan_scope: DiskReconcileScanScope,
    pub folder_conflicts: Vec<FolderNameConflictGroup>,
    pub rename_confirmations: Vec<RenameConfirmationGroup>,
    pub error_message: Option<String>,
    pub changed_roots: Vec<String>,
    pub objects_changed: bool,
    pub folders_changed: bool,
    pub collections_changed: bool,
    pub runtime_file_changed: bool,
    pub thumbnail_roots: Vec<String>,
    pub cleared_selection_paths: Vec<String>,
    pub path_updates: Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: CollectionReferenceImpact,
    pub change_summary: DiskReconcileChangeSummary,
    pub pending_runtime_effects: PendingRuntimeEffects,
    pub warnings: Vec<DiskReconcileWarning>,
}
