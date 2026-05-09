import { describe, expect, it } from 'vitest';
import type { FilterDef } from '../../types/object';
import { areObjectMetaFiltersEqual, sanitizeObjectMetaFilters } from './objectFilterState';

describe('objectFilterState', () => {
  const categoryFilters: FilterDef[] = [
    { key: 'element', label: 'Element', options: ['Pyro', 'Hydro'] },
    { key: 'weapon', label: 'Weapon', options: ['Sword'] },
  ];

  it('compares object meta filters by semantic value', () => {
    expect(
      areObjectMetaFiltersEqual(
        { element: ['Pyro'], weapon: ['Sword'] },
        { weapon: ['Sword'], element: ['Pyro'] },
      ),
    ).toBe(true);
    expect(areObjectMetaFiltersEqual({ element: ['Pyro'] }, { element: ['Hydro'] })).toBe(false);
  });

  it('sanitizes invalid filter keys without changing valid values', () => {
    expect(
      sanitizeObjectMetaFilters(
        { element: ['Pyro'], weapon: ['Sword'], rarity: ['5'] },
        categoryFilters,
      ),
    ).toEqual({
      element: ['Pyro'],
      weapon: ['Sword'],
    });
  });
});
