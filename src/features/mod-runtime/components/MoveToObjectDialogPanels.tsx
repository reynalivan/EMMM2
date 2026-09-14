import { Check, FolderTree, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ObjectSummary } from '@/entities/game-object';
import type { WorkspaceMoveTarget } from '@/shared/api/tauri/bindings.gen';

interface MoveToObjectDialogPanelsProps {
  dialogId: string;
  objects: ObjectSummary[];
  availableObjects: ObjectSummary[];
  currentObjectId?: string;
  selectedObjectId: string;
  searchTerm: string;
  onSearchChange: (value: string) => void;
  onSelectObject: (objectId: string) => void;
  moveTargets: WorkspaceMoveTarget[];
  filteredMoveTargets: WorkspaceMoveTarget[];
  targetsLoading: boolean;
  locationSearchTerm: string;
  onLocationSearchChange: (value: string) => void;
  targetSubpath: string | null;
  onSelectTarget: (targetSubpath: string | null) => void;
}

export default function MoveToObjectDialogPanels({
  dialogId,
  objects,
  availableObjects,
  currentObjectId,
  selectedObjectId,
  searchTerm,
  onSearchChange,
  onSelectObject,
  moveTargets,
  filteredMoveTargets,
  targetsLoading,
  locationSearchTerm,
  onLocationSearchChange,
  targetSubpath,
  onSelectTarget,
}: MoveToObjectDialogPanelsProps) {
  const { t } = useTranslation('folder_grid');

  return (
    <div className="grid gap-5 p-6 lg:grid-cols-2">
      <section className="min-w-0 rounded-xl border border-base-300 bg-base-200/20 p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-semibold">{t('move.label')}</h3>
            <p className="mt-1 text-xs text-base-content/55">{t('move.object_hint')}</p>
          </div>
          <span className="badge badge-ghost badge-sm whitespace-nowrap">
            {t('move.object_count', { count: objects.length })}
          </span>
        </div>

        <label htmlFor={`${dialogId}-object-search`} className="sr-only">
          {t('move.search_objects_label')}
        </label>
        <div className="relative mt-4">
          <Search
            size={16}
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-base-content/45"
          />
          <input
            id={`${dialogId}-object-search`}
            type="search"
            className="input input-bordered w-full pl-9"
            placeholder={t('move.placeholder')}
            value={searchTerm}
            onChange={(event) => onSearchChange(event.target.value)}
          />
        </div>

        <div className="mt-3 flex min-h-72 max-h-80 flex-col gap-1 overflow-y-auto rounded-lg border border-base-300 bg-base-100 p-1 scrollbar-thin">
          {availableObjects.length === 0 && (
            <div className="grid flex-1 place-items-center p-4 text-center text-sm text-base-content/50">
              {t('move.no_results')}
            </div>
          )}
          {availableObjects.map((object) => {
            const isCurrentObject = object.id === currentObjectId;
            const isSelected = selectedObjectId === object.id;

            return (
              <button
                key={object.id}
                type="button"
                aria-pressed={isSelected}
                className={`workspace-interactive flex items-center justify-between gap-3 rounded-md px-3 py-3 text-left text-sm ${
                  isSelected ? 'bg-primary font-semibold text-primary-content' : 'hover:bg-base-200'
                }`}
                onClick={() => onSelectObject(object.id)}
              >
                <span className="min-w-0 truncate">
                  {object.name}
                  {isCurrentObject && (
                    <span className="ml-2 text-xs opacity-70">{t('move.current_marker')}</span>
                  )}
                </span>
                {isSelected && <Check size={16} aria-hidden="true" />}
              </button>
            );
          })}
        </div>
      </section>

      <section className="min-w-0 rounded-xl border border-base-300 bg-base-200/20 p-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-semibold">{t('move.location_label')}</h3>
            <p className="mt-1 text-xs text-base-content/55">{t('move.location_hint')}</p>
          </div>
          {selectedObjectId && (
            <span className="badge badge-ghost badge-sm whitespace-nowrap">
              {t('move.location_count', { count: moveTargets.length })}
            </span>
          )}
        </div>

        <label htmlFor={`${dialogId}-location-search`} className="sr-only">
          {t('move.search_locations_label')}
        </label>
        <div className="relative mt-4">
          <Search
            size={16}
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-base-content/45"
          />
          <input
            id={`${dialogId}-location-search`}
            type="search"
            className="input input-bordered w-full pl-9"
            placeholder={t('move.location_placeholder')}
            value={locationSearchTerm}
            disabled={!selectedObjectId}
            onChange={(event) => onLocationSearchChange(event.target.value)}
          />
        </div>

        <div className="mt-3 flex min-h-72 max-h-80 flex-col gap-1 overflow-y-auto rounded-lg border border-base-300 bg-base-100 p-1 scrollbar-thin">
          {!selectedObjectId && (
            <div className="grid flex-1 place-items-center p-4 text-center text-sm text-base-content/50">
              {t('move.select_object_first')}
            </div>
          )}
          {selectedObjectId && targetsLoading && (
            <div className="grid flex-1 place-items-center p-4 text-center text-sm text-base-content/50">
              <span
                className="loading loading-spinner loading-sm text-primary"
                aria-hidden="true"
              />
              <span className="sr-only">{t('move.loading_targets')}</span>
            </div>
          )}
          {selectedObjectId && !targetsLoading && filteredMoveTargets.length === 0 && (
            <div className="grid flex-1 place-items-center p-4 text-center text-sm text-base-content/50">
              {t('move.no_locations')}
            </div>
          )}
          {selectedObjectId &&
            !targetsLoading &&
            filteredMoveTargets.map((target) => {
              const isSelected = targetSubpath === target.target_subpath;

              return (
                <button
                  key={target.target_subpath ?? '__root__'}
                  type="button"
                  aria-pressed={isSelected}
                  className={`workspace-interactive flex items-center gap-2 rounded-md py-3 pr-3 text-left text-sm ${
                    isSelected
                      ? 'bg-primary font-semibold text-primary-content'
                      : 'hover:bg-base-200'
                  }`}
                  style={{ paddingLeft: `${12 + target.depth * 16}px` }}
                  onClick={() => onSelectTarget(target.target_subpath)}
                >
                  <FolderTree size={16} aria-hidden="true" className="shrink-0 opacity-70" />
                  <span className="min-w-0 truncate">{target.display_path}</span>
                  {isSelected && <Check size={16} aria-hidden="true" className="ml-auto" />}
                </button>
              );
            })}
        </div>
      </section>
    </div>
  );
}
