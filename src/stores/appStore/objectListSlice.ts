import { areObjectMetaFiltersEqual } from '../../features/object-list/utils/objectFilterState';
import type { AppSliceCreator } from './sliceTypes';

export interface ObjectListSlice {
  // Epic 3: Sidebar State
  selectedObjectType: string | null;
  sidebarSearchQuery: string;
  collapsedCategories: Set<string>;
  objectMetaFilters: Record<string, string[]>;
  objectSortBy: 'name' | 'date' | 'rarity';
  objectStatusFilter: 'all' | 'enabled' | 'disabled';

  setSelectedObjectType: (type: string | null) => void;
  setSidebarSearch: (query: string) => void;
  toggleCategoryCollapse: (category: string) => void;
  setObjectMetaFilters: (filters: Record<string, string[]>) => void;
  setObjectSortBy: (sortBy: 'name' | 'date' | 'rarity') => void;
  setObjectStatusFilter: (filter: 'all' | 'enabled' | 'disabled') => void;
}

export const createObjectListSlice: AppSliceCreator<ObjectListSlice> = (set) => ({
  selectedObjectType: null,
  sidebarSearchQuery: '',
  collapsedCategories: new Set(),
  objectMetaFilters: {},
  objectSortBy: 'name',
  objectStatusFilter: 'all',

  setSelectedObjectType: (type) =>
    set((state) => (state.selectedObjectType === type ? state : { selectedObjectType: type })),
  setSidebarSearch: (query) =>
    set((state) => (state.sidebarSearchQuery === query ? state : { sidebarSearchQuery: query })),
  toggleCategoryCollapse: (category) =>
    set((state) => {
      const next = new Set(state.collapsedCategories);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return { collapsedCategories: next };
    }),
  setObjectMetaFilters: (filters) =>
    set((state) =>
      areObjectMetaFiltersEqual(state.objectMetaFilters, filters)
        ? state
        : { objectMetaFilters: filters },
    ),
  setObjectSortBy: (sortBy) =>
    set((state) => (state.objectSortBy === sortBy ? state : { objectSortBy: sortBy })),
  setObjectStatusFilter: (filter) =>
    set((state) => (state.objectStatusFilter === filter ? state : { objectStatusFilter: filter })),
});
