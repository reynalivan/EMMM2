import { QueryClient } from '@tanstack/react-query';
import { waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { DiskReconcileResult } from '../../lib/bindings';
import { publishDiskReconcileRefresh } from './reconcileRefresh';

function createResult(overrides: Partial<DiskReconcileResult>): DiskReconcileResult {
  return {
    folder_conflicts: [],
    rename_confirmations: [],
    game_id: 'game-1',
    reason: 'WatcherBatch',
    status: 'Applied',
    error_message: null,
    changed_roots: [],
    objects_changed: false,
    folders_changed: false,
    collections_changed: false,
    runtime_file_changed: false,
    thumbnail_roots: [],
    cleared_selection_paths: [],
    path_updates: [],
    pending_runtime_effects: {
      collections_dirty: false,
      overlay_refresh: false,
    },
    warnings: [],
    collection_reference_impact: {
      affected_collection_count: 0,
      affected_collection_names: [],
      rewritten_paths: [],
      missing_paths: [],
    },
    change_summary: {
      object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      mod_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      object_sample_names: [],
      mod_sample_names: [],
      has_user_visible_changes: false,
    },
    ...overrides,
  };
}

describe('publishDiskReconcileRefresh', () => {
  it('invalidates conflicts when a runtime file changes', async () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(['conflicts', 'game-1'], [{ hash: 'abcdef12' }]);

    publishDiskReconcileRefresh(queryClient, createResult({ runtime_file_changed: true }), false);

    await waitFor(() => {
      expect(queryClient.getQueryState(['conflicts', 'game-1'])?.isInvalidated).toBe(true);
    });
  });
});
