/**
 * Guards the bulk handlers against the defect that started this surface's
 * rework: per-item IPC errors were caught, logged, and then followed by an
 * unconditional success toast, so a failed bulk pin looked like it worked.
 */
import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useObjectBulkActions } from './useObjectBulkActions';
import type { WorkspaceObjectNode } from '../../../types/workspace';

const pinObject = vi.fn();
const updateObject = vi.fn();
const toastSuccess = vi.fn();
const toastError = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    pinObject: (...args: unknown[]) => pinObject(...args),
    updateObject: (...args: unknown[]) => updateObject(...args),
    bulkToggleFavorite: vi.fn(),
    bulkUpdateInfo: vi.fn(),
  },
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccess(...args),
    error: (...args: unknown[]) => toastError(...args),
  },
}));

vi.mock('@tanstack/react-query', () => ({
  useQueryClient: () => ({}),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

// Run the wrapped mutation directly: the optimistic patch and its trailing
// refresh are not what these tests are about.
vi.mock('../../../hooks/objectQueryCache', () => ({
  runObjectBatchMutation: async ({ mutation }: { mutation: () => Promise<void> }) => {
    await mutation();
  },
}));

vi.mock('../../../hooks/useObjectMutations', () => ({
  useDeleteObject: () => ({ mutateAsync: vi.fn() }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('../../runtime-sync/queryRefresh', () => ({
  publishRuntimeDescriptor: vi.fn(),
}));

vi.mock('../../workspace-runtime/optimistic/descriptorBuilders', () => ({
  buildRuntimeMutationDescriptor: vi.fn(),
}));

vi.mock('../../workspace-runtime/actions/useWorkspaceSwitchActions', () => ({
  useWorkspaceSwitchActions: () => ({ setNodeEnabled: vi.fn() }),
}));

vi.mock('../utils/runBulkAutoRecognize', () => ({
  runBulkAutoRecognize: vi.fn(),
}));

const objects = [
  { id: 'a', name: 'Ayaka', tags: '["old"]' },
  { id: 'b', name: 'Yelan', tags: '["old"]' },
] as unknown as WorkspaceObjectNode[];

function setup() {
  const { result } = renderHook(() => useObjectBulkActions({ objects, setIsSyncing: vi.fn() }));
  return result;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('handleBulkPin', () => {
  it('sends the pin payload Rust actually expects', async () => {
    pinObject.mockResolvedValue(undefined);

    await setup().current.handleBulkPin(new Set(['a']), true);

    // `isPinned` here instead of `pin` is what failed serde silently before.
    expect(pinObject).toHaveBeenCalledWith({ id: 'a', pin: true });
  });

  it('reports success once every id succeeded', async () => {
    pinObject.mockResolvedValue(undefined);

    await setup().current.handleBulkPin(new Set(['a', 'b']), true);

    expect(toastSuccess).toHaveBeenCalledTimes(1);
    expect(toastError).not.toHaveBeenCalled();
  });

  it('does NOT claim success when every id failed', async () => {
    pinObject.mockRejectedValue(new Error('database is locked'));

    await setup().current.handleBulkPin(new Set(['a', 'b']), true);

    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledTimes(1);
  });

  it('reports an error on partial failure rather than a success toast', async () => {
    pinObject.mockImplementation(({ id }: { id: string }) =>
      id === 'b' ? Promise.reject(new Error('gone')) : Promise.resolve(undefined),
    );

    await setup().current.handleBulkPin(new Set(['a', 'b']), false);

    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledTimes(1);
    // Both ids were attempted; one failure must not abort the rest.
    expect(pinObject).toHaveBeenCalledTimes(2);
  });
});

describe('bulk tag handlers', () => {
  it('does NOT claim success when a tag write failed', async () => {
    updateObject.mockRejectedValue(new Error('write failed'));

    await setup().current.handleBulkAddTags(new Set(['a']), ['nsfw']);

    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledTimes(1);
  });

  it('reports success when the tag write landed', async () => {
    updateObject.mockResolvedValue(undefined);

    await setup().current.handleBulkRemoveTags(new Set(['a']), ['old']);

    expect(updateObject).toHaveBeenCalledWith({ id: 'a', updates: { tags: [] } });
    expect(toastSuccess).toHaveBeenCalledTimes(1);
    expect(toastError).not.toHaveBeenCalled();
  });
});
