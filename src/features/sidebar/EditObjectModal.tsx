import { useGameSchema } from '../../hooks/useObjects';
import type { ObjectSummary, FilterDef } from '../../types/object';
import { type ModFolder } from '../../hooks/useFolders';
import { X, Upload, Image as ImageIcon } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
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

  // Thumbnail state
  // We keep this local as it's a UI-first state before commit
  const [selectedThumbnailPath, setSelectedThumbnailPath] = useState<string | null>(null);

  // Pending DB sync entry — shown for confirmation before overwriting form fields
  const [pendingSyncEntry, setPendingSyncEntry] = useState<DbEntryFull | null>(null);

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
  } = useEditObjectForm(open, object, onClose, selectedThumbnailPath);

  // MasterDB Sync Logic
  const objectType = watch('object_type');
  const { isSyncMode, setIsSyncMode, dbSearch, setDbSearch, isDbOpen, setIsDbOpen, dbOptions } =
    useMasterDbSync(objectType);

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

  if (!open || !object) return null;

  // Handler: user selects from dropdown — show confirmation instead of direct overwrite
  const handleDbSelectPending = (entry: DbEntryFull) => {
    setDbSearch(entry.name);
    setIsDbOpen(false);
    setPendingSyncEntry(entry);
  };

  // Handler: user confirms the pending sync entry — actually overwrite form fields
  const handleConfirmSync = () => {
    if (!pendingSyncEntry) return;
    const entry = pendingSyncEntry;

    setValue('name', entry.name);

    // Auto-fill category
    if (entry.object_type) {
      setValue('object_type', entry.object_type);
    }

    // Auto-fill metadata from DB entry
    if (entry.metadata) {
      const meta: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(entry.metadata)) {
        meta[k] = v;
      }
      setValue('metadata', meta);
    }

    // Auto-fill thumbnail from DB
    if (entry.thumbnail_path) {
      setSelectedThumbnailPath(entry.thumbnail_path);
    }

    setPendingSyncEntry(null);
  };

  // Handler: user cancels the pending sync
  const handleCancelSync = () => {
    setPendingSyncEntry(null);
  };

  // Derived thumbnail
  // Casting is safe due to isFolder/isObject checks in hook, but here we just need proper access
  const existingThumbnail = isFolder
    ? (object as ModFolder).thumbnail_path
    : (object as ObjectSummary).thumbnail_path;

  const displayThumbnail = selectedThumbnailPath
    ? `asset://${selectedThumbnailPath}`
    : existingThumbnail
      ? `asset://${existingThumbnail}`
      : null;

  const handleThumbnailClick = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });

      if (selected) {
        setSelectedThumbnailPath(selected as string);
      }
    } catch (err) {
      console.error('Failed to select image', err);
    }
  };

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-11/12 max-w-2xl">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={onClose}
          aria-label="Close"
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-lg mb-4">Edit Metadata</h3>

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
                <button
                  type="button"
                  className="btn btn-sm btn-outline w-full gap-2"
                  onClick={handleThumbnailClick}
                >
                  <Upload size={14} />
                  Change
                </button>
                {selectedThumbnailPath && (
                  <div className="text-xs opacity-50 truncate max-w-[128px]">Selected</div>
                )}
              </div>

              {/* Right Col: Fields */}
              <div className="flex flex-col gap-3 w-full">
                {/* Name & Sync Toggle */}
                <div className="form-control w-full">
                  <div className="flex justify-between items-center py-1">
                    <label className="label-text font-medium">Name</label>
                    {activeGame && (
                      <label className="label cursor-pointer gap-2 p-0">
                        <span className="label-text text-xs opacity-70">Sync from DB</span>
                        <input
                          type="checkbox"
                          className="toggle toggle-xs toggle-primary"
                          checked={isSyncMode}
                          onChange={(e) => setIsSyncMode(e.target.checked)}
                          disabled={!watch('object_type')}
                        />
                      </label>
                    )}
                  </div>

                  {isSyncMode ? (
                    <div className="dropdown w-full">
                      <input
                        type="text"
                        placeholder="Search database..."
                        className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
                        value={dbSearch}
                        onChange={(e) => setDbSearch(e.target.value)}
                        onFocus={() => setIsDbOpen(true)}
                        onBlur={() => setTimeout(() => setIsDbOpen(false), 200)}
                      />
                      {isDbOpen && dbOptions.length > 0 && (
                        <ul className="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-full max-h-60 overflow-y-auto block z-50 border border-base-300 top-full mt-1">
                          {dbOptions.map((opt) => (
                            <li key={opt.name}>
                              <button
                                type="button"
                                onMouseDown={(e) => {
                                  e.preventDefault(); // Prevent blur
                                  handleDbSelectPending(opt);
                                }}
                              >
                                {opt.name}
                                {opt.aliases && opt.aliases.length > 0 && (
                                  <span className="text-xs opacity-50 ml-2">
                                    ({opt.aliases[0]})
                                  </span>
                                )}
                                {opt.object_type && (
                                  <span className="badge badge-xs badge-ghost ml-1">
                                    {opt.object_type}
                                  </span>
                                )}
                              </button>
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                  ) : (
                    <input
                      type="text"
                      className={`input input-bordered w-full ${errors.name ? 'input-error' : ''}`}
                      {...register('name')}
                    />
                  )}
                  {errors.name && (
                    <span className="text-error text-xs mt-1">{errors.name.message}</span>
                  )}
                </div>

                {/* Sync Confirmation Card — shown when user picks from DB dropdown */}
                {pendingSyncEntry && (
                  <div className="rounded-xl bg-base-200/60 border border-primary/20 p-3 flex flex-col gap-2">
                    <div className="text-xs font-semibold text-primary">
                      Apply data from database?
                    </div>
                    <div className="flex gap-3 items-center">
                      {pendingSyncEntry.thumbnail_path && (
                        <div className="w-10 h-10 rounded-lg bg-base-300 overflow-hidden shrink-0">
                          <img
                            src={`asset://${pendingSyncEntry.thumbnail_path}`}
                            alt=""
                            className="w-full h-full object-cover"
                          />
                        </div>
                      )}
                      <div className="flex flex-col gap-0.5 min-w-0">
                        <span className="text-sm font-medium truncate">
                          {pendingSyncEntry.name}
                        </span>
                        <div className="flex gap-1 flex-wrap">
                          {pendingSyncEntry.object_type && (
                            <span className="badge badge-xs badge-primary badge-outline">
                              {pendingSyncEntry.object_type}
                            </span>
                          )}
                          {pendingSyncEntry.metadata &&
                            Object.entries(pendingSyncEntry.metadata).map(([k, v]) => (
                              <span key={k} className="badge badge-xs badge-ghost" title={k}>
                                {String(v)}
                              </span>
                            ))}
                        </div>
                      </div>
                    </div>
                    <div className="flex gap-2 justify-end">
                      <button
                        type="button"
                        className="btn btn-xs btn-ghost"
                        onClick={handleCancelSync}
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        className="btn btn-xs btn-primary"
                        onClick={handleConfirmSync}
                      >
                        Apply
                      </button>
                    </div>
                  </div>
                )}

                {/* Category Dropdown */}
                <div className="form-control w-full">
                  <label className="label py-1">
                    <span className="label-text font-medium">Category</span>
                  </label>
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
                  {errors.object_type && (
                    <span className="text-error text-xs mt-1">{errors.object_type.message}</span>
                  )}
                </div>

                {/* Dynamic Metadata Fields — per-category filters */}
                {categoryFilters.map((filter) => (
                  <div key={filter.key} className="form-control w-full">
                    <label className="label py-1">
                      <span className="label-text">{filter.label}</span>
                    </label>
                    {filter.options && filter.options.length > 0 ? (
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

                {/* Safe Mode */}
                <div className="form-control w-full mt-2">
                  <label className="label cursor-pointer justify-start gap-4 border rounded-lg p-3 hover:bg-base-200 transition-colors">
                    <input
                      type="checkbox"
                      className="toggle toggle-success"
                      {...register('is_safe')}
                    />
                    <div className="flex flex-col">
                      <span className="label-text font-bold">Safe Mode (SFW)</span>
                      <span className="label-text-alt opacity-70">Disable to mark as NSFW</span>
                    </div>
                  </label>
                </div>
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
