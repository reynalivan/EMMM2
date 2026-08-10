/**
 * ScanReviewModal — Bulk review of scan results before committing to DB.
 * Shows a scrollable list of scanned folders with matched objects,
 * confidence badges, and override search from MasterDB entries.
 *
 * Builds on the existing SyncConfirmModal pattern (same MatchedDbEntry type).
 * # Covers: US-2.3 (Review & Organize UI)
 */

import { X, Check } from 'lucide-react';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { type ScanPreviewItem, type ConfirmedScanItem } from '../../../lib/services/scanService';

import type { GameConfig } from '../../../types/game';
import ScanReviewGroup from './ScanReviewGroup';
import ScanReviewFilters, { type ScanReviewTab } from './ScanReviewFilters';
import { groupByConfidence, type MasterDbEntry } from './scanReviewHelpers';

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
  items: initialItems,
  masterDbEntries,
  isCommitting,
  onConfirm,
  onClose,
}: ScanReviewModalProps) {
  const { t } = useTranslation(['objects', 'common']);
  const [items, setItems] = useState(initialItems);
  // Overrides: folder_path -> MasterDbEntry
  const [overrides, setOverrides] = useState<Record<string, MasterDbEntry | null>>({});
  // Skips: folder_path -> boolean
  const [skips, setSkips] = useState<Record<string, boolean>>({});
  const [renames, setRenames] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [activeMainTab, setActiveMainTab] = useState<ScanReviewTab>('All');
  const [activeFilters, setActiveFilters] = useState<Set<string>>(new Set());
  const [globalSearch, setGlobalSearch] = useState('');

  useEffect(() => setItems(initialItems), [initialItems]);

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
        matchedEntryKey: ov ? ov.matched_entry_key : item.matchedEntryKey,
        matchedAliasName: ov ? ov.name : item.matchedAliasName,
        matchedConfidence: ov ? 1 : item.confidenceScore / 100,
        matchedReason: ov ? 'Manual override' : item.matchDetail,
        objectType: ov ? ov.object_type : item.objectType,
        thumbnailPath: ov ? ov.thumbnail_path : item.thumbnailPath,
        tagsJson: ov ? JSON.stringify(ov.tags) : item.tagsJson,
        metadataJson: ov ? (ov.metadata ? JSON.stringify(ov.metadata) : null) : item.metadataJson,
        hashDbJson: item.hashDbJson,
        customSkinsJson: item.customSkinsJson,
        dbThumbnail: item.dbThumbnail,
        skip: isSkipped,
        moveFromTemp: item.moveFromTemp,
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
      if (ov || item.matchedEntryKey) return 'Matched';
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
  const visibleGroups = useMemo(
    () => groupByConfidence(visibleItems, overrides),
    [visibleItems, overrides],
  );
  const includedCount = useMemo(
    () => items.filter((item) => !skips[item.folderPath]).length,
    [items, skips],
  );

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

  const handleSetGroupSkipped = useCallback((groupItems: ScanPreviewItem[], skipped: boolean) => {
    setSkips((previous) => {
      const next = { ...previous };
      groupItems.forEach((item) => {
        next[item.folderPath] = skipped;
      });
      return next;
    });
  }, []);

  const handleRename = useCallback((folderPath: string, newName: string | null) => {
    setRenames((previous) => {
      const next = { ...previous };
      if (newName) next[folderPath] = newName;
      else delete next[folderPath];
      return next;
    });
  }, []);

  const handleItemReplaced = useCallback((oldPath: string, replacement: ScanPreviewItem) => {
    setItems((previous) =>
      previous.map((item) => (item.folderPath === oldPath ? replacement : item)),
    );
    setOverrides((previous) => remapRecordKey(previous, oldPath, replacement.folderPath));
    setSkips((previous) => remapRecordKey(previous, oldPath, replacement.folderPath));
    setRenames((previous) => remapRecordKey(previous, oldPath, replacement.folderPath));
    setSelected((previous) => {
      if (!previous.has(oldPath)) return previous;
      const next = new Set(previous);
      next.delete(oldPath);
      next.add(replacement.folderPath);
      return next;
    });
  }, []);

  if (!open) return null;

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-[95%] max-w-5xl h-[85vh] flex flex-col p-4 sm:p-6">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={onClose}
          aria-label={t('common:close')}
          disabled={isCommitting}
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-xl mb-3">
          {t('objects:scan_review.title', { count: items.length })}
        </h3>

        <ScanReviewFilters
          items={items}
          overrides={overrides}
          selectedCount={selected.size}
          activeTab={activeMainTab}
          activeFilters={activeFilters}
          search={globalSearch}
          itemTab={getItemTab}
          onTabChange={(tab) => {
            setActiveMainTab(tab);
            setActiveFilters(new Set());
            setSelected(new Set());
          }}
          onToggleFilter={toggleFilter}
          onSearchChange={setGlobalSearch}
          onDeclineSelected={handleDeclineSelected}
        />

        {/* Main list */}
        <div className="flex-1 mt-2 overflow-y-auto overflow-x-hidden border border-base-300/30 rounded-lg bg-base-200/30 relative">
          <table className="table table-sm table-pin-rows">
            <thead className="text-[10px] uppercase tracking-wider text-base-content/70 z-150 [&_th]:bg-base-300/40 [&_th]:backdrop-blur-md">
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
                    title={t('objects:scan_review.select_deselect_all_tip')}
                  />
                </th>
                <th className="pl-4">{t('objects:scan_review.headers.folder_name')}</th>
                <th>{t('objects:scan_review.headers.target_detected')}</th>
                <th>{t('objects:scan_review.headers.type')}</th>
                <th className="text-center">{t('objects:scan_review.headers.percentage')}</th>
                <th className="text-center border-l border-base-content/5">
                  {t('objects:scan_review.headers.action')}
                </th>
              </tr>
            </thead>
            <tbody>
              {visibleItems.length > 0 && (
                <>
                  <tr className="bg-base-300/30 hover:bg-base-300/30 pointer-events-none hidden" />
                  {visibleGroups.map((group) => (
                    <ScanReviewGroup
                      key={group.tier}
                      tier={group.tier}
                      items={group.items}
                      overrides={overrides}
                      skips={skips}
                      selected={selected}
                      renames={renames}
                      masterDbEntries={masterDbEntries}
                      activeGame={activeGame}
                      onOverride={handleOverride}
                      onToggleSkip={handleToggleSkip}
                      onToggleSelect={handleToggleSelect}
                      onRename={handleRename}
                      onItemReplaced={handleItemReplaced}
                      onSetGroupSkipped={handleSetGroupSkipped}
                    />
                  ))}
                </>
              )}
              {items.length === 0 && (
                <tr>
                  <td colSpan={6} className="text-center py-8 text-base-content/40 text-sm">
                    {t('objects:scan_review.no_folders')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Actions */}
        <div className="modal-action border-t border-base-200 pt-4 mt-3">
          {includedCount < items.length && (
            <span className="mr-auto text-xs opacity-60">
              {t('objects:scan_review.skipped_summary', { count: items.length - includedCount })}
            </span>
          )}
          <button className="btn btn-sm" onClick={onClose} disabled={isCommitting}>
            {t('common:action.cancel')}
          </button>
          <button
            className="btn btn-sm btn-primary gap-2"
            onClick={handleConfirm}
            disabled={isCommitting || includedCount === 0}
          >
            {isCommitting ? (
              <span className="loading loading-spinner loading-xs" />
            ) : (
              <Check size={14} />
            )}
            {isCommitting
              ? t('objects:scan_review.committing_state')
              : t('objects:scan_review.confirm_button_label', { count: includedCount })}
          </button>
        </div>
      </div>
      <div className="modal-backdrop" onClick={isCommitting ? undefined : onClose} />
    </div>
  );
}

function remapRecordKey<T>(record: Record<string, T>, oldKey: string, newKey: string) {
  if (!(oldKey in record)) return record;
  const next = { ...record, [newKey]: record[oldKey] };
  delete next[oldKey];
  return next;
}
