import type { SortField, SortOrder, ViewMode } from '../../types/mod';
import type { AppSliceCreator } from './sliceTypes';

export interface ExplorerSlice {
  // Epic 4: Explorer State
  sortField: SortField;
  sortOrder: SortOrder;
  viewMode: ViewMode;
  explorerSubPath: string | undefined;
  explorerSearchQuery: string;
  explorerScrollOffset: number;

  setSortField: (field: SortField) => void;
  setSortOrder: (order: SortOrder) => void;
  setViewMode: (mode: ViewMode) => void;
  setExplorerSubPath: (subPath: string | undefined) => void;
  setExplorerSearch: (query: string) => void;
  setExplorerScrollOffset: (offset: number) => void;
}

export const createExplorerSlice: AppSliceCreator<ExplorerSlice> = (set) => ({
  sortField: 'name',
  sortOrder: 'asc',
  viewMode: 'grid',
  explorerSubPath: undefined,
  explorerSearchQuery: '',
  explorerScrollOffset: 0,

  setSortField: (field) => set({ sortField: field }),
  setSortOrder: (order) => set({ sortOrder: order }),
  setViewMode: (mode) => set({ viewMode: mode }),
  setExplorerSubPath: (subPath) => set({ explorerSubPath: subPath }),
  setExplorerSearch: (query) => set({ explorerSearchQuery: query }),
  setExplorerScrollOffset: (offset) => set({ explorerScrollOffset: offset }),
});
