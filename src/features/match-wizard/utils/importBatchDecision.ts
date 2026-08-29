import type { DestinationSuggestion, ImportBatch, ImportDecision } from '../../../core/tauri/bindings.gen';

export function destinationDecision(
  batch: ImportBatch,
  suggestion: DestinationSuggestion,
): ImportDecision {
  if (suggestion.kind === 'create_canonical') return 'create_canonical';
  if (suggestion.kind === 'specific_target') return 'keep_specific_target';
  if (batch.targetMode === 'specific' && suggestion.objectId !== batch.targetObjectId) {
    return 'reallocate';
  }
  return 'confirm';
}
