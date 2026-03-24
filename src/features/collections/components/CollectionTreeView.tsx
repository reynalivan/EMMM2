/**
 * CollectionTreeView — Hierarchical tree: Object > [Dir...] > Mod.
 *
 * Used in:
 *  - CollectionPreviewPanel (sidebar)
 *  - ApplyCollectionModal (before/after panels)
 *  - ModeSwitchConfirmModal (corridor preview)
 *
 * Renders all nodes expanded by default.
 */

import { useState } from 'react';
import { ChevronDown, ChevronRight, FolderOpen, Folder, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { CollectionMember } from '../../../types/collection';
import { buildModTree } from '../utils/buildModTree';
import type { ObjectTreeNode, DirTreeNode, ModTreeNode } from '../utils/buildModTree';

interface CollectionTreeViewProps {
  members: CollectionMember[];
  /** Accent color class applied to numeric badges. E.g. "text-primary", "text-success/70". */
  colorClass?: string;
  emptyMessage?: string;
}

// ── Leaf: Mod ────────────────────────────────────────────────────────────────

function ModLeaf({ mod, depth = 0 }: { mod: ModTreeNode; depth?: number }) {
  const dimmed = mod.effectively_disabled;
  return (
    <div
      className={`flex items-center gap-2 pr-3 py-1 text-xs rounded transition-opacity ${dimmed ? 'opacity-35' : 'opacity-90 hover:opacity-100'}`}
      style={{ paddingLeft: `${depth * 0.75 + 1.5}rem` }}
      title={mod.mod_path}
    >
      <span className="text-base-content/20 shrink-0 font-mono text-[10px]">└</span>
      <Package
        size={11}
        className={`shrink-0 ${mod.is_enabled ? 'text-base-content/50' : 'text-base-content/25'}`}
      />
      <span className="truncate flex-1 font-medium">{mod.name}</span>
    </div>
  );
}

// ── Branch: Recursive Folder ──────────────────────────────────────────────────

function FolderRow({ node, depth = 0 }: { node: DirTreeNode; depth?: number }) {
  const [collapsed, setCollapsed] = useState(false);
  const hasChildren = node.children.length > 0;

  return (
    <div className="mb-0.5">
      <button
        onClick={() => setCollapsed((v) => !v)}
        className="w-full flex items-center gap-2 py-1 pr-3 text-left hover:bg-base-content/5 transition-colors rounded"
        style={{ paddingLeft: `${depth * 0.75 + 0.5}rem` }}
      >
        <span className="text-base-content/30 shrink-0">
          {hasChildren ? (
            collapsed ? (
              <ChevronRight size={12} />
            ) : (
              <ChevronDown size={12} />
            )
          ) : (
            <span className="w-3" />
          )}
        </span>
        {collapsed ? (
          <Folder size={13} className="text-base-content/40 shrink-0" />
        ) : (
          <FolderOpen size={13} className="text-primary/50 shrink-0" />
        )}
        <span className="text-xs font-semibold truncate flex-1 text-base-content/70">{node.name}</span>
        
        {/* Optional: Show count if folder has many items */}
        {hasChildren && (
          <span className="text-[9px] text-base-content/25 shrink-0 font-mono">
            {node.children.length}
          </span>
        )}
      </button>

      {!collapsed && hasChildren && (
        <div>
          {node.children.map((child) => (
            child.kind === 'dir' ? (
              <FolderRow key={child.id} node={child} depth={depth + 1} />
            ) : (
              <ModLeaf key={child.id} mod={child} depth={depth + 1} />
            )
          ))}
        </div>
      )}
    </div>
  );
}

// ── Root: Object ──────────────────────────────────────────────────────────────

function ObjectRow({ node, colorClass }: { node: ObjectTreeNode; colorClass: string }) {
  const { t } = useTranslation(['collections', 'common']);
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="mb-4 last:mb-0">
      <button
        onClick={() => setCollapsed((v) => !v)}
        className="w-full flex items-center gap-2 pl-2 pr-3 py-2 text-left rounded-lg bg-base-300/20 hover:bg-base-300/40 transition-colors group"
      >
        <span className="text-base-content/40 shrink-0">
          {collapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
        </span>
        <span
          className={`font-bold text-xs uppercase tracking-wider truncate flex-1 ${
            node.is_enabled ? 'text-base-content/90' : 'text-base-content/40'
          }`}
        >
          {node.id === '__uncategorized__'
            ? t('tree.uncategorized', 'Uncategorized')
            : node.name}
        </span>
        
        {!node.is_enabled && (
          <span className="badge badge-xs badge-neutral opacity-60 shrink-0 text-[9px] h-4">
            {t('tree.object_off', 'Off')}
          </span>
        )}
        
        <span className={`text-[10px] font-mono shrink-0 font-bold opacity-80 ${colorClass}`}>
          {t('list.item.mod_count', { count: node.modCount })}
        </span>
      </button>

      {!collapsed && (
        <div className="mt-1 space-y-0.5">
          {node.children.length > 0 ? (
            node.children.map((child) =>
              child.kind === 'dir' ? (
                <FolderRow key={child.id} node={child} depth={0} />
              ) : (
                <ModLeaf key={child.id} mod={child} depth={0} />
              ),
            )
          ) : (
            <div className="pl-6 py-2 text-[10px] text-base-content/25 italic">
              {t('common:status.no_subfolders', 'No subfolders found')}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Export ────────────────────────────────────────────────────────────────────

export function CollectionTreeView({
  members,
  colorClass = 'text-primary',
  emptyMessage,
}: CollectionTreeViewProps) {
  const { t } = useTranslation('collections');
  const tree = buildModTree(members);

  if (tree.length === 0) {
    return (
      <div className="text-center p-6 text-sm text-base-content/40 border border-base-content/10 border-dashed rounded-xl bg-base-200/50">
        {emptyMessage ?? t('preview.empty')}
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {tree.map((obj) => (
        <ObjectRow key={obj.id} node={obj} colorClass={colorClass} />
      ))}
    </div>
  );
}
