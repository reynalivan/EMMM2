import { FolderInput, FolderPlus } from 'lucide-react';
import type { RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import type { DropZone } from '../hooks/useObjectListDropZones';
import type { WorkspaceObjectNode } from '@/entities/workspace';
import { LiquidSurface } from '@/shared/ui/liquid';

interface ObjectListDropIndicatorsProps {
  isDragging: boolean;
  activeDropZone: DropZone | null;
  hoveredItemId: string | null;
  tooltipTop: number;
  objects: WorkspaceObjectNode[];
  selectedObjectType: string | null;
  objectCount: number;
  onShowAll: () => void;
  bottomRef: RefObject<HTMLDivElement | null>;
}

export default function ObjectListDropIndicators({
  isDragging,
  activeDropZone,
  hoveredItemId,
  tooltipTop,
  objects,
  selectedObjectType,
  objectCount,
  onShowAll,
  bottomRef,
}: ObjectListDropIndicatorsProps) {
  const { t } = useTranslation(['objects']);
  const hoveredObject = hoveredItemId
    ? objects.find((object) => object.id === hoveredItemId)
    : null;

  return (
    <>
      {isDragging && activeDropZone === 'item' && hoveredObject && (
        <div
          className="absolute right-4 z-40 flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-primary-content shadow-xl pointer-events-none"
          style={{ top: tooltipTop }}
        >
          <FolderInput size={14} />
          <span className="text-xs font-semibold whitespace-nowrap">
            {t('item.move_to', { name: hoveredObject.name })}
          </span>
        </div>
      )}

      {isDragging ? (
        <div
          ref={bottomRef}
          className={`relative z-30 border-t border-dashed border-t-2 px-3 py-5 transition-all duration-200 ${
            activeDropZone === 'new-object'
              ? 'border-primary bg-primary/15'
              : 'border-base-300/50 bg-base-200/70'
          }`}
          style={{ animation: 'slideUp 200ms ease-out' }}
        >
          <div
            className={`flex items-center justify-center gap-2 ${
              activeDropZone === 'new-object' ? 'text-primary' : 'text-base-content/50'
            }`}
          >
            <FolderPlus
              size={18}
              className={activeDropZone === 'new-object' ? 'animate-pulse' : ''}
            />
            <span className="text-xs font-medium">{t('item.append_new')}</span>
          </div>
        </div>
      ) : (
        <footer
          ref={bottomRef}
          data-testid="object-list-count-overlay"
          className="pointer-events-none absolute inset-x-0 bottom-0 z-30 flex items-center justify-end gap-3 px-2 pb-2 pt-1.5"
        >
          {selectedObjectType && (
            <button
              className="pointer-events-auto text-[10px] text-primary/60 transition-colors hover:text-primary"
              onClick={onShowAll}
            >
              {t('item.show_all')}
            </button>
          )}
          <LiquidSurface
            liquidRole="overlay"
            className="rounded-lg px-2.5 py-1 text-[10px] font-medium tabular-nums text-base-content/55 shadow-sm"
          >
            <span aria-live="polite">{t('item.object_count', { count: objectCount })}</span>
          </LiquidSurface>
        </footer>
      )}
    </>
  );
}
