import { describe, expect, it } from 'vitest';
import { render, screen } from '../../../testing/test-utils';
import { CollectionTreeView } from './CollectionTreeView';
import type { PreviewTreeNode } from '../../../types/collection';

function createTree(): PreviewTreeNode[] {
  return [
    {
      kind: 'object',
      id: 'object-1',
      name: 'AINOZ',
      path: 'AINOZ',
      object_id: 'object-1',
      node_type: null,
      is_enabled: true,
      is_effectively_active: true,
      inactive_reason: null,
      show_inactive_chip: false,
      status_kind: null,
      collapse_children: false,
      warnings: [],
      mod_count: 3,
      children: [
        {
          kind: 'folder',
          id: 'outer',
          name: 'Outer',
          path: 'AINOZ/Outer',
          object_id: 'object-1',
          node_type: 'ContainerFolder',
          is_enabled: true,
          is_effectively_active: true,
          inactive_reason: null,
          show_inactive_chip: false,
          status_kind: null,
          collapse_children: false,
          warnings: [],
          mod_count: null,
          children: [
            {
              kind: 'folder',
              id: 'mod-pack',
              name: 'Pack Alpha',
              path: 'AINOZ/Outer/Pack Alpha',
              object_id: 'object-1',
              node_type: 'ModPackRoot',
              is_enabled: true,
              is_effectively_active: true,
              inactive_reason: null,
              show_inactive_chip: false,
              status_kind: null,
              collapse_children: true,
              warnings: [],
              mod_count: null,
              children: [],
            },
          ],
        },
        {
          kind: 'folder',
          id: 'variants',
          name: 'ambercn_vest_school_uniform_toggle_v2',
          path: 'AINOZ/ambercn_vest_school_uniform_toggle_v2',
          object_id: 'object-1',
          node_type: 'VariantContainer',
          is_enabled: true,
          is_effectively_active: true,
          inactive_reason: null,
          show_inactive_chip: false,
          status_kind: null,
          collapse_children: true,
          warnings: ['[WARNING] Corrupt INI file: variants.ini (0 KB)'],
          mod_count: null,
          children: [],
        },
        {
          kind: 'mod',
          id: 'flat-mod',
          name: 'Loose Skin.ini',
          path: 'AINOZ/Loose Skin.ini',
          object_id: 'object-1',
          node_type: 'FlatModRoot',
          is_enabled: true,
          is_effectively_active: true,
          inactive_reason: null,
          show_inactive_chip: false,
          status_kind: null,
          collapse_children: false,
          warnings: [],
          mod_count: null,
          children: [],
        },
        {
          kind: 'folder',
          id: 'inactive-section',
          name: 'Inactive Containers',
          path: null,
          object_id: 'object-1',
          node_type: 'InactiveContainerSection',
          is_enabled: false,
          is_effectively_active: false,
          inactive_reason:
            'Children in this folder are treated as inactive because this container is disabled.',
          show_inactive_chip: false,
          status_kind: null,
          collapse_children: false,
          warnings: [],
          mod_count: 0,
          children: [
            {
              kind: 'folder',
              id: 'disabled-container',
              name: 'DISABLED School Extras',
              path: 'AINOZ/DISABLED School Extras',
              object_id: 'object-1',
              node_type: 'ContainerFolder',
              is_enabled: false,
              is_effectively_active: false,
              inactive_reason:
                'Children in this folder are treated as inactive because this container is disabled.',
              show_inactive_chip: true,
              status_kind: 'inactive_container',
              collapse_children: false,
              warnings: [],
              mod_count: null,
              children: [
                {
                  kind: 'folder',
                  id: 'inactive-variant',
                  name: 'School Variant',
                  path: 'AINOZ/DISABLED School Extras/School Variant',
                  object_id: 'object-1',
                  node_type: 'VariantContainer',
                  is_enabled: true,
                  is_effectively_active: false,
                  inactive_reason: null,
                  show_inactive_chip: false,
                  status_kind: 'disabled_by_container',
                  collapse_children: true,
                  warnings: [],
                  mod_count: null,
                  children: [],
                },
              ],
            },
          ],
        },
      ],
    },
  ];
}

describe('CollectionTreeView', () => {
  it('renders terminal flat, mod pack, and variant rows without nested variant children', () => {
    render(<CollectionTreeView nodes={createTree()} />);

    expect(screen.getByText('AINOZ')).toBeInTheDocument();
    expect(screen.getByText('Pack Alpha')).toBeInTheDocument();
    expect(screen.getByText('ambercn_vest_school_uniform_toggle_v2')).toBeInTheDocument();
    expect(screen.getByText('Loose Skin.ini')).toBeInTheDocument();
    expect(screen.queryByText('1.school_uniform')).not.toBeInTheDocument();
  });

  it('shows muted type chips for container, variants, mod pack, and flat mod nodes', () => {
    render(<CollectionTreeView nodes={createTree()} />);

    expect(screen.getAllByText('Container').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Variants').length).toBeGreaterThan(0);
    expect(screen.getByText('Mod Pack')).toBeInTheDocument();
    expect(screen.getByText('Flat Mod')).toBeInTheDocument();

    const variantsChip = screen.getAllByText('Variants')[0];
    expect(variantsChip.className).toContain('bg-base-200/70');
    expect(variantsChip.className).not.toContain('badge-secondary');
  });

  it('renders inactive container section separately with disabled chips', () => {
    render(<CollectionTreeView nodes={createTree()} />);

    expect(screen.getByText('tree.inactive_section')).toBeInTheDocument();
    expect(screen.getByText('tree.disabled')).toBeInTheDocument();
    expect(screen.getByText('tree.disabled_by_container')).toBeInTheDocument();
  });

  it('shows warning icon tooltip for corrupt variant container', () => {
    render(<CollectionTreeView nodes={createTree()} />);

    expect(
      screen.getByLabelText('[WARNING] Corrupt INI file: variants.ini (0 KB)'),
    ).toBeInTheDocument();
  });
});
