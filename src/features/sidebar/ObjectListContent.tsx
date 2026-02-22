/**
 * ObjectListContent — virtualized list rendering for Object mode.
 * Includes sticky selected-item indicator with solid bg. Extracted from ObjectList (350-line limit).
 */

import { type RefObject } from 'react';
import { type Virtualizer } from '@tanstack/react-virtual';
import { ChevronUp, ChevronDown } from 'lucide-react';
import ObjectRowItem from './ObjectRowItem';
import CategorySection from './CategorySection';
import { ObjectContextMenu, type ContextMenuTarget } from './ObjectContextMenu';
import { ContextMenu } from '../../components/ui/ContextMenu';
import type { FlatItem } from './useObjectListVirtualizer';

interface ContentProps {
  parentRef: RefObject<HTMLDivElement | null>;
  rowVirtualizer: Virtualizer<HTMLDivElement, Element>;
  flatObjectItems: FlatItem[];
  selectedObject: string | null;
  selectedObjectType: string | null;
  setSelectedObject: (id: string | null) => void;
  setSelectedObjectType: (type: string | null) => void;
  isMobile: boolean;
  /* Sticky */
  stickyPosition: 'top' | 'bottom' | null;
  selectedIndex: number;
  scrollToSelected: () => void;
  /* Context menu deps */
  contextMenuProps: ContextMenuHandlerProps;
  /* DnD per-item highlight */
  isDragging?: boolean;
  hoveredItemId?: string | null;
}

/** Shared props for building ObjectContextMenu */
export interface ContextMenuHandlerProps {
  isSyncing: boolean;
  categoryNames: { name: string; label?: string }[];
  handleEdit: (id: string) => void;
  handleSyncWithDb: (id: string, name: string) => void;
  handleDelete: (path: string) => void;
  handleDeleteObject: (id: string) => void;
  handleToggle: (path: string, currentEnabled: boolean) => void;
  handleOpen: (path: string) => void;
  handlePin: (id: string) => void;
  handleFavorite: (path: string) => void;
  handleMoveCategory: (id: string, category: string, type: 'object' | 'folder') => void;
  handleRevealInExplorer?: (id: string) => void;
  handleEnableObject?: (id: string) => void;
  handleDisableObject?: (id: string) => void;
}

/** Build the context menu content for an item */
function renderContextMenu(item: ContextMenuTarget, ctx: ContextMenuHandlerProps) {
  return (
    <ObjectContextMenu
      item={item}
      isSyncing={ctx.isSyncing}
      categories={ctx.categoryNames}
      onEditObject={ctx.handleEdit}
      onEditFolder={() => {}}
      onSyncWithDb={ctx.handleSyncWithDb}
      onDelete={ctx.handleDelete}
      onDeleteObject={ctx.handleDeleteObject}
      onToggle={ctx.handleToggle}
      onOpen={ctx.handleOpen}
      onPin={ctx.handlePin}
      onFavorite={ctx.handleFavorite}
      onMoveCategory={ctx.handleMoveCategory}
      onRevealInExplorer={ctx.handleRevealInExplorer}
      onEnableObject={ctx.handleEnableObject}
      onDisableObject={ctx.handleDisableObject}
    />
  );
}

export default function ObjectListContent({
  parentRef,
  rowVirtualizer,
  flatObjectItems,
  selectedObject,
  selectedObjectType,
  setSelectedObject,
  setSelectedObjectType,
  isMobile,
  stickyPosition,
  selectedIndex,
  scrollToSelected,
  contextMenuProps,
  isDragging,
  hoveredItemId,
}: ContentProps) {
  const ctx = contextMenuProps;

  return (
    <div className="flex-1 relative min-h-0">
      {/* Scroll container */}
      <div
        ref={parentRef}
        className="absolute inset-0 overflow-y-auto custom-scrollbar px-2 py-1.5"
      >
        <div className="relative w-full" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
          {rowVirtualizer.getVirtualItems().map((virtualItem) => {
            const item = flatObjectItems[virtualItem.index];
            if (!item) return null;

            if (item.type === 'header') {
              return (
                <div
                  key={`cat-${item.category.name}`}
                  className="absolute top-0 left-0 w-full"
                  style={{
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <CategorySection
                    category={item.category}
                    count={item.count}
                    isSelected={selectedObjectType === item.category.name}
                    onSelect={() =>
                      setSelectedObjectType(
                        selectedObjectType === item.category.name ? null : item.category.name,
                      )
                    }
                  />
                </div>
              );
            }

            if (item.type === 'sub-header') {
              return (
                <div
                  key={`sub-${item.parentCategory}-${item.label}`}
                  className="absolute top-0 left-0 w-full pl-4"
                  style={{
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <div className="flex items-center gap-2 px-2 py-1 text-base-content/40">
                    <span className="text-[10px] font-medium uppercase tracking-wider">
                      {item.label}
                    </span>
                    <span className="badge badge-xs badge-ghost">{item.count}</span>
                  </div>
                </div>
              );
            }

            return (
              <div
                key={item.obj.id}
                className="absolute top-0 left-0 w-full"
                style={{
                  height: `${virtualItem.size}px`,
                  transform: `translateY(${virtualItem.start}px)`,
                }}
              >
                <ContextMenu
                  content={renderContextMenu(
                    {
                      type: 'object',
                      id: item.obj.id,
                      name: item.obj.name,
                      objectType: item.obj.object_type,
                      enabledCount: item.obj.enabled_count,
                      modCount: item.obj.mod_count,
                      isPinned: item.obj.is_pinned,
                    },
                    ctx,
                  )}
                >
                  <ObjectRowItem
                    obj={item.obj}
                    isSelected={selectedObject === item.obj.id}
                    isMobile={isMobile}
                    onClick={() => setSelectedObject(item.obj.id)}
                    isDropTarget={isDragging && hoveredItemId === item.obj.id}
                  />
                </ContextMenu>
              </div>
            );
          })}
        </div>
      </div>

      {/* Sticky selected-item indicator */}
      <StickyRow
        stickyPosition={stickyPosition}
        selectedIndex={selectedIndex}
        scrollToSelected={scrollToSelected}
        flatObjectItems={flatObjectItems}
        isMobile={isMobile}
        contextMenuProps={ctx}
      />
    </div>
  );
}

/* ── Sticky Row — solid bg, border, shadow, direction indicator ── */

interface StickyRowProps {
  stickyPosition: 'top' | 'bottom' | null;
  selectedIndex: number;
  scrollToSelected: () => void;
  flatObjectItems: FlatItem[];
  isMobile: boolean;
  contextMenuProps: ContextMenuHandlerProps;
}

function StickyRow({
  stickyPosition,
  selectedIndex,
  scrollToSelected,
  flatObjectItems,
  isMobile,
  contextMenuProps: ctx,
}: StickyRowProps) {
  if (!stickyPosition || selectedIndex < 0) return null;

  const isTop = stickyPosition === 'top';
  const DirectionIcon = isTop ? ChevronUp : ChevronDown;

  const item = flatObjectItems[selectedIndex];
  if (!item || item.type !== 'row') return null;

  return (
    <div
      className={`absolute left-0 right-0 z-10 px-2 cursor-pointer ${
        isTop
          ? '-top-1 pt-1 border-b border-base-300/30'
          : '-bottom-1 pb-1 border-t border-base-300/30 shadow-[0_-4px_12px_rgba(0,0,0,0.3)]'
      }`}
      onClick={scrollToSelected}
    >
      <div className="bg-base-100/95 backdrop-blur-md rounded-lg relative">
        <ContextMenu
          content={renderContextMenu(
            {
              type: 'object',
              id: item.obj.id,
              name: item.obj.name,
              objectType: item.obj.object_type,
              enabledCount: item.obj.enabled_count,
              modCount: item.obj.mod_count,
              isPinned: item.obj.is_pinned,
            },
            ctx,
          )}
        >
          <ObjectRowItem obj={item.obj} isSelected isMobile={isMobile} onClick={scrollToSelected} />
        </ContextMenu>
        {/* Direction indicator */}
        <div className="absolute right-3 top-1/2 -translate-y-1/2">
          <DirectionIcon size={14} className="text-primary/60 animate-bounce" />
        </div>
      </div>
    </div>
  );
}
