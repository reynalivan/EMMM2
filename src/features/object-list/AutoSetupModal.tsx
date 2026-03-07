/**
 * AutoSetupModal — allows users to batch create objects from the MasterDB.
 * Used in the empty state of the ObjectList.
 */

import { useState, useMemo, useEffect } from 'react';
import { X, CheckSquare, Square, Download } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { useMasterDb, useCreateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { folderKeys } from '../../hooks/useFolders';
import { toast } from '../../stores/useToastStore';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

interface DbEntryFull {
  name: string;
  tags?: string[];
  object_type: string;
  metadata?: Record<string, unknown>;
  thumbnail_path?: string;
  custom_skins?: unknown[];
}

interface AutoSetupModalProps {
  open: boolean;
  onClose: () => void;
}

export default function AutoSetupModal({ open, onClose }: AutoSetupModalProps) {
  const { activeGame } = useActiveGame();
  const { data: dbJson, isLoading: isDbLoading } = useMasterDb();
  const createObject = useCreateObject();
  const queryClient = useQueryClient();

  const [search, setSearch] = useState('');
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());
  const [isCreating, setIsCreating] = useState(false);
  const [progress, setProgress] = useState(0);

  // Parse MasterDB JSON into UI format
  const dbEntries = useMemo<DbEntryFull[]>(() => {
    if (!dbJson) return [];
    try {
      const parsed = JSON.parse(dbJson);
      // Depending on the exact format of the JSON (array vs { entries: array })
      const entries = Array.isArray(parsed) ? parsed : parsed.entries || [];
      return entries as DbEntryFull[];
    } catch (err) {
      console.error('Failed to parse MasterDB JSON:', err);
      return [];
    }
  }, [dbJson]);

  // Filter entries based on search
  const filteredEntries = useMemo(() => {
    if (!search.trim()) return dbEntries;
    const q = search.toLowerCase();
    return dbEntries.filter(
      (entry) =>
        entry.name.toLowerCase().includes(q) ||
        (entry.tags && entry.tags.some((t) => t.toLowerCase().includes(q))),
    );
  }, [dbEntries, search]);

  // Default: check all entries when modal opens or dbEntries changes (if empty)
  useEffect(() => {
    if (open && selectedNames.size === 0 && filteredEntries.length > 0) {
      const allNames = new Set(filteredEntries.map((e) => e.name));
      setSelectedNames(allNames);
    }
    // Only run when open state changes (resetting selection logic could be annoying if they're typing)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, dbEntries.length]);

  if (!open || !activeGame) return null;

  const handleToggleAll = () => {
    if (selectedNames.size === filteredEntries.length) {
      // All currently filtered are selected -> deselect all
      setSelectedNames(new Set());
    } else {
      // Select all filtered
      const allNames = new Set(filteredEntries.map((e) => e.name));
      setSelectedNames(allNames);
    }
  };

  const handleToggleEntry = (name: string) => {
    const next = new Set(selectedNames);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    setSelectedNames(next);
  };

  const handleCreate = async () => {
    if (selectedNames.size === 0) return;

    setIsCreating(true);
    setProgress(0);
    const entriesToCreate = dbEntries.filter((e) => selectedNames.has(e.name));

    let successCount = 0;
    let failCount = 0;

    try {
      await invoke('set_watcher_suppression_cmd', { suppressed: true });

      for (let i = 0; i < entriesToCreate.length; i++) {
        const entry = entriesToCreate[i];
        try {
          await createObject.mutateAsync({
            game_id: activeGame.id,
            name: entry.name,
            object_type: entry.object_type,
            sub_category: null,
            is_safe: true,
            metadata: entry.metadata,
            thumbnail_url: entry.thumbnail_path,
          });
          successCount++;
        } catch (err) {
          console.warn(`Failed to create object ${entry.name}:`, err);
          // Continue loop even on failure (e.g., duplicates)
          failCount++;
        }
        setProgress(i + 1);
      }

      if (failCount > 0) {
        toast.warning(`Created ${successCount} objects. ${failCount} failed (likely duplicates).`);
      } else {
        toast.success(`Successfully created ${successCount} objects.`);
      }

      onClose();
      // Reset state after closing
      setSelectedNames(new Set());
      setSearch('');
    } finally {
      // Delay disabling suppression to allow backend file watcher debounce (500ms) to clear
      setTimeout(async () => {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
      }, 1000);
      setIsCreating(false);
    }
  };

  const handleClose = () => {
    if (isCreating) return;
    setSearch('');
    onClose();
  };

  const isAllSelected = filteredEntries.length > 0 && selectedNames.size === filteredEntries.length;

  return (
    <div className={`modal modal-open`}>
      <div className="modal-box w-11/12 max-w-4xl max-h-[90vh] flex flex-col p-4 bg-base-100 shadow-2xl overflow-hidden relative">
        <button
          className="btn btn-sm btn-circle btn-ghost absolute right-4 top-4"
          onClick={handleClose}
          disabled={isCreating}
          aria-label="Close"
        >
          <X size={20} />
        </button>

        <h3 className="font-bold text-xl mb-2">Auto Setup Folders</h3>
        <p className="text-sm text-base-content/60 mb-4">
          Select objects from the {activeGame.name} database to automatically create folders for
          them.
        </p>

        {isDbLoading ? (
          <div className="flex-1 flex flex-col items-center justify-center py-20">
            <span className="loading loading-spinner text-primary w-10"></span>
            <p className="mt-4 text-base-content/60">Loading database...</p>
          </div>
        ) : (
          <>
            {/* Toolbar */}
            <div className="flex items-center gap-4 mb-4">
              <input
                type="text"
                placeholder="Search objects..."
                className="input input-sm input-bordered w-full max-w-xs focus:border-primary"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                disabled={isCreating}
              />
              <button
                className="btn btn-sm btn-ghost gap-2"
                onClick={handleToggleAll}
                disabled={isCreating || filteredEntries.length === 0}
              >
                {isAllSelected ? <Square size={16} /> : <CheckSquare size={16} />}
                {isAllSelected ? 'Deselect All' : 'Select All'}
              </button>
              <span className="text-sm font-medium text-base-content/70 ml-auto">
                {selectedNames.size} / {filteredEntries.length} selected
              </span>
            </div>

            {/* List Container */}
            <div className="flex-1 overflow-y-auto min-h-0 border border-base-300 rounded-lg bg-base-200/50 p-2">
              {filteredEntries.length === 0 ? (
                <div className="h-full flex items-center justify-center text-base-content/40">
                  No objects found matching "{search}"
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
                  {filteredEntries.map((entry) => {
                    const isSelected = selectedNames.has(entry.name);
                    const thumbUrl = entry.thumbnail_path
                      ? convertFileSrc(entry.thumbnail_path)
                      : undefined;

                    return (
                      <label
                        key={entry.name}
                        className={`flex items-center gap-3 p-2 rounded-lg border transition-all cursor-pointer select-none ${
                          isSelected
                            ? 'border-primary bg-primary/10 shadow-sm'
                            : 'border-base-300 bg-base-100 hover:border-base-content/20'
                        } ${isCreating ? 'opacity-50 pointer-events-none' : ''}`}
                      >
                        <input
                          type="checkbox"
                          className="checkbox checkbox-primary checkbox-sm shrink-0"
                          checked={isSelected}
                          onChange={() => handleToggleEntry(entry.name)}
                          disabled={isCreating}
                        />
                        <div className="avatar">
                          <div className="w-10 h-10 rounded-md bg-base-300 shrink-0 object-cover">
                            {thumbUrl ? (
                              <img src={thumbUrl} alt={entry.name} />
                            ) : (
                              <div className="w-full h-full flex items-center justify-center text-xs opacity-30">
                                NA
                              </div>
                            )}
                          </div>
                        </div>
                        <div className="flex-1 min-w-0 flex flex-col">
                          <span className="font-semibold text-sm truncate" title={entry.name}>
                            {entry.name}
                          </span>
                          <div className="flex flex-wrap gap-1 mt-0.5">
                            <span className="badge badge-outline text-[9px] h-4 leading-4 px-1">
                              {entry.object_type}
                            </span>
                            {!!entry.metadata?.rarity && (
                              <span className="badge badge-ghost text-[9px] h-4 leading-4 px-1 border-warning/30 text-warning">
                                {String(entry.metadata.rarity).replace('-Star', '★')}
                              </span>
                            )}
                          </div>
                        </div>
                      </label>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Footer / Actions */}
            <div className="mt-4 flex flex-col gap-2">
              {isCreating && (
                <div className="w-full bg-base-300 rounded-full h-2.5 overflow-hidden">
                  <div
                    className="bg-primary h-2.5 rounded-full transition-all duration-300 ease-out"
                    style={{ width: `${(progress / Math.max(1, selectedNames.size)) * 100}%` }}
                  ></div>
                </div>
              )}
              <div className="flex justify-end gap-2">
                <button className="btn" onClick={handleClose} disabled={isCreating}>
                  Cancel
                </button>
                <button
                  className="btn btn-primary gap-2 min-w-35"
                  onClick={handleCreate}
                  disabled={selectedNames.size === 0 || isCreating}
                >
                  {isCreating ? (
                    <>
                      <span className="loading loading-spinner loading-sm"></span>
                      {progress} / {selectedNames.size}
                    </>
                  ) : (
                    <>
                      <Download size={16} />
                      Add Selected
                    </>
                  )}
                </button>
              </div>
            </div>
          </>
        )}
      </div>
      <div className="modal-backdrop bg-base-300/60" onClick={handleClose}></div>
    </div>
  );
}
