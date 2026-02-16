/**
 * ScanReviewModal — Bulk review of scan results before committing to DB.
 * Shows a scrollable list of scanned folders with matched objects,
 * confidence badges, and override search from MasterDB entries.
 *
 * Builds on the existing SyncConfirmModal pattern (same MatchedDbEntry type).
 * # Covers: US-2.3 (Review & Organize UI)
 */

import { X, Check, Search, SkipForward, ChevronDown } from 'lucide-react';
import { useState, useMemo, useCallback } from 'react';
import type { ScanPreviewItem, ConfirmedScanItem } from '../../services/scanService';

/** MasterDB entry for the override search dropdown. */
export interface MasterDbEntry {
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
}

interface ScanReviewModalProps {
  open: boolean;
  items: ScanPreviewItem[];
  masterDbEntries: MasterDbEntry[];
  isCommitting: boolean;
  onConfirm: (items: ConfirmedScanItem[]) => void;
  onClose: () => void;
}

/** Confidence badge color mapping. */
function confidenceBadge(confidence: string) {
  switch (confidence) {
    case 'High':
      return 'badge-success';
    case 'Medium':
      return 'badge-warning';
    case 'Low':
      return 'badge-error';
    default:
      return 'badge-ghost';
  }
}

/** Single row in the review table. */
function ReviewRow({
  item,
  override,
  onOverride,
  onToggleSkip,
  isSkipped,
  masterDbEntries,
}: {
  item: ScanPreviewItem;
  override: MasterDbEntry | null;
  onOverride: (entry: MasterDbEntry | null) => void;
  onToggleSkip: () => void;
  isSkipped: boolean;
  masterDbEntries: MasterDbEntry[];
}) {
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  const displayMatch = override?.name ?? item.matchedObject;
  const displayType = override?.object_type ?? item.objectType;
  const confidence = override ? 'Manual' : item.confidence;

  const filteredEntries = useMemo(() => {
    if (!searchQuery.trim()) return masterDbEntries.slice(0, 50);
    const q = searchQuery.toLowerCase();
    return masterDbEntries
      .filter(
        (e) =>
          e.name.toLowerCase().includes(q) ||
          e.object_type.toLowerCase().includes(q) ||
          e.tags.some((t) => t.toLowerCase().includes(q)),
      )
      .slice(0, 50);
  }, [searchQuery, masterDbEntries]);

  return (
    <div
      className={`flex items-center gap-3 px-3 py-2 border-b border-base-300/20 transition-all duration-150 ${
        isSkipped ? 'opacity-40' : ''
      } ${item.alreadyMatched ? 'bg-base-200/20' : ''}`}
    >
      {/* Folder name */}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-base-content truncate" title={item.folderPath}>
          {item.displayName}
        </p>
        {item.matchDetail && (
          <p className="text-[10px] text-base-content/40 truncate">{item.matchDetail}</p>
        )}
      </div>

      {/* Match result / override */}
      <div className="relative flex items-center gap-2 min-w-0 shrink-0">
        <button
          className="btn btn-xs btn-ghost gap-1 max-w-48 truncate"
          onClick={() => setSearchOpen(!searchOpen)}
          disabled={isSkipped}
          title={displayMatch ?? 'No match — click to assign'}
        >
          <span className="truncate">
            {displayMatch ?? <span className="text-base-content/30 italic">Unmatched</span>}
          </span>
          <ChevronDown size={12} className="shrink-0" />
        </button>

        {displayType && (
          <span className="badge badge-xs badge-primary badge-outline shrink-0">{displayType}</span>
        )}
        {confidence !== 'None' && (
          <span
            className={`badge badge-xs shrink-0 ${
              confidence === 'Manual' ? 'badge-info' : confidenceBadge(confidence)
            }`}
          >
            {confidence === 'Manual'
              ? 'Manual'
              : `${item.matchLevel.replace('L', '').replace('-', ' ')} ${confidence}`}
          </span>
        )}

        {/* Search dropdown */}
        {searchOpen && (
          <div className="absolute top-full right-0 z-50 mt-1 w-72 bg-base-200 rounded-lg shadow-xl border border-base-300/50 overflow-hidden">
            <div className="p-2 border-b border-base-300/30">
              <div className="relative">
                <Search
                  size={12}
                  className="absolute left-2 top-1/2 -translate-y-1/2 text-base-content/30"
                />
                <input
                  type="text"
                  className="input input-xs w-full pl-7 bg-base-100/60 border-base-300/30"
                  placeholder="Search characters, weapons..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  autoFocus
                />
              </div>
            </div>
            <ul className="max-h-48 overflow-y-auto">
              {/* Clear override option */}
              {override && (
                <li>
                  <button
                    className="w-full text-left px-3 py-1.5 text-xs hover:bg-base-300/30 text-error/70"
                    onClick={() => {
                      onOverride(null);
                      setSearchOpen(false);
                      setSearchQuery('');
                    }}
                  >
                    ✕ Clear override (revert to auto-match)
                  </button>
                </li>
              )}
              {filteredEntries.map((entry) => (
                <li key={entry.name}>
                  <button
                    className="w-full text-left px-3 py-1.5 text-xs hover:bg-base-300/30 flex items-center gap-2"
                    onClick={() => {
                      onOverride(entry);
                      setSearchOpen(false);
                      setSearchQuery('');
                    }}
                  >
                    <span className="font-medium truncate flex-1">{entry.name}</span>
                    <span className="badge badge-xs badge-outline shrink-0">
                      {entry.object_type}
                    </span>
                  </button>
                </li>
              ))}
              {filteredEntries.length === 0 && (
                <li className="px-3 py-2 text-xs text-base-content/40">No results</li>
              )}
            </ul>
          </div>
        )}
      </div>

      {/* Skip toggle */}
      <button
        className={`btn btn-xs btn-square ${isSkipped ? 'btn-warning' : 'btn-ghost text-base-content/30 hover:text-warning'}`}
        onClick={onToggleSkip}
        title={isSkipped ? 'Include this mod' : 'Skip this mod'}
      >
        <SkipForward size={12} />
      </button>
    </div>
  );
}

export default function ScanReviewModal({
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

  const handleOverride = useCallback((folderPath: string, entry: MasterDbEntry | null) => {
    setOverrides((prev) => ({ ...prev, [folderPath]: entry }));
  }, []);

  const handleToggleSkip = useCallback((folderPath: string) => {
    setSkips((prev) => ({ ...prev, [folderPath]: !prev[folderPath] }));
  }, []);

  const { matchedCount, unmatchedCount, skippedCount, alreadyMatchedCount } = useMemo(() => {
    let matched = 0;
    let unmatched = 0;
    let skipped = 0;
    let alreadyMatched = 0;
    for (const item of items) {
      if (skips[item.folderPath]) {
        skipped++;
      } else if (item.alreadyMatched) {
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
      skippedCount: skipped,
      alreadyMatchedCount: alreadyMatched,
    };
  }, [items, overrides, skips]);

  const handleConfirm = useCallback(() => {
    const confirmed: ConfirmedScanItem[] = items.map((item) => {
      const ov = overrides[item.folderPath];
      // Only skip if user explicitly skipped — don't auto-skip alreadyMatched
      // (backend handles dedup via ensure_object_exists)
      const isSkipped = !!skips[item.folderPath];

      return {
        folderPath: item.folderPath,
        displayName: item.displayName,
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
  }, [items, overrides, skips, onConfirm]);

  if (!open) return null;

  // Split into new/unmatched items (actionable) and already-matched (info only)
  const newItems = items.filter((i) => !i.alreadyMatched);
  const existingItems = items.filter((i) => i.alreadyMatched);

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-11/12 max-w-3xl max-h-[85vh] flex flex-col">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={onClose}
          aria-label="Close"
          disabled={isCommitting}
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-lg mb-1">Review Scan Results</h3>
        <p className="text-xs text-base-content/50 mb-3">
          {items.length} folders scanned •{' '}
          <span className="text-success">{matchedCount} matched</span> •{' '}
          <span className="text-warning">{unmatchedCount} unmatched</span> •{' '}
          <span className="text-base-content/30">{alreadyMatchedCount} existing</span>
          {skippedCount > 0 && (
            <>
              {' '}
              • <span className="text-info">{skippedCount} skipped</span>
            </>
          )}
        </p>

        {/* Main list */}
        <div className="flex-1 overflow-y-auto border border-base-300/30 rounded-lg bg-base-200/30">
          {newItems.length > 0 && (
            <>
              <div className="sticky top-0 z-10 px-3 py-1 bg-base-300/50 backdrop-blur-sm text-[10px] uppercase tracking-wider text-base-content/40 font-medium">
                New & Unmatched ({newItems.length})
              </div>
              {newItems.map((item) => (
                <ReviewRow
                  key={item.folderPath}
                  item={item}
                  override={overrides[item.folderPath] ?? null}
                  onOverride={(e) => handleOverride(item.folderPath, e)}
                  onToggleSkip={() => handleToggleSkip(item.folderPath)}
                  isSkipped={!!skips[item.folderPath]}
                  masterDbEntries={masterDbEntries}
                />
              ))}
            </>
          )}
          {existingItems.length > 0 && (
            <>
              <div className="sticky top-0 z-10 px-3 py-1 bg-base-300/50 backdrop-blur-sm text-[10px] uppercase tracking-wider text-base-content/40 font-medium">
                Already Matched ({existingItems.length})
              </div>
              {existingItems.map((item) => (
                <ReviewRow
                  key={item.folderPath}
                  item={item}
                  override={overrides[item.folderPath] ?? null}
                  onOverride={(e) => handleOverride(item.folderPath, e)}
                  onToggleSkip={() => handleToggleSkip(item.folderPath)}
                  isSkipped={!!skips[item.folderPath] || item.alreadyMatched}
                  masterDbEntries={masterDbEntries}
                />
              ))}
            </>
          )}
          {items.length === 0 && (
            <div className="flex items-center justify-center p-8 text-base-content/40 text-sm">
              No mod folders found.
            </div>
          )}
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
            {isCommitting ? 'Committing...' : `Confirm ${matchedCount + unmatchedCount} Mods`}
          </button>
        </div>
      </div>
      <div className="modal-backdrop" onClick={isCommitting ? undefined : onClose} />
    </div>
  );
}
