import { describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryObserver } from '@tanstack/react-query';
import { waitFor } from '@testing-library/react';
import {
  cancelRuntimeDescriptorQueries,
  publishQueryInvalidations,
  publishRuntimeDescriptor,
  runtimeQueryKeys,
  refreshStartupReadModels,
} from './queryRefresh';
import type { RuntimeEffectDescriptor } from '../../shared/lib/runtimeEffects';

function createQueryClientMock() {
  return {
    cancelQueries: vi.fn().mockResolvedValue(undefined),
    invalidateQueries: vi.fn().mockResolvedValue(undefined),
  };
}

describe('queryRefresh', () => {
  it('cancels a cold pre-baseline dashboard read before refetching current data', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    let releaseOld!: (value: string) => void;
    const oldRead = new Promise<string>((resolve) => {
      releaseOld = resolve;
    });
    const queryFn = vi
      .fn<() => Promise<string>>()
      .mockImplementationOnce(() => oldRead)
      .mockResolvedValue('baseline-data');
    const observer = new QueryObserver(client, {
      queryKey: ['dashboard-stats', 'game-a'],
      queryFn,
    });
    const unsubscribe = observer.subscribe(() => undefined);
    try {
      expect(queryFn).toHaveBeenCalledTimes(1);
      expect(client.getQueryData(['dashboard-stats', 'game-a'])).toBeUndefined();
      await refreshStartupReadModels(client, 'game-a', () => true);
      expect(queryFn).toHaveBeenCalledTimes(2);
      expect(client.getQueryData(['dashboard-stats', 'game-a'])).toBe('baseline-data');
      releaseOld('stale-before-baseline');
      await oldRead;
      expect(client.getQueryData(['dashboard-stats', 'game-a'])).toBe('baseline-data');
    } finally {
      unsubscribe();
      client.clear();
    }
  });

  it('does not refresh a superseded game or unrelated game queries', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const queryFn = vi.fn().mockResolvedValue('old-game');
    const observer = new QueryObserver(client, {
      queryKey: ['dashboard-stats', 'game-b'],
      queryFn,
    });
    const unsubscribe = observer.subscribe(() => undefined);
    try {
      await waitFor(() =>
        expect(client.getQueryData(['dashboard-stats', 'game-b'])).toBe('old-game'),
      );
      await refreshStartupReadModels(client, 'game-a', () => true);
      await refreshStartupReadModels(client, 'game-b', () => false);
      expect(queryFn).toHaveBeenCalledTimes(1);
    } finally {
      unsubscribe();
      client.clear();
    }
  });

  it('batches refresh events in the same tick and merges refetch types', async () => {
    const queryClient = createQueryClientMock();
    const objectDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['objectRowsChanged'],
    };
    const folderDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['folderStructureChanged'],
    };

    await Promise.all([
      publishRuntimeDescriptor(queryClient as never, objectDescriptor, 'active'),
      publishRuntimeDescriptor(queryClient as never, folderDescriptor, 'inactive'),
    ]);

    expect(queryClient.invalidateQueries).toHaveBeenCalledTimes(5);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'all',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectRows,
      refetchType: 'all',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectCounts,
      refetchType: 'all',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderStructure,
      refetchType: 'all',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.previewDetails,
      refetchType: 'all',
    });
  });

  it('does not let none downgrade an active batched refresh', async () => {
    const queryClient = createQueryClientMock();
    const activeDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['objectRowsChanged'],
    };
    const passiveDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['dashboardChanged'],
    };

    await Promise.all([
      publishRuntimeDescriptor(queryClient as never, activeDescriptor, 'active'),
      publishRuntimeDescriptor(queryClient as never, passiveDescriptor, 'none'),
    ]);

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.dashboard,
      refetchType: 'active',
    });
  });

  it('skips refresh work when descriptor has no refresh events', async () => {
    const queryClient = createQueryClientMock();
    const descriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: [],
    };

    await publishRuntimeDescriptor(queryClient as never, descriptor, 'active');

    expect(queryClient.invalidateQueries).not.toHaveBeenCalled();
  });

  it('centralizes dynamic query invalidations and de-duplicates keys', async () => {
    const queryClient = createQueryClientMock();
    const key = ['v2-collections', 'list', 'game-1', true] as const;

    await publishQueryInvalidations(queryClient as never, [key, key], 'active');

    expect(queryClient.invalidateQueries).toHaveBeenCalledTimes(1);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: key,
      refetchType: 'active',
    });
  });

  it('de-duplicates physical query keys when logical scopes share a key', async () => {
    const queryClient = createQueryClientMock();
    const descriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['folderStructureChanged', 'folderMetadataChanged'],
    };

    await publishRuntimeDescriptor(queryClient as never, descriptor, 'active');

    expect(queryClient.invalidateQueries).toHaveBeenCalledTimes(5);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderStructure,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.previewDetails,
      refetchType: 'active',
    });
  });

  it('cancels every affected query family once before committed refresh work', async () => {
    const queryClient = createQueryClientMock();
    const descriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['folderStructureChanged', 'runtimeStateChanged'],
    };

    await cancelRuntimeDescriptorQueries(queryClient as never, descriptor);

    expect(queryClient.cancelQueries).toHaveBeenCalledTimes(6);
    expect(queryClient.cancelQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
    });
    expect(queryClient.cancelQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.collectionRuntime,
    });
  });
});
