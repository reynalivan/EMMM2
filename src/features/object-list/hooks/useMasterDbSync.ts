import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useActiveGame } from '../../../hooks/useActiveGame';

/** Single entry from the flat-array Master DB (Rust canonical format). */
export interface DbEntry {
  name: string;
  tags?: string[];
  object_type: string;
  custom_skins?: {
    name: string;
    aliases?: string[];
    thumbnail_skin_path?: string;
    rarity?: string;
  }[];
  thumbnail_path?: string;
  metadata?: Record<string, unknown>;
}

/** Full DB entry for UI consumption — includes metadata for auto-fill. */
export interface DbEntryFull {
  name: string;
  aliases?: string[];
  tags?: string[];
  object_type: string;
  metadata?: Record<string, unknown>;
  thumbnail_path?: string;
  custom_skins?: {
    name: string;
    aliases?: string[];
    thumbnail_skin_path?: string;
    rarity?: string;
  }[];
}

/**
 * Transform flat DbEntry[] → DbEntryFull[]

 * Maps `tags` → `aliases` for UI compatibility. Preserves full metadata + thumbnail.
 */
function mapToUiFormat(entries: DbEntry[]): DbEntryFull[] {
  return entries.map((entry) => ({
    name: entry.name,
    aliases: entry.tags,
    tags: entry.tags, // Added for UI consumption
    object_type: entry.object_type,
    metadata: entry.metadata,
    thumbnail_path: entry.thumbnail_path,
    custom_skins: entry.custom_skins, // Added for UI consumption
  }));
}

export function useMasterDbSync(objectType: string | undefined, originalName?: string) {
  const { activeGame } = useActiveGame();

  const [isSyncMode, setIsSyncMode] = useState(false);
  const [dbSearch, setDbSearch] = useState('');
  const [isDbOpen, setIsDbOpen] = useState(false);

  // Smart Suggestions (Top 4 matches for originalName) using Rust backend
  const { data: suggestions = [] } = useQuery({
    queryKey: ['suggest-master-db', activeGame?.game_type, originalName],
    queryFn: async () => {
      if (!activeGame || !originalName) return [];
      try {
        const res = await invoke<{ item: DbEntry; score: number }[]>('search_master_db', {
          gameType: activeGame.game_type,
          query: originalName,
          objectType: null,
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
        const res = await invoke<{ item: DbEntry; score: number }[]>('search_master_db', {
          gameType: activeGame.game_type,
          query: dbSearch.trim(),
          objectType: objectType || null,
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
    masterDb: null, // Legacy export, no longer returning full object
    isLoading: isQueryLoading && isSyncMode, // Only load if sync mode active
    error,
  };
}
