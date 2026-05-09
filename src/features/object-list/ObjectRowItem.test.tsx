import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import ObjectRowItem from './ObjectRowItem';
import type { WorkspaceCapabilities, WorkspaceObjectNode } from '../../types/workspace';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, string>) => {
      if (key === 'item.matched_alias' && values?.alias) {
        return `Matched: ${values.alias}`;
      }
      if (key === 'item.disabled_overlay') {
        return 'Disabled';
      }
      if (key === 'item.conflict_tooltip') {
        return 'Conflict';
      }
      return key;
    },
  }),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'game-1',
      mod_path: 'E:/Mods',
    },
  }),
}));

vi.mock('../../hooks/useThumbnail', () => ({
  useThumbnail: () => ({
    data: null,
  }),
}));

const baseObject: WorkspaceObjectNode = {
  id: 'obj-1',
  name: 'Albedo',
  display_name: 'Albedo',
  node_kind: 'object',
  display_mode: 'unknown',
  type_chip: null,
  folder_path: 'Albedo',
  matched_entry_key: null,
  matched_alias_name: null,
  matched_confidence: null,
  matched_reason: null,
  matched_source: null,
  object_type: 'Character',
  sub_category: null,
  status: 1,
  created_at: null,
  mod_count: 10,
  enabled_count: 2,
  thumbnail_path: null,
  is_pinned: false,
  is_auto_sync: false,
  is_object_disabled: false,
  has_naming_conflict: false,
  is_effectively_active: true,
  inactive_reason: null,
  warning_state: 'none',
  primary_warning: null,
  switch_state: 'enabled',
  switch_reason: null,
  switch_policy_key: 'object',
  capabilities: {
    can_toggle: true,
    can_rename: true,
    can_delete: true,
    can_move: false,
    can_toggle_safe: false,
    can_sync: true,
    can_enable_only_this: false,
    can_pin: true,
    can_edit_metadata: true,
    can_reveal_in_explorer: true,
    can_move_category: true,
    can_open_in_explorer: true,
  } satisfies WorkspaceCapabilities,
  metadata: '{}',
  tags: '[]',
  hash_db: null,
  custom_skins: null,
  active_mod_paths: null,
};

describe('ObjectRowItem', () => {
  it('renders a combined active and total mod count badge', () => {
    render(
      <ObjectRowItem obj={baseObject} isSelected={false} isMobile={false} onClick={vi.fn()} />,
    );

    expect(screen.getByText('2/10')).toBeInTheDocument();
  });

  it('hides the combined badge when the object has no terminal mods', () => {
    render(
      <ObjectRowItem
        obj={{
          ...baseObject,
          mod_count: 0,
          enabled_count: 0,
        }}
        isSelected={false}
        isMobile={false}
        onClick={vi.fn()}
      />,
    );

    expect(screen.queryByText('0/0')).not.toBeInTheDocument();
  });

  it('updates visible row fields when the object props change', () => {
    const { rerender } = render(
      <ObjectRowItem obj={baseObject} isSelected={false} isMobile={false} onClick={vi.fn()} />,
    );

    rerender(
      <ObjectRowItem
        obj={{
          ...baseObject,
          name: 'Albedo Prime',
          folder_path: 'DISABLED Albedo',
          is_object_disabled: true,
          enabled_count: 1,
          mod_count: 3,
        }}
        isSelected={false}
        isMobile={false}
        onClick={vi.fn()}
      />,
    );

    expect(screen.getByText('Albedo Prime')).toBeInTheDocument();
    expect(screen.getByText('1/3')).toBeInTheDocument();
    expect(screen.getByTestId('power-off-overlay')).toBeInTheDocument();
  });

  it('dims inactive rows when they have zero active mods', () => {
    render(
      <ObjectRowItem
        obj={{
          ...baseObject,
          enabled_count: 0,
          mod_count: 3,
          is_effectively_active: false,
          inactive_reason: null,
        }}
        isSelected={false}
        isMobile={false}
        onClick={vi.fn()}
      />,
    );

    const row = screen.getByRole('button');
    expect(row.className).toContain('bg-base-200/25');
    expect(screen.getByText('0/3').className).toContain('bg-base-300/35');
  });
});
