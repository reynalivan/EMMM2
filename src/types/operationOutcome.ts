export type OperationOutcomeKind =
  | 'success'
  | 'partial_success'
  | 'aborted_with_rollback'
  | 'aborted_without_side_effect';

export type OperationIssueSeverity = 'info' | 'warning' | 'error';

export interface OperationIssue {
  severity: OperationIssueSeverity;
  code: string;
  message: string;
}
