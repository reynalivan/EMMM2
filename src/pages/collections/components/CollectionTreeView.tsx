import { useCallback, useMemo, useState } from 'react';
import { defaultRangeExtractor, useVirtualizer } from '@tanstack/react-virtual';
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
import type { PreviewTreeNode } from '@/entities/collection';
import { ModThumbnail } from '@/entities/mod';
import { buildCollectionPreviewNodeSemantics } from '../collectionPreviewSemantics';

interface CollectionTreeViewProps {
  nodes?: PreviewTreeNode[];
  gameId?: string | null;
  colorClass?: string;
  emptyMessage?: string;
  scrollElement?: HTMLDivElement | null;
  treeIdentity?: string;
}

const TYPE_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-base-content/10 bg-base-200/70 text-[9px] uppercase tracking-wide text-base-content/55';
const STATUS_CHIP_CLASS_NAME =
  'badge badge-xs h-4 border border-warning/20 bg-warning/10 text-[9px] uppercase tracking-wide text-warning/80';
const SECTION_NODE_TYPE = 'InactiveContainerSection';
const LAZY_CHILDREN_THRESHOLD = 80;
const VIRTUAL_TREE_THRESHOLD = 80;

function shouldStartCollapsed(node: PreviewTreeNode): boolean {
  return Math.max(node.mod_count ?? 0, node.children.length) >= LAZY_CHILDREN_THRESHOLD;
}

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

function isModRoot(node: PreviewTreeNode) {
  return (
    node.kind === 'mod' ||
    node.node_type === 'FlatModRoot' ||
    node.node_type === 'ModPackRoot' ||
    node.node_type === 'VariantContainer'
  );
}

function NodeVisual({
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

function TreeLeaf({
  node,
  depth,
  gameId,
}: {
  node: PreviewTreeNode;
  depth: number;
  gameId: string;
}) {
  const hasActiveModDetail =
    node.kind === 'mod' && node.is_effectively_active && Boolean(node.path);

  return (
    <div
      className={`group relative flex items-center gap-2 rounded-lg border border-transparent py-1.5 pr-3 text-xs transition-[background-color,border-color,color] duration-150 ${
        node.is_effectively_active
          ? 'opacity-95 hover:border-base-content/8 hover:bg-base-content/[0.03]'
          : 'opacity-55 hover:bg-base-content/[0.02]'
      }`}
      style={{ paddingLeft: `${depth * 1.1 + 1.15}rem` }}
      title={node.path ?? node.name}
    >
      <span className="font-mono text-[10px] text-base-content/18">└</span>
      <NodeVisual node={node} gameId={gameId} expanded={false} />
      <span className="min-w-0 flex-1">
        <span className="block truncate font-medium text-base-content/80">{node.name}</span>
        {hasActiveModDetail && (
          <span
            className="block truncate font-mono text-[10px] text-base-content/55"
            title={node.path ?? undefined}
          >
            {node.path}
          </span>
        )}
      </span>
      <NodeTypeChip nodeType={node.node_type} />
      <StatusChip node={node} />
      <WarningIcon node={node} />
    </div>
  );
}

function TreeFolder({
  node,
  depth,
  gameId,
}: {
  node: PreviewTreeNode;
  depth: number;
  gameId: string;
}) {
  const hasChildren = node.children.length > 0 && !node.collapse_children;
  const [collapsed, setCollapsed] = useState(() => shouldStartCollapsed(node));

  return (
    <div className="mb-1">
      <button
        type="button"
        onClick={() => {
          if (hasChildren) {
            setCollapsed((value) => !value);
          }
        }}
        aria-expanded={hasChildren ? !collapsed : undefined}
        className={`group flex w-full items-center gap-2 rounded-lg border border-transparent py-1.5 pr-3 text-left transition-[background-color,border-color,color] duration-150 ${
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
        <NodeVisual node={node} gameId={gameId} expanded={!collapsed} />
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
              <TreeLeaf key={child.id} node={child} depth={depth + 1} gameId={gameId} />
            ) : (
              <TreeFolder key={child.id} node={child} depth={depth + 1} gameId={gameId} />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function InactiveSection({ node, gameId }: { node: PreviewTreeNode; gameId: string }) {
  const { t } = useTranslation('collections');
  const hasChildren = node.children.length > 0;
  const [collapsed, setCollapsed] = useState(() => shouldStartCollapsed(node));

  return (
    <div className="mt-3 rounded-xl border border-warning/15 bg-warning/[0.045]">
      <button
        type="button"
        className="flex w-full items-center gap-2 border-b border-warning/10 px-3 py-2 text-left"
        onClick={() => {
          if (hasChildren) {
            setCollapsed((value) => !value);
          }
        }}
        aria-expanded={hasChildren ? !collapsed : undefined}
      >
        <span className="shrink-0 text-warning/70">
          {hasChildren ? (
            collapsed ? (
              <ChevronRight size={13} />
            ) : (
              <ChevronDown size={13} />
            )
          ) : (
            <span className="block w-[13px]" />
          )}
        </span>
        <AlertTriangle size={13} className="text-warning/75" />
        <div className="min-w-0 flex-1">
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-warning/80">
            {t('tree.inactive_section')}
          </p>
          <p className="text-[10px] text-base-content/50">{t('tree.inactive_section_desc')}</p>
        </div>
      </button>
      {!collapsed && (
        <div className="p-2">
          {node.children.map((child) =>
            child.kind === 'mod' ? (
              <TreeLeaf key={child.id} node={child} depth={0} gameId={gameId} />
            ) : (
              <TreeFolder key={child.id} node={child} depth={0} gameId={gameId} />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function ObjectRow({
  node,
  colorClass,
  gameId,
}: {
  node: PreviewTreeNode;
  colorClass: string;
  gameId: string;
}) {
  const { t } = useTranslation(['collections', 'common']);
  const inactiveSection = node.children.find((child) => child.node_type === SECTION_NODE_TYPE);
  const activeChildren = node.children.filter((child) => child.node_type !== SECTION_NODE_TYPE);
  const [collapsed, setCollapsed] = useState(() => shouldStartCollapsed(node));

  return (
    <div className="mb-4 last:mb-0">
      <button
        type="button"
        onClick={() => setCollapsed((value) => !value)}
        aria-expanded={!collapsed}
        className="group flex w-full items-center gap-2 rounded-xl border border-base-content/8 bg-base-300/[0.18] px-3 py-2.5 text-left transition-[background-color,border-color] duration-150 hover:border-base-content/12 hover:bg-base-300/[0.28]"
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
                  <TreeLeaf key={child.id} node={child} depth={0} gameId={gameId} />
                ) : (
                  <TreeFolder key={child.id} node={child} depth={0} gameId={gameId} />
                ),
              )}
            </div>
          ) : !inactiveSection ? (
            <div className="py-3 pl-3 text-[10px] italic text-base-content/25">
              {t('common:status.no_subfolders')}
            </div>
          ) : null}
          {inactiveSection ? <InactiveSection node={inactiveSection} gameId={gameId} /> : null}
        </div>
      )}
    </div>
  );
}

type FlatTreeRowKind = 'object' | 'folder' | 'leaf' | 'inactive';

interface FlatTreeRow {
  key: string;
  kind: FlatTreeRowKind;
  node: PreviewTreeNode;
  depth: number;
}

function countTreeNodes(nodes: PreviewTreeNode[]): number {
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

function flattenTree(
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

function VirtualTreeRow({
  row,
  gameId,
  colorClass,
  isExpanded,
  onToggle,
  onFocus,
}: {
  row: FlatTreeRow;
  gameId: string;
  colorClass: string;
  isExpanded: boolean;
  onToggle: (node: PreviewTreeNode) => void;
  onFocus: (id: string) => void;
}) {
  const { t } = useTranslation(['collections', 'common']);
  const { node } = row;
  const hasChildren = node.children.length > 0 && !node.collapse_children;

  if (row.kind === 'leaf') {
    return <TreeLeaf node={node} depth={row.depth} gameId={gameId} />;
  }

  if (row.kind === 'inactive') {
    return (
      <div className="mt-2 rounded-xl border border-warning/15 bg-warning/[0.045]">
        <button
          type="button"
          className="flex w-full items-center gap-2 px-3 py-2 text-left"
          onClick={() => hasChildren && onToggle(node)}
          onFocus={() => onFocus(row.key)}
          aria-expanded={hasChildren ? isExpanded : undefined}
        >
          <span className="shrink-0 text-warning/70">
            {hasChildren ? (
              isExpanded ? (
                <ChevronDown size={13} />
              ) : (
                <ChevronRight size={13} />
              )
            ) : (
              <span className="block w-[13px]" />
            )}
          </span>
          <AlertTriangle size={13} className="text-warning/75" />
          <div className="min-w-0 flex-1">
            <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-warning/80">
              {t('collections:tree.inactive_section')}
            </p>
            <p className="text-[10px] text-base-content/50">
              {t('collections:tree.inactive_section_desc')}
            </p>
          </div>
        </button>
      </div>
    );
  }

  if (row.kind === 'object') {
    return (
      <button
        type="button"
        onClick={() => onToggle(node)}
        onFocus={() => onFocus(row.key)}
        aria-expanded={isExpanded}
        className="group flex w-full items-center gap-2 rounded-xl border border-base-content/8 bg-base-300/[0.18] px-3 py-2.5 text-left transition-[background-color,border-color] duration-150 hover:border-base-content/12 hover:bg-base-300/[0.28]"
      >
        <span className="shrink-0 text-base-content/40">
          {isExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        </span>
        <span
          className={`min-w-0 flex-1 truncate text-xs font-bold uppercase tracking-wider ${
            node.is_enabled ? 'text-base-content/92' : 'text-base-content/40'
          }`}
        >
          {node.id === '__uncategorized__' ? t('collections:tree.uncategorized') : node.name}
        </span>
        {!node.is_enabled && (
          <span className="badge badge-xs badge-neutral h-4 text-[9px] opacity-60">
            {t('collections:tree.object_off')}
          </span>
        )}
        <span className={`shrink-0 text-[10px] font-mono font-bold opacity-85 ${colorClass}`}>
          {t('collections:list.item.mod_count', { count: node.mod_count ?? 0 })}
        </span>
      </button>
    );
  }

  return (
    <button
      type="button"
      onClick={() => hasChildren && onToggle(node)}
      onFocus={() => onFocus(row.key)}
      aria-expanded={hasChildren ? isExpanded : undefined}
      className={`group flex w-full items-center gap-2 rounded-lg border border-transparent py-1.5 pr-3 text-left transition-[background-color,border-color,color] duration-150 ${
        hasChildren ? 'hover:border-base-content/8 hover:bg-base-content/[0.03]' : ''
      } ${node.is_effectively_active ? '' : 'opacity-65'}`}
      style={{ paddingLeft: `${row.depth * 1.1 + 0.45}rem` }}
      title={node.path ?? node.name}
    >
      <span className="shrink-0 text-base-content/30">
        {hasChildren ? (
          isExpanded ? (
            <ChevronDown size={12} />
          ) : (
            <ChevronRight size={12} />
          )
        ) : (
          <span className="block w-3" />
        )}
      </span>
      <NodeVisual node={node} gameId={gameId} expanded={isExpanded} />
      <span className="min-w-0 flex-1 truncate text-xs font-semibold text-base-content/78">
        {node.name}
      </span>
      <NodeTypeChip nodeType={node.node_type} />
      <StatusChip node={node} />
      <WarningIcon node={node} />
    </button>
  );
}

function VirtualCollectionTree({
  nodes,
  gameId,
  colorClass,
  scrollElement,
  treeIdentity,
}: Required<Pick<CollectionTreeViewProps, 'nodes' | 'colorClass'>> &
  Pick<CollectionTreeViewProps, 'gameId' | 'scrollElement' | 'treeIdentity'>) {
  const [fallbackScrollElement, setFallbackScrollElement] = useState<HTMLDivElement | null>(null);
  const [expansionState, setExpansionState] = useState(() => ({
    treeIdentity,
    values: new Map<string, boolean>(),
  }));
  const [focusedRowKey, setFocusedRowKey] = useState<string | null>(null);
  const activeExpansion = useMemo(
    () =>
      expansionState.treeIdentity === treeIdentity
        ? expansionState.values
        : new Map<string, boolean>(),
    [expansionState, treeIdentity],
  );
  const isExpanded = useCallback(
    (node: PreviewTreeNode) => activeExpansion.get(node.id) ?? !shouldStartCollapsed(node),
    [activeExpansion],
  );
  const rows = useMemo(() => flattenTree(nodes, isExpanded), [isExpanded, nodes]);
  const focusedIndex = rows.findIndex((row) => row.key === focusedRowKey);
  const getRowKey = useCallback((index: number) => rows[index]!.key, [rows]);
  const rangeExtractor = useCallback(
    (range: Parameters<typeof defaultRangeExtractor>[0]) => {
      const indices = defaultRangeExtractor(range);
      if (focusedIndex >= 0 && !indices.includes(focusedIndex)) {
        indices.push(focusedIndex);
        indices.sort((left, right) => left - right);
      }
      return indices;
    },
    [focusedIndex],
  );
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollElement ?? fallbackScrollElement,
    estimateSize: (index) => (rows[index]?.kind === 'inactive' ? 58 : 38),
    getItemKey: getRowKey,
    measureElement: (element) => element.getBoundingClientRect().height,
    overscan: 5,
    rangeExtractor,
    initialRect: { width: 0, height: 1_000 },
  });
  const virtualItems = virtualizer.getVirtualItems();
  const renderedRows =
    virtualItems.length > 0
      ? virtualItems.map((virtualItem) => ({
          key: virtualItem.key,
          index: virtualItem.index,
          start: virtualItem.start,
        }))
      : rows.slice(0, 20).map((row, index) => ({
          key: row.key,
          index,
          start: index * 38,
        }));
  const totalSize =
    virtualizer.getTotalSize() ||
    rows.reduce((size, row) => size + (row.kind === 'inactive' ? 58 : 38), 0);
  const toggle = useCallback(
    (node: PreviewTreeNode) => {
      setExpansionState((current) => {
        const values = current.treeIdentity === treeIdentity ? new Map(current.values) : new Map();
        values.set(node.id, !(values.get(node.id) ?? !shouldStartCollapsed(node)));
        return { treeIdentity, values };
      });
    },
    [treeIdentity],
  );

  return (
    <div
      ref={scrollElement ? undefined : setFallbackScrollElement}
      className={scrollElement ? 'relative w-full' : 'max-h-[40rem] overflow-y-auto'}
      role="tree"
    >
      <div className="relative w-full" style={{ height: `${totalSize}px` }}>
        {renderedRows.map((virtualItem) => {
          const row = rows[virtualItem.index];
          if (!row) return null;

          return (
            <div
              key={virtualItem.key}
              ref={virtualizer.measureElement}
              data-index={virtualItem.index}
              className="absolute left-0 top-0 w-full pb-1"
              style={{ transform: `translateY(${virtualItem.start}px)` }}
            >
              <VirtualTreeRow
                row={row}
                gameId={gameId ?? ''}
                colorClass={colorClass}
                isExpanded={isExpanded(row.node)}
                onToggle={toggle}
                onFocus={setFocusedRowKey}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function CollectionTreeView({
  nodes,
  gameId,
  colorClass = 'text-primary',
  emptyMessage,
  scrollElement,
  treeIdentity,
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

  if (countTreeNodes(tree) > VIRTUAL_TREE_THRESHOLD) {
    return (
      <VirtualCollectionTree
        nodes={tree}
        gameId={gameId}
        colorClass={colorClass}
        scrollElement={scrollElement}
        treeIdentity={treeIdentity}
      />
    );
  }

  return (
    <div className="space-y-1">
      {tree.map((objectNode) => (
        <ObjectRow
          key={objectNode.id}
          node={objectNode}
          colorClass={colorClass}
          gameId={gameId ?? ''}
        />
      ))}
    </div>
  );
}
