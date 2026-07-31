import {
  rewriteWorkspacePathValue,
  type WorkspacePathRewriteInput,
} from '../../features/workspace-runtime/pathRewrite';
import type { AppSliceCreator } from './sliceTypes';

export interface SelectionSlice {
  // Selection State
  selectedObjectFolderPath: string | null;
  selectedModPath: string | null;
  gridSelection: Set<string>;

  setSelectedObjectFolderPath: (folderPath: string | null) => void;
  toggleGridSelection: (id: string, multi?: boolean) => void;
  clearGridSelection: () => void;
  setGridSelection: (selection: Set<string>) => void;
  replaceGridSelections: (rewrites: WorkspacePathRewriteInput[]) => void;
}

export const createSelectionSlice: AppSliceCreator<SelectionSlice> = (set) => ({
  selectedObjectFolderPath: null,
  selectedModPath: null,
  gridSelection: new Set(),

  setSelectedObjectFolderPath: (folderPath) =>
    set({
      selectedObjectFolderPath: folderPath,
      // Auto-navigate to grid on mobile when object selected
      mobileActivePane: folderPath ? 'grid' : 'sidebar',
    }),

  toggleGridSelection: (id, multi = false) =>
    set((state) => {
      const newSet = new Set(multi ? state.gridSelection : []);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }

      // Auto-navigate to details on mobile when item selected (single select)
      const nextMobilePane = newSet.size > 0 && !multi ? 'details' : state.mobileActivePane;

      return {
        gridSelection: newSet,
        selectedModPath: newSet.size > 0 ? id : null,
        mobileActivePane: nextMobilePane,
      };
    }),

  clearGridSelection: () => set({ gridSelection: new Set(), selectedModPath: null }),

  setGridSelection: (selection) =>
    set((state) => {
      // Auto-navigate to details on mobile when item selected (single select)
      const nextMobilePane = selection.size === 1 ? 'details' : state.mobileActivePane;
      const selectionEntries = Array.from(selection);
      return {
        gridSelection: selection,
        selectedModPath:
          selectionEntries.length > 0 ? selectionEntries[selectionEntries.length - 1] : null,
        mobileActivePane: nextMobilePane,
      };
    }),

  replaceGridSelections: (rewrites) =>
    set((state) => {
      if (rewrites.length === 0) {
        return state;
      }

      const originalEntries = Array.from(state.gridSelection);
      const rewrittenEntries = originalEntries.map((path) => {
        return rewriteWorkspacePathValue(path, rewrites) ?? path;
      });
      const changed = rewrittenEntries.some((path, index) => {
        return path !== originalEntries[index];
      });
      const rewrittenSelectedModPath = state.selectedModPath
        ? rewriteWorkspacePathValue(state.selectedModPath, rewrites)
        : null;
      const selectedModPathChanged =
        !!rewrittenSelectedModPath && rewrittenSelectedModPath !== state.selectedModPath;

      if (!changed && !selectedModPathChanged) {
        return state;
      }

      const newSet = new Set(rewrittenEntries);
      return {
        gridSelection: newSet,
        selectedModPath: selectedModPathChanged ? rewrittenSelectedModPath : state.selectedModPath,
      };
    }),
});
