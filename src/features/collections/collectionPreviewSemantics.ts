import type { TFunction } from 'i18next';
import type { PreviewTreeNode } from '../../types/collection';

export interface CollectionPreviewNodeSemantics {
  typeLabelKey: string | null;
  statusLabel: string | null;
  warningTitle: string | null;
}

const NODE_TYPE_LABEL_KEYS: Record<string, string> = {
  VariantContainer: 'tree.types.variant_container',
  ModPackRoot: 'tree.types.mod_pack_root',
  FlatModRoot: 'tree.types.flat_mod_root',
  ContainerFolder: 'tree.types.container_folder',
};

export function buildCollectionPreviewNodeSemantics(
  t: TFunction,
  node: Pick<
    PreviewTreeNode,
    'node_type' | 'status_kind' | 'show_inactive_chip' | 'warnings' | 'inactive_reason'
  >,
): CollectionPreviewNodeSemantics {
  const typeLabelKey = node.node_type ? (NODE_TYPE_LABEL_KEYS[node.node_type] ?? null) : null;
  const statusLabel = getCollectionNodeStatusLabel(t, node);
  const warningTitle = getCollectionNodeWarningTitle(node);

  return {
    typeLabelKey,
    statusLabel,
    warningTitle,
  };
}

function getCollectionNodeStatusLabel(
  t: TFunction,
  node: Pick<PreviewTreeNode, 'status_kind' | 'show_inactive_chip'>,
): string | null {
  if (node.status_kind === 'disabled_by_container') {
    return t('tree.disabled_by_container');
  }

  if (node.status_kind === 'inactive_container' || node.show_inactive_chip) {
    return t('tree.disabled');
  }

  return null;
}

function getCollectionNodeWarningTitle(
  node: Pick<PreviewTreeNode, 'warnings' | 'inactive_reason'>,
): string | null {
  const messages = [...node.warnings];
  if (node.inactive_reason) {
    messages.push(node.inactive_reason);
  }

  if (messages.length === 0) {
    return null;
  }

  return messages.join('\n');
}
