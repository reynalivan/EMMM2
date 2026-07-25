import type { AppSliceCreator } from './sliceTypes';

export type WorkspaceView =
  'dashboard' | 'mods' | 'collections' | 'settings' | 'browser' | 'downloads' | 'storage-optimizer';
export type MobilePane = 'sidebar' | 'grid' | 'details';

export interface NavigationSlice {
  // Navigation State
  workspaceView: WorkspaceView;
  currentPath: string[];

  // Mobile Navigation State
  mobileActivePane: MobilePane;

  // Context-Aware Selection
  activePane: 'objectList' | 'folderGrid';

  setWorkspaceView: (view: WorkspaceView) => void;
  setCurrentPath: (path: string[]) => void;
  setMobilePane: (pane: MobilePane) => void;
  setActivePane: (pane: 'objectList' | 'folderGrid') => void;
}

export const createNavigationSlice: AppSliceCreator<NavigationSlice> = (set) => ({
  workspaceView: 'dashboard',
  currentPath: [],
  mobileActivePane: 'sidebar',
  activePane: 'objectList',

  setWorkspaceView: (view) => set({ workspaceView: view }),
  setCurrentPath: (path) => set({ currentPath: path }),
  setMobilePane: (pane) => set({ mobileActivePane: pane }),
  setActivePane: (pane) => set({ activePane: pane }),
});
