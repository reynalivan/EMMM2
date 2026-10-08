import type { PreviewTreeNode } from '@/entities/collection';
import type { ApplyPreviewChange } from '../applyPreviewDiff';
export interface CollectionTreeViewProps {
  nodes?: PreviewTreeNode[];
  gameId?: string | null;
  colorClass?: string;
  emptyMessage?: string;
  scrollElement?: HTMLDivElement | null;
  treeIdentity?: string;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
}

export type CollectionTreeNodeChange = ApplyPreviewChange;

export const SECTION_NODE_TYPE = 'InactiveContainerSection';
const LAZY_CHILDREN_THRESHOLD = 80;
export function shouldStartCollapsed(node: PreviewTreeNode): boolean {
  return Math.max(node.mod_count ?? 0, node.children.length) >= LAZY_CHILDREN_THRESHOLD;
}

type FlatTreeRowKind = 'object' | 'folder' | 'leaf' | 'inactive';

export interface FlatTreeRow {
  key: string;
  kind: FlatTreeRowKind;
  node: PreviewTreeNode;
  depth: number;
}

export function countTreeNodes(nodes: PreviewTreeNode[]): number {
  return nodes.reduce((count, node) => count + 1 + countTreeNodes(node.children), 0);
}

function appendFolderRows(
  node: PreviewTreeNode,
  depth: number,
  rows: FlatTreeRow[],
  isExpanded: (node: PreviewTreeNode) => boolean,
) {
  if (node.kind === 'mod') {
    rows.push({ key: `leaf:${node.id}`, kind: 'leaf', node, depth });
    return;
  }

  rows.push({ key: `folder:${node.id}`, kind: 'folder', node, depth });
  if (node.children.length > 0 && !node.collapse_children && isExpanded(node)) {
    node.children.forEach((child) => appendFolderRows(child, depth + 1, rows, isExpanded));
  }
}

export function flattenTree(
  nodes: PreviewTreeNode[],
  isExpanded: (node: PreviewTreeNode) => boolean,
): FlatTreeRow[] {
  const rows: FlatTreeRow[] = [];
  for (const objectNode of nodes) {
    rows.push({ key: `object:${objectNode.id}`, kind: 'object', node: objectNode, depth: 0 });
    if (!isExpanded(objectNode)) continue;

    const inactiveSections = objectNode.children.filter(
      (child) => child.node_type === SECTION_NODE_TYPE,
    );
    objectNode.children
      .filter((child) => child.node_type !== SECTION_NODE_TYPE)
      .forEach((child) => appendFolderRows(child, 0, rows, isExpanded));

    for (const inactiveSection of inactiveSections) {
      rows.push({
        key: `inactive:${inactiveSection.id}`,
        kind: 'inactive',
        node: inactiveSection,
        depth: 0,
      });
      if (isExpanded(inactiveSection)) {
        inactiveSection.children.forEach((child) => appendFolderRows(child, 0, rows, isExpanded));
      }
    }
  }
  return rows;
}
