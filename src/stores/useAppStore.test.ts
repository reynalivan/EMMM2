import { beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from './useAppStore';

// Snapshot the pristine state (defaults + actions) once, restore before each test.
const initialState = useAppStore.getState();

beforeEach(() => {
  useAppStore.setState(initialState, true);
});

describe('useAppStore smoke net', () => {
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
    });
  });

  describe('navigation / routing', () => {
    it('switches workspace view and path', () => {
      useAppStore.getState().setWorkspaceView('collections');
      useAppStore.getState().setCurrentPath(['Mods', 'Diluc']);
      expect(useAppStore.getState().workspaceView).toBe('collections');
      expect(useAppStore.getState().currentPath).toEqual(['Mods', 'Diluc']);
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
      });

      useAppStore.getState().setDiskReconcileTimestamp('g1', 1234);
      expect(useAppStore.getState().diskReconcileByGame.g1).toEqual({
        at: 1234,
        pending: false,
        unavailable: null,
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
  });
});
