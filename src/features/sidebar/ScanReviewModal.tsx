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
  Pencil,
  ExternalLink,
  FolderOpen,
} from 'lucide-react';
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import FolderTooltip from './FolderTooltip';
import {
  scanService,
  type ScanPreviewItem,
  type ConfirmedScanItem,
} from '../../services/scanService';

import type { GameConfig } from '../../types/game';

/** MasterDB entry for the override search dropdown. */
export interface MasterDbEntry {
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
}

interface ScanReviewModalProps {
  activeGame: GameConfig | null;
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
  isSelected,
  onToggleSelect,
  masterDbEntries,
  renamedName,
  onRename,
  activeGame,
}: {
  item: ScanPreviewItem;
  override: MasterDbEntry | null;
  onOverride: (entry: MasterDbEntry | null) => void;
  onToggleSkip: () => void;
  isSkipped: boolean;
  isSelected: boolean;
  onToggleSelect: () => void;
  masterDbEntries: MasterDbEntry[];
  renamedName: string | null;
  onRename: (newName: string | null) => void;
  activeGame: GameConfig | null;
}) {
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [visibleCount, setVisibleCount] = useState(30);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const [isRenaming, setIsRenaming] = useState(false);
  const [editName, setEditName] = useState('');
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const dropdownRef = useRef<HTMLTableDataCellElement | null>(null);
  const [dropdownPosition, setDropdownPosition] = useState<'top' | 'bottom'>('bottom');
  const [dynamicScores, setDynamicScores] = useState<Record<string, number>>({});
  const hasFetchedScoresRef = useRef(false);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setSearchOpen(false);
      }
    }
    if (searchOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [searchOpen]);

  useEffect(() => {
    if (!searchOpen || !activeGame || hasFetchedScoresRef.current) return;

    let isMounted = true;
    hasFetchedScoresRef.current = true;

    const fetchDynamicScores = async () => {
      // Find entries that are not already scored
      const candidatesToScore = masterDbEntries
        .filter((e) => !item.scoredCandidates.some((sc) => sc.name === e.name))
        .map((e) => e.name);

      if (candidatesToScore.length === 0) return;

      const chunkSize = 50;
      for (let i = 0; i < candidatesToScore.length; i += chunkSize) {
        if (!isMounted || !searchOpen) break;

        const chunk = candidatesToScore.slice(i, i + chunkSize);
        try {
          const scores = await scanService.scoreCandidatesBatch(
            item.folderPath,
            chunk,
            activeGame.game_type,
          );

          if (isMounted) {
            setDynamicScores((prev) => ({ ...prev, ...scores }));
          }
        } catch (error) {
          console.error('Failed to fetch score chunk:', error);
        }
      }
    };

    fetchDynamicScores();

    return () => {
      isMounted = false;
    };
  }, [searchOpen, activeGame, item.folderPath, item.scoredCandidates, masterDbEntries]);

  const displayFolderName = renamedName ?? item.displayName;

  const startRename = useCallback(() => {
    setEditName(displayFolderName);
    setIsRenaming(true);
    setTimeout(() => renameInputRef.current?.select(), 0);
  }, [displayFolderName]);

  const commitRename = useCallback(() => {
    const trimmed = editName.trim();
    if (trimmed && trimmed !== item.displayName) {
      onRename(trimmed);
    } else {
      onRename(null);
    }
    setIsRenaming(false);
  }, [editName, item.displayName, onRename]);

  const cancelRename = useCallback(() => {
    setIsRenaming(false);
  }, []);

  const handleSearchChange = useCallback((val: string) => {
    setSearchQuery(val);
    setVisibleCount(30);
  }, []);

  const handleToggleDropdown = useCallback(() => {
    setSearchOpen((prev) => {
      if (!prev) {
        setVisibleCount(30);
        if (dropdownRef.current) {
          const rect = dropdownRef.current.getBoundingClientRect();
          if (window.innerHeight - rect.bottom < 320) {
            setDropdownPosition('top');
          } else {
            setDropdownPosition('bottom');
          }
        }
      }
      return !prev;
    });
  }, []);

  // IntersectionObserver for lazy scroll loading
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !searchOpen) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          setVisibleCount((prev) => prev + 30);
        }
      },
      { threshold: 0.1 },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [searchOpen, searchQuery]);

  const displayMatch = override?.name ?? item.matchedObject;
  const displayType = override?.object_type ?? item.objectType;
  const confidence = override ? 'Manual' : item.confidence;

  // Build a score map from item.scoredCandidates for O(1) lookup
  const scoreMap = useMemo(() => {
    const map = new Map<string, number>();
    for (const sc of item.scoredCandidates) {
      map.set(sc.name, sc.scorePct);
    }
    return map;
  }, [item.scoredCandidates]);

  // Merge scored candidates with masterDB entries, split into groups
  const { candidateEntries, otherEntries } = useMemo(() => {
    type MergedEntry = MasterDbEntry & { scorePct?: number };
    const q = searchQuery.trim().toLowerCase();

    // Attach score to all entries
    let entries: MergedEntry[] = masterDbEntries.map((e) => {
      const mapScore = scoreMap.get(e.name);
      return {
        ...e,
        scorePct: mapScore !== undefined ? mapScore : dynamicScores[e.name],
      };
    });

    // Filter by search query
    if (q) {
      entries = entries.filter(
        (e) =>
          e.name.toLowerCase().includes(q) ||
          e.object_type.toLowerCase().includes(q) ||
          e.tags.some((t) => t.toLowerCase().includes(q)),
      );
    }

    // Split into scored candidates (scoreMap > 0 OR dynamically scored >= 50) vs others
    const candidates = entries.filter(
      (e) =>
        (scoreMap.has(e.name) && e.scorePct !== undefined && e.scorePct > 0) ||
        (!scoreMap.has(e.name) && e.scorePct !== undefined && e.scorePct >= 50),
    );
    const others = entries.filter(
      (e) =>
        !(
          (scoreMap.has(e.name) && e.scorePct !== undefined && e.scorePct > 0) ||
          (!scoreMap.has(e.name) && e.scorePct !== undefined && e.scorePct >= 50)
        ),
    );

    // Sort candidates by score desc, others by score desc then alphabetically
    candidates.sort((a, b) => b.scorePct! - a.scorePct! || a.name.localeCompare(b.name));
    others.sort((a, b) => {
      const scoreA = a.scorePct ?? -1;
      const scoreB = b.scorePct ?? -1;
      return scoreB - scoreA || a.name.localeCompare(b.name);
    });

    return { candidateEntries: candidates, otherEntries: others };
  }, [searchQuery, masterDbEntries, scoreMap, dynamicScores]);

  const displayThumb = override?.thumbnail_path ?? item.thumbnailPath;

  const contextMenuContent = (
    <>
      <ContextMenuItem
        icon={ExternalLink}
        onClick={() => invoke('open_in_explorer', { path: item.folderPath }).catch(console.error)}
      >
        Reveal Source Folder
      </ContextMenuItem>
      <ContextMenuItem
        icon={FolderOpen}
        disabled={(!item.matchedObject && !override) || !activeGame}
        onClick={() => {
          const objName = override?.name ?? item.matchedObject;
          // Find the object ID from masterDbEntries based on its name
          const entry = masterDbEntries.find((e) => e.name === objName);

          if (
            objName &&
            activeGame &&
            entry &&
            entry.metadata &&
            typeof entry.metadata.id === 'string'
          ) {
            invoke('reveal_object_in_explorer', {
              objectId: entry.metadata.id,
              modsPath: activeGame.mod_path,
              objectName: objName,
            }).catch(console.error);
          } else if (objName && activeGame) {
            // fallback
            invoke('reveal_object_in_explorer', {
              objectId: objName,
              modsPath: activeGame.mod_path,
              objectName: objName,
            }).catch(console.error);
          }
        }}
      >
        Reveal Destination Folder
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pencil} onClick={startRename}>
        Rename Folder
      </ContextMenuItem>
    </>
  );

  return (
    <ContextMenu content={contextMenuContent}>
      <tr
        className={`group transition-all duration-150 ${
          isSkipped ? 'opacity-40 bg-base-300/10' : ''
        } ${item.alreadyMatched ? 'bg-base-200/20' : ''}`}
      >
        <td className="w-10 text-center">
          <input
            type="checkbox"
            className={`checkbox checkbox-sm checkbox-primary rounded transition-opacity duration-200 ${
              isSelected ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            checked={isSelected}
            onChange={onToggleSelect}
            disabled={isSkipped && !isSelected}
          />
        </td>
        <td className="max-w-xs truncate">
          <FolderTooltip folderPath={item.folderPath} thumbnailPath={item.thumbnailPath}>
            <div className="flex flex-col">
              {isRenaming ? (
                <div className="flex items-center gap-1">
                  <input
                    ref={renameInputRef}
                    className="input input-xs input-bordered w-full text-sm font-medium"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') commitRename();
                      if (e.key === 'Escape') cancelRename();
                    }}
                    onBlur={commitRename}
                    autoFocus
                  />
                  <button
                    type="button"
                    className="btn btn-xs btn-ghost btn-square text-success hover:bg-success/20"
                    onMouseDown={(e) => {
                      e.preventDefault();
                      commitRename();
                    }}
                    title="Confirm Rename"
                  >
                    <Check size={14} />
                  </button>
                  <button
                    type="button"
                    className="btn btn-xs btn-ghost btn-square text-error hover:bg-error/20"
                    onMouseDown={(e) => {
                      e.preventDefault();
                      cancelRename();
                    }}
                    title="Cancel Rename"
                  >
                    <X size={14} />
                  </button>
                </div>
              ) : (
                <span
                  className="font-medium text-sm text-base-content truncate cursor-default"
                  onDoubleClick={startRename}
                >
                  {displayFolderName}
                  {renamedName && <Pencil size={10} className="inline ml-1.5 text-info/60" />}
                  {item.alreadyMatched && (
                    <span className="badge badge-xs badge-ghost ml-2 opacity-60">Existing</span>
                  )}
                </span>
              )}
              {item.matchDetail && (
                <span className="text-[10px] text-base-content/50 truncate mt-0.5">
                  {item.matchDetail}
                </span>
              )}
            </div>
          </FolderTooltip>
        </td>

        <td ref={dropdownRef} className="relative group/search max-w-64 w-64">
          <button
            className={`btn btn-sm h-8 min-h-8 w-full justify-start pl-2 pr-2 gap-2 flex-nowrap ${
              override ? 'btn-info btn-outline' : 'btn-ghost bg-base-200/50 hover:bg-base-300/60'
            }`}
            onClick={handleToggleDropdown}
            disabled={isSkipped}
            title={displayMatch ?? 'No match — click to assign'}
          >
            {displayThumb ? (
              <div className="avatar">
                <div className="w-5 rounded-full border border-base-300/50">
                  <img src={convertFileSrc(displayThumb)} alt={displayMatch || ''} />
                </div>
              </div>
            ) : (
              <Search size={14} className="opacity-50 shrink-0" />
            )}
            <span className="truncate flex-1 text-left font-medium text-sm">
              {displayMatch ?? (
                <span className="text-base-content/30 italic font-normal">Unmatched</span>
              )}
            </span>
            <ChevronDown size={14} className="opacity-50 shrink-0" />
          </button>

          {searchOpen && (
            <div
              className={`absolute left-0 z-100 w-80 bg-base-200/95 backdrop-blur-md rounded-lg shadow-xl border border-base-300/50 overflow-hidden ${
                dropdownPosition === 'top' ? 'bottom-full mb-1' : 'top-full mt-1'
              }`}
            >
              <div className="p-2 border-b border-base-300/30 bg-base-300/30">
                <div className="relative">
                  <Search
                    size={14}
                    className="absolute left-2.5 top-1/2 -translate-y-1/2 text-base-content/40"
                  />
                  <input
                    type="text"
                    className="input input-sm w-full pl-8 bg-base-100/60 border-base-300/30 placeholder:text-base-content/30"
                    placeholder="Search characters, weapons..."
                    value={searchQuery}
                    onChange={(e) => handleSearchChange(e.target.value)}
                    autoFocus
                  />
                </div>
              </div>
              <div className="max-h-72 overflow-y-auto w-full flex flex-col">
                {override && (
                  <button
                    className="flex items-center gap-1.5 px-3 py-2 text-error/80 hover:bg-error/10 text-sm transition-colors"
                    onClick={() => {
                      onOverride(null);
                      setSearchOpen(false);
                      setSearchQuery('');
                    }}
                  >
                    <X size={14} /> Clear override
                  </button>
                )}

                {/* ── Candidates Group ── */}
                {candidateEntries.length > 0 && (
                  <>
                    <div className="px-3 py-1.5 text-[10px] uppercase tracking-widest text-base-content/40 font-semibold bg-base-300/20 border-b border-base-300/20 sticky top-0 z-10 backdrop-blur-sm">
                      Candidates ({candidateEntries.length})
                    </div>
                    {candidateEntries.slice(0, visibleCount).map((entry) => (
                      <button
                        key={entry.name}
                        className="flex flex-col gap-0.5 px-3 py-2 hover:bg-base-300/30 transition-colors text-left w-full border-b border-base-300/10 last:border-b-0"
                        onClick={() => {
                          onOverride(entry);
                          setSearchOpen(false);
                          setSearchQuery('');
                        }}
                      >
                        <div className="flex items-center gap-2 w-full">
                          {entry.thumbnail_path ? (
                            <div className="avatar">
                              <div className="w-6 rounded-full bg-base-300 ring-1 ring-base-300/50">
                                <img src={convertFileSrc(entry.thumbnail_path)} alt="" />
                              </div>
                            </div>
                          ) : (
                            <div className="w-6 h-6 rounded-full bg-base-300/50 flex items-center justify-center">
                              <Search size={10} className="opacity-30" />
                            </div>
                          )}
                          <span className="truncate font-semibold text-sm flex-1">
                            {entry.name}
                          </span>
                          <span
                            className={`badge badge-xs font-mono tabular-nums ${getConfidenceColor(
                              entry.scorePct! >= 90
                                ? 'Excellent'
                                : entry.scorePct! >= 75
                                  ? 'High'
                                  : entry.scorePct! >= 45
                                    ? 'Medium'
                                    : 'Low',
                            )}`}
                          >
                            {entry.scorePct}%
                          </span>
                        </div>
                        <div className="flex items-center gap-1 ml-8 flex-wrap">
                          <span className="badge badge-xs bg-base-300/50 border-base-300/60 text-base-content/60 uppercase text-[9px]">
                            {entry.object_type}
                          </span>
                          {entry.tags.slice(0, 3).map((tag) => (
                            <span
                              key={tag}
                              className="badge badge-xs badge-ghost text-[9px] text-base-content/40"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      </button>
                    ))}
                  </>
                )}

                {/* ── Other Group ── */}
                {otherEntries.length > 0 && (
                  <>
                    <div className="px-3 py-1.5 text-[10px] uppercase tracking-widest text-base-content/40 font-semibold bg-base-300/20 border-b border-base-300/20 sticky top-0 z-10 backdrop-blur-sm">
                      Other ({otherEntries.length})
                    </div>
                    {otherEntries
                      .slice(0, Math.max(0, visibleCount - candidateEntries.length))
                      .map((entry) => (
                        <button
                          key={entry.name}
                          className="flex items-center gap-2 px-3 py-1.5 hover:bg-base-300/30 transition-colors text-left w-full"
                          onClick={() => {
                            onOverride(entry);
                            setSearchOpen(false);
                            setSearchQuery('');
                          }}
                        >
                          {entry.thumbnail_path ? (
                            <div className="avatar">
                              <div className="w-5 rounded-full bg-base-300">
                                <img src={convertFileSrc(entry.thumbnail_path)} alt="" />
                              </div>
                            </div>
                          ) : (
                            <div className="w-5 h-5 rounded-full bg-base-300/40" />
                          )}
                          <span className="truncate font-medium text-sm flex-1 text-base-content/70">
                            {entry.name}
                          </span>

                          {/* Lazy-loaded score display */}
                          {entry.scorePct !== undefined ? (
                            <span className="badge badge-xs bg-transparent border-0 font-mono text-[10px] text-base-content/40 tabular-nums">
                              {entry.scorePct}%
                            </span>
                          ) : (
                            <div className="w-6 h-3 rounded bg-base-300/40 animate-pulse ml-2"></div>
                          )}

                          <span className="badge badge-xs bg-base-300/40 border-base-300/50 text-base-content/40 uppercase text-[9px]">
                            {entry.object_type}
                          </span>
                        </button>
                      ))}
                  </>
                )}

                {/* Lazy scroll sentinel */}
                {visibleCount < candidateEntries.length + otherEntries.length && (
                  <div
                    ref={sentinelRef}
                    className="p-2 text-center text-[10px] text-base-content/30"
                  >
                    Loading more...
                  </div>
                )}

                {candidateEntries.length === 0 && otherEntries.length === 0 && (
                  <div className="p-4 text-center text-xs text-base-content/40">
                    No results found
                  </div>
                )}
              </div>
            </div>
          )}
        </td>

        <td className="w-24">
          {displayType ? (
            <span className="badge badge-sm bg-base-300/50 border-base-300/60 text-base-content/70">
              {displayType}
            </span>
          ) : (
            <span className="text-xs text-base-content/30 italic">Unknown</span>
          )}
        </td>

        <td className="w-28 text-center">
          {!override && confidence !== 'None' ? (
            <div
              className={`badge badge-sm badge-outline gap-1 ${getConfidenceColor(confidence)}`}
              title={`${confidence} Confidence - ${matchLevelLabel(item.matchLevel)}`}
            >
              {getConfidenceIcon(confidence)}
              <span className="font-medium">{item.confidenceScore}%</span>
            </div>
          ) : (
            <span className="text-xs text-base-content/30">—</span>
          )}
        </td>

        <td className="w-12 text-center">
          <button
            className={`btn btn-xs btn-square ${isSkipped ? 'btn-warning bg-warning/20' : 'btn-ghost text-base-content/30 hover:text-warning'}`}
            onClick={onToggleSkip}
            title={isSkipped ? 'Include this mod' : 'Skip this mod'}
          >
            <SkipForward size={14} />
          </button>
        </td>
      </tr>
    </ContextMenu>
  );
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
        <div className="flex items-center justify-between mt-3 mb-2 min-h-[32px]">
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
                    <ReviewRow
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
