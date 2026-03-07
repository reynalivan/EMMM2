/**
 * ScanReviewModal — Bulk review of scan results before committing to DB.
 * Shows a scrollable list of scanned folders with matched objects,
 * confidence badges, and override search from MasterDB entries.
 *
 * Builds on the existing SyncConfirmModal pattern (same MatchedDbEntry type).
 * # Covers: US-2.3 (Review & Organize UI)
 */

import { X, Check, Ban, Search } from 'lucide-react';
import { useState, useMemo, useCallback } from 'react';
import { type ScanPreviewItem, type ConfirmedScanItem } from '../../lib/services/scanService';

import type { GameConfig } from '../../types/game';
import ScanReviewRow from './ScanReviewRow';
import { type MasterDbEntry } from './scanReviewHelpers';

interface ScanReviewModalProps {
  activeGame: GameConfig | null;
  open: boolean;
  items: ScanPreviewItem[];
  masterDbEntries: MasterDbEntry[];
  isCommitting: boolean;
  onConfirm: (items: ConfirmedScanItem[]) => void;
  onClose: () => void;
}

export default function ScanReviewModal({
  activeGame,
  open,
  items,
  masterDbEntries,
  isCommitting,
  onConfirm,
  onClose,
}: ScanReviewModalProps) {
  // Overrides: folder_path -> MasterDbEntry
  const [overrides, setOverrides] = useState<Record<string, MasterDbEntry | null>>({});
  // Skips: folder_path -> boolean
  const [skips, setSkips] = useState<Record<string, boolean>>({});
  const [renames, setRenames] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [activeMainTab, setActiveMainTab] = useState<
    'All' | 'Matched' | 'Unmatched' | 'Existing' | 'Skipped'
  >('All');
  const [activeFilters, setActiveFilters] = useState<Set<string>>(new Set());
  const [globalSearch, setGlobalSearch] = useState('');

  const handleOverride = useCallback((folderPath: string, entry: MasterDbEntry | null) => {
    setOverrides((prev) => ({ ...prev, [folderPath]: entry }));
  }, []);

  const handleToggleSkip = useCallback((folderPath: string) => {
    setSkips((prev) => ({ ...prev, [folderPath]: !prev[folderPath] }));
  }, []);

  const handleToggleSelect = useCallback((folderPath: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(folderPath)) next.delete(folderPath);
      else next.add(folderPath);
      return next;
    });
  }, []);

  const handleToggleSelectAll = useCallback(
    (currentItems: ScanPreviewItem[], isAllSelected: boolean) => {
      setSelected((prev) => {
        const next = new Set(prev);
        if (isAllSelected) {
          currentItems.forEach((i) => next.delete(i.folderPath));
        } else {
          currentItems.forEach((i) => next.add(i.folderPath));
        }
        return next;
      });
    },
    [],
  );

  const { matchedCount, unmatchedCount, alreadyMatchedCount } = useMemo(() => {
    let matched = 0;
    let unmatched = 0;
    let alreadyMatched = 0;
    for (const item of items) {
      if (item.alreadyMatched) {
        alreadyMatched++;
      } else if (overrides[item.folderPath] || item.matchedObject) {
        matched++;
      } else {
        unmatched++;
      }
    }
    return {
      matchedCount: matched,
      unmatchedCount: unmatched,
      alreadyMatchedCount: alreadyMatched,
    };
  }, [items, overrides]);

  const handleConfirm = useCallback(() => {
    const confirmed: ConfirmedScanItem[] = items.map((item) => {
      const ov = overrides[item.folderPath];
      // Only skip if user explicitly skipped — don't auto-skip alreadyMatched
      // (backend handles dedup via ensure_object_exists)
      const isSkipped = !!skips[item.folderPath];

      return {
        folderPath: item.folderPath,
        displayName: renames[item.folderPath] ?? item.displayName,
        isDisabled: item.isDisabled,
        matchedObject: ov ? ov.name : item.matchedObject,
        objectType: ov ? ov.object_type : item.objectType,
        thumbnailPath: ov ? ov.thumbnail_path : item.thumbnailPath,
        tagsJson: ov ? JSON.stringify(ov.tags) : item.tagsJson,
        metadataJson: ov ? (ov.metadata ? JSON.stringify(ov.metadata) : null) : item.metadataJson,
        skip: isSkipped,
      };
    });
    onConfirm(confirmed);
  }, [items, overrides, skips, renames, onConfirm]);

  // Determine item's underlying tab association
  const getItemTab = useCallback(
    (item: ScanPreviewItem) => {
      if (skips[item.folderPath]) return 'Skipped';
      if (item.alreadyMatched) return 'Existing';
      const ov = overrides[item.folderPath];
      if (ov || item.matchedObject) return 'Matched';
      return 'Unmatched';
    },
    [overrides, skips],
  );

  const visibleItems = useMemo(() => {
    return items.filter((item) => {
      // Main tab filter
      if (activeMainTab !== 'All' && getItemTab(item) !== activeMainTab) return false;

      // Confidence chips filter
      if (activeFilters.size > 0 && activeMainTab !== 'Existing') {
        const conf = overrides[item.folderPath] ? 'Manual' : item.confidence;
        // Assume chips might be Excellent, High, Medium, Low, Manual
        if (conf && !activeFilters.has(conf)) return false;
      }

      // Global search filter
      if (globalSearch) {
        const q = globalSearch.toLowerCase();
        const display = (renames[item.folderPath] ?? item.displayName).toLowerCase();
        if (!display.includes(q)) return false;
      }

      return true;
    });
  }, [items, overrides, activeMainTab, activeFilters, globalSearch, renames, getItemTab]);

  const toggleFilter = useCallback((conf: string) => {
    setActiveFilters((prev) => {
      const next = new Set(prev);
      if (next.has(conf)) next.delete(conf);
      else next.add(conf);
      return next;
    });
  }, []);

  const handleDeclineSelected = useCallback(() => {
    setSkips((prev) => {
      const next = { ...prev };
      selected.forEach((folderPath) => {
        next[folderPath] = true;
      });
      return next;
    });
    setSelected(new Set());
  }, [selected]);

  if (!open) return null;

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-[95%] max-w-5xl h-[85vh] flex flex-col p-4 sm:p-6">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={onClose}
          aria-label="Close"
          disabled={isCommitting}
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-xl mb-3">Review {items.length} Scan Results</h3>

        {/* Header Controls: Main Tabs & Delete Bulk Action */}
        <div className="flex items-end justify-between mb-1 mt-2">
          {/* Main Tabs Container */}
          <div className="flex gap-4 border-b border-base-300 px-1 flex-1">
            {['All', 'Matched', 'Unmatched', 'Existing', 'Skipped'].map((tab) => {
              const count =
                tab === 'All' ? items.length : items.filter((i) => getItemTab(i) === tab).length;

              let pillClass = 'bg-base-300/50 text-base-content/60 border-transparent';
              if (tab === 'Matched') pillClass = 'bg-success/10 text-success border-success/20';
              if (tab === 'Unmatched') pillClass = 'bg-error/10 text-error border-error/20';
              if (tab === 'Skipped') pillClass = 'bg-warning/10 text-warning border-warning/20';

              return (
                <button
                  key={tab}
                  className={`pb-3 px-2 text-sm font-medium transition-colors relative flex items-center gap-1.5 ${
                    activeMainTab === tab
                      ? 'text-primary'
                      : 'text-base-content/60 hover:text-base-content/80'
                  }`}
                  onClick={() => {
                    setActiveMainTab(
                      tab as 'All' | 'Matched' | 'Unmatched' | 'Existing' | 'Skipped',
                    );
                    setActiveFilters(new Set()); // Reset subclass filters
                    setSelected(new Set()); // Reset selection
                  }}
                >
                  {tab}
                  <span
                    className={`text-[10px] uppercase font-bold px-1.5 py-0.5 leading-none rounded-full border ${pillClass}`}
                  >
                    {count}
                  </span>
                  {activeMainTab === tab && (
                    <div className="absolute -bottom-px left-0 right-0 h-0.5 bg-primary" />
                  )}
                </button>
              );
            })}
          </div>

          {selected.size > 0 && (
            <button
              className="btn btn-xs h-7 min-h-0 btn-error shadow-sm shadow-error/10 hover:shadow-error/20 btn-outline ml-4 mb-2.5"
              onClick={handleDeclineSelected}
              title="Bulk skip all selected items"
            >
              <Ban size={12} /> Flag as Skip ({selected.size})
            </button>
          )}
        </div>

        {/* Filters & Search Layer */}
        <div className="flex items-center justify-between mt-3 mb-2 min-h-8">
          <div className="flex items-center gap-2">
            {activeMainTab !== 'Existing' && (
              <>
                <span className="text-xs uppercase font-semibold text-base-content/40 tracking-wider mr-2">
                  Filter:
                </span>
                {['Excellent', 'High', 'Medium', 'Low', 'Manual'].map((conf) => {
                  // calculate count for this chip based on activeMainTab
                  const itemsForChip = items.filter((item) => {
                    if (activeMainTab !== 'All' && getItemTab(item) !== activeMainTab) return false;
                    const itemConf = overrides[item.folderPath] ? 'Manual' : item.confidence;
                    return itemConf === conf;
                  });
                  const count = itemsForChip.length;

                  return (
                    <button
                      key={conf}
                      onClick={() => toggleFilter(conf)}
                      className={`badge badge-sm cursor-pointer transition-all gap-1 pl-2 pr-1 h-6 ${
                        activeFilters.has(conf)
                          ? 'badge-primary'
                          : 'badge-outline border-base-300 text-base-content/60 hover:bg-base-200'
                      }`}
                    >
                      {conf}
                      <span
                        className={`text-[9px] rounded-full px-1 py-0.5 leading-none ${
                          activeFilters.has(conf)
                            ? 'bg-primary-content/20 text-primary-content'
                            : 'bg-base-300/50'
                        }`}
                      >
                        {count}
                      </span>
                    </button>
                  );
                })}
              </>
            )}
          </div>

          {/* Search Bar */}
          <div className="relative w-64 ml-auto">
            <Search
              size={14}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40"
            />
            <input
              type="text"
              className="input input-sm w-full pl-9 pr-3 bg-base-200/50 border-base-300 focus:border-primary/50"
              placeholder="Search by folder name..."
              value={globalSearch}
              onChange={(e) => setGlobalSearch(e.target.value)}
            />
          </div>
        </div>

        {/* Main list */}
        <div className="flex-1 mt-2 overflow-y-auto overflow-x-hidden border border-base-300/30 rounded-lg bg-base-200/30 relative">
          <table className="table table-sm table-pin-rows">
            <thead className="text-[10px] uppercase tracking-wider text-base-content/70 z-150 [&_th]:bg-black/40 [&_th]:backdrop-blur-md">
              <tr>
                <th className="w-10 text-center">
                  <input
                    type="checkbox"
                    className="checkbox checkbox-xs rounded border-base-content/40"
                    checked={
                      visibleItems.length > 0 &&
                      visibleItems.every((i) => selected.has(i.folderPath))
                    }
                    onChange={() =>
                      handleToggleSelectAll(
                        visibleItems,
                        visibleItems.length > 0 &&
                          visibleItems.every((i) => selected.has(i.folderPath)),
                      )
                    }
                    title="Select/Deselect all visible items"
                  />
                </th>
                <th className="pl-4">Folder Name</th>
                <th>Target Detected</th>
                <th>Type</th>
                <th className="text-center">Percentage</th>
                <th className="text-center border-l border-white/5">Action</th>
              </tr>
            </thead>
            <tbody>
              {visibleItems.length > 0 && (
                <>
                  <tr className="bg-base-300/30 hover:bg-base-300/30 pointer-events-none hidden" />
                  {visibleItems.map((item) => (
                    <ScanReviewRow
                      key={item.folderPath}
                      item={item}
                      override={overrides[item.folderPath] ?? null}
                      onOverride={(e) => handleOverride(item.folderPath, e)}
                      onToggleSkip={() => handleToggleSkip(item.folderPath)}
                      isSkipped={!!skips[item.folderPath]}
                      isSelected={selected.has(item.folderPath)}
                      onToggleSelect={() => handleToggleSelect(item.folderPath)}
                      masterDbEntries={masterDbEntries}
                      renamedName={renames[item.folderPath] ?? null}
                      onRename={(n) =>
                        setRenames((prev) => {
                          const next = { ...prev };
                          if (n) next[item.folderPath] = n;
                          else delete next[item.folderPath];
                          return next;
                        })
                      }
                      activeGame={activeGame}
                    />
                  ))}
                </>
              )}
              {items.length === 0 && (
                <tr>
                  <td colSpan={6} className="text-center py-8 text-base-content/40 text-sm">
                    No mod folders found.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Actions */}
        <div className="modal-action border-t border-base-200 pt-4 mt-3">
          <button className="btn btn-sm" onClick={onClose} disabled={isCommitting}>
            Cancel
          </button>
          <button
            className="btn btn-sm btn-primary gap-2"
            onClick={handleConfirm}
            disabled={isCommitting || items.length === 0}
          >
            {isCommitting ? (
              <span className="loading loading-spinner loading-xs" />
            ) : (
              <Check size={14} />
            )}
            {isCommitting
              ? 'Committing...'
              : `Confirm ${matchedCount + unmatchedCount + alreadyMatchedCount} Mods`}
          </button>
        </div>
      </div>
      <div className="modal-backdrop" onClick={isCommitting ? undefined : onClose} />
    </div>
  );
}
