import { useGameSchema } from '../../hooks/useObjects';
import type { ObjectSummary, FilterDef } from '../../types/object';
import { type ModFolder } from '../../hooks/useFolders';
import {
  X,
  Upload,
  Image as ImageIcon,
  Trash2,
  Search,
  ChevronDown,
  CheckCircle,
  Ban,
  Sparkles,
} from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useState, useMemo, useEffect, useRef } from 'react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useEditObjectForm } from './hooks/useEditObjectForm';
import { useMasterDbSync, type DbEntryFull } from './hooks/useMasterDbSync';

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

  // Tabs state: Manual vs Auto Sync
  const [activeTab, setActiveTab] = useState<'manual' | 'auto'>('manual');

  // Reset state when object changes or modal opens
  useEffect(() => {
    if (open && object) {
      setSelectedThumbnailPath(null);
      setThumbnailAction('keep');
      setActiveTab('manual');
      setIsDbOpen(false); // Ensure closed on open
      setDbSearch('');
    }
  }, [open, object?.id]); // Depend on object.id to detect switches

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
  }, []);

  // Core Form Logic
  const {
    form: {
      register,
      setValue,
      watch,
      formState: { errors },
    },
    isPending,
    isLoadingDetails,
    handleSubmit,
    isFolder,
    isObject,
  } = useEditObjectForm(open, object, onClose, selectedThumbnailPath, thumbnailAction);

  // Original Name for Context & Suggestions
  const originalName = object?.name || '';

  // MasterDB Sync Logic (Include originalName for suggestions)
  const objectType = watch('object_type');
  const {
    isSyncMode,
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

  // Update sync mode when tab changes
  useEffect(() => {
    // Sync mode activation
    if (activeTab === 'auto') {
      setIsSyncMode(true);
    } else {
      setIsSyncMode(false);
    }
  }, [activeTab, setIsSyncMode]);

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
      <div className="modal-box relative w-11/12 max-w-2xl overflow-visible">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2 z-[60]"
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
          <form onSubmit={handleSubmit} className="gap-4 flex flex-col">
            <div className="grid grid-cols-1 md:grid-cols-[auto_1fr] gap-6">
              {/* Left Col: Thumbnail */}
              <div className="flex flex-col gap-2 items-center">
                <span className="label-text self-start">Thumbnail</span>
                <div className="w-32 h-32 rounded-xl bg-base-300 overflow-hidden flex items-center justify-center border border-base-content/10 relative shadow-inner">
                  {displayThumbnail ? (
                    <img
                      src={displayThumbnail}
                      alt="Thumbnail"
                      className={`w-full h-full object-cover ${selectedThumbnailPath ? 'opacity-50' : ''}`}
                    />
                  ) : (
                    <ImageIcon size={48} className="opacity-20" />
                  )}
                </div>
                <div className="flex gap-2 w-full">
                  <button
                    type="button"
                    className="btn btn-sm btn-outline flex-1 gap-2"
                    onClick={handleThumbnailClick}
                    disabled={activeTab === 'auto'}
                  >
                    <Upload size={14} />
                    Change
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-outline btn-error square px-2"
                    onClick={handleDeleteThumbnail}
                    disabled={activeTab === 'auto' || !displayThumbnail}
                    title="Delete Thumbnail"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
                {thumbnailAction === 'update' && (
                  <div className="text-xs opacity-50 truncate max-w-[128px]">Selected</div>
                )}
                {thumbnailAction === 'delete' && (
                  <div className="text-xs text-error opacity-70">Will be deleted</div>
                )}
              </div>

              {/* Right Col: Fields */}
              <div className="flex flex-col gap-3 w-full">
                {/* Tabs: Manual vs Auto Sync */}
                <div role="tablist" className="tabs tabs-bordered w-full mb-2">
                  <a
                    role="tab"
                    className={`tab ${activeTab === 'manual' ? 'tab-active font-bold border-b-2 border-primary' : ''}`}
                    onClick={() => setActiveTab('manual')}
                  >
                    Manual
                  </a>
                  <a
                    role="tab"
                    className={`tab gap-2 ${activeTab === 'auto' ? 'tab-active font-bold border-b-2 border-primary' : ''}`}
                    onClick={() => activeGame && setActiveTab('auto')}
                    style={{
                      opacity: activeGame ? 1 : 0.5,
                      cursor: activeGame ? 'pointer' : 'not-allowed',
                    }}
                  >
                    Auto Sync
                    {suggestions.length > 0 && (
                      <div className="badge badge-sm badge-secondary">{suggestions.length}</div>
                    )}
                  </a>
                </div>

                {/* Name & Search/Suggestions */}
                <div className="form-control w-full relative">
                  <label className="label py-1">
                    <span className="label-text font-medium">Name</span>
                  </label>

                  {activeTab === 'manual' ? (
                    <>
                      <input
                        type="text"
                        className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
                        {...register('name')}
                      />
                      {errors.name && (
                        <span className="text-error text-xs mt-1">{errors.name.message}</span>
                      )}
                    </>
                  ) : (
                    // Auto Sync Mode: Read-Only Label + Suggestions + Manual Search Overlay
                    <div className="flex flex-col gap-2">
                      {/* Current Linked Value (Read-alike) */}
                      <div className="relative w-full" ref={searchContainerRef}>
                        <label
                          tabIndex={0}
                          className="input input-bordered w-full flex items-center gap-2 cursor-pointer bg-base-200/50 hover:bg-base-200 transition-colors"
                          onClick={(e) => {
                            e.preventDefault(); // Prevent double toggling if label behavior interferes
                            setIsDbOpen(!isDbOpen);
                          }}
                        >
                          <Search className="w-4 h-4 opacity-50" />
                          <span className="flex-1 truncate">
                            {dbSearch || 'Click to search database...'}
                          </span>
                          <ChevronDown className="w-4 h-4 opacity-50" />
                        </label>

                        {/* Floating Search Overlay - Standard Absolute Position */}
                        {isDbOpen && (
                          <div className="absolute top-full left-0 w-full mt-1 p-2 shadow-xl bg-base-100 rounded-box border border-base-300 z-[9999]">
                            <input
                              type="text"
                              placeholder="Type to filter..."
                              className="input input-sm input-bordered w-full mb-2"
                              autoFocus
                              value={dbSearch}
                              onChange={(e) => setDbSearch(e.target.value)}
                              onClick={(e) => e.stopPropagation()} // Prevent close on click
                            />
                            <div className="max-h-60 overflow-y-auto">
                              {isLoading ? (
                                <div className="p-4 text-center text-sm opacity-50">Loading...</div>
                              ) : dbOptions.length > 0 ? (
                                <ul className="menu menu-xs p-0 w-full">
                                  {dbOptions.map((opt) => (
                                    <li key={opt.name} className="w-full">
                                      <button
                                        type="button"
                                        onClick={() => handleDbSelect(opt)}
                                        className="flex items-center gap-3 py-2 w-full"
                                      >
                                        {opt.thumbnail_path ? (
                                          <img
                                            src={convertFileSrc(opt.thumbnail_path)}
                                            className="w-8 h-8 rounded-md object-cover bg-base-300 flex-shrink-0"
                                            alt=""
                                          />
                                        ) : (
                                          <div className="w-8 h-8 rounded-md bg-base-300 flex items-center justify-center flex-shrink-0">
                                            <ImageIcon size={14} className="opacity-30" />
                                          </div>
                                        )}
                                        <div className="flex flex-col items-start overflow-hidden flex-1">
                                          <span className="font-bold truncate w-full text-left">
                                            {opt.name}
                                          </span>
                                          {opt.aliases && (
                                            <span className="text-xs opacity-50 truncate w-full text-left">
                                              {opt.aliases[0]}
                                            </span>
                                          )}
                                        </div>
                                      </button>
                                    </li>
                                  ))}
                                </ul>
                              ) : (
                                <div className="p-4 text-center text-sm opacity-50">
                                  {error ? 'Error loading DB' : 'No matches found'}
                                </div>
                              )}
                            </div>
                          </div>
                        )}
                      </div>

                      {/* Smart Suggestions Cards */}
                      {suggestions.length > 0 && (
                        <div className="flex flex-col gap-1 mt-1">
                          <div className="flex items-center gap-1 text-xs font-bold opacity-60 px-1">
                            <Sparkles size={12} className="text-secondary" />
                            <span>Smart Suggestions</span>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            {suggestions.map((sugg) => (
                              <div
                                key={sugg.name}
                                className="flex items-center gap-2 p-2 border border-base-200 rounded-lg hover:border-primary/50 hover:bg-base-200/50 cursor-pointer transition-all"
                                onClick={() => handleDbSelect(sugg)}
                              >
                                {sugg.thumbnail_path ? (
                                  <img
                                    src={convertFileSrc(sugg.thumbnail_path)}
                                    className="w-8 h-8 rounded-md object-cover bg-base-300"
                                    alt=""
                                  />
                                ) : (
                                  <div className="w-8 h-8 rounded-md bg-base-300 flex items-center justify-center">
                                    <ImageIcon size={14} className="opacity-30" />
                                  </div>
                                )}
                                <div className="flex flex-col overflow-hidden">
                                  <span className="text-xs font-bold truncate">{sugg.name}</span>
                                  <span className="text-[10px] opacity-50 truncate">
                                    {(sugg.score * 100).toFixed(0)}% Match
                                  </span>
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>

                {/* Category Dropdown (Read-only in Sync Mode) */}
                <div className="form-control w-full">
                  <label className="label py-1">
                    <span className="label-text font-medium">Category</span>
                  </label>
                  {activeTab === 'auto' ? (
                    <div className="px-3 py-2 bg-base-200/50 rounded-lg border border-base-300 text-sm opacity-80 min-h-10 flex items-center">
                      {gameSchema?.categories.find((c) => c.name === objectType)?.label ||
                        objectType ||
                        'None'}
                    </div>
                  ) : (
                    <select
                      className={`select select-bordered w-full ${errors.object_type ? 'select-error' : ''}`}
                      {...register('object_type')}
                    >
                      <option value="">Select Category</option>
                      {gameSchema?.categories.map((cat) => (
                        <option key={cat.name} value={cat.name}>
                          {cat.label ?? cat.name}
                        </option>
                      ))}
                    </select>
                  )}
                </div>

                {/* Dynamic Metadata Fields */}
                {categoryFilters.map((filter) => (
                  <div key={filter.key} className="form-control w-full">
                    <label className="label py-1">
                      <span className="label-text">{filter.label}</span>
                    </label>
                    {activeTab === 'auto' ? (
                      <div className="px-3 py-2 bg-base-200/50 rounded-lg border border-base-300 text-sm opacity-80 min-h-10 flex items-center">
                        {(watch(`metadata.${filter.key}`) as string) || 'None'}
                      </div>
                    ) : filter.options && filter.options.length > 0 ? (
                      <select
                        className="select select-bordered w-full select-sm"
                        {...register(`metadata.${filter.key}`)}
                      >
                        <option value="">None</option>
                        {filter.options.map((opt) => (
                          <option key={opt} value={opt}>
                            {opt}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        type="text"
                        className="input input-bordered input-sm"
                        {...register(`metadata.${filter.key}`)}
                      />
                    )}
                  </div>
                ))}
              </div>
            </div>

            <div className="modal-action border-t border-base-200 pt-4 mt-4">
              <button type="button" className="btn" onClick={onClose} disabled={isPending}>
                Cancel
              </button>
              <button type="submit" className="btn btn-primary min-w-[120px]" disabled={isPending}>
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
