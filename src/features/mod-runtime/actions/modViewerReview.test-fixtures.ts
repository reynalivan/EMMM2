import type { DiskReconcileResult } from '@/shared/api/tauri/bindings';

export function watcherResult(changedRoots: string[]): DiskReconcileResult {
  return {
    game_id: 'gimi',
    reconcile_revision: 1,
    reason: 'WatcherBatch',
    status: 'Applied',
    scan_scope: 'Scoped',
    folder_conflicts: [],
    rename_confirmations: [],
    error_message: null,
    changed_roots: changedRoots,
    objects_changed: false,
    folders_changed: false,
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
      mod_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      object_sample_names: [],
      mod_sample_names: [],
      has_user_visible_changes: false,
    },
    pending_runtime_effects: { collections_dirty: false, overlay_refresh: false },
    warnings: [],
  };
}
