import { QueryClient } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vitest';
import { modHealthKeys } from '@/entities/mod';
import type { DiskReconcileResult } from '@/shared/api/tauri/bindings';

const mocks = vi.hoisted(() => ({
  analyzeModHealth: vi.fn(),
  toastInfo: vi.fn(),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    analyzeModHealth: mocks.analyzeModHealth,
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    info: mocks.toastInfo,
  },
}));

import {
  dismissModViewerExternalReview,
  recordModViewerLaunchSnapshot,
  reconcileModViewerExternalReviews,
} from './modViewerExternalReviewState';

function receipt(folder: string, hash: string) {
  return {
    game_id: 'gimi',
    mod_folder: folder,
    file_manifest: [
      {
        relative_path: 'mod.ini',
        size_bytes: 42,
        blake3: hash,
      },
    ],
  };
}

function watcherResult(changedRoots: string[]): DiskReconcileResult {
  return {
    game_id: 'gimi',
    reconcile_revision: 1,
    reason: 'WatcherBatch',
    status: 'Applied',
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

describe('Mod Viewer external review state', () => {
  it('replaces a folder snapshot when the viewer launches again', () => {
    const queryClient = new QueryClient();

    recordModViewerLaunchSnapshot(queryClient, receipt('Mods/Example', 'before'));
    recordModViewerLaunchSnapshot(queryClient, receipt('Mods/Example', 'replacement'));

    const snapshots = queryClient.getQueryData<
      Array<{ manifest: Array<{ content_hash: string }> }>
    >(modHealthKeys.viewerSnapshots());
    expect(snapshots).toHaveLength(1);
    expect(snapshots?.[0]?.manifest[0]?.content_hash).toBe('replacement');
  });

  it('dismisses both the review and its active snapshot', () => {
    const queryClient = new QueryClient();
    recordModViewerLaunchSnapshot(queryClient, receipt('Mods/Example', 'before'));
    queryClient.setQueryData(modHealthKeys.viewerReview('gimi', 'Mods/Example'), {
      changes: [],
      collectionImpact: null,
      fingerprint: 'review',
    });

    dismissModViewerExternalReview(queryClient, 'gimi', 'Mods/Example');

    expect(queryClient.getQueryData(modHealthKeys.viewerSnapshots())).toEqual([]);
    expect(
      queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'Mods/Example')),
    ).toBeUndefined();
  });

  it('creates an updated review only for a snapshot under the reconciled root', async () => {
    const queryClient = new QueryClient();
    recordModViewerLaunchSnapshot(queryClient, receipt('E:/Mods/Character/Example', 'before'));
    recordModViewerLaunchSnapshot(queryClient, receipt('E:/Mods/Weapon/Other', 'before'));
    mocks.analyzeModHealth.mockResolvedValue({
      file_manifest: [
        {
          relative_path: 'mod.ini',
          size_bytes: 43,
          blake3: 'after',
        },
      ],
    });

    await reconcileModViewerExternalReviews(
      watcherResult(['Character']),
      queryClient,
      'E:/Mods',
      null,
    );

    expect(mocks.analyzeModHealth).toHaveBeenCalledOnce();
    expect(mocks.analyzeModHealth).toHaveBeenCalledWith('gimi', 'E:/Mods/Character/Example');
    expect(
      queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'E:/Mods/Character/Example')),
    ).toMatchObject({
      changes: [
        {
          kind: 'modified',
          category: 'ini',
          relativePath: 'mod.ini',
        },
      ],
    });
    expect(
      queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'E:/Mods/Weapon/Other')),
    ).toBeUndefined();
    expect(mocks.toastInfo).toHaveBeenCalledOnce();
  });
});
