import { type Ref } from 'react';
import { UseFormReturn } from 'react-hook-form';
import type { EditObjectFormData } from './hooks/useEditObjectForm';
import type { GameSchema, FilterDef } from '../../types/object';
import type { DbEntryFull } from './hooks/useMasterDbSync';
import { Search, ChevronDown, Sparkles, Image as ImageIcon } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';

interface EditObjectTabAutoProps {
  form: UseFormReturn<EditObjectFormData>;
  gameSchema: GameSchema | null | undefined;
  categoryFilters: FilterDef[];
  selectedSyncEntry: DbEntryFull | null;
  isDbOpen: boolean;
  setIsDbOpen: (open: boolean) => void;
  dbSearch: string;
  setDbSearch: (search: string) => void;
  isLoading: boolean;
  dbOptions: DbEntryFull[];
  error: Error | null;
  suggestions: (DbEntryFull & { score: number })[];
  handleDbSelect: (entry: DbEntryFull) => void;
  searchContainerRef: Ref<HTMLDivElement>;
}

export function EditObjectTabAuto({
  form,
  gameSchema,
  categoryFilters,
  selectedSyncEntry,
  isDbOpen,
  setIsDbOpen,
  dbSearch,
  setDbSearch,
  isLoading,
  dbOptions,
  error,
  suggestions,
  handleDbSelect,
  searchContainerRef,
}: EditObjectTabAutoProps) {
  const { watch, setValue } = form;
  const objectType = watch('object_type');

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3">
      {/* Name & Search/Suggestions */}
      <div className="form-control w-full relative">
        <label className="label py-1">
          <span className="label-text font-medium">Name</span>
        </label>

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
              <span className="flex-1 truncate">{dbSearch || 'Click to search database...'}</span>
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
      </div>

      {/* Category Dropdown (Read-only in Sync Mode) */}
      <div className="form-control w-full">
        <label className="label py-1">
          <span className="label-text font-medium">Category</span>
        </label>
        <div className="px-3 py-1 font-semibold text-base-content hover:bg-base-200/50 rounded transition-colors inline-block self-start">
          {selectedSyncEntry
            ? gameSchema?.categories.find((c) => c.name === objectType)?.label ||
              objectType ||
              'None'
            : '-'}
        </div>
      </div>

      {/* Dynamic Metadata Fields */}
      {categoryFilters.map((filter) => (
        <div key={filter.key} className="form-control w-full">
          <label className="label py-1">
            <span className="label-text">{filter.label}</span>
          </label>
          <div className="px-3 py-1 font-semibold text-base-content hover:bg-base-200/50 rounded transition-colors inline-block self-start">
            {selectedSyncEntry ? (watch(`metadata.${filter.key}`) as string) || 'None' : '-'}
          </div>
        </div>
      ))}

      {/* Tags and Custom Skins */}
      {selectedSyncEntry && (
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
            {selectedSyncEntry.custom_skins && selectedSyncEntry.custom_skins.length > 0 ? (
              <div className="flex flex-col gap-2">
                <select
                  className="select select-bordered w-full select-sm"
                  value={watch('has_custom_skin') ? watch('custom_skin.name') || '' : ''}
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
                        src={convertFileSrc(watch('custom_skin.thumbnail_skin_path')!)}
                        className="w-10 h-10 object-cover rounded shadow-sm bg-base-300"
                        alt={watch('custom_skin.name')}
                      />
                    ) : (
                      <div className="w-10 h-10 rounded bg-base-300 flex items-center justify-center shadow-sm">
                        <ImageIcon size={16} className="opacity-30" />
                      </div>
                    )}
                    <div className="flex flex-col">
                      <span className="font-semibold">{watch('custom_skin.name')}</span>
                      {watch('custom_skin.aliases') && watch('custom_skin.aliases')!.length > 0 && (
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
      )}
    </div>
  );
}
