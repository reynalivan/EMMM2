import { beforeEach, describe, expect, it, vi } from 'vitest';
import { waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useToastStore } from '@/shared/ui/toast';
import type { DiskReconcileResult, FolderNameConflictGroup } from '@/shared/api/tauri/bindings';
import { useAppStore } from './useAppStore';

const folderConflict: FolderNameConflictGroup = {
  group_id: 'alice-blue',
  identity: 'alice/blue',
  display_name: 'Blue',
  candidates: [
    { path: 'C:/Mods/Alice/Blue', folder_name: 'Blue', base_name: 'Blue', is_enabled: true },
    {
      path: 'C:/Mods/Alice/DISABLED Blue',
      folder_name: 'DISABLED Blue',
      base_name: 'Blue',
      is_enabled: false,
    },
  ],
};

function diskReconcileResult(overrides: Partial<DiskReconcileResult> = {}): DiskReconcileResult {
  return {
    game_id: 'g1',
    reconcile_revision: 1,
    reason: 'WatcherBatch',
    status: 'Applied',
    scan_scope: 'Scoped',
    folder_conflicts: [],
    rename_confirmations: [],
    error_message: null,
    changed_roots: [],
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
    ...overrides,
  };
}

// Snapshot the pristine state (defaults + actions) once, restore before each test.
const initialState = useAppStore.getState();

beforeEach(() => {
  vi.clearAllMocks();
  useAppStore.setState(initialState, true);
  useToastStore.setState({ toasts: [] });
});

describe('useAppStore smoke net', () => {
  describe('startup recovery', () => {
    it('captures source-unavailable recovery before publishing the active game', async () => {
      vi.mocked(listen).mockImplementationOnce(async (_event, handler) => {
        handler({
          payload: {
            game_id: 'genshin',
            status: 'SourceUnavailable',
            error_message: 'Mods drive is unavailable',
            folder_conflicts: [],
            rename_confirmations: [],
          },
        } as never);
        return () => undefined;
      });
      vi.mocked(invoke).mockResolvedValueOnce({
        active_game_id: 'genshin',
        auto_close_launcher: false,
      });

      await useAppStore.getState().initStore();

      const state = useAppStore.getState();
      expect(state.activeGameId).toBe('genshin');
      expect(state.diskReconcileByGame.genshin?.unavailable).toBe('Mods drive is unavailable');
    });

    it('shows a visible toast when startup recovery fails', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('startup disk scan failed'));

      await useAppStore.getState().initStore();

      const toasts = useToastStore.getState().toasts;
      expect(toasts[toasts.length - 1]?.message).toContain('startup disk scan failed');
    });

    it('applies a newer recovery event received while startup queries are pending', async () => {
      let emitReconcile!: (event: { payload: DiskReconcileResult }) => void;
      let releasePrefetch!: () => void;
      const pendingPrefetch = new Promise<void>((resolve) => {
        releasePrefetch = resolve;
      });
      vi.mocked(listen).mockImplementationOnce(async (_event, handler) => {
        emitReconcile = handler as unknown as (event: { payload: DiskReconcileResult }) => void;
        return () => undefined;
      });
      vi.mocked(invoke).mockImplementation((command) => {
        if (command === 'get_settings') {
          emitReconcile({
            payload: diskReconcileResult({
              game_id: 'genshin',
              reconcile_revision: 1,
              status: 'AppliedWithFolderConflicts',
              folder_conflicts: [folderConflict],
            }),
          });
          return Promise.resolve({ active_game_id: 'genshin', auto_close_launcher: false });
        }
        return pendingPrefetch;
      });

      const initializing = useAppStore.getState().initStore();
      try {
        await waitFor(() => expect(useAppStore.getState().activeGameId).toBe('genshin'));
        emitReconcile({
          payload: diskReconcileResult({ game_id: 'genshin', reconcile_revision: 2 }),
        });

        expect(useAppStore.getState().folderConflictReportsByGame.genshin).toMatchObject({
          revision: 2,
          status: 'resolvedExternally',
          groups: [],
        });
      } finally {
        releasePrefetch();
        await initializing;
        vi.mocked(invoke).mockReset();
      }
    });
  });

  describe('initial state', () => {
    it('has the expected defaults per domain', () => {
      const s = useAppStore.getState();
      expect(s.activeGameId).toBeNull();
      expect(s.workspaceView).toBe('dashboard');
      expect(s.currentPath).toEqual([]);
      expect(s.mobileActivePane).toBe('sidebar');
      expect(s.isPreviewOpen).toBe(true);
      expect(s.gridSelection.size).toBe(0);
      expect(s.selectedModPath).toBeNull();
      expect(s.workspacePreviewTransition).toEqual({ kind: 'idle', pendingTarget: null });
      expect(s.workspaceDialogState).toEqual({ kind: 'none' });
      expect(s.leftPanelWidth).toBe(260);
      expect(s.rightPanelWidth).toBe(320);
      expect(s.activePane).toBe('objectList');
      expect(s.safetyFilter).toBe('all');
    });
  });

  describe('navigation / routing', () => {
    it('switches workspace view and path', () => {
      useAppStore.getState().setWorkspaceView('collections');
      useAppStore.getState().setCurrentPath(['Mods', 'Diluc']);
      expect(useAppStore.getState().workspaceView).toBe('collections');
      expect(useAppStore.getState().currentPath).toEqual(['Mods', 'Diluc']);
    });

    it('supports Mod Inbox as a workspace view', () => {
      useAppStore.getState().setWorkspaceView('mod-inbox');

      expect(useAppStore.getState().workspaceView).toBe('mod-inbox');
    });

    it('selecting an object folder auto-navigates mobile pane to grid, deselecting back to sidebar', () => {
      useAppStore.getState().setSelectedObjectFolderPath('C:/mods/Diluc');
      expect(useAppStore.getState().selectedObjectFolderPath).toBe('C:/mods/Diluc');
      expect(useAppStore.getState().mobileActivePane).toBe('grid');

      useAppStore.getState().setSelectedObjectFolderPath(null);
      expect(useAppStore.getState().mobileActivePane).toBe('sidebar');
    });
  });

  describe('grid selection', () => {
    it('single toggle selects, sets selectedModPath, and jumps mobile pane to details', () => {
      useAppStore.getState().toggleGridSelection('mod-a');
      const s = useAppStore.getState();
      expect(s.gridSelection.has('mod-a')).toBe(true);
      expect(s.selectedModPath).toBe('mod-a');
      expect(s.mobileActivePane).toBe('details');
    });

    it('single toggle on the already-selected id keeps it selected (current behavior, no deselect)', () => {
      useAppStore.getState().toggleGridSelection('mod-a');
      useAppStore.getState().toggleGridSelection('mod-a');
      expect(useAppStore.getState().gridSelection.has('mod-a')).toBe(true);
      expect(useAppStore.getState().selectedModPath).toBe('mod-a');
    });

    it('multi toggle adds and removes without touching mobile pane', () => {
      useAppStore.getState().toggleGridSelection('mod-a', true);
      useAppStore.getState().toggleGridSelection('mod-b', true);
      expect(useAppStore.getState().gridSelection.size).toBe(2);
      expect(useAppStore.getState().mobileActivePane).toBe('sidebar');

      useAppStore.getState().toggleGridSelection('mod-b', true);
      expect(useAppStore.getState().gridSelection.has('mod-b')).toBe(false);
      expect(useAppStore.getState().selectedModPath).toBe('mod-b');
    });

    it('setGridSelection uses the last entry as selectedModPath', () => {
      useAppStore.getState().setGridSelection(new Set(['mod-a', 'mod-b']));
      expect(useAppStore.getState().selectedModPath).toBe('mod-b');
      expect(useAppStore.getState().mobileActivePane).toBe('sidebar');
    });

    it('clearGridSelection empties selection and selectedModPath', () => {
      useAppStore.getState().setGridSelection(new Set(['mod-a']));
      useAppStore.getState().clearGridSelection();
      expect(useAppStore.getState().gridSelection.size).toBe(0);
      expect(useAppStore.getState().selectedModPath).toBeNull();
    });

    it('replaceGridSelections rewrites paths in selection and selectedModPath', () => {
      useAppStore.getState().setGridSelection(new Set(['C:/mods/Old/skin', 'C:/mods/Other']));
      useAppStore
        .getState()
        .replaceGridSelections([{ oldPath: 'C:/mods/Old', newPath: 'C:/mods/New' }]);
      const s = useAppStore.getState();
      expect(s.gridSelection.has('C:/mods/New/skin')).toBe(true);
      expect(s.gridSelection.has('C:/mods/Other')).toBe(true);
      expect(s.selectedModPath).toBe('C:/mods/Other');
    });
  });

  describe('game switching', () => {
    it('does not publish the new game before the backend resets its recovery gate', async () => {
      let releaseBackend!: () => void;
      const backendReady = new Promise<void>((resolve) => {
        releaseBackend = resolve;
      });
      vi.mocked(invoke).mockReturnValueOnce(backendReady.then(() => null));

      const switching = useAppStore.getState().setActiveGameId('genshin');
      expect(useAppStore.getState().activeGameId).toBeNull();

      releaseBackend();
      await switching;
      expect(useAppStore.getState().activeGameId).toBe('genshin');
    });

    it('setActiveGameId resets selection, sidebar and explorer navigation state', async () => {
      useAppStore.setState({
        selectedObjectType: 'Character',
        sidebarSearchQuery: 'diluc',
        objectMetaFilters: { element: ['Pyro'] },
        objectStatusFilter: 'enabled',
        currentPath: ['Mods'],
        explorerSubPath: 'sub',
        explorerSearchQuery: 'q',
        explorerScrollOffset: 120,
        gridSelection: new Set(['mod-a']),
        selectedModPath: 'mod-a',
        workspacePreviewDirty: true,
        workspaceView: 'mods',
      });

      await useAppStore.getState().setActiveGameId('genshin');

      const s = useAppStore.getState();
      expect(s.activeGameId).toBe('genshin');
      expect(s.selectedObjectType).toBeNull();
      expect(s.sidebarSearchQuery).toBe('');
      expect(s.objectMetaFilters).toEqual({});
      expect(s.objectStatusFilter).toBe('all');
      expect(s.currentPath).toEqual([]);
      expect(s.explorerSubPath).toBeUndefined();
      expect(s.explorerSearchQuery).toBe('');
      expect(s.gridSelection.size).toBe(0);
      expect(s.selectedModPath).toBeNull();
      expect(s.workspacePreviewDirty).toBe(false);
      expect(s.workspaceDialogState).toEqual({ kind: 'none' });
      // Current behavior: these survive a game switch.
      expect(s.workspaceView).toBe('mods');
      expect(s.explorerScrollOffset).toBe(120);
    });
  });

  describe('persisted behavior settings', () => {
    it('rolls back the optimistic auto-close value when persistence fails', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('settings write failed'));

      await useAppStore.getState().setAutoCloseLauncher(true);

      expect(useAppStore.getState().autoCloseLauncher).toBe(false);
      const toasts = useToastStore.getState().toasts;
      expect(toasts[toasts.length - 1]?.message).toContain('settings write failed');
    });
  });

  describe('sidebar / object list', () => {
    it('toggleCategoryCollapse adds then removes a category', () => {
      useAppStore.getState().toggleCategoryCollapse('Weapons');
      expect(useAppStore.getState().collapsedCategories.has('Weapons')).toBe(true);
      useAppStore.getState().toggleCategoryCollapse('Weapons');
      expect(useAppStore.getState().collapsedCategories.has('Weapons')).toBe(false);
    });
  });

  describe('layout / preview', () => {
    it('togglePreview flips and setPanelWidths sets both widths', () => {
      useAppStore.getState().togglePreview();
      expect(useAppStore.getState().isPreviewOpen).toBe(false);
      useAppStore.getState().setPanelWidths(300, 400);
      expect(useAppStore.getState().leftPanelWidth).toBe(300);
      expect(useAppStore.getState().rightPanelWidth).toBe(400);
    });

    it('setMobilePane updates directly', () => {
      useAppStore.getState().setMobilePane('details');
      expect(useAppStore.getState().mobileActivePane).toBe('details');
    });
  });

  describe('persist rehydration', () => {
    const merge = (persisted: unknown) =>
      useAppStore.persist.getOptions().merge!(persisted, useAppStore.getState());

    it('restores persisted keys and revives collapsedCategories as a Set', () => {
      const merged = merge({ leftPanelWidth: 420, collapsedCategories: ['Weapons', 'UI'] });

      expect(merged.leftPanelWidth).toBe(420);
      expect(merged.collapsedCategories).toBeInstanceOf(Set);
      expect(merged.collapsedCategories.has('Weapons')).toBe(true);
      // Untouched keys still come from the live store, actions included.
      expect(merged.rightPanelWidth).toBe(320);
      expect(typeof merged.setPanelWidths).toBe('function');
    });

    it('falls back to defaults when nothing was persisted', () => {
      const merged = merge(undefined);

      expect(merged.leftPanelWidth).toBe(260);
      expect(merged.collapsedCategories.size).toBe(0);
    });
  });

  describe('disk reconcile bookkeeping', () => {
    it('timestamp write clears pending and unavailable flags for that game', () => {
      useAppStore.getState().markDiskReconcilePending('g1', true);
      useAppStore.getState().setDiskSourceUnavailable('g1', 'gone');
      expect(useAppStore.getState().diskReconcileByGame.g1).toEqual({
        at: 0,
        pending: false,
        unavailable: 'gone',
        progress: null,
        revision: 0,
      });

      useAppStore.getState().setDiskReconcileTimestamp('g1', 1234);
      expect(useAppStore.getState().diskReconcileByGame.g1).toEqual({
        at: 1234,
        pending: false,
        unavailable: null,
        progress: null,
        revision: 0,
      });
    });

    it('preserves a newer conflict report and labels a newer empty report as external', () => {
      const store = useAppStore.getState();
      expect(
        store.applyFolderConflictReconcileResult(
          diskReconcileResult({
            reconcile_revision: 4,
            status: 'AppliedWithFolderConflicts',
            folder_conflicts: [folderConflict],
          }),
        ),
      ).toBe(true);

      expect(
        useAppStore
          .getState()
          .applyFolderConflictReconcileResult(diskReconcileResult({ reconcile_revision: 3 })),
      ).toBe(false);
      expect(useAppStore.getState().folderConflictReportsByGame.g1).toMatchObject({
        revision: 4,
        status: 'open',
        groups: [folderConflict],
      });

      expect(
        useAppStore
          .getState()
          .applyFolderConflictReconcileResult(diskReconcileResult({ reconcile_revision: 5 })),
      ).toBe(true);
      expect(useAppStore.getState().folderConflictReportsByGame.g1).toMatchObject({
        revision: 5,
        status: 'resolvedExternally',
        groups: [],
      });
      expect(
        useAppStore
          .getState()
          .applyFolderConflictReconcileResult(diskReconcileResult({ reconcile_revision: 5 })),
      ).toBe(false);
    });

    it('keeps an open conflict while the source is unavailable, then accepts a verified empty report', () => {
      const store = useAppStore.getState();
      expect(
        store.applyFolderConflictReconcileResult(
          diskReconcileResult({
            reconcile_revision: 1,
            status: 'AppliedWithFolderConflicts',
            folder_conflicts: [folderConflict],
          }),
        ),
      ).toBe(true);
      expect(
        useAppStore
          .getState()
          .applyFolderConflictReconcileResult(
            diskReconcileResult({ reconcile_revision: 2, status: 'SourceUnavailable' }),
          ),
      ).toBe(true);
      expect(useAppStore.getState().folderConflictReportsByGame.g1).toMatchObject({
        revision: 1,
        status: 'open',
        groups: [folderConflict],
      });

      expect(
        useAppStore
          .getState()
          .applyFolderConflictReconcileResult(
            diskReconcileResult({ reconcile_revision: 3, status: 'NeedsRenameConfirmation' }),
          ),
      ).toBe(true);
      expect(useAppStore.getState().folderConflictReportsByGame.g1).toMatchObject({
        revision: 3,
        status: 'resolvedExternally',
        groups: [],
      });
    });
  });

  describe('explorer prefs', () => {
    it('sort/view/search setters apply directly', () => {
      useAppStore.getState().setSortField('modified_at');
      useAppStore.getState().setSortOrder('desc');
      useAppStore.getState().setViewMode('list');
      useAppStore.getState().setExplorerSearch('abc');
      const s = useAppStore.getState();
      expect(s.sortField).toBe('modified_at');
      expect(s.sortOrder).toBe('desc');
      expect(s.viewMode).toBe('list');
      expect(s.explorerSearchQuery).toBe('abc');
    });

    it('updates the shared safety filter', () => {
      useAppStore.getState().setSafetyFilter('unsafe');

      expect(useAppStore.getState().safetyFilter).toBe('unsafe');
    });
  });
});
