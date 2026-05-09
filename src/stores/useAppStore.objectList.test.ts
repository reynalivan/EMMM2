import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from './useAppStore';

describe('useAppStore object-list setters', () => {
  beforeEach(() => {
    useAppStore.setState({
      selectedObjectType: null,
      sidebarSearchQuery: '',
      objectMetaFilters: {},
      objectSortBy: 'name',
      objectStatusFilter: 'all',
    });
  });

  it('does not publish when setting equivalent object meta filters', () => {
    useAppStore.setState({ objectMetaFilters: { element: ['Pyro'] } });
    const listener = vi.fn();
    const unsubscribe = useAppStore.subscribe(listener);

    useAppStore.getState().setObjectMetaFilters({ element: ['Pyro'] });

    expect(listener).not.toHaveBeenCalled();
    unsubscribe();
  });

  it('does not publish when setting equivalent primitive object-list controls', () => {
    useAppStore.setState({
      selectedObjectType: 'Character',
      sidebarSearchQuery: 'diluc',
      objectSortBy: 'date',
      objectStatusFilter: 'enabled',
    });
    const listener = vi.fn();
    const unsubscribe = useAppStore.subscribe(listener);

    useAppStore.getState().setSelectedObjectType('Character');
    useAppStore.getState().setSidebarSearch('diluc');
    useAppStore.getState().setObjectSortBy('date');
    useAppStore.getState().setObjectStatusFilter('enabled');

    expect(listener).not.toHaveBeenCalled();
    unsubscribe();
  });
});
