import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { createExplorerSlice, type ExplorerSlice } from './appStore/explorerSlice';
import { createGameSlice, type GameSlice } from './appStore/gameSlice';
import { createLayoutSlice, type LayoutSlice } from './appStore/layoutSlice';
import { createNavigationSlice, type NavigationSlice } from './appStore/navigationSlice';
import { createObjectListSlice, type ObjectListSlice } from './appStore/objectListSlice';
import { createSelectionSlice, type SelectionSlice } from './appStore/selectionSlice';
import {
  createWorkspaceRuntimeSlice,
  type WorkspaceRuntimeSlice,
} from './appStore/workspaceRuntimeSlice';

export interface AppState
  extends
    GameSlice,
    NavigationSlice,
    LayoutSlice,
    SelectionSlice,
    ObjectListSlice,
    ExplorerSlice,
    WorkspaceRuntimeSlice {}

/** What actually lands in LocalStorage: `partialize`'s output after JSON round-trip. */
type PersistedAppState = Omit<Partial<AppState>, 'collapsedCategories'> & {
  collapsedCategories?: string[];
};

// Custom debounced storage to prevent LocalStorage spam
const debouncedStorage = {
  getItem: (name: string) => {
    return localStorage.getItem(name);
  },
  setItem: (() => {
    let timeoutId: number | null = null;
    return (name: string, value: string) => {
      if (timeoutId) window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        localStorage.setItem(name, value);
      }, 300); // 300ms debounce
    };
  })(),
  removeItem: (name: string) => {
    localStorage.removeItem(name);
  },
};

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      ...createGameSlice(set, get),
      ...createNavigationSlice(set, get),
      ...createLayoutSlice(set, get),
      ...createSelectionSlice(set, get),
      ...createObjectListSlice(set, get),
      ...createExplorerSlice(set, get),
      ...createWorkspaceRuntimeSlice(set, get),
    }),
    {
      name: 'vibecode-storage',
      storage: createJSONStorage(() => debouncedStorage),
      partialize: (state) => ({
        leftPanelWidth: state.leftPanelWidth,
        rightPanelWidth: state.rightPanelWidth,
        isPreviewOpen: state.isPreviewOpen,
        // Epic 4: Persist explorer preferences
        sortField: state.sortField,
        sortOrder: state.sortOrder,
        viewMode: state.viewMode,
        currentPath: state.currentPath,
        explorerSubPath: state.explorerSubPath,
        explorerScrollOffset: state.explorerScrollOffset,

        // Epic 3: Persist collapsed categories (serializable array)
        collapsedCategories: Array.from(state.collapsedCategories),
      }),
      // The default shallow merge already restores everything `partialize` wrote;
      // only `collapsedCategories` needs a hand because a Set can't survive JSON.
      // ponytail: keep it this thin — a new persisted key belongs in `partialize` alone.
      merge: (persistedState: unknown, currentState) => {
        const pState = (persistedState ?? {}) as PersistedAppState;

        return {
          ...currentState,
          ...pState,
          collapsedCategories: pState.collapsedCategories
            ? new Set(pState.collapsedCategories)
            : currentState.collapsedCategories,
        };
      },
    },
  ),
);
