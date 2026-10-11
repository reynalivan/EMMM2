import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DragPosition } from '@/shared/lib/hooks/useFileDrop';
import { useFolderGridImport } from './useFolderGridImport';

const mocks = vi.hoisted(() => ({
  objects: vi.fn(),
  openWizard: vi.fn(),
  drop: null as null | ((paths: string[], position: DragPosition) => void),
}));
vi.mock('@tauri-apps/api/path', () => ({ join: async (...parts: string[]) => parts.join('/') }));
vi.mock('@/shared/api/tauri/bindings', () => ({ commands: { getObjectsCmd: mocks.objects } }));
vi.mock('@/features/import-batches', () => ({ openImportBatchWizard: mocks.openWizard }));
vi.mock('@/shared/lib/hooks/useFileDrop', () => ({
  useFileDrop: ({ onDrop }: { onDrop: (paths: string[], position: DragPosition) => void }) => {
    mocks.drop = onDrop;
    return { isDragging: false, dragPosition: null };
  },
}));
vi.mock('@/shared/lib/hooks/useDragAutoScroll', () => ({ useDragAutoScroll: vi.fn() }));

describe('folder grid import routing', () => {
  const paths = ['D:/Incoming/Mod'];
  const selectedPath = 'D:/Mods/Characters/A';

  function options(
    explorerSubPath = selectedPath,
    selectedObjectFolderPath: string | null = selectedPath,
  ) {
    const grid = document.createElement('div');
    grid.getBoundingClientRect = () => ({
      x: 300,
      y: 0,
      left: 300,
      top: 0,
      right: 900,
      bottom: 600,
      width: 600,
      height: 600,
      toJSON: () => ({}),
    });
    return {
      parentRef: { current: grid },
      dropTargetRef: { current: grid },
      activeGameId: 'game-a',
      activeModPath: 'D:/Mods',
      selectedObjectFolderPath,
      explorerSubPath,
    };
  }

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.objects.mockResolvedValue({ objects: [{ id: 'object-a', folder_path: 'Characters/A' }] });
    mocks.drop = null;
  });

  it('ignores native drops on the sidebar without querying or launching a second wizard', async () => {
    renderHook(() => useFolderGridImport(options()));
    await act(async () => mocks.drop?.(paths, { x: 30, y: 100 }));
    expect(mocks.objects).not.toHaveBeenCalled();
    expect(mocks.openWizard).not.toHaveBeenCalled();
  });

  it('launches one specific import for a drop inside the grid', async () => {
    renderHook(() => useFolderGridImport(options()));
    await act(async () => mocks.drop?.(paths, { x: 500, y: 100 }));
    expect(mocks.openWizard).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        targetObjectId: 'object-a',
        targetSubpath: null,
        paths,
      }),
    );
  });

  it('accepts drops on an empty grid whose scroll container is hidden', async () => {
    const input = options();
    input.parentRef = { current: document.createElement('div') };
    renderHook(() => useFolderGridImport(input));
    await act(async () => mocks.drop?.(paths, { x: 500, y: 100 }));
    expect(mocks.openWizard).toHaveBeenCalledOnce();
  });

  it.each([
    [selectedPath, null],
    [`${selectedPath}/Variants/Blue`, 'Variants/Blue'],
    ['d:\\mods\\characters\\a\\Variants\\Blue', 'Variants/Blue'],
  ])('sends an object-relative subpath for %s', async (explorerSubPath, targetSubpath) => {
    const { result } = renderHook(() => useFolderGridImport(options(explorerSubPath ?? undefined)));
    await act(async () => result.current.handleImportFiles(paths));
    expect(mocks.openWizard).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        flow: 'specific_import',
        targetObjectId: 'object-a',
        targetSubpath,
      }),
    );
  });

  it('does not append the current explorer path to an auto-selected destination', async () => {
    const { result } = renderHook(() => useFolderGridImport(options('D:/Mods/Unregistered', null)));
    await act(async () => result.current.handleImportFiles(paths));
    expect(mocks.openWizard).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        flow: 'auto_import',
        targetObjectId: null,
        targetSubpath: null,
      }),
    );
  });

  it('resolves the registered object when workspace navigation uses game-relative paths', async () => {
    const input = options('Characters/A/Variants', 'Characters/A');
    const { result } = renderHook(() => useFolderGridImport(input));
    await act(async () => result.current.handleImportFiles(paths));
    expect(mocks.openWizard).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        flow: 'specific_import',
        targetObjectId: 'object-a',
        targetSubpath: 'Variants',
      }),
    );
  });
});
