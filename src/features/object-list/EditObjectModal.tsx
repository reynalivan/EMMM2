import { useGameSchema } from '../../hooks/useObjects';
import type { ObjectSummary, FilterDef } from '../../types/object';
import { type ModFolder } from '../../hooks/useFolders';
import { X } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useState, useMemo, useEffect, useRef } from 'react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useEditObjectForm } from './hooks/useEditObjectForm';
import { useMasterDbSync, type DbEntryFull } from './hooks/useMasterDbSync';
import { EditObjectTabManual } from './EditObjectTabManual';
import { EditObjectTabAuto } from './EditObjectTabAuto';
import { EditObjectTabThumbnail } from './EditObjectTabThumbnail';

interface EditObjectModalProps {
  open: boolean;
  object: ObjectSummary | ModFolder | null;
  onClose: () => void;
}

export default function EditObjectModal({ open, object, onClose }: EditObjectModalProps) {
  const { activeGame } = useActiveGame();
  const { data: gameSchema } = useGameSchema();

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

  if (!open || !object) return null;

  return (
    <div className="modal modal-open">
      <div className="modal-box relative flex h-[90vh] max-h-[90vh] w-11/12 max-w-2xl flex-col overflow-hidden">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2 z-60"
          onClick={onClose}
          aria-label="Close"
        >
          <X size={16} />
        </button>

        {/* Header with Context */}
        <h3 className="font-bold text-lg mb-1">Edit Metadata</h3>
        <p className="text-sm opacity-50 mb-4 truncate">
          Original: <span className="font-mono">{originalName}</span>
        </p>

        {isLoadingDetails ? (
          <div className="flex justify-center p-8">Loading details...</div>
        ) : (
          <form onSubmit={handleSubmit} className="flex h-full min-h-0 flex-col gap-4">
            {/* Tabs: Manual vs Auto Sync */}
            <div
              role="tablist"
              className="tabs tabs-bordered -mx-6 border-b border-base-200 bg-base-100 px-6"
            >
              <a
                role="tab"
                className={`tab ${activeTab === 'manual' ? 'tab-active font-bold border-b-2 border-primary' : ''}`}
                onClick={() => handleTabSwitch('manual')}
              >
                Manual
              </a>
              <a
                role="tab"
                className={`tab gap-2 ${activeTab === 'auto' ? 'tab-active font-bold border-b-2 border-primary' : ''}`}
                onClick={() => activeGame && handleTabSwitch('auto')}
                style={{
                  opacity: activeGame ? 1 : 0.5,
                  cursor: activeGame ? 'pointer' : 'not-allowed',
                }}
              >
                Auto Sync
                {watch('is_auto_sync') ? (
                  <div className="badge badge-sm badge-success text-white">Active</div>
                ) : (
                  suggestions.length > 0 && (
                    <div className="badge badge-sm badge-secondary">{suggestions.length}</div>
                  )
                )}
              </a>
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
                Cancel
              </button>
              <button
                type="submit"
                className="btn btn-primary min-w-30"
                disabled={isPending || (activeTab === 'auto' && !selectedSyncEntry)}
              >
                {isPending ? <span className="loading loading-spinner"></span> : 'Save Changes'}
              </button>
            </div>
          </form>
        )}
      </div>
      <div className="modal-backdrop" onClick={onClose}></div>
    </div>
  );
}
