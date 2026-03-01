import { useGameSchema } from '../../hooks/useObjects';
import type { ObjectSummary, FilterDef } from '../../types/object';
import { type ModFolder } from '../../hooks/useFolders';
import { X, Upload, Image as ImageIcon, Trash2, Search, ChevronDown, Sparkles } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useState, useMemo, useEffect, useRef } from 'react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useEditObjectForm } from './hooks/useEditObjectForm';
import { useMasterDbSync, type DbEntryFull } from './hooks/useMasterDbSync';
import { Controller } from 'react-hook-form';
import { TagInput } from '../../components/ui/TagInput';

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

  const isAutoSync = watch('is_auto_sync');
  const activeTab = isAutoSync ? 'auto' : 'manual';

  const hasCustomSkin = watch('has_custom_skin');

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
                <div className="flex min-w-0 flex-1 flex-col gap-3">
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
                              e.preventDefault();
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
                            <div className="absolute left-0 top-full z-80 mt-1 w-full rounded-box border border-base-300 bg-base-100 p-2 shadow-xl">
                              <input
                                type="text"
                                placeholder="Type to filter..."
                                className="input input-sm input-bordered w-full mb-2"
                                autoFocus
                                value={dbSearch}
                                onChange={(e) => setDbSearch(e.target.value)}
                                onClick={(e) => e.stopPropagation()}
                              />
                              <div className="h-64 overflow-y-auto">
                                {isLoading ? (
                                  <div className="flex h-full items-center justify-center text-center text-sm opacity-50">
                                    Loading...
                                  </div>
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
                                              className="w-8 h-8 rounded-md object-cover bg-base-300 shrink-0"
                                              alt=""
                                            />
                                          ) : (
                                            <div className="w-8 h-8 rounded-md bg-base-300 flex items-center justify-center shrink-0">
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
                                  <div className="flex h-full items-center justify-center text-center text-sm opacity-50">
                                    {error ? 'Error loading DB' : 'No matches found'}
                                  </div>
                                )}
                              </div>
                              <p className="mt-2 h-4 text-[10px] opacity-60">
                                Showing first results. Type more letters to narrow down.
                              </p>
                            </div>
                          )}
                        </div>

                        {/* Smart Suggestions Cards (Hidden if Auto Sync is Active) */}
                        {suggestions.length > 0 && !watch('is_auto_sync') && (
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
                      <div className="px-3 py-1 font-semibold text-base-content hover:bg-base-200/50 rounded transition-colors inline-block self-start">
                        {selectedSyncEntry
                          ? gameSchema?.categories.find((c) => c.name === objectType)?.label ||
                            objectType ||
                            'None'
                          : '-'}
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

                  {/* Safe/NSFW Toggle */}
                  {activeTab === 'manual' && (
                    <div className="form-control mt-2">
                      <label className="label cursor-pointer justify-start gap-3 w-fit">
                        <input
                          type="checkbox"
                          className="toggle toggle-primary"
                          {...register('is_safe')}
                        />
                        <span className="label-text font-medium flex items-center gap-2">
                          Safe Mode (SFW)
                        </span>
                      </label>
                      <p className="text-[10px] opacity-70 ml-13 -mt-1">
                        If unchecked, this item will be hidden when Privacy Mode is enabled.
                      </p>
                    </div>
                  )}

                  {/* Dynamic Metadata Fields */}
                  {categoryFilters.map((filter) => (
                    <div key={filter.key} className="form-control w-full">
                      <label className="label py-1">
                        <span className="label-text">{filter.label}</span>
                      </label>
                      {activeTab === 'auto' ? (
                        <div className="px-3 py-1 font-semibold text-base-content hover:bg-base-200/50 rounded transition-colors inline-block self-start">
                          {selectedSyncEntry
                            ? (watch(`metadata.${filter.key}`) as string) || 'None'
                            : '-'}
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

                  {/* Tags and Custom Skins */}
                  {activeTab === 'auto' ? (
                    selectedSyncEntry && (
                      <>
                        {/* Tags */}
                        <div className="form-control w-full mt-2">
                          <label className="label py-1">
                            <span className="label-text">Tags (Aliases)</span>
                          </label>
                          <div className="flex flex-wrap gap-1 px-3">
                            {selectedSyncEntry.tags && selectedSyncEntry.tags.length > 0 ? (
                              selectedSyncEntry.tags.map((tag) => (
                                <div key={tag} className="badge badge-outline badge-sm opacity-80">
                                  {tag}
                                </div>
                              ))
                            ) : (
                              <div className="font-semibold text-base-content opacity-50">-</div>
                            )}
                          </div>
                        </div>

                        {/* Custom Skin Selection */}
                        <div className="form-control w-full mt-2">
                          <label className="label py-1">
                            <span className="label-text font-medium">Mapped Skin</span>
                          </label>
                          {selectedSyncEntry.custom_skins &&
                          selectedSyncEntry.custom_skins.length > 0 ? (
                            <div className="flex flex-col gap-2">
                              <select
                                className="select select-bordered w-full select-sm"
                                value={
                                  watch('has_custom_skin') ? watch('custom_skin.name') || '' : ''
                                }
                                onChange={(e) => {
                                  const skinName = e.target.value;
                                  if (!skinName) {
                                    setValue('has_custom_skin', false);
                                    setValue('custom_skin', {
                                      name: '',
                                      aliases: [],
                                      thumbnail_skin_path: '',
                                      rarity: '',
                                    });
                                  } else {
                                    const foundSkin = selectedSyncEntry.custom_skins!.find(
                                      (s) => s.name === skinName,
                                    );
                                    if (foundSkin) {
                                      setValue('has_custom_skin', true);
                                      setValue('custom_skin', foundSkin);
                                    }
                                  }
                                }}
                              >
                                <option value="">Default / Base Skin</option>
                                {selectedSyncEntry.custom_skins.map((skin) => (
                                  <option key={skin.name} value={skin.name}>
                                    {skin.name}
                                  </option>
                                ))}
                              </select>

                              {/* Selected Skin Preview */}
                              {watch('has_custom_skin') && watch('custom_skin.name') && (
                                <div className="flex items-center gap-3 text-sm p-3 border border-base-200 rounded-lg bg-base-100/50 mt-1">
                                  {watch('custom_skin.thumbnail_skin_path') ? (
                                    <img
                                      src={convertFileSrc(
                                        watch('custom_skin.thumbnail_skin_path')!,
                                      )}
                                      className="w-10 h-10 object-cover rounded shadow-sm bg-base-300"
                                      alt={watch('custom_skin.name')}
                                    />
                                  ) : (
                                    <div className="w-10 h-10 rounded bg-base-300 flex items-center justify-center shadow-sm">
                                      <ImageIcon size={16} className="opacity-30" />
                                    </div>
                                  )}
                                  <div className="flex flex-col">
                                    <span className="font-semibold">
                                      {watch('custom_skin.name')}
                                    </span>
                                    {watch('custom_skin.aliases') &&
                                      watch('custom_skin.aliases')!.length > 0 && (
                                        <span className="text-[10px] opacity-70">
                                          Aliases: {watch('custom_skin.aliases')!.join(', ')}
                                        </span>
                                      )}
                                  </div>
                                </div>
                              )}
                            </div>
                          ) : (
                            <div className="font-semibold text-base-content hover:bg-base-200/50 rounded transition-colors inline-block self-start px-3 py-1 bg-base-200/30">
                              Default / Base Skin
                            </div>
                          )}
                        </div>
                      </>
                    )
                  ) : (
                    <>
                      <div className="form-control w-full mt-2">
                        <label className="label py-1">
                          <span className="label-text">Tags</span>
                        </label>
                        <Controller
                          control={form.control}
                          name="tags"
                          render={({ field }) => (
                            <TagInput
                              tags={field.value || []}
                              onChange={field.onChange}
                              placeholder="Add tags (space/comma to enter)"
                            />
                          )}
                        />
                      </div>

                      {/* Manual Custom Skin */}
                      <div className="form-control w-full mt-4">
                        <label className="label py-1">
                          <span className="label-text font-bold text-lg">Skin Mapping</span>
                        </label>
                        <div className="flex gap-4 mt-1 mb-2 px-1">
                          <label className="label cursor-pointer justify-start gap-2">
                            <input
                              type="radio"
                              className="radio radio-primary radio-sm"
                              value="false"
                              checked={!hasCustomSkin}
                              onChange={() => {
                                setValue('has_custom_skin', false);
                                setValue('custom_skin', {
                                  name: '',
                                  aliases: [],
                                  thumbnail_skin_path: '',
                                  rarity: '',
                                });
                              }}
                            />
                            <span className="label-text">Default / Base Skin</span>
                          </label>
                          <label className="label cursor-pointer justify-start gap-2">
                            <input
                              type="radio"
                              className="radio radio-primary radio-sm"
                              value="true"
                              checked={hasCustomSkin}
                              onChange={() => setValue('has_custom_skin', true)}
                            />
                            <span className="label-text">Custom Skin</span>
                          </label>
                        </div>

                        {hasCustomSkin && (
                          <div className="flex flex-col gap-3 mt-2 p-4 border border-base-300 rounded-lg bg-base-200/30">
                            <div className="form-control w-full">
                              <label className="label py-1">
                                <span className="label-text">Skin Name</span>
                              </label>
                              <input
                                type="text"
                                className={`input input-bordered input-sm w-full ${errors.custom_skin?.name ? 'input-error' : ''}`}
                                placeholder="e.g. Red Dead of Night"
                                {...register('custom_skin.name')}
                              />
                              {errors.custom_skin?.name && (
                                <label className="label py-0 pt-1">
                                  <span className="label-text-alt text-error">
                                    {errors.custom_skin.name.message}
                                  </span>
                                </label>
                              )}
                            </div>

                            <div className="form-control w-full">
                              <label className="label py-1">
                                <span className="label-text">Thumbnail Path (Optional)</span>
                              </label>
                              <input
                                type="text"
                                className="input input-bordered input-sm w-full"
                                placeholder="e.g. skins/red_dead_of_night.png"
                                {...register('custom_skin.thumbnail_skin_path')}
                              />
                            </div>

                            <div className="form-control w-full">
                              <label className="label py-1">
                                <span className="label-text">Rarity (Optional)</span>
                              </label>
                              <input
                                type="text"
                                className="input input-bordered input-sm w-full"
                                placeholder="e.g. 5-Star"
                                {...register('custom_skin.rarity')}
                              />
                            </div>

                            <div className="form-control w-full">
                              <label className="label py-1">
                                <span className="label-text">Aliases</span>
                              </label>
                              <Controller
                                control={form.control}
                                name="custom_skin.aliases"
                                render={({ field: { value, onChange } }) => (
                                  <TagInput
                                    tags={value || []}
                                    onChange={onChange}
                                    placeholder="Add aliases (space/comma to enter)"
                                  />
                                )}
                              />
                            </div>
                          </div>
                        )}
                      </div>
                    </>
                  )}
                </div>

                {/* Right Col (Visual): Thumbnail */}
                <div className="flex w-full shrink-0 flex-col items-center gap-2 md:w-32">
                  <span className="label-text self-center font-medium">Thumbnail</span>
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
                  {activeTab === 'manual' && (
                    <div className="flex gap-2 w-full">
                      <button
                        type="button"
                        className="btn btn-sm btn-outline flex-1 gap-2"
                        onClick={handleThumbnailClick}
                      >
                        <Upload size={14} />
                        Change
                      </button>
                      <button
                        type="button"
                        className="btn btn-sm btn-outline btn-error square px-2"
                        onClick={handleDeleteThumbnail}
                        disabled={!displayThumbnail}
                        title="Delete Thumbnail"
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  )}
                  {thumbnailAction === 'update' && (
                    <div className="text-xs opacity-50 truncate max-w-[128px]">Selected</div>
                  )}
                  {thumbnailAction === 'delete' && (
                    <div className="text-xs text-error opacity-70">Will be deleted</div>
                  )}
                </div>
              </div>
            </div>

            <div className="modal-action mt-0 border-t border-base-200 bg-base-100/90 pt-4 backdrop-blur-sm">
              <button type="button" className="btn" onClick={onClose} disabled={isPending}>
                Cancel
              </button>
              <button
                type="submit"
                className="btn btn-primary min-w-[120px]"
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
