use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportFlow {
    AutoImport,
    SpecificImport,
    Browser,
    ReadyToMove,
}

impl ImportFlow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AutoImport => "auto_import",
            Self::SpecificImport => "specific_import",
            Self::Browser => "browser",
            Self::ReadyToMove => "ready_to_move",
        }
    }
}

impl std::str::FromStr for ImportFlow {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto_import" => Ok(Self::AutoImport),
            "specific_import" => Ok(Self::SpecificImport),
            "browser" => Ok(Self::Browser),
            "ready_to_move" => Ok(Self::ReadyToMove),
            _ => Err(format!("unsupported import flow '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode {
    Auto,
    Specific,
}

impl TargetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Specific => "specific",
        }
    }
}

impl std::str::FromStr for TargetMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "specific" => Ok(Self::Specific),
            _ => Err(format!("unsupported target mode '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceKind {
    Folder,
    ArchiveRoot,
    BrowserDownload,
    ReadyToMove,
}

impl ImportSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::ArchiveRoot => "archive_root",
            Self::BrowserDownload => "browser_download",
            Self::ReadyToMove => "ready_to_move",
        }
    }
}

impl std::str::FromStr for ImportSourceKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "folder" => Ok(Self::Folder),
            "archive_root" => Ok(Self::ArchiveRoot),
            "browser_download" => Ok(Self::BrowserDownload),
            "ready_to_move" => Ok(Self::ReadyToMove),
            _ => Err(format!("unsupported import source kind '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum StableCategory {
    Character,
    Weapon,
    UI,
    Other,
}

impl StableCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Weapon => "Weapon",
            Self::UI => "UI",
            Self::Other => "Other",
        }
    }
}

impl std::str::FromStr for StableCategory {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Character" => Ok(Self::Character),
            "Weapon" => Ok(Self::Weapon),
            "UI" => Ok(Self::UI),
            "Other" => Ok(Self::Other),
            _ => Err(format!("unsupported stable category '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportBatchStatus {
    Draft,
    Analyzing,
    AwaitingReview,
    Ready,
    Committing,
    Partial,
    Done,
    Failed,
    Cancelled,
}

impl ImportBatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Analyzing => "analyzing",
            Self::AwaitingReview => "awaiting_review",
            Self::Ready => "ready",
            Self::Committing => "committing",
            Self::Partial => "partial",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for ImportBatchStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "analyzing" => Ok(Self::Analyzing),
            "awaiting_review" => Ok(Self::AwaitingReview),
            "ready" => Ok(Self::Ready),
            "committing" => Ok(Self::Committing),
            "partial" => Ok(Self::Partial),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unsupported import batch status '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportItemStatus {
    Discovered,
    Staged,
    AwaitingCategory,
    AwaitingDestination,
    Ready,
    Committing,
    Reconciling,
    FinalizingMetadata,
    Done,
    Skipped,
    Partial,
    MetadataPending,
    Failed,
    Cancelled,
}

impl ImportItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Staged => "staged",
            Self::AwaitingCategory => "awaiting_category",
            Self::AwaitingDestination => "awaiting_destination",
            Self::Ready => "ready",
            Self::Committing => "committing",
            Self::Reconciling => "reconciling",
            Self::FinalizingMetadata => "finalizing_metadata",
            Self::Done => "done",
            Self::Skipped => "skipped",
            Self::Partial => "partial",
            Self::MetadataPending => "metadata_pending",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn can_refresh_object_suggestions(self) -> bool {
        matches!(
            self,
            Self::AwaitingDestination | Self::Ready | Self::Skipped
        )
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use ImportItemStatus as S;
        matches!(
            (self, next),
            (S::Discovered, S::Staged | S::Failed | S::Cancelled)
                | (S::Staged, S::AwaitingCategory | S::Failed | S::Cancelled)
                | (
                    S::AwaitingCategory,
                    S::AwaitingDestination | S::Skipped | S::Cancelled
                )
                | (S::AwaitingDestination, S::Ready | S::Skipped | S::Cancelled)
                | (
                    S::Skipped,
                    S::AwaitingCategory | S::AwaitingDestination | S::Cancelled
                )
                | (S::Ready, S::Committing | S::Skipped | S::Cancelled)
                | (
                    S::Committing,
                    S::Ready | S::Reconciling | S::Skipped | S::Failed | S::Partial
                )
                | (
                    S::Reconciling,
                    S::FinalizingMetadata | S::Partial | S::Failed
                )
                | (
                    S::FinalizingMetadata,
                    S::Done | S::MetadataPending | S::Partial | S::Failed
                )
                | (S::MetadataPending, S::FinalizingMetadata | S::Cancelled)
                | (
                    S::Partial,
                    S::Reconciling | S::FinalizingMetadata | S::Cancelled
                )
                | (S::Failed, S::Staged | S::AwaitingCategory | S::Cancelled)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Done | Self::Skipped | Self::Failed | Self::Cancelled
        )
    }
}

impl std::str::FromStr for ImportItemStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "discovered" => Ok(Self::Discovered),
            "staged" => Ok(Self::Staged),
            "awaiting_category" => Ok(Self::AwaitingCategory),
            "awaiting_destination" => Ok(Self::AwaitingDestination),
            "ready" => Ok(Self::Ready),
            "committing" => Ok(Self::Committing),
            "reconciling" => Ok(Self::Reconciling),
            "finalizing_metadata" => Ok(Self::FinalizingMetadata),
            "done" => Ok(Self::Done),
            "skipped" => Ok(Self::Skipped),
            "partial" => Ok(Self::Partial),
            "metadata_pending" => Ok(Self::MetadataPending),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unsupported import item status '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceTier {
    High,
    Medium,
    Low,
    NoMatch,
}

impl ConfidenceTier {
    pub fn from_percentage(value: u8) -> Self {
        match value.min(100) {
            75..=100 => Self::High,
            45..=74 => Self::Medium,
            15..=44 => Self::Low,
            _ => Self::NoMatch,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::NoMatch => "no_match",
        }
    }
}

impl std::str::FromStr for ConfidenceTier {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            "no_match" => Ok(Self::NoMatch),
            _ => Err(format!("unsupported confidence tier '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ImportDecision {
    Pending,
    Confirm,
    KeepSpecificTarget,
    Reallocate,
    CreateCanonical,
    Skip,
}

impl ImportDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirm => "confirm",
            Self::KeepSpecificTarget => "keep_specific_target",
            Self::Reallocate => "reallocate",
            Self::CreateCanonical => "create_canonical",
            Self::Skip => "skip",
        }
    }
}

impl std::str::FromStr for ImportDecision {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "confirm" => Ok(Self::Confirm),
            "keep_specific_target" => Ok(Self::KeepSpecificTarget),
            "reallocate" => Ok(Self::Reallocate),
            "create_canonical" => Ok(Self::CreateCanonical),
            "skip" => Ok(Self::Skip),
            _ => Err(format!("unsupported import decision '{value}'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceFingerprint {
    pub path: String,
    pub modified_unix_ms: String,
    pub size_bytes: String,
    pub file_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MatchEvidence {
    pub source: String,
    pub value: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CategorySuggestion {
    pub category: StableCategory,
    pub sub_category: Option<String>,
    pub confidence_percentage: u8,
    pub confidence_tier: ConfidenceTier,
    pub evidence: Vec<MatchEvidence>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalSuggestion {
    pub entry_key: String,
    pub name: String,
    pub matched_alias: Option<String>,
    pub confidence_percentage: u8,
    pub confidence_tier: ConfidenceTier,
    pub evidence: Vec<MatchEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DestinationKind {
    SpecificTarget,
    ExistingObject,
    CreateCanonical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DestinationSuggestion {
    pub kind: DestinationKind,
    pub object_id: Option<String>,
    pub canonical_entry_key: Option<String>,
    pub folder_name: String,
    pub target_path: String,
    pub confidence_percentage: u8,
    pub confidence_tier: ConfidenceTier,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    pub id: String,
    pub batch_id: String,
    pub source_kind: ImportSourceKind,
    pub source_path: String,
    pub staging_path: Option<String>,
    pub planned_name: String,
    pub status: ImportItemStatus,
    pub match_category: Option<StableCategory>,
    pub sub_category: Option<String>,
    pub classification_metadata: serde_json::Value,
    pub category_suggestions: Vec<CategorySuggestion>,
    pub canonical_suggestions: Vec<CanonicalSuggestion>,
    pub destination_suggestions: Vec<DestinationSuggestion>,
    pub selected_entry_key: Option<String>,
    pub selected_alias_name: Option<String>,
    pub destination_object_id: Option<String>,
    pub destination_path: Option<String>,
    pub confidence_percentage: u8,
    pub confidence_tier: ConfidenceTier,
    pub evidence: Vec<MatchEvidence>,
    pub decision: ImportDecision,
    pub fingerprint: Option<SourceFingerprint>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatch {
    pub id: String,
    pub game_id: String,
    pub flow: ImportFlow,
    pub target_mode: TargetMode,
    pub target_object_id: Option<String>,
    pub target_subpath: Option<String>,
    pub status: ImportBatchStatus,
    pub source_archive_path: Option<String>,
    pub items: Vec<ImportItem>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceInput {
    pub path: String,
    pub source_kind: Option<ImportSourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateImportBatchInput {
    pub game_id: String,
    pub flow: ImportFlow,
    pub target_mode: TargetMode,
    pub target_object_id: Option<String>,
    pub target_subpath: Option<String>,
    pub sources: Vec<ImportSourceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetImportItemClassificationInput {
    pub item_id: String,
    pub category: StableCategory,
    pub sub_category: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameImportItemInput {
    pub item_id: String,
    pub planned_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SetImportItemDecisionInput {
    pub item_id: String,
    pub decision: ImportDecision,
    pub destination_object_id: Option<String>,
    pub destination_path: Option<String>,
    pub canonical_entry_key: Option<String>,
    pub matched_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitImportBatchInput {
    pub batch_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModInboxRootState {
    Missing,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ModInboxEntryKind {
    Folder,
    Archive,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum ModInboxLayout {
    DirectMod,
    FolderPack,
    Wrapper,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInboxEntry {
    pub entry_key: String,
    pub name: String,
    pub path: String,
    pub kind: ModInboxEntryKind,
    pub archive_format: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_unix_ms: String,
    pub layout: ModInboxLayout,
    pub detected_root_count: u32,
    pub pending_batch_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedModInboxDestination {
    pub object_id: Option<String>,
    pub object_name: Option<String>,
    pub placed_path: String,
    pub planned_name: String,
    pub status: ImportItemStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedModInboxSource {
    pub source_id: String,
    pub name: String,
    pub source_kind: ModInboxEntryKind,
    pub original_path: String,
    pub processed_path: Option<String>,
    pub processed_at: String,
    pub source_deleted_at: Option<String>,
    pub destinations: Vec<ProcessedModInboxDestination>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInboxSnapshot {
    pub game_id: String,
    pub root_path: String,
    pub root_state: ModInboxRootState,
    pub ready_entries: Vec<ModInboxEntry>,
    pub processed_sources: Vec<ProcessedModInboxSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateModInboxBatchInput {
    pub game_id: String,
    pub entry_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProcessedModInboxSourcesInput {
    pub game_id: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchReport {
    pub batch_id: String,
    pub moved: u32,
    pub reallocated: u32,
    pub created_canonical_folders: u32,
    pub skipped: u32,
    pub collisions: u32,
    pub metadata_pending: u32,
    pub failed: u32,
}
