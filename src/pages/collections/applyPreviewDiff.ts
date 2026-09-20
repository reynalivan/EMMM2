import type { ApplyPreview, PreviewTreeNode } from '@/entities/collection';

export type ApplyPreviewChange =
  'will_enable' | 'will_disable' | 'unchanged' | 'excluded_by_safe_mode';

export type ApplyPreviewDiff = {
  currentChanges: ReadonlyMap<string, ApplyPreviewChange>;
  targetChanges: ReadonlyMap<string, ApplyPreviewChange>;
  enableCount: number;
  disableCount: number;
  unchangedCount: number;
  excludedBySafeModeCount: number;
};

type ApplyPreviewTrees = Pick<
  ApplyPreview,
  'current_tree_nodes' | 'target_tree_nodes' | 'effective_target_tree_nodes'
>;

function collectModNodeIds(nodes: PreviewTreeNode[], ids = new Set<string>()): Set<string> {
  for (const node of nodes) {
    if (node.kind === 'mod') {
      ids.add(node.id);
    }
    collectModNodeIds(node.children, ids);
  }
  return ids;
}

export function buildApplyPreviewDiff(preview: ApplyPreviewTrees): ApplyPreviewDiff {
  const currentNodeIds = collectModNodeIds(preview.current_tree_nodes);
  const targetNodeIds = collectModNodeIds(preview.target_tree_nodes);
  const effectiveTargetNodeIds = collectModNodeIds(preview.effective_target_tree_nodes);
  const currentChanges = new Map<string, ApplyPreviewChange>();
  const targetChanges = new Map<string, ApplyPreviewChange>();
  let enableCount = 0;
  let disableCount = 0;
  let unchangedCount = 0;
  let excludedBySafeModeCount = 0;

  for (const nodeId of currentNodeIds) {
    if (effectiveTargetNodeIds.has(nodeId)) {
      currentChanges.set(nodeId, 'unchanged');
      unchangedCount += 1;
    } else {
      currentChanges.set(nodeId, 'will_disable');
      disableCount += 1;
    }
  }

  for (const nodeId of targetNodeIds) {
    if (!effectiveTargetNodeIds.has(nodeId)) {
      targetChanges.set(nodeId, 'excluded_by_safe_mode');
      excludedBySafeModeCount += 1;
    } else if (currentNodeIds.has(nodeId)) {
      targetChanges.set(nodeId, 'unchanged');
    } else {
      targetChanges.set(nodeId, 'will_enable');
      enableCount += 1;
    }
  }

  return {
    currentChanges,
    targetChanges,
    enableCount,
    disableCount,
    unchangedCount,
    excludedBySafeModeCount,
  };
}

export function filterPreviewTreeToChanges(
  nodes: PreviewTreeNode[],
  changes: ReadonlyMap<string, ApplyPreviewChange>,
  includedChanges: ReadonlySet<ApplyPreviewChange>,
): PreviewTreeNode[] {
  return nodes.flatMap((node) => {
    const filtered = filterNodeToChanges(node, changes, includedChanges);
    return filtered ? [filtered] : [];
  });
}

function filterNodeToChanges(
  node: PreviewTreeNode,
  changes: ReadonlyMap<string, ApplyPreviewChange>,
  includedChanges: ReadonlySet<ApplyPreviewChange>,
): PreviewTreeNode | null {
  if (node.kind === 'mod') {
    return includedChanges.has(changes.get(node.id) ?? 'unchanged') ? node : null;
  }

  const children = node.children.flatMap((child) => {
    const filtered = filterNodeToChanges(child, changes, includedChanges);
    return filtered ? [filtered] : [];
  });
  if (children.length === 0) {
    return null;
  }

  return {
    ...node,
    children,
    mod_count: node.mod_count === null ? null : countModNodes(children),
  };
}

function countModNodes(nodes: PreviewTreeNode[]): number {
  return nodes.reduce(
    (count, node) => count + Number(node.kind === 'mod') + countModNodes(node.children),
    0,
  );
}
