import { useGameSchema } from '../hooks/useObjectQueries';
import type { ObjectSummary, FilterDef } from '@/entities/game-object';
import type { ModFolder } from '@/entities/game-object';
import { X } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useState, useMemo, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
import { useEditObjectForm } from '../hooks/useEditObjectForm';
import { useMasterDbSync, type DbEntryFull } from '../hooks/useMasterDbSync';
import { EditObjectTabManual } from './EditObjectTabManual';
import { EditObjectTabAuto } from './EditObjectTabAuto';
import { EditObjectTabThumbnail } from './EditObjectTabThumbnail';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import { LiquidSurface } from '@/shared/ui/liquid';

interface EditObjectModalProps {
  open: boolean;
  object: ObjectSummary | ModFolder | null;
  onClose: () => void;
}

export default function EditObjectModal({ open, object, onClose }: EditObjectModalProps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const { data: gameSchema } = useGameSchema();
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Thumbnail state (UI only)
  const [selectedThumbnailPath, setSelectedThumbnailPath] = useState<string | null>(null);
  const [thumbnailAction, setThumbnailAction] = useState<'keep' | 'update' | 'delete'>('keep');

  // Auto Sync Entry State
  const [selectedSyncEntry, setSelectedSyncEntry] = useState<DbEntryFull | null>(null);

  // Core Form Logic
  const {
    form, // exposing full form for getValues/setValue above
    form: { setValue, watch },
    isPending,
    isLoadingDetails,
    handleSubmit,
    isFolder,
    isObject,
  } = useEditObjectForm(open, object, onClose, selectedThumbnailPath, thumbnailAction);

  const activeTab = watch('is_auto_sync') ? 'auto' : 'manual';

  // Original Name for Context & Suggestions
  const originalName = object?.name || '';

  // MasterDB Sync Logic (Include originalName for suggestions)
  const objectType = watch('object_type');
  const {
    setIsSyncMode,
    dbSearch,
    setDbSearch,
    isDbOpen,
    setIsDbOpen,
    dbOptions,
    suggestions,
    isLoading,
    error,
  } = useMasterDbSync(objectType, originalName);

  // Click outside handler for search overlay
  const searchContainerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        searchContainerRef.current &&
        !searchContainerRef.current.contains(event.target as Node)
      ) {
        setIsDbOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [setIsDbOpen]);

  // Reset state when object changes or modal opens
  // Moved AFTER useMasterDbSync because it uses setDbSearch/setIsDbOpen
  useEffect(() => {
    if (open && object) {
      const t = setTimeout(() => {
        setSelectedThumbnailPath(null);
        setThumbnailAction('keep');
        setIsDbOpen(false); // Ensure closed on open
      }, 0);
      return () => clearTimeout(t);
    }
  }, [open, object, setIsDbOpen]); // Depend on object to detect switches

  // Initialize dbSearch when data finishes loading
  const initializedObjectId = useRef<string | null>(null);
  useEffect(() => {
    if (open && object && !isLoadingDetails) {
      const objId = isFolder ? (object as ModFolder).path : (object as ObjectSummary).id;
      if (initializedObjectId.current !== objId) {
        initializedObjectId.current = objId;
        const t = setTimeout(() => {
          setDbSearch(form.getValues('is_auto_sync') ? form.getValues('name') : '');
        }, 0);
        return () => clearTimeout(t);
      }
    } else if (!open) {
      initializedObjectId.current = null;
      const t = setTimeout(() => setSelectedSyncEntry(null), 0);
      return () => clearTimeout(t);
    }
  }, [open, object, isLoadingDetails, isFolder, form, setDbSearch]);

  // Hydrate auto-sync selection once search results (dbOptions) are available
  useEffect(() => {
    if (open && activeTab === 'auto' && dbOptions.length > 0 && !selectedSyncEntry) {
      const defaultName = form.getValues('name');
      if (defaultName) {
        const exactMatch = dbOptions.find((e) => e.name === defaultName);
        if (exactMatch) {
          const t = setTimeout(() => {
            setSelectedSyncEntry(exactMatch);
            setDbSearch(exactMatch.name);
          }, 0);
          return () => clearTimeout(t);
        }
      }
    }
  }, [open, activeTab, dbOptions, selectedSyncEntry, form, setDbSearch]);

  // Update sync mode when tab changes
  useEffect(() => {
    // Sync mode activation
    if (activeTab === 'auto') {
      setIsSyncMode(true);
    } else {
      setIsSyncMode(false);
    }
  }, [activeTab, setIsSyncMode]);

  // Handle manual tab switch: clear selection if we switch to manual
  const handleTabSwitch = (type: 'manual' | 'auto') => {
    if (type === 'manual') {
      setValue('is_auto_sync', false);
      setIsDbOpen(false);
    } else {
      setValue('is_auto_sync', true);
      setIsDbOpen(false);
      if (!selectedSyncEntry) {
        setDbSearch(form.getValues('name') || '');
      }
    }
  };

  // Derive per-category filters from selected category
  const categoryFilters: FilterDef[] = useMemo(() => {
    if (!gameSchema || !objectType) return [];
    const cat = gameSchema.categories.find((c) => c.name === objectType);
    return cat?.filters ?? [];
  }, [gameSchema, objectType]);

  // Reset metadata when user switches categories to clear stale keys
  const prevCategoryRef = useRef<string>('');
  useEffect(() => {
    if (prevCategoryRef.current && prevCategoryRef.current !== objectType) {
      setValue('metadata', {});
    }
    prevCategoryRef.current = objectType ?? '';
  }, [objectType, setValue]);

  // Handler: user selects from dropdown — immediately apply to form (Auto Sync)
  const handleDbSelect = (entry: DbEntryFull) => {
    setSelectedSyncEntry(entry);
    setDbSearch(entry.name);
    setIsDbOpen(false);

    // Auto-fill and lock
    setValue('name', entry.name);

    if (entry.object_type) {
      setValue('object_type', entry.object_type);
    }

    if (entry.metadata) {
      const meta: Record<string, unknown> = {};
      Object.entries(entry.metadata).forEach(([k, v]) => {
        meta[k] = v;
      });
      setValue('metadata', meta);
    }

    if (entry.thumbnail_path) {
      setSelectedThumbnailPath(entry.thumbnail_path);
      setThumbnailAction('update');
    }
  };

  // Derived thumbnail logic
  const existingThumbnail = isFolder
    ? (object as ModFolder).thumbnail_path
    : isObject
      ? (object as ObjectSummary).thumbnail_path
      : null;

  const displayThumbnail = useMemo(() => {
    if (thumbnailAction === 'delete') return null;
    if (thumbnailAction === 'update' && selectedThumbnailPath) {
      try {
        return convertFileSrc(selectedThumbnailPath);
      } catch {
        return `asset://${selectedThumbnailPath}`; // Fallback
      }
    }
    if (thumbnailAction === 'keep' && existingThumbnail) {
      try {
        return convertFileSrc(existingThumbnail);
      } catch {
        return `asset://${existingThumbnail}`; // Fallback
      }
    }
    return null;
  }, [thumbnailAction, selectedThumbnailPath, existingThumbnail]);

  const handleThumbnailClick = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });

      if (selected && typeof selected === 'string') {
        setSelectedThumbnailPath(selected);
        setThumbnailAction('update');
      }
    } catch (err) {
      console.error('Failed to select image', err);
    }
  };

  const handleDeleteThumbnail = () => {
    setThumbnailAction('delete');
    setSelectedThumbnailPath(null);
  };

  useDialogSync(dialogRef, open && Boolean(object));

  if (!open || !object) return null;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      aria-labelledby="edit-object-title"
      onClose={onClose}
    >
      <div className="modal-box relative flex h-[90vh] max-h-[90vh] w-11/12 max-w-2xl flex-col overflow-hidden">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2 z-60"
          onClick={onClose}
          aria-label={t('common:actions.close')}
        >
          <X size={16} />
        </button>

        {/* Header with Context */}
        <h3 id="edit-object-title" className="font-bold text-lg mb-1">
          {t('edit_modal.title')}
        </h3>
        <p className="text-sm text-muted mb-4 truncate">
          {t('edit_modal.original')}: <span className="font-mono">{originalName}</span>
        </p>

        {isLoadingDetails ? (
          <div className="flex justify-center p-8">{t('common:states.loading')}</div>
        ) : (
          <form onSubmit={handleSubmit} className="flex h-full min-h-0 flex-col gap-4">
            <div role="tablist" aria-label={t('edit_modal.title')}>
              <LiquidSurface
                liquidRole="control"
                className="rounded-[var(--radius-box)]"
                contentClassName="flex gap-1 p-1"
              >
                <button
                  type="button"
                  role="tab"
                  aria-selected={activeTab === 'manual'}
                  className={`flex min-h-8 items-center justify-center rounded-[calc(var(--radius-box)-0.25rem)] px-3 text-sm font-medium transition-[background-color,color] duration-150 ${
                    activeTab === 'manual'
                      ? 'bg-base-content/[0.08] text-base-content'
                      : 'text-base-content/55 hover:bg-base-content/[0.05] hover:text-base-content'
                  }`}
                  onClick={() => handleTabSwitch('manual')}
                >
                  {t('edit_modal.tabs.manual')}
                </button>
                <button
                  type="button"
                  role="tab"
                  aria-selected={activeTab === 'auto'}
                  disabled={!activeGame}
                  className={`flex min-h-8 items-center justify-center gap-2 rounded-[calc(var(--radius-box)-0.25rem)] px-3 text-sm font-medium transition-[background-color,color] duration-150 disabled:cursor-not-allowed disabled:opacity-50 ${
                    activeTab === 'auto'
                      ? 'bg-base-content/[0.08] text-base-content'
                      : 'text-base-content/55 hover:bg-base-content/[0.05] hover:text-base-content'
                  }`}
                  onClick={() => handleTabSwitch('auto')}
                >
                  {t('edit_modal.tabs.auto')}
                  {watch('is_auto_sync') ? (
                    <span className="badge badge-sm badge-success text-success-content">
                      {t('edit_modal.badges.active')}
                    </span>
                  ) : (
                    suggestions.length > 0 && (
                      <span className="badge badge-sm badge-secondary">{suggestions.length}</span>
                    )
                  )}
                </button>
              </LiquidSurface>
            </div>

            <div className="min-h-0 -mx-6 flex-1 overflow-y-scroll px-6 pt-4 [scrollbar-gutter:stable]">
              <div className="flex flex-col gap-6 md:flex-row">
                {/* Left Col (Visual): Fields */}
                {activeTab === 'auto' ? (
                  <EditObjectTabAuto
                    form={form}
                    gameSchema={gameSchema}
                    categoryFilters={categoryFilters}
                    selectedSyncEntry={selectedSyncEntry}
                    isDbOpen={isDbOpen}
                    setIsDbOpen={setIsDbOpen}
                    dbSearch={dbSearch}
                    setDbSearch={setDbSearch}
                    isLoading={isLoading}
                    dbOptions={dbOptions}
                    error={error}
                    suggestions={suggestions}
                    handleDbSelect={handleDbSelect}
                    searchContainerRef={searchContainerRef}
                  />
                ) : (
                  <EditObjectTabManual
                    form={form}
                    gameSchema={gameSchema}
                    categoryFilters={categoryFilters}
                    isObject={isObject}
                  />
                )}

                {/* Right Col (Visual): Thumbnail */}
                <EditObjectTabThumbnail
                  displayThumbnail={displayThumbnail}
                  selectedThumbnailPath={selectedThumbnailPath}
                  thumbnailAction={thumbnailAction}
                  activeTab={activeTab}
                  handleThumbnailClick={handleThumbnailClick}
                  handleDeleteThumbnail={handleDeleteThumbnail}
                />
              </div>
            </div>

            <div className="modal-action mt-0 border-t border-base-200 bg-base-100/90 pt-4 backdrop-blur-sm">
              <button type="button" className="btn" onClick={onClose} disabled={isPending}>
                {t('common:actions.cancel')}
              </button>
              <button
                type="submit"
                className="btn btn-primary min-w-30"
                disabled={isPending || (activeTab === 'auto' && !selectedSyncEntry)}
              >
                {isPending ? (
                  <span className="loading loading-spinner"></span>
                ) : (
                  t('common:actions.save_changes')
                )}
              </button>
            </div>
          </form>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
