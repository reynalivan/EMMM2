import type { PreviewTreeNode } from '@/entities/collection';
import type { CollectionTreeNodeChange } from './collectionTreeStructure';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ChevronDown, ChevronRight } from 'lucide-react';
import {
  NodeTypeChip,
  StatusChip,
  ChangeChip,
  WarningIcon,
  NodeVisual,
} from './CollectionTreeNodeVisuals';
import { shouldStartCollapsed, SECTION_NODE_TYPE } from './collectionTreeStructure';
export function TreeLeaf({
  node,
  depth,
  gameId,
  nodeChanges,
}: {
  node: PreviewTreeNode;
  depth: number;
  gameId: string;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
}) {
  const hasActiveModDetail =
    node.kind === 'mod' && node.is_effectively_active && Boolean(node.path);
  const change = nodeChanges?.get(node.id);

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
        <span
          className={`block truncate font-medium ${
            change === 'will_disable'
              ? 'text-error/80 line-through'
              : change === 'will_enable'
                ? 'text-success/85'
                : change === 'excluded_by_safe_mode'
                  ? 'text-warning/85'
                  : 'text-base-content/80'
          }`}
        >
          {node.name}
        </span>
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
      <ChangeChip change={change} />
      <WarningIcon node={node} />
    </div>
  );
}

function TreeFolder({
  node,
  depth,
  gameId,
  nodeChanges,
}: {
  node: PreviewTreeNode;
  depth: number;
  gameId: string;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
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
              <TreeLeaf
                key={child.id}
                node={child}
                depth={depth + 1}
                gameId={gameId}
                nodeChanges={nodeChanges}
              />
            ) : (
              <TreeFolder
                key={child.id}
                node={child}
                depth={depth + 1}
                gameId={gameId}
                nodeChanges={nodeChanges}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function InactiveSection({
  node,
  gameId,
  nodeChanges,
}: {
  node: PreviewTreeNode;
  gameId: string;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
}) {
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
              <TreeLeaf
                key={child.id}
                node={child}
                depth={0}
                gameId={gameId}
                nodeChanges={nodeChanges}
              />
            ) : (
              <TreeFolder
                key={child.id}
                node={child}
                depth={0}
                gameId={gameId}
                nodeChanges={nodeChanges}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

export function ObjectRow({
  node,
  colorClass,
  gameId,
  nodeChanges,
}: {
  node: PreviewTreeNode;
  colorClass: string;
  gameId: string;
  nodeChanges?: ReadonlyMap<string, CollectionTreeNodeChange>;
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
        <ChangeChip change={nodeChanges?.get(node.id)} />
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
                  <TreeLeaf
                    key={child.id}
                    node={child}
                    depth={0}
                    gameId={gameId}
                    nodeChanges={nodeChanges}
                  />
                ) : (
                  <TreeFolder
                    key={child.id}
                    node={child}
                    depth={0}
                    gameId={gameId}
                    nodeChanges={nodeChanges}
                  />
                ),
              )}
            </div>
          ) : !inactiveSection ? (
            <div className="py-3 pl-3 text-[10px] italic text-base-content/25">
              {t('common:status.no_subfolders')}
            </div>
          ) : null}
          {inactiveSection ? (
            <InactiveSection node={inactiveSection} gameId={gameId} nodeChanges={nodeChanges} />
          ) : null}
        </div>
      )}
    </div>
  );
}
