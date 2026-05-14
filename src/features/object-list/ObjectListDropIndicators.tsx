import { FolderInput, FolderPlus } from 'lucide-react';
import type { RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import type { DropZone } from './useObjectListDropZones';
import type { WorkspaceObjectNode } from '../../types/workspace';

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

      <div
        ref={bottomRef}
        className={`px-3 border-t transition-all duration-200 relative z-30 ${
          isDragging
            ? activeDropZone === 'new-object'
              ? 'py-5 border-primary bg-primary/15 border-dashed border-t-2'
              : 'py-5 border-base-300/50 bg-base-200/70 border-dashed border-t-2'
            : 'py-1.5 border-base-300/20'
        }`}
        style={isDragging ? { animation: 'slideUp 200ms ease-out' } : undefined}
      >
        {isDragging ? (
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
        ) : (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-base-content/30">
              {t('item.object_count', { count: objectCount })}
            </span>
            <div className="flex items-center gap-3">
              {selectedObjectType && (
                <button
                  className="text-[10px] text-primary/60 hover:text-primary transition-colors"
                  onClick={onShowAll}
                >
                  {t('item.show_all')}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </>
  );
}
