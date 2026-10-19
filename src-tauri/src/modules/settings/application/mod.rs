pub mod config;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct SaveSettingsResult {
    pub settings: config::AppSettings,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
}
