import type { ApplyPreview, PreviewTreeNode } from '@/entities/collection';

export type ApplyPreviewChange =
  'will_enable' | 'will_disable' | 'unchanged' | 'excluded_by_safe_mode' | 'missing';

export type ApplyPreviewDiff = {
  currentChanges: ReadonlyMap<string, ApplyPreviewChange>;
  targetChanges: ReadonlyMap<string, ApplyPreviewChange>;
  enableCount: number;
  disableCount: number;
  unchangedCount: number;
  excludedBySafeModeCount: number;
  objectEnableCount: number;
  objectDisableCount: number;
};

type ApplyPreviewTrees = Pick<
  ApplyPreview,
  'current_tree_nodes' | 'target_tree_nodes' | 'effective_target_tree_nodes'
>;

function collectNodes(
  nodes: PreviewTreeNode[],
  result = new Map<string, PreviewTreeNode>(),
): Map<string, PreviewTreeNode> {
  for (const node of nodes) {
    if (node.kind !== 'folder') {
      result.set(node.id, node);
    }
    collectNodes(node.children, result);
  }
  return result;
}

export function buildApplyPreviewDiff(preview: ApplyPreviewTrees): ApplyPreviewDiff {
  const currentNodes = collectNodes(preview.current_tree_nodes);
  const targetNodes = collectNodes(preview.target_tree_nodes);
  const effectiveTargetNodes = collectNodes(preview.effective_target_tree_nodes);
  const currentChanges = new Map<string, ApplyPreviewChange>();
  const targetChanges = new Map<string, ApplyPreviewChange>();
  let enableCount = 0;
  let disableCount = 0;
  let unchangedCount = 0;
  let excludedBySafeModeCount = 0;
  let objectEnableCount = 0;
  let objectDisableCount = 0;

  for (const [nodeId, node] of currentNodes) {
    if (node.kind !== 'mod' || node.status_kind === 'missing') continue;
    const parentWillEnable = node.object_id && effectiveTargetNodes.get(node.object_id)?.is_enabled;
    if (!node.is_effectively_active && !parentWillEnable) continue;
    if (effectiveTargetNodes.get(nodeId)?.is_effectively_active) {
      currentChanges.set(nodeId, 'unchanged');
      unchangedCount += 1;
    } else {
      currentChanges.set(nodeId, 'will_disable');
      disableCount += 1;
    }
  }

  for (const [nodeId, node] of targetNodes) {
    if (node.kind === 'object') {
      const current = currentNodes.get(nodeId);
      if (!current || current.is_enabled === node.is_enabled) continue;
      const change = node.is_enabled ? 'will_enable' : 'will_disable';
      currentChanges.set(nodeId, change);
      targetChanges.set(nodeId, change);
      objectEnableCount += Number(node.is_enabled);
      objectDisableCount += Number(!node.is_enabled);
      continue;
    }
    if (node.status_kind === 'missing') {
      targetChanges.set(nodeId, 'missing');
    } else if (!effectiveTargetNodes.has(nodeId)) {
      targetChanges.set(nodeId, 'excluded_by_safe_mode');
      excludedBySafeModeCount += 1;
    } else if (!effectiveTargetNodes.get(nodeId)?.is_effectively_active) {
      targetChanges.set(nodeId, 'unchanged');
    } else if (currentNodes.get(nodeId)?.is_effectively_active) {
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
    objectEnableCount,
    objectDisableCount,
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
  if (children.length === 0 && !includedChanges.has(changes.get(node.id) ?? 'unchanged')) {
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
