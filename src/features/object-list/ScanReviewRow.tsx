import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import {
  X,
  Check,
  Search,
  SkipForward,
  ChevronDown,
  Pencil,
  ExternalLink,
  FolderOpen,
} from 'lucide-react';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import FolderTooltip from './FolderTooltip';
import { scanService, type ScanPreviewItem } from '../../lib/services/scanService';
import type { GameConfig } from '../../types/game';
import {
  type MasterDbEntry,
  getConfidenceColor,
  getConfidenceIcon,
  matchLevelLabel,
} from './scanReviewHelpers';

/** Single row in the review table. */
export default function ScanReviewRow({
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
          <FolderTooltip
            folderPath={item.folderPath}
            thumbnailPath={item.thumbnailPath}
            gameId={activeGame?.id || ''}
          >
            <div className="flex flex-col">
              {isRenaming ? (
                <div className="flex items-center gap-1">
                  <input
                    ref={renameInputRef}
                    className="input input-xs input-bordered w-full text-sm font-medium"
                    value={editName}
                    onChange={(e) => {
                      let val = e.target.value;
                      if (/^(disabled|disable|dis)[_\-\s]+/i.test(val)) {
                        val = val.replace(/^(disabled|disable|dis)[_\-\s]+/i, '');
                      }
                      setEditName(val);
                    }}
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
