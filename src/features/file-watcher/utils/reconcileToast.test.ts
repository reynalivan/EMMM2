import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DiskReconcileResult } from '../../../core/tauri/bindings';
import { toast } from '../../../stores/useToastStore';
import { maybeShowExternalChangeToast } from './reconcileToast';

vi.mock('../../../stores/useToastStore', () => ({
  toast: { info: vi.fn() },
}));

function manualResult(): DiskReconcileResult {
  return {
    game_id: 'game-1',
    reason: 'ManualRepair',
    status: 'Applied',
    folder_conflicts: [],
    rename_confirmations: [],
    error_message: null,
    changed_roots: ['E:\\Mods\\Alice'],
    objects_changed: false,
    folders_changed: true,
    collections_changed: false,
    runtime_file_changed: false,
    thumbnail_roots: [],
    cleared_selection_paths: [],
    path_updates: [],
    collection_reference_impact: {
      affected_collection_count: 0,
      affected_collection_names: [],
      rewritten_paths: [],
      missing_paths: [],
    },
    change_summary: {
      object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      mod_changes: { added: 1, removed: 0, renamed: 0, modified: 0 },
      object_sample_names: [],
      mod_sample_names: ['Alice'],
      has_user_visible_changes: true,
    },
    pending_runtime_effects: { collections_dirty: false, overlay_refresh: false },
    warnings: [],
  };
}

describe('maybeShowExternalChangeToast', () => {
  beforeEach(() => vi.clearAllMocks());

  it('does not duplicate the terminal status toast for a manual full reconcile', () => {
    maybeShowExternalChangeToast(manualResult());

    expect(toast.info).not.toHaveBeenCalled();
  });
});
