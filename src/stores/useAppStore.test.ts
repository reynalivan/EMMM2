import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useAppStore } from './useAppStore';
import { act } from '@testing-library/react';

// Mock localStorage and tauri api
vi.stubGlobal('localStorage', {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

describe('useAppStore', () => {
  beforeEach(() => {
    // Reset store state
    act(() => {
      useAppStore.setState({
        activeGameId: null,
        safeMode: true,
        autoCloseLauncher: false,
        isStoreInitialized: false,
        workspaceView: 'dashboard',
        currentPath: [],
        activeCollectionId: null,
        selectedObject: null,
        gridSelection: new Set(),
        leftPanelWidth: 260,
        rightPanelWidth: 320,
        selectedObjectType: null,
        sidebarSearchQuery: '',
        collapsedCategories: new Set(),
        mobileActivePane: 'sidebar',
        isPreviewOpen: true,
        sortField: 'name',
        sortOrder: 'asc',
        viewMode: 'grid',
        explorerSubPath: undefined,
        explorerSearchQuery: '',
        explorerScrollOffset: 0,
        conflictDialog: { open: false, conflict: null },
      });
    });
    vi.clearAllMocks();
  });

  it('initializes with default state', () => {
    const state = useAppStore.getState();
    expect(state.activeGameId).toBeNull();
    expect(state.safeMode).toBe(true);
    expect(state.workspaceView).toBe('dashboard');
  });

  it('setWorkspaceView updates view', () => {
    act(() => {
      useAppStore.getState().setWorkspaceView('mods');
    });
    expect(useAppStore.getState().workspaceView).toBe('mods');
  });

  it('setCurrentPath updates path', () => {
    act(() => {
      useAppStore.getState().setCurrentPath(['a', 'b']);
    });
    expect(useAppStore.getState().currentPath).toEqual(['a', 'b']);
  });

  it('toggleGridSelection adds items (single select mode)', () => {
    act(() => {
      useAppStore.getState().toggleGridSelection('item1');
    });
    expect(useAppStore.getState().gridSelection.has('item1')).toBe(true);
    expect(useAppStore.getState().mobileActivePane).toBe('details');

    // In single select mode (multi=false), clicking again does not unselect
    act(() => {
      useAppStore.getState().toggleGridSelection('item1');
    });
    expect(useAppStore.getState().gridSelection.has('item1')).toBe(true);
  });

  it('toggleGridSelection toggles items off when multi=true', () => {
    act(() => {
      useAppStore.getState().toggleGridSelection('item1', true);
    });
    expect(useAppStore.getState().gridSelection.has('item1')).toBe(true);

    act(() => {
      useAppStore.getState().toggleGridSelection('item1', true);
    });
    expect(useAppStore.getState().gridSelection.has('item1')).toBe(false);
  });

  it('toggleGridSelection with multi=true preserves other items', () => {
    act(() => {
      useAppStore.getState().setGridSelection(new Set(['item1', 'item2']));
      useAppStore.getState().toggleGridSelection('item3', true);
    });
    expect(useAppStore.getState().gridSelection.has('item1')).toBe(true);
    expect(useAppStore.getState().gridSelection.has('item3')).toBe(true);
  });

  it('toggleCategoryCollapse toggles category state', () => {
    act(() => {
      useAppStore.getState().toggleCategoryCollapse('cat1');
    });
    expect(useAppStore.getState().collapsedCategories.has('cat1')).toBe(true);

    act(() => {
      useAppStore.getState().toggleCategoryCollapse('cat1');
    });
    expect(useAppStore.getState().collapsedCategories.has('cat1')).toBe(false);
  });

  it('setActiveGameId resets explorer and sidebar states and calls invoke', async () => {
    const invoke = (await import('@tauri-apps/api/core')).invoke;

    // Set some state to test resetting
    act(() => {
      useAppStore.setState({
        explorerSubPath: '/sub',
        currentPath: ['a', 'b'],
        explorerSearchQuery: 'search',
        selectedObject: 'obj',
        gridSelection: new Set(['1']),
        sidebarSearchQuery: 'side',
        selectedObjectType: 'type',
        collapsedCategories: new Set(['cat1']),
      });
    });

    await act(async () => {
      await useAppStore.getState().setActiveGameId('game1');
    });

    const state = useAppStore.getState();
    expect(state.activeGameId).toBe('game1');
    expect(state.explorerSubPath).toBeUndefined();
    expect(state.currentPath).toEqual([]);
    expect(state.explorerSearchQuery).toBe('');
    expect(state.selectedObject).toBeNull();
    expect(state.gridSelection.size).toBe(0);
    expect(state.sidebarSearchQuery).toBe('');
    expect(state.selectedObjectType).toBeNull();
    expect(state.collapsedCategories.size).toBe(0);

    expect(invoke).toHaveBeenCalledWith('set_active_game', { gameId: 'game1' });
  });

  it('setSafeMode calls invoke and updates state', async () => {
    const invoke = (await import('@tauri-apps/api/core')).invoke;

    await act(async () => {
      await useAppStore.getState().setSafeMode(false);
    });

    expect(useAppStore.getState().safeMode).toBe(false);
    expect(invoke).toHaveBeenCalledWith('set_safe_mode_enabled', { enabled: false });
  });

  it('setAutoCloseLauncher calls invoke and updates state', async () => {
    const invoke = (await import('@tauri-apps/api/core')).invoke;

    await act(async () => {
      await useAppStore.getState().setAutoCloseLauncher(true);
    });

    expect(useAppStore.getState().autoCloseLauncher).toBe(true);
    expect(invoke).toHaveBeenCalledWith('set_auto_close_launcher', { enabled: true });
  });

  it('initStore fetches settings from backend', async () => {
    const invoke = (await import('@tauri-apps/api/core')).invoke;
    vi.mocked(invoke).mockResolvedValueOnce({
      active_game_id: 'game-123',
      safe_mode: { enabled: true },
      auto_close_launcher: true,
    });

    await act(async () => {
      await useAppStore.getState().initStore();
    });

    const state = useAppStore.getState();
    expect(state.activeGameId).toBe('game-123');
    expect(state.safeMode).toBe(true);
    expect(state.autoCloseLauncher).toBe(true);
    expect(state.isStoreInitialized).toBe(true);
    expect(invoke).toHaveBeenCalledWith('get_settings');
  });

  it('openConflictDialog displays dialog with conflict data', () => {
    const mockConflict = {
      type: 'RenameConflict',
      attempted_target: 'a',
      existing_path: 'b',
    } as import('./useAppStore').RenameConflictError;

    act(() => {
      useAppStore.getState().openConflictDialog(mockConflict);
    });

    expect(useAppStore.getState().conflictDialog.open).toBe(true);
    expect(useAppStore.getState().conflictDialog.conflict).toEqual(mockConflict);
  });
});
