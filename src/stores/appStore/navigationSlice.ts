import type { AppSliceCreator } from './sliceTypes';

export type WorkspaceView =
  | 'dashboard'
  | 'mods'
  | 'mod-inbox'
  | 'collections'
  | 'settings'
  | 'browser'
  | 'downloads'
  | 'storage-optimizer';
export type MobilePane = 'sidebar' | 'grid' | 'details';
export type SettingsTab =
  | 'general'
  | 'games'
  | 'browser'
  | 'privacy'
  | 'hotkeys'
  | 'ai'
  | 'maintenance'
  | 'updates'
  | 'logs';

export interface NavigationSlice {
  // Navigation State
  workspaceView: WorkspaceView;
  settingsTab: SettingsTab;
  currentPath: string[];

  // Mobile Navigation State
  mobileActivePane: MobilePane;

  // Context-Aware Selection
  activePane: 'objectList' | 'folderGrid';

  setWorkspaceView: (view: WorkspaceView) => void;
  setSettingsTab: (tab: SettingsTab) => void;
  setCurrentPath: (path: string[]) => void;
  setMobilePane: (pane: MobilePane) => void;
  setActivePane: (pane: 'objectList' | 'folderGrid') => void;
}

export const createNavigationSlice: AppSliceCreator<NavigationSlice> = (set) => ({
  workspaceView: 'dashboard',
  settingsTab: 'general',
  currentPath: [],
  mobileActivePane: 'sidebar',
  activePane: 'objectList',

  setWorkspaceView: (view) => set({ workspaceView: view }),
  setSettingsTab: (tab) => set({ settingsTab: tab }),
  setCurrentPath: (path) => set({ currentPath: path }),
  setMobilePane: (pane) => set({ mobileActivePane: pane }),
  setActivePane: (pane) => set({ activePane: pane }),
});
