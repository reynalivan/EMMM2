import { describe, expect, it } from 'vitest';
import type { ImportBatch } from '../../shared/api/tauri/bindings.gen';
import { needsSourceAnalysis, selectLatestResumableBatch } from './resume';

function batch(
  id: string,
  status: ImportBatch['status'],
  itemStatus: ImportBatch['items'][number]['status'],
): ImportBatch {
  return {
    id,
    gameId: 'gimi',
    flow: 'auto_import',
    targetMode: 'auto',
    targetObjectId: null,
    targetSubpath: null,
    status,
    sourceArchivePath: null,
    createdAt: '2026-08-29T00:00:00Z',
    updatedAt: '2026-08-29T00:00:00Z',
    items: [
      {
        id: `${id}-item`,
        batchId: id,
        sourceKind: 'folder',
        sourcePath: 'C:/Downloads/Ayaka',
        stagingPath: null,
        plannedName: 'DISABLED Ayaka',
        status: itemStatus,
        matchCategory: null,
        subCategory: null,
        classificationMetadata: {},
        categorySuggestions: [],
        canonicalSuggestions: [],
        destinationSuggestions: [],
        selectedEntryKey: null,
        selectedAliasName: null,
        destinationObjectId: null,
        destinationPath: null,
        confidencePercentage: 0,
        confidenceTier: 'no_match',
        evidence: [],
        decision: 'pending',
        fingerprint: null,
        result: null,
        error: null,
      },
    ],
  };
}

describe('persisted import batch recovery', () => {
  it('selects the latest non-terminal batch for the active game', () => {
    expect(
      selectLatestResumableBatch(
        [batch('done', 'done', 'done'), batch('partial', 'partial', 'partial')],
        'gimi',
      )?.id,
    ).toBe('partial');
  });

  it('re-analyzes interrupted source staging but not committed recovery', () => {
    expect(needsSourceAnalysis(batch('staging', 'partial', 'failed'))).toBe(true);
    expect(needsSourceAnalysis(batch('commit', 'partial', 'partial'))).toBe(false);
  });
});
