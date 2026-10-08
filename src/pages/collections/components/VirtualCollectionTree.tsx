import type { PreviewTreeNode } from '@/entities/collection';
import type { CollectionTreeNodeChange } from './collectionTreeStructure';
import { useCallback, useMemo, useState } from 'react';
import { defaultRangeExtractor, useVirtualizer } from '@tanstack/react-virtual';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ChevronDown, ChevronRight } from 'lucide-react';
import {
  NodeTypeChip,
  StatusChip,
  ChangeChip,
  WarningIcon,
  NodeVisual,
} from './CollectionTreeNodeVisuals';
import { TreeLeaf } from './RecursiveCollectionTree';
import {
  shouldStartCollapsed,
  flattenTree,
  type FlatTreeRow,
  type CollectionTreeViewProps,
} from './collectionTreeStructure';
function VirtualTreeRow({
  row,
  gameId,
  colorClass,
  isExpanded,
  onToggle,
  onFocus,
  nodeChanges,
}: {
  row: FlatTreeRow;
  gameId: string;
  colorClass: string;
  isExpanded: boolean;
  onToggle: (node: PreviewTreeNode) => void;
  onFocus: (id: string) => void;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
}) {
  const { t } = useTranslation(['collections', 'common']);
  const { node } = row;
  const hasChildren = node.children.length > 0 && !node.collapse_children;

  if (row.kind === 'leaf') {
    return <TreeLeaf node={node} depth={row.depth} gameId={gameId} nodeChanges={nodeChanges} />;
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
        <ChangeChip change={nodeChanges?.get(node.id)} />
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

export function VirtualCollectionTree({
  nodes,
  gameId,
  colorClass,
  scrollElement,
  treeIdentity,
  nodeChanges,
}: Required<Pick<CollectionTreeViewProps, 'nodes' | 'colorClass'>> &
  Pick<CollectionTreeViewProps, 'gameId' | 'scrollElement' | 'treeIdentity' | 'nodeChanges'>) {
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
                nodeChanges={nodeChanges}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
