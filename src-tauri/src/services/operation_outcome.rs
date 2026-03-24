use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationOutcomeKind {
    Success,
    PartialSuccess,
    AbortedWithRollback,
    AbortedWithoutSideEffect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationIssueSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationIssue {
    pub severity: OperationIssueSeverity,
    pub code: String,
    pub message: String,
}

impl OperationIssue {
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: OperationIssueSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}
