import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useObjectListVirtualizer } from './useObjectListVirtualizer';

import type { WorkspaceObjectNode } from '../../types/workspace';

vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: vi.fn(() => ({
    measurementsCache: [],
    scrollToIndex: vi.fn(),
  })),
}));

describe('useObjectListVirtualizer', () => {
  const mockSchema = {
    categories: [
      { name: 'Character', icon: 'User', color: 'primary' },
      { name: 'Weapon', icon: 'Sword', color: 'secondary' },
      { name: 'Other', icon: 'Package', color: 'neutral' },
    ],
  };

  const mockObjects: WorkspaceObjectNode[] = [
    {
      id: '1',
      name: 'Zeta',
      display_name: 'Zeta',
      node_kind: 'object',
      object_type: 'Character',
      is_pinned: false,
      thumbnail_path: null,
      folder_path: 'zeta',
      mod_count: 0,
      enabled_count: 0,
      is_object_disabled: false,
      has_naming_conflict: false,
      is_effectively_active: false,
      inactive_reason: null,
      warning_state: 'none',
      primary_warning: null,
      metadata: '{}',
      tags: '[]',
    } as WorkspaceObjectNode,
    {
      id: '2',
      name: 'Alpha',
      display_name: 'Alpha',
      node_kind: 'object',
      object_type: 'Character',
      is_pinned: false,
      thumbnail_path: null,
      folder_path: 'alpha',
      mod_count: 0,
      enabled_count: 0,
      is_object_disabled: false,
      has_naming_conflict: false,
      is_effectively_active: false,
      inactive_reason: null,
      warning_state: 'none',
      primary_warning: null,
      metadata: '{}',
      tags: '[]',
    } as WorkspaceObjectNode,
    {
      id: '3',
      name: 'Sword 1',
      display_name: 'Sword 1',
      node_kind: 'object',
      object_type: 'Weapon',
      is_pinned: false,
      thumbnail_path: null,
      folder_path: 'sword-1',
      mod_count: 0,
      enabled_count: 0,
      is_object_disabled: false,
      has_naming_conflict: false,
      is_effectively_active: false,
      inactive_reason: null,
      warning_state: 'none',
      primary_warning: null,
      metadata: '{}',
      tags: '[]',
    } as WorkspaceObjectNode,
    {
      id: '4',
      name: 'Random',
      display_name: 'Random',
      node_kind: 'object',
      object_type: 'Other',
      sub_category: 'Misc',
      is_pinned: false,
      thumbnail_path: null,
      folder_path: 'random',
      mod_count: 0,
      enabled_count: 0,
      is_object_disabled: false,
      has_naming_conflict: false,
      is_effectively_active: false,
      inactive_reason: null,
      warning_state: 'none',
      primary_warning: null,
      metadata: '{}',
      tags: '[]',
    } as WorkspaceObjectNode,
    {
      id: '5',
      name: 'Glitch',
      display_name: 'Glitch',
      node_kind: 'object',
      object_type: 'Unknown',
      is_pinned: false,
      thumbnail_path: null,
      folder_path: 'glitch',
      mod_count: 0,
      enabled_count: 0,
      is_object_disabled: false,
      has_naming_conflict: false,
      is_effectively_active: false,
      inactive_reason: null,
      warning_state: 'none',
      primary_warning: null,
      metadata: '{}',
      tags: '[]',
    } as WorkspaceObjectNode,
  ];

  it('flattens objects correctly into headers and rows', () => {
    const { result } = renderHook(() =>
      useObjectListVirtualizer({
        objects: mockObjects,
        schema: mockSchema as unknown as import('../../types/object').GameSchema,
        selectedObjectFolderPath: null,
        isMobile: false,
      }),
    );

    const items = result.current.flatObjectItems;

    // Expected order:
    // Header: Character
    // Row: Alpha (sorted alphabetically)
    // Row: Zeta
    // Header: Weapon
    // Row: Sword 1
    // Header: Other
    // Sub-header: Misc
    // Row: Random
    // Header: Uncategorized
    // Row: Glitch

    expect(items).toHaveLength(10);
    expect(items[0]).toEqual(expect.objectContaining({ type: 'header', count: 2 }));
    expect(items[1]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Alpha' }) }),
    );
    expect(items[2]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Zeta' }) }),
    );
    expect(items[3]).toEqual(expect.objectContaining({ type: 'header', count: 1 }));
    expect(items[4]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Sword 1' }) }),
    );
    expect(items[5]).toEqual(expect.objectContaining({ type: 'header', count: 1 }));
    expect(items[6]).toEqual(
      expect.objectContaining({ type: 'sub-header', label: 'Misc', count: 1 }),
    );
    expect(items[7]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Random' }) }),
    );
    expect(items[8]).toEqual(expect.objectContaining({ type: 'header', count: 1 }));
    expect(items[9]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Glitch' }) }),
    );
  });

  it('computes selectedIndex based on selectedObject', () => {
    const { result, rerender } = renderHook((props) => useObjectListVirtualizer(props), {
      initialProps: {
        objects: mockObjects,
        schema: mockSchema as unknown as import('../../types/object').GameSchema,
        selectedObjectFolderPath: 'zeta', // Zeta
        isMobile: false,
      },
    });

    expect(result.current.selectedIndex).toBe(2); // Zeta is 3rd item (index 2)

    rerender({
      objects: mockObjects,
      schema: mockSchema as unknown as import('../../types/object').GameSchema,
      selectedObjectFolderPath: 'none',
      isMobile: false,
    });

    expect(result.current.selectedIndex).toBe(-1);
  });

  it('keeps pinned objects at the top of each section', () => {
    const { result } = renderHook(() =>
      useObjectListVirtualizer({
        objects: [
          {
            ...mockObjects[0],
            name: 'Beta',
            is_pinned: false,
          },
          {
            ...mockObjects[1],
            name: 'Alpha',
            is_pinned: true,
          },
        ],
        schema: mockSchema as unknown as import('../../types/object').GameSchema,
        selectedObjectFolderPath: null,
        isMobile: false,
      }),
    );

    expect(result.current.flatObjectItems[1]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Alpha' }) }),
    );
    expect(result.current.flatObjectItems[2]).toEqual(
      expect.objectContaining({ type: 'row', obj: expect.objectContaining({ name: 'Beta' }) }),
    );
  });
});
