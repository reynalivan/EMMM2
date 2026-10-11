import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { modHealthKeys } from '@/entities/mod';
import { watcherResult } from './modViewerReview.test-fixtures';

const mocks = vi.hoisted(() => ({ analyzeModHealth: vi.fn() }));

vi.unmock('@tanstack/react-query');

vi.mock('@/entities/game', () => ({ useActiveGame: () => ({ activeGame: { id: 'gimi' } }) }));
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { analyzeModHealth: mocks.analyzeModHealth },
}));
vi.mock('@/entities/mod', async () => {
  const { modHealthKeys } = await import('@/entities/mod/model/queryKeys');
  return { modHealthKeys };
});

import {
  recordModViewerLaunchSnapshot,
  reconcileModViewerExternalReviews,
  useModViewerExternalReview,
} from './modViewerExternalReviewState';

const folder = 'E:/SyntheticAudit/Mods/Example';

function ReviewHarness() {
  const { review, dismiss } = useModViewerExternalReview(folder);
  return (
    <>
      <button type="button" onClick={dismiss}>
        Dismiss
      </button>
      <output>{review ? 'visible' : 'none'}</output>
    </>
  );
}

function mountReview() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  recordModViewerLaunchSnapshot(queryClient, {
    game_id: 'gimi',
    mod_folder: folder,
    file_manifest: [{ relative_path: 'mod.ini', size_bytes: 42, blake3: 'before' }],
  });
  queryClient.setQueryData(modHealthKeys.viewerReview('gimi', folder), {
    changes: [{ kind: 'modified', category: 'ini', relativePath: 'mod.ini' }],
    collectionImpact: null,
    fingerprint: 'review',
  });
  render(
    <QueryClientProvider client={queryClient}>
      <ReviewHarness />
    </QueryClientProvider>,
  );
  return queryClient;
}

describe('mounted Mod Viewer review dismissal', () => {
  it('clears the existing review when the latest analysis returns to the launch baseline', async () => {
    const queryClient = mountReview();
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('visible'));
    mocks.analyzeModHealth.mockResolvedValue({
      file_manifest: [{ relative_path: 'mod.ini', size_bytes: 42, blake3: 'before' }],
    });

    await act(async () => {
      await reconcileModViewerExternalReviews(
        watcherResult(['Example']),
        queryClient,
        'E:/SyntheticAudit/Mods',
        null,
      );
    });

    expect(mocks.analyzeModHealth).toHaveBeenCalledWith('gimi', folder);
    expect(queryClient.getQueryData(modHealthKeys.viewerSnapshots())).toHaveLength(1);
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('none'));
  });

  it('clears a mounted review when a new viewer launch replaces its baseline', async () => {
    const queryClient = mountReview();
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('visible'));

    act(() => {
      recordModViewerLaunchSnapshot(queryClient, {
        game_id: 'gimi',
        mod_folder: folder,
        file_manifest: [{ relative_path: 'mod.ini', size_bytes: 43, blake3: 'replacement' }],
      });
    });

    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('none'));
  });

  it('clears the mounted observer when the review is dismissed', async () => {
    const queryClient = mountReview();
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('visible'));
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));

    expect(queryClient.getQueryData(modHealthKeys.viewerReview('gimi', folder))).toBeNull();
    await waitFor(() => expect(screen.getByRole('status').textContent).toBe('none'));
  });
});
