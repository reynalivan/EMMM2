import { describe, expect, it } from 'vitest';
import type { PreviewTreeNode } from '@/entities/collection';
import { buildApplyPreviewDiff, filterPreviewTreeToChanges } from './applyPreviewDiff';

function mod(id: string): PreviewTreeNode {
  return {
    kind: 'mod',
    id,
    name: id,
    path: id,
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
  };
}

function object(children: PreviewTreeNode[]): PreviewTreeNode {
  return {
    kind: 'object',
    id: 'object-1',
    name: 'Object',
    path: 'object',
    object_id: 'object-1',
    node_type: null,
    is_enabled: true,
    is_effectively_active: true,
    inactive_reason: null,
    show_inactive_chip: false,
    status_kind: null,
    collapse_children: false,
    warnings: [],
    mod_count: children.length,
    children,
  };
}

describe('buildApplyPreviewDiff', () => {
  it('derives effective Safe Mode changes while retaining excluded target mods', () => {
    const diff = buildApplyPreviewDiff({
      current_tree_nodes: [object([mod('root::shared'), mod('root::old')])],
      target_tree_nodes: [object([mod('root::shared'), mod('root::new'), mod('root::unsafe')])],
      effective_target_tree_nodes: [object([mod('root::shared'), mod('root::new')])],
    });

    expect(diff).toMatchObject({
      enableCount: 1,
      disableCount: 1,
      unchangedCount: 1,
      excludedBySafeModeCount: 1,
    });
    expect(diff.currentChanges.get('root::old')).toBe('will_disable');
    expect(diff.targetChanges.get('root::new')).toBe('will_enable');
    expect(diff.targetChanges.get('root::unsafe')).toBe('excluded_by_safe_mode');
  });

  it('keeps only changed branches in changes-only mode', () => {
    const diff = buildApplyPreviewDiff({
      current_tree_nodes: [object([mod('root::shared'), mod('root::old')])],
      target_tree_nodes: [object([mod('root::shared'), mod('root::new')])],
      effective_target_tree_nodes: [object([mod('root::shared'), mod('root::new')])],
    });

    const changesOnly = filterPreviewTreeToChanges(
      [object([mod('root::shared'), mod('root::old')])],
      diff.currentChanges,
      new Set(['will_disable']),
    );

    expect(changesOnly[0]?.children.map((node) => node.id)).toEqual(['root::old']);
    expect(changesOnly[0]?.mod_count).toBe(1);
  });
});
