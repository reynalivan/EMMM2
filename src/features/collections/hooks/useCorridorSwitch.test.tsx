import React from 'react';
import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useCorridorSwitch } from './useCorridorSwitch';
import { useAppStore } from '../../../stores/useAppStore';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

const switchCorridorMock = vi.fn();
const getCorridorStateMock = vi.fn();
const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const publishRuntimeDescriptorMock = vi.fn();
const buildRuntimeMutationDescriptorMock = vi.fn((kind: unknown) => ({ kind }));

vi.mock('../../../lib/bindings', () => ({
  commands: {
    switchCorridor: (input: unknown) => switchCorridorMock(input),
    getCorridorState: (input: unknown) => getCorridorStateMock(input),
  },
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: (message: unknown) => toastSuccessMock(message),
    error: (message: unknown) => toastErrorMock(message),
  },
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
  publishRuntimeDescriptor: (queryClient: unknown, descriptor: unknown, behavior: unknown) =>
    publishRuntimeDescriptorMock(queryClient, descriptor, behavior),
  publishQueryInvalidations: (queryClient: unknown, keys: unknown, behavior: unknown) =>
    publishRuntimeDescriptorMock(queryClient, { keys }, behavior),
}));

vi.mock('../../workspace-runtime/optimistic/descriptorBuilders', () => ({
  buildRuntimeMutationDescriptor: (kind: unknown) => buildRuntimeMutationDescriptorMock(kind),
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('useCorridorSwitch', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      safeMode: false,
      gridSelection: new Set(['E:/Mods/Unsafe Variant']),
      selectedObjectFolderPath: 'Objects/Nahida',
      selectedModPath: 'E:/Mods/Unsafe Variant',
      explorerSubPath: 'Objects/Nahida/Unsafe',
      currentPath: ['Objects', 'Nahida', 'Unsafe'],
      mobileActivePane: 'details',
    });
  });

  it('formats structured backend errors into readable text', async () => {
    switchCorridorMock.mockRejectedValue({
      type: 'Corridor',
      payload: {
        GameNotFound: {
          game_id: 'g-1',
        },
      },
    });

    const { result } = renderHook(() => useCorridorSwitch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await expect(
        result.current.mutateAsync({ gameId: 'g-1', targetSafe: false }),
      ).rejects.toBeDefined();
    });

    expect(toastErrorMock).toHaveBeenCalledWith("Game 'g-1' not found");
  });

  it('clears corridor-sensitive selection state after a successful switch', async () => {
    switchCorridorMock.mockResolvedValue({
      active_safe: true,
      mods_disabled: 2,
      mods_restored: 1,
      restored_collection_id: 'collection-safe',
      warnings: ['restore-stage: skipped missing folder'],
    });
    getCorridorStateMock.mockResolvedValue({
      is_safe: true,
      active_collection_id: 'collection-safe',
      active_collection_name: 'Safe Preset',
      active_collection_is_unsaved: false,
      is_dirty: false,
    });
    publishRuntimeDescriptorMock.mockResolvedValue(undefined);

    const { result } = renderHook(() => useCorridorSwitch(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({ gameId: 'g-1', targetSafe: true });
    });

    const state = useAppStore.getState();
    expect(state.safeMode).toBe(true);
    expect(state.selectedObjectFolderPath).toBeNull();
    expect(state.selectedModPath).toBeNull();
    expect(state.explorerSubPath).toBeUndefined();
    expect(state.currentPath).toEqual([]);
    expect(state.gridSelection.size).toBe(0);
    expect(state.mobileActivePane).toBe('sidebar');
    expect(getCorridorStateMock).toHaveBeenCalledWith({ gameId: 'g-1', isSafe: true });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      'SAFE Mode Enabled — Disabled 2, Restored 1 mod(s) — 1 warning(s)',
    );
  });
});
