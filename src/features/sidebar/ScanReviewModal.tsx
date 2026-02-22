/**
 * ScanReviewModal — Bulk review of scan results before committing to DB.
 * Shows a scrollable list of scanned folders with matched objects,
 * confidence badges, and override search from MasterDB entries.
 *
 * Builds on the existing SyncConfirmModal pattern (same MatchedDbEntry type).
 * # Covers: US-2.3 (Review & Organize UI)
 */

import {
  X,
  Check,
  Search,
  SkipForward,
  ChevronDown,
  AlertTriangle,
  Info,
  CheckCircle,
  Ban,
} from 'lucide-react';
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

/** Confidence color and icon mapping. */
function getConfidenceColor(confidence: string) {
  switch (confidence) {
    case 'Excellent':
      return 'text-success border-success/30 bg-success/5';
    case 'High':
      return 'text-info border-info/30 bg-info/5';
    case 'Medium':
      return 'text-warning border-warning/30 bg-warning/5';
    case 'Low':
      return 'text-error border-error/30 bg-error/5';
    default:
      return 'text-base-content/50 border-base-content/20';
  }
}

function getConfidenceIcon(confidence: string) {
  switch (confidence) {
    case 'Excellent':
    case 'High':
      return <CheckCircle size={10} />;
    case 'Medium':
      return <Info size={10} />;
    case 'Low':
      return <AlertTriangle size={10} />;
    default:
      return null;
  }
}

/** Map staged match level to user-friendly label. */
function matchLevelLabel(level: string): string {
  switch (level) {
    case 'AutoMatched':
      return 'Auto-match';
    case 'NeedsReview':
      return 'Review';
    case 'NoMatch':
      return 'No Match';
    default:
      return level;
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
        isSkipped ? 'opacity-40 bg-base-300/10' : ''
      } ${item.alreadyMatched ? 'bg-base-200/20' : ''}`}
    >
      {/* Folder name */}
      <div className="flex-1 min-w-0 pr-2">
        <p className="text-sm font-medium text-base-content truncate" title={item.folderPath}>
          {item.displayName}
          {item.alreadyMatched && (
            <span className="badge badge-xs badge-ghost ml-2 opacity-60">Existing</span>
          )}
        </p>
        {item.matchDetail && (
          <p className="text-[11px] text-base-content/60 truncate mt-0.5" title={item.matchDetail}>
            {item.matchDetail}
          </p>
        )}
      </div>

      {/* Percentage / Confidence Badge */}
      {!override && confidence !== 'None' && (
        <div
          className={`badge badge-sm badge-outline gap-1 shrink-0 ${getConfidenceColor(
            confidence,
          )}`}
          title={`${confidence} Confidence - ${matchLevelLabel(item.matchLevel)}`}
        >
          {getConfidenceIcon(confidence)}
          <span className="font-medium">{item.confidenceScore}%</span>
        </div>
      )}

      {/* Match result / override dropdown */}
      <div className="relative flex items-center gap-2 min-w-0 shrink-0 ml-2">
        <button
          className={`btn btn-xs gap-1 max-w-48 truncate ${
            override ? 'btn-info btn-outline' : 'btn-ghost bg-base-200/50'
          }`}
          onClick={() => setSearchOpen(!searchOpen)}
          disabled={isSkipped}
          title={displayMatch ?? 'No match — click to assign'}
        >
          <Search size={10} className="opacity-50" />
          <span className="truncate hidden sm:inline">
            {displayMatch ?? <span className="text-base-content/30 italic">Unmatched</span>}
          </span>
          <ChevronDown size={12} className="shrink-0 opacity-50" />
        </button>

        {displayType && (
          <span className="badge badge-xs bg-base-300/50 border-base-300/60 shrink-0 text-base-content/70">
            {displayType}
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
  const [activeTab, setActiveTab] = useState<string>('All');

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

  const newItems = items.filter((i) => !i.alreadyMatched);
  const existingItems = items.filter((i) => i.alreadyMatched);

  const tabCounts = useMemo(() => {
    const counts: Record<string, number> = {
      All: newItems.length,
      Excellent: 0,
      High: 0,
      Medium: 0,
      Low: 0,
    };
    for (const item of newItems) {
      if (item.confidence && counts[item.confidence] !== undefined) {
        counts[item.confidence]++;
      }
    }
    return counts;
  }, [newItems]);

  const visibleNewItems = useMemo(() => {
    if (activeTab === 'All') return newItems;
    return newItems.filter((item) => item.confidence === activeTab);
  }, [newItems, activeTab]);

  const handleDeclineVisible = useCallback(() => {
    setSkips((prev) => {
      const next = { ...prev };
      visibleNewItems.forEach((item) => {
        next[item.folderPath] = true;
      });
      return next;
    });
  }, [visibleNewItems]);

  if (!open) return null;

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

        {/* Tabs & Bulk Action */}
        <div className="flex items-center justify-between px-4 pb-0 mb-3 border-base-300/30">
          <div className="tabs tabs-boxed bg-base-300/30 p-1 gap-1">
            {['All', 'Excellent', 'High', 'Medium', 'Low'].map((tab) => (
              <button
                key={tab}
                className={`tab tab-sm h-7 transition-all ${
                  activeTab === tab
                    ? 'tab-active bg-base-100 shadow-sm font-medium'
                    : 'text-base-content/60 hover:text-base-content'
                }`}
                onClick={() => setActiveTab(tab)}
              >
                {tab}
                <span className="badge badge-xs bg-base-200 border-base-300 ml-1.5 opacity-80">
                  {tabCounts[tab]}
                </span>
              </button>
            ))}
          </div>

          <button
            className="btn btn-xs btn-outline btn-error gap-1 opacity-80 hover:opacity-100"
            onClick={handleDeclineVisible}
            disabled={visibleNewItems.length === 0}
            title="Skip all currently visible items in this tab"
          >
            <Ban size={12} /> Flag as Decline ({visibleNewItems.length})
          </button>
        </div>

        {/* Main list */}
        <div className="flex-1 overflow-y-auto border border-base-300/30 rounded-lg bg-base-200/30">
          {visibleNewItems.length > 0 && (
            <>
              <div className="sticky top-0 z-10 px-3 py-1 bg-base-300/50 backdrop-blur-sm text-[10px] uppercase tracking-wider text-base-content/40 font-medium border-b border-base-300/20">
                New & Unmatched ({visibleNewItems.length})
              </div>
              {visibleNewItems.map((item) => (
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
