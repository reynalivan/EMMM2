import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, Search, X } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { scanService, type ScanPreviewItem } from '../../lib/services/scanService';
import type { GameConfig } from '../../types/game';
import type { MasterDbEntry } from './scanReviewHelpers';
import { EntryGroup, SearchInput, type MergedScanReviewEntry } from './ScanReviewMatchEntries';

interface ScanReviewMatchCellProps {
  item: ScanPreviewItem;
  override: MasterDbEntry | null;
  onOverride: (entry: MasterDbEntry | null) => void;
  isSkipped: boolean;
  masterDbEntries: MasterDbEntry[];
  activeGame: GameConfig | null;
}

const VISIBLE_INCREMENT = 30;
const SCORE_CHUNK_SIZE = 50;

export function ScanReviewMatchCell({
  item,
  override,
  onOverride,
  isSkipped,
  masterDbEntries,
  activeGame,
}: ScanReviewMatchCellProps) {
  const { t } = useTranslation(['objects', 'common']);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [visibleCount, setVisibleCount] = useState(VISIBLE_INCREMENT);
  const [dropdownPosition, setDropdownPosition] = useState<'top' | 'bottom'>('bottom');
  const [dynamicScores, setDynamicScores] = useState<Record<string, number>>({});
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const dropdownRef = useRef<HTMLTableDataCellElement | null>(null);
  const hasFetchedScoresRef = useRef(false);
  const displayMatch = override?.name ?? item.matchedAliasName;
  const displayThumb = override?.thumbnail_path ?? item.thumbnailPath;

  const scoreMap = useMemo(() => {
    const map = new Map<string, number>();
    for (const candidate of item.scoredCandidates ?? []) {
      map.set(candidate.name, candidate.scorePct);
    }
    return map;
  }, [item.scoredCandidates]);

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
    if (!searchOpen || !activeGame || hasFetchedScoresRef.current) {
      return;
    }

    let isMounted = true;
    const gameType = activeGame.game_type;
    hasFetchedScoresRef.current = true;

    async function fetchDynamicScores() {
      const candidatesToScore = masterDbEntries
        .filter((entry) => !scoreMap.has(entry.name))
        .map((entry) => entry.name);

      if (candidatesToScore.length === 0) {
        return;
      }

      for (let index = 0; index < candidatesToScore.length; index += SCORE_CHUNK_SIZE) {
        if (!isMounted || !searchOpen) {
          break;
        }

        const chunk = candidatesToScore.slice(index, index + SCORE_CHUNK_SIZE);
        try {
          const scores = await scanService.scoreCandidatesBatch(item.folderPath, chunk, gameType);

          if (isMounted) {
            setDynamicScores((previousScores) => ({ ...previousScores, ...scores }));
          }
        } catch (error) {
          console.error('Failed to fetch score chunk:', error);
        }
      }
    }

    void fetchDynamicScores();

    return () => {
      isMounted = false;
    };
  }, [activeGame, item.folderPath, masterDbEntries, scoreMap, searchOpen]);

  const { candidateEntries, otherEntries } = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    let entries: MergedScanReviewEntry[] = masterDbEntries.map((entry) => {
      const mappedScore = scoreMap.get(entry.name);
      return {
        ...entry,
        scorePct: mappedScore !== undefined ? mappedScore : dynamicScores[entry.name],
      };
    });

    if (query) {
      entries = entries.filter(
        (entry) =>
          entry.name.toLowerCase().includes(query) ||
          entry.object_type.toLowerCase().includes(query) ||
          entry.tags.some((tag) => tag.toLowerCase().includes(query)),
      );
    }

    const candidates = entries.filter((entry) => isCandidateEntry(entry, scoreMap));
    const others = entries.filter((entry) => !isCandidateEntry(entry, scoreMap));
    candidates.sort(compareScoredEntries);
    others.sort(compareScoredEntries);

    return { candidateEntries: candidates, otherEntries: others };
  }, [dynamicScores, masterDbEntries, scoreMap, searchQuery]);

  const handleSearchChange = useCallback((value: string) => {
    setSearchQuery(value);
    setVisibleCount(VISIBLE_INCREMENT);
  }, []);

  const handleToggleDropdown = useCallback(() => {
    setSearchOpen((previousOpen) => {
      if (!previousOpen) {
        setVisibleCount(VISIBLE_INCREMENT);
        const rect = dropdownRef.current?.getBoundingClientRect();
        setDropdownPosition(rect && window.innerHeight - rect.bottom < 320 ? 'top' : 'bottom');
      }

      return !previousOpen;
    });
  }, []);

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !searchOpen) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          setVisibleCount((previousCount) => previousCount + VISIBLE_INCREMENT);
        }
      },
      { threshold: 0.1 },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [searchOpen, searchQuery]);

  return (
    <td ref={dropdownRef} className="relative group/search max-w-64 w-64">
      <button
        className={`btn btn-sm h-8 min-h-8 w-full justify-start pl-2 pr-2 gap-2 flex-nowrap ${
          override ? 'btn-info btn-outline' : 'btn-ghost bg-base-200/50 hover:bg-base-300/60'
        }`}
        onClick={handleToggleDropdown}
        disabled={isSkipped}
        title={displayMatch ?? t('context.click_to_assign')}
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
            <span className="text-base-content/30 italic font-normal">
              {t('objects:item.status_unmatched')}
            </span>
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
          <SearchInput searchQuery={searchQuery} onSearchChange={handleSearchChange} />
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
                <X size={14} /> {t('objects:scan_review.tabs.clear_override')}
              </button>
            )}
            <EntryGroup
              entries={candidateEntries.slice(0, visibleCount)}
              label={t('objects:scan_review.tabs.candidates')}
              totalCount={candidateEntries.length}
              onSelect={(entry) => {
                onOverride(entry);
                setSearchOpen(false);
                setSearchQuery('');
              }}
              variant="candidate"
            />
            <EntryGroup
              entries={otherEntries.slice(0, Math.max(0, visibleCount - candidateEntries.length))}
              label={t('objects:scan_review.tabs.other')}
              totalCount={otherEntries.length}
              onSelect={(entry) => {
                onOverride(entry);
                setSearchOpen(false);
                setSearchQuery('');
              }}
              variant="other"
            />
            {visibleCount < candidateEntries.length + otherEntries.length && (
              <div ref={sentinelRef} className="p-2 text-center text-[10px] text-base-content/30">
                {t('common:states.loading_more')}
              </div>
            )}
            {candidateEntries.length === 0 && otherEntries.length === 0 && (
              <div className="p-4 text-center text-xs text-base-content/40">
                {t('objects:scan_review.tabs.no_results')}
              </div>
            )}
          </div>
        </div>
      )}
    </td>
  );
}

function isCandidateEntry(entry: MergedScanReviewEntry, scoreMap: Map<string, number>): boolean {
  const hasScoredCandidate = scoreMap.has(entry.name);
  return (
    (hasScoredCandidate && entry.scorePct !== undefined && entry.scorePct > 0) ||
    (!hasScoredCandidate && entry.scorePct !== undefined && entry.scorePct >= 50)
  );
}

function compareScoredEntries(first: MergedScanReviewEntry, second: MergedScanReviewEntry): number {
  return (second.scorePct ?? -1) - (first.scorePct ?? -1) || first.name.localeCompare(second.name);
}
