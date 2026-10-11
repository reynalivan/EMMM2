import { QueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { modHealthKeys } from '@/entities/mod';
import { watcherResult } from './modViewerReview.test-fixtures';

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

describe('Mod Viewer external review state', () => {
  it('replaces the enabled snapshot when a disabled spelling launches and ignores its old analysis', async () => {
    const queryClient = new QueryClient();
    const enabled = 'E:/Mods/Example';
    const disabled = 'E:/Mods/DISABLED Example';
    recordModViewerLaunchSnapshot(queryClient, receipt(enabled, 'before'));
    let finish:
      | ((value: {
          file_manifest: Array<{ relative_path: string; size_bytes: number; blake3: string }>;
        }) => void)
      | undefined;
    mocks.analyzeModHealth.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const pending = reconcileModViewerExternalReviews(
      watcherResult(['Example']),
      queryClient,
      'E:/Mods',
      null,
    );

    recordModViewerLaunchSnapshot(queryClient, receipt(disabled, 'replacement'));
    expect(queryClient.getQueryData(modHealthKeys.viewerSnapshots())).toHaveLength(1);
    finish?.({
      file_manifest: [{ relative_path: 'mod.ini', size_bytes: 43, blake3: 'old-result' }],
    });
    await pending;

    expect(queryClient.getQueryData(modHealthKeys.viewerReview('gimi', disabled))).toBeNull();
    expect(mocks.toastInfo).not.toHaveBeenCalled();
  });

  beforeEach(() => vi.clearAllMocks());

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
    expect(queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'Mods/Example'))).toBeNull();
  });

  it('does not publish a review when its launch is dismissed during analysis', async () => {
    const queryClient = new QueryClient();
    recordModViewerLaunchSnapshot(queryClient, receipt('E:/Mods/Example', 'before'));
    let finishAnalysis!: (report: {
      file_manifest: Array<{ relative_path: string; size_bytes: number; blake3: string }>;
    }) => void;
    mocks.analyzeModHealth.mockReturnValueOnce(
      new Promise((resolve) => {
        finishAnalysis = resolve;
      }),
    );

    const reconciliation = reconcileModViewerExternalReviews(
      watcherResult(['Example']),
      queryClient,
      'E:/Mods',
      null,
    );
    dismissModViewerExternalReview(queryClient, 'gimi', 'E:/Mods/Example');
    finishAnalysis({
      file_manifest: [{ relative_path: 'mod.ini', size_bytes: 43, blake3: 'after' }],
    });
    await reconciliation;

    expect(
      queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'E:/Mods/Example')),
    ).toBeNull();
    expect(
      queryClient.getQueryData(modHealthKeys.report('gimi', 'E:/Mods/Example')),
    ).toBeUndefined();
    expect(mocks.toastInfo).not.toHaveBeenCalled();
  });

  it('ignores an older analysis when a newer reconciliation finishes first', async () => {
    const queryClient = new QueryClient();
    recordModViewerLaunchSnapshot(queryClient, receipt('E:/Mods/Example', 'before'));
    const finish: Array<
      (report: {
        file_manifest: Array<{ relative_path: string; size_bytes: number; blake3: string }>;
      }) => void
    > = [];
    mocks.analyzeModHealth.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish.push(resolve);
        }),
    );

    const older = reconcileModViewerExternalReviews(
      watcherResult(['Example']),
      queryClient,
      'E:/Mods',
      null,
    );
    const newer = reconcileModViewerExternalReviews(
      watcherResult(['Example']),
      queryClient,
      'E:/Mods',
      null,
    );
    const report = (blake3: string) => ({
      file_manifest: [{ relative_path: 'mod.ini', size_bytes: 43, blake3 }],
    });
    finish[1]?.(report('newer'));
    await newer;
    finish[0]?.(report('older'));
    await older;

    expect(queryClient.getQueryData(modHealthKeys.report('gimi', 'E:/Mods/Example'))).toEqual(
      report('newer'),
    );
    expect(
      queryClient.getQueryData(modHealthKeys.viewerReview('gimi', 'E:/Mods/Example')),
    ).toMatchObject({
      changes: [{ relativePath: 'mod.ini' }],
    });
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
    ).toBeNull();
    expect(mocks.toastInfo).toHaveBeenCalledOnce();
  });
});
