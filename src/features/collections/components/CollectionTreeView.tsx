import { useState } from 'react';
import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  Layers,
  Package,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { PreviewTreeNode } from '../../../types/collection';
import { buildCollectionPreviewNodeSemantics } from '../collectionPreviewSemantics';

interface CollectionTreeViewProps {
  nodes?: PreviewTreeNode[];
  colorClass?: string;
  emptyMessage?: string;
}

const TYPE_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-base-content/10 bg-base-200/70 text-[9px] uppercase tracking-wide text-base-content/55';
const STATUS_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-warning/20 bg-warning/10 text-[9px] uppercase tracking-wide text-warning/80';
const SECTION_NODE_TYPE = 'InactiveContainerSection';

function NodeTypeChip({ nodeType }: { nodeType: string | null }) {
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

function StatusChip({
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

function WarningIcon({ node }: { node: Pick<PreviewTreeNode, 'warnings' | 'inactive_reason'> }) {
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

function TreeLeaf({ node, depth }: { node: PreviewTreeNode; depth: number }) {
  return (
    <div
      className={`group flex items-center gap-2 rounded-lg border border-transparent py-1.5 pr-3 text-xs transition-all ${
        node.is_effectively_active
          ? 'opacity-95 hover:border-base-content/8 hover:bg-base-content/[0.03]'
          : 'opacity-55 hover:bg-base-content/[0.02]'
      }`}
      style={{ paddingLeft: `${depth * 1.1 + 1.15}rem` }}
      title={node.path ?? node.name}
    >
      <span className="font-mono text-[10px] text-base-content/18">└</span>
      {iconForNode(node, false)}
      <span className="min-w-0 flex-1 truncate font-medium text-base-content/80">{node.name}</span>
      <NodeTypeChip nodeType={node.node_type} />
      <StatusChip node={node} />
      <WarningIcon node={node} />
    </div>
  );
}

function TreeFolder({ node, depth }: { node: PreviewTreeNode; depth: number }) {
  const hasChildren = node.children.length > 0 && !node.collapse_children;
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="mb-1">
      <button
        type="button"
        onClick={() => {
          if (hasChildren) {
            setCollapsed((value) => !value);
          }
        }}
        className={`group flex w-full items-center gap-2 rounded-lg border border-transparent py-1.5 pr-3 text-left transition-all ${
          hasChildren ? 'hover:border-base-content/8 hover:bg-base-content/[0.03]' : ''
        } ${node.is_effectively_active ? '' : 'opacity-65'}`}
        style={{ paddingLeft: `${depth * 1.1 + 0.45}rem` }}
        title={node.path ?? node.name}
      >
        <span className="shrink-0 text-base-content/30">
          {hasChildren ? (
            collapsed ? (
              <ChevronRight size={12} />
            ) : (
              <ChevronDown size={12} />
            )
          ) : (
            <span className="block w-3" />
          )}
        </span>
        {iconForNode(node, !collapsed)}
        <span className="min-w-0 flex-1 truncate text-xs font-semibold text-base-content/78">
          {node.name}
        </span>
        <NodeTypeChip nodeType={node.node_type} />
        <StatusChip node={node} />
        <WarningIcon node={node} />
      </button>

      {hasChildren && !collapsed && (
        <div className="relative ml-3 border-l border-base-content/8 pl-1.5">
          {node.children.map((child) =>
            child.kind === 'mod' ? (
              <TreeLeaf key={child.id} node={child} depth={depth + 1} />
            ) : (
              <TreeFolder key={child.id} node={child} depth={depth + 1} />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function InactiveSection({ node }: { node: PreviewTreeNode }) {
  const { t } = useTranslation('collections');

  return (
    <div className="mt-3 rounded-xl border border-warning/15 bg-warning/[0.045]">
      <div className="flex items-center gap-2 border-b border-warning/10 px-3 py-2">
        <AlertTriangle size={13} className="text-warning/75" />
        <div className="min-w-0 flex-1">
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-warning/80">
            {t('tree.inactive_section')}
          </p>
          <p className="text-[10px] text-base-content/50">{t('tree.inactive_section_desc')}</p>
        </div>
      </div>
      <div className="p-2">
        {node.children.map((child) =>
          child.kind === 'mod' ? (
            <TreeLeaf key={child.id} node={child} depth={0} />
          ) : (
            <TreeFolder key={child.id} node={child} depth={0} />
          ),
        )}
      </div>
    </div>
  );
}

function ObjectRow({ node, colorClass }: { node: PreviewTreeNode; colorClass: string }) {
  const { t } = useTranslation(['collections', 'common']);
  const [collapsed, setCollapsed] = useState(false);
  const inactiveSection = node.children.find((child) => child.node_type === SECTION_NODE_TYPE);
  const activeChildren = node.children.filter((child) => child.node_type !== SECTION_NODE_TYPE);

  return (
    <div className="mb-4 last:mb-0">
      <button
        type="button"
        onClick={() => setCollapsed((value) => !value)}
        className="group flex w-full items-center gap-2 rounded-xl border border-base-content/8 bg-base-300/[0.18] px-3 py-2.5 text-left transition-all hover:border-base-content/12 hover:bg-base-300/[0.28]"
      >
        <span className="shrink-0 text-base-content/40">
          {collapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
        </span>
        <span
          className={`min-w-0 flex-1 truncate text-xs font-bold uppercase tracking-wider ${
            node.is_enabled ? 'text-base-content/92' : 'text-base-content/40'
          }`}
        >
          {node.id === '__uncategorized__' ? t('tree.uncategorized') : node.name}
        </span>
        {!node.is_enabled && (
          <span className="badge badge-xs badge-neutral h-4 text-[9px] opacity-60">
            {t('tree.object_off')}
          </span>
        )}
        <span className={`shrink-0 text-[10px] font-mono font-bold opacity-85 ${colorClass}`}>
          {t('list.item.mod_count', { count: node.mod_count ?? 0 })}
        </span>
      </button>

      {!collapsed && (
        <div className="mt-2 rounded-2xl bg-base-200/[0.18] p-2">
          {activeChildren.length > 0 ? (
            <div className="space-y-0.5">
              {activeChildren.map((child) =>
                child.kind === 'mod' ? (
                  <TreeLeaf key={child.id} node={child} depth={0} />
                ) : (
                  <TreeFolder key={child.id} node={child} depth={0} />
                ),
              )}
            </div>
          ) : !inactiveSection ? (
            <div className="py-3 pl-3 text-[10px] italic text-base-content/25">
              {t('common:status.no_subfolders')}
            </div>
          ) : null}
          {inactiveSection ? <InactiveSection node={inactiveSection} /> : null}
        </div>
      )}
    </div>
  );
}

export function CollectionTreeView({
  nodes,
  colorClass = 'text-primary',
  emptyMessage,
}: CollectionTreeViewProps) {
  const { t } = useTranslation('collections');
  const tree = nodes ?? [];

  if (tree.length === 0) {
    return (
      <div className="rounded-xl border border-base-content/10 border-dashed bg-base-200/50 p-6 text-center text-sm text-base-content/40">
        {emptyMessage ?? t('preview.empty')}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {tree.map((objectNode) => (
        <ObjectRow key={objectNode.id} node={objectNode} colorClass={colorClass} />
      ))}
    </div>
  );
}
