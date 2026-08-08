use std::collections::BTreeSet;

use crate::services::disk_reconcile::types::{
    DiskReconcileChangeCounts, DiskReconcileChangeSummary,
};

#[derive(Debug, Default)]
pub(crate) struct ChangeSummaryBuilder {
    object_changes: DiskReconcileChangeCounts,
    mod_changes: DiskReconcileChangeCounts,
    object_sample_names: BTreeSet<String>,
    mod_sample_names: BTreeSet<String>,
}

impl ChangeSummaryBuilder {
    pub(crate) fn record_object_added(&mut self, name: &str) {
        self.object_changes.added += 1;
        record_sample(&mut self.object_sample_names, name);
    }

    pub(crate) fn record_object_removed(&mut self, name: &str) {
        self.object_changes.removed += 1;
        record_sample(&mut self.object_sample_names, name);
    }

    pub(crate) fn record_object_renamed(&mut self, name: &str) {
        self.object_changes.renamed += 1;
        record_sample(&mut self.object_sample_names, name);
    }

    pub(crate) fn record_mod_added(&mut self, name: &str) {
        self.mod_changes.added += 1;
        record_sample(&mut self.mod_sample_names, name);
    }

    pub(crate) fn record_mod_removed(&mut self, name: &str) {
        self.mod_changes.removed += 1;
        record_sample(&mut self.mod_sample_names, name);
    }

    pub(crate) fn record_mod_renamed(&mut self, name: &str) {
        self.mod_changes.renamed += 1;
        record_sample(&mut self.mod_sample_names, name);
    }

    pub(crate) fn record_mod_modified(&mut self, name: &str) {
        self.mod_changes.modified += 1;
        record_sample(&mut self.mod_sample_names, name);
    }

    pub(crate) fn build(self) -> DiskReconcileChangeSummary {
        let has_user_visible_changes = self.object_changes.any() || self.mod_changes.any();

        DiskReconcileChangeSummary {
            object_changes: self.object_changes,
            mod_changes: self.mod_changes,
            object_sample_names: self.object_sample_names.into_iter().take(8).collect(),
            mod_sample_names: self.mod_sample_names.into_iter().take(8).collect(),
            has_user_visible_changes,
        }
    }
}

fn record_sample(samples: &mut BTreeSet<String>, name: &str) {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        samples.insert(trimmed.to_string());
    }
}
