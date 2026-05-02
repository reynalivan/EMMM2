import { commands } from '../../../lib/bindings';
import type { DbEntry } from '../../../types/object';
import { useActiveGame } from '../../../hooks/useActiveGame';

import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';

/** Full DB entry for UI consumption — includes metadata for auto-fill. */
export interface DbEntryFull {
  name: string;
  aliases?: string[];
  tags?: string[];
  object_type: string;
  metadata?: Record<string, unknown> | null;
  thumbnail_path?: string;
  folder_path?: string | null;
  custom_skins?: {
    name: string;
    aliases?: string[];
    thumbnail_skin_path?: string | null;
    rarity?: string | null;
  }[];
}

/**
 * Transform flat DbEntry[] → DbEntryFull[]
 * Maps `tags` → `aliases` for UI compatibility. Preserves full metadata + thumbnail.
 */
function mapToUiFormat(entries: DbEntry[]): DbEntryFull[] {
  return entries.map((entry) => ({
    name: entry.name,
    aliases: entry.tags || [],
    tags: entry.tags || [],
    object_type: entry.object_type,
    metadata: entry.metadata,
    thumbnail_path: entry.thumbnail_path || undefined,
    folder_path: null,
    custom_skins: entry.custom_skins || [],
  }));
}

export function useMasterDbSync(
  objectType: string | undefined,
  originalName?: string,
): {
  isSyncMode: boolean;
  setIsSyncMode: (mode: boolean) => void;
  dbSearch: string;
  setDbSearch: (search: string) => void;
  isDbOpen: boolean;
  setIsDbOpen: (open: boolean) => void;
  dbOptions: DbEntryFull[];
  suggestions: (DbEntryFull & { score: number })[];
  isLoading: boolean;
  error: Error | null;
} {
  const { activeGame } = useActiveGame();

  const [isSyncMode, setIsSyncMode] = useState(false);
  const [dbSearch, setDbSearch] = useState('');
  const [isDbOpen, setIsDbOpen] = useState(false);

  // Smart Suggestions (Top 4 matches for originalName) using Rust backend
  // Note: use `?? []` (not `= []`) so that both null AND undefined collapse to [].
  // The global test mock returns data:null which bypasses the `= []` default.
  const { data: rawSuggestions } = useQuery({
    queryKey: ['suggest-master-db', activeGame?.game_type, originalName],
    queryFn: async () => {
      if (!activeGame || !originalName) return [];
      try {
        const res = await commands.searchMasterDb({
          gameType: activeGame.game_type,
          query: originalName,
          objectType: undefined,
        });

        // Take top 4 that meet a threshold
        const FUZZY_THRESHOLD = 0.25;
        return res
          .filter((r) => r.score >= FUZZY_THRESHOLD)
          .slice(0, 4)
          .map((r) => ({
            ...mapToUiFormat([r.item])[0],
            score: r.score,
          }));
      } catch (e) {
        console.error('[useMasterDbSync] Failed to load suggestions:', e);
        return [];
      }
    },
    enabled: !!activeGame && !!originalName && isSyncMode,
    staleTime: 1000 * 60 * 60, // 1 hour for suggestions of the exact originalName
  });
  // Normalise: global test mock returns data:null, which bypasses `= []` default.
  // `?? []` handles both null and undefined safely.
  const suggestions = (rawSuggestions as (DbEntryFull & { score: number })[] | null) ?? [];

  // Filter DB options using Rust backend for exact and fuzzy matching
  const {
    data: dbOptions = [],
    isLoading: isQueryLoading,
    error,
  } = useQuery({
    queryKey: ['search-master-db', activeGame?.game_type, dbSearch, objectType],
    queryFn: async () => {
      if (!activeGame) return [];
      try {
        const res = await commands.searchMasterDb({
          gameType: activeGame.game_type,
          query: dbSearch.trim(),
          objectType: objectType || undefined,
        });
        return mapToUiFormat(res.map((r) => r.item));
      } catch (e) {
        console.error('[useMasterDbSync] Failed to search MasterDB:', e);
        throw e;
      }
    },
    enabled: !!activeGame && isSyncMode,
    staleTime: 1000 * 60 * 5, // Cache for 5 mins to prevent spamming while typing
  });

  return {
    isSyncMode,
    setIsSyncMode,
    dbSearch,
    setDbSearch,
    isDbOpen,
    setIsDbOpen,
    dbOptions,
    suggestions,
    isLoading: isQueryLoading && isSyncMode, // Only load if sync mode active
    error,
  };
}
