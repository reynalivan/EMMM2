pub use super::adapters::sqlite::projection_receipts::ProjectionReceipt;
pub(crate) use super::adapters::sqlite::projection_receipts::{
    delete as delete_projection_receipt, insert_tx as insert_projection_receipt_tx,
};
pub use super::admission::{
    admit_immutable_mutation, ImmutableMutationKind, ImmutableMutationPermit,
};
pub use super::application::*;
pub use super::coordinator::{MutationCoordinator, MutationGuard};
pub use super::journal::{
    MutationStepKind, Operation, OperationPlan, PlannedStep, StepSettlement, StepStatus,
    COLLECTION_APPLY_OPERATION_KIND, FOLDER_CREATE_OPERATION_KIND, FOLDER_CREATE_STAGE_PREFIX,
};

#[cfg(test)]
pub(crate) mod recovery_testing {
    pub use super::super::journal::{OperationJournal, OperationStatus};
    pub use super::super::recovery::{RecoveryRoots, RecoveryRunner};
}

#[cfg(debug_assertions)]
pub mod testing {
    pub mod application {
        pub use super::super::super::application::*;
    }
}
