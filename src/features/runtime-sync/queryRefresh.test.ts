import { describe, expect, it, vi } from 'vitest';
import {
  publishRuntimeDescriptor,
  runtimeQueryKeys,
} from './queryRefresh';
import type { RuntimeEffectDescriptor } from '../workspace-runtime/optimistic/descriptor';

function createQueryClientMock() {
  return {
    invalidateQueries: vi.fn().mockResolvedValue(undefined),
  };
}

describe('queryRefresh', () => {
  it('batches refresh events in the same tick and merges refetch types', async () => {
    const queryClient = createQueryClientMock();
    const objectDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      objectCountDeltas: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: ['objectRowsChanged'],
    };
    const folderDescriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      objectCountDeltas: [],
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

  it('skips refresh work when descriptor has no refresh events', async () => {
    const queryClient = createQueryClientMock();
    const descriptor: RuntimeEffectDescriptor = {
      rewrites: [],
      invalidatedPaths: [],
      objectCountDeltas: [],
      thumbnailPaths: [],
      removedQueryKeys: [],
      invalidatedQueryKeys: [],
      refreshEvents: [],
    };

    await publishRuntimeDescriptor(queryClient as never, descriptor, 'active');

    expect(queryClient.invalidateQueries).not.toHaveBeenCalled();
  });
});
