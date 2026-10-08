import type { PreviewTreeNode } from '@/entities/collection';
import type { CollectionTreeNodeChange } from './collectionTreeStructure';
import { AlertTriangle, Folder, FolderOpen, Layers, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModThumbnail } from '@/entities/mod';
import { buildCollectionPreviewNodeSemantics } from '../collectionPreviewSemantics';
const TYPE_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-base-content/10 bg-base-200/70 text-[9px] uppercase tracking-wide text-base-content/55';
const STATUS_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-warning/20 bg-warning/10 text-[9px] uppercase tracking-wide text-warning/80';
export function NodeTypeChip({ nodeType }: { nodeType: string | null }) {
  const { t } = useTranslation('collections');
  const semantics = buildCollectionPreviewNodeSemantics(t, {
    node_type: nodeType,
    status_kind: null,
    show_inactive_chip: false,
    warnings: [],
    inactive_reason: null,
  });
  if (!semantics.typeLabelKey) {
    return null;
  }

  return <span className={TYPE_CHIP_CLASS_NAME}>{t(semantics.typeLabelKey)}</span>;
}

export function StatusChip({
  node,
}: {
  node: Pick<PreviewTreeNode, 'status_kind' | 'show_inactive_chip'>;
}) {
  const { t } = useTranslation('collections');
  const semantics = buildCollectionPreviewNodeSemantics(t, {
    node_type: null,
    status_kind: node.status_kind,
    show_inactive_chip: node.show_inactive_chip,
    warnings: [],
    inactive_reason: null,
  });
  if (!semantics.statusLabel) {
    return null;
  }

  return <span className={STATUS_CHIP_CLASS_NAME}>{semantics.statusLabel}</span>;
}

export function ChangeChip({ change }: { change: CollectionTreeNodeChange | undefined }) {
  const { t } = useTranslation('collections');

  if (!change || change === 'unchanged' || change === 'missing') {
    return null;
  }

  const presentation = {
    will_enable: {
      className: 'border-success/20 bg-success/10 text-success/85',
      label: t('apply.diff.will_enable'),
    },
    will_disable: {
      className: 'border-error/20 bg-error/10 text-error/85',
      label: t('apply.diff.will_disable'),
    },
    excluded_by_safe_mode: {
      className: 'border-warning/20 bg-warning/10 text-warning/85',
      label: t('apply.diff.excluded_safe_mode'),
    },
  }[change];

  return (
    <span
      className={`badge badge-xs h-4 border text-[9px] uppercase tracking-wide ${presentation.className}`}
    >
      {presentation.label}
    </span>
  );
}

export function WarningIcon({
  node,
}: {
  node: Pick<PreviewTreeNode, 'warnings' | 'inactive_reason'>;
}) {
  const { t } = useTranslation('collections');
  const semantics = buildCollectionPreviewNodeSemantics(t, {
    node_type: null,
    status_kind: null,
    show_inactive_chip: false,
    warnings: node.warnings,
    inactive_reason: node.inactive_reason,
  });
  if (!semantics.warningTitle) {
    return null;
  }

  return (
    <span
      className="shrink-0 text-warning/80"
      title={semantics.warningTitle}
      aria-label={semantics.warningTitle}
    >
      <AlertTriangle size={12} />
    </span>
  );
}

function iconForNode(node: PreviewTreeNode, expanded: boolean) {
  if (node.kind === 'mod') {
    return <Package size={11} className="shrink-0 text-base-content/45" />;
  }
  if (node.node_type === 'VariantContainer') {
    return <Layers size={12} className="shrink-0 text-base-content/45" />;
  }
  return expanded ? (
    <FolderOpen size={12} className="shrink-0 text-base-content/45" />
  ) : (
    <Folder size={12} className="shrink-0 text-base-content/40" />
  );
}

function isModRoot(node: PreviewTreeNode) {
  return (
    node.kind === 'mod' ||
    node.node_type === 'FlatModRoot' ||
    node.node_type === 'ModPackRoot' ||
    node.node_type === 'VariantContainer'
  );
}

export function NodeVisual({
  node,
  gameId,
  expanded,
}: {
  node: PreviewTreeNode;
  gameId: string;
  expanded: boolean;
}) {
  if (isModRoot(node) && node.path) {
    return <ModThumbnail gameId={gameId} folderPath={node.path} sizeClassName="size-8" />;
  }

  return iconForNode(node, expanded);
}
