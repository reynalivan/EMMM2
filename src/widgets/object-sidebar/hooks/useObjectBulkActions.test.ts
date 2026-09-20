/**
 * Guards the bulk handlers against the defect that started this surface's
 * rework: per-item IPC errors were caught, logged, and then followed by an
 * unconditional success toast, so a failed bulk pin looked like it worked.
 */
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useObjectBulkActions } from './useObjectBulkActions';
import type { WorkspaceObjectNode } from '@/entities/workspace';
import { useAppStore } from '@/app/store';

const pinObject = vi.fn();
const updateObject = vi.fn();
const bulkSetModSafety = vi.fn();
const buildRuntimeMutationDescriptor = vi.fn();
const publishRuntimeDescriptor = vi.fn();
const toastSuccess = vi.fn();
const toastError = vi.fn();
const executeWorkspaceObjectBulkSwitch = vi.fn();
const applyWorkspaceSwitchEffects = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    pinObject: (...args: unknown[]) => pinObject(...args),
    updateObjectCmd: (...args: unknown[]) => updateObject(...args),
    bulkToggleFavorite: vi.fn(),
    bulkSetModSafety: (...args: unknown[]) => bulkSetModSafety(...args),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccess(...args),
    error: (...args: unknown[]) => toastError(...args),
  },
}));

vi.mock('@tanstack/react-query', async () => ({
  ...(await vi.importActual<typeof import('@tanstack/react-query')>('@tanstack/react-query')),
  useQueryClient: () => ({}),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

// Run the wrapped mutation directly: the optimistic patch and its trailing
// refresh are not what these tests are about.
vi.mock('@/features/workspace-runtime', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/features/workspace-runtime')>()),
  runObjectBatchMutation: async ({ mutation }: { mutation: () => Promise<void> }) => {
    await mutation();
  },
  executeWorkspaceObjectBulkSwitch: (...args: unknown[]) =>
    executeWorkspaceObjectBulkSwitch(...args),
  applyWorkspaceSwitchEffects: (...args: unknown[]) => applyWorkspaceSwitchEffects(...args),
  useDeleteObject: () => ({ mutateAsync: vi.fn() }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
  // `hooks/bulkToastMessages` pulls in the i18n singleton, which needs this plugin.
  initReactI18next: { type: '3rdParty', init: () => {} },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  publishRuntimeDescriptor: (...args: unknown[]) => publishRuntimeDescriptor(...args),
}));

vi.mock('@/features/workspace-runtime/optimistic/descriptorBuilders', () => ({
  buildRuntimeMutationDescriptor: (...args: unknown[]) => buildRuntimeMutationDescriptor(...args),
}));

vi.mock('@/features/workspace-runtime/actions/useWorkspaceSwitchActions', () => ({
  useWorkspaceSwitchActions: () => ({ setNodeEnabled: vi.fn() }),
}));

vi.mock('../utils/runBulkClassifyAndMatch', () => ({
  runBulkClassifyAndMatch: vi.fn(),
}));

const objects = [
  { id: 'a', name: 'Ayaka', tags: '["old"]', folder_path: 'Ayaka' },
  { id: 'b', name: 'Yelan', tags: '["old"]', folder_path: 'Yelan' },
] as unknown as WorkspaceObjectNode[];

const objectSwitchResult = {
  status: 'applied',
  primary_path: 'Ayaka',
  changed_folder_paths: ['Ayaka', 'Yelan'],
  changed_object_ids: ['a', 'b'],
  duplicates: [],
  parent_enable_requirement: null,
  impact: { rewrites: [], refresh_scopes: [] },
  sync_warning: null,
  runtime_sync_generation: 7,
};

function setup() {
  const { result } = renderHook(() => useObjectBulkActions({ objects, setIsSyncing: vi.fn() }));
  return result;
}

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState({ activeGameId: 'game-1' });
});

describe('handleBulkPin', () => {
  it('sends the pin payload Rust actually expects', async () => {
    pinObject.mockResolvedValue(undefined);

    await setup().current.handleBulkPin(new Set(['a']), true);

    // `isPinned` here instead of `pin` is what failed serde silently before.
    expect(pinObject).toHaveBeenCalledWith('a', true);
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
    pinObject.mockImplementation((id: string) =>
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

    expect(updateObject).toHaveBeenCalledWith('a', { tags: [] });
    expect(toastSuccess).toHaveBeenCalledTimes(1);
    expect(toastError).not.toHaveBeenCalled();
  });
});

describe('handleBulkSafe', () => {
  it('publishes the shared safety refresh descriptor', async () => {
    bulkSetModSafety.mockResolvedValue({ success: ['Ayaka'], failures: [] });
    buildRuntimeMutationDescriptor.mockReturnValue({ refreshEvents: [] });
    publishRuntimeDescriptor.mockResolvedValue(undefined);

    await setup().current.handleBulkSafe(new Set(['a']), false);

    expect(bulkSetModSafety).toHaveBeenCalledWith('game-1', ['Ayaka'], false);
    expect(buildRuntimeMutationDescriptor).toHaveBeenCalledWith('safetyClassification');
    expect(publishRuntimeDescriptor).toHaveBeenCalled();
  });
});

describe('object enable batch', () => {
  it('uses one atomic workspace command and one trailing effect publication', async () => {
    executeWorkspaceObjectBulkSwitch.mockResolvedValue(objectSwitchResult);
    applyWorkspaceSwitchEffects.mockResolvedValue(undefined);

    await setup().current.handleBulkEnable(new Set(['a', 'b']));

    expect(executeWorkspaceObjectBulkSwitch).toHaveBeenCalledTimes(1);
    expect(executeWorkspaceObjectBulkSwitch).toHaveBeenCalledWith('game-1', ['a', 'b'], true);
    expect(applyWorkspaceSwitchEffects).toHaveBeenCalledTimes(1);
    expect(toastSuccess).toHaveBeenCalledTimes(1);
  });

  it('does not publish effects or toast when every selected object is already in the target state', async () => {
    executeWorkspaceObjectBulkSwitch.mockResolvedValue({
      ...objectSwitchResult,
      status: 'noop',
      changed_folder_paths: [],
      changed_object_ids: [],
      runtime_sync_generation: null,
    });

    await setup().current.handleBulkEnable(new Set(['a', 'b']));

    expect(applyWorkspaceSwitchEffects).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it('suppresses a duplicate submit while the atomic batch is in flight', async () => {
    let complete: ((value: typeof objectSwitchResult) => void) | undefined;
    executeWorkspaceObjectBulkSwitch.mockReturnValue(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    applyWorkspaceSwitchEffects.mockResolvedValue(undefined);
    const hook = setup();

    let first!: Promise<void>;
    act(() => {
      first = hook.current.handleBulkEnable(new Set(['a', 'b']));
    });
    await act(async () => {
      await hook.current.handleBulkEnable(new Set(['a', 'b']));
    });

    expect(executeWorkspaceObjectBulkSwitch).toHaveBeenCalledTimes(1);
    await act(async () => {
      complete?.(objectSwitchResult);
      await first;
    });
  });
});
