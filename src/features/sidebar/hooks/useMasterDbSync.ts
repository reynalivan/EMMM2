import { useState, useMemo } from 'react';
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

/** Categorized view for sidebar UI consumption. */
interface MasterDbCategorized {
  [category: string]: DbEntryFull[];
}

/**
 * Transform flat DbEntry[] → categorized MasterDb grouped by object_type.
 * Maps `tags` → `aliases` for UI compatibility. Preserves full metadata + thumbnail.
 */
function categorizeMasterDb(entries: DbEntry[]): MasterDbCategorized {
  const result: MasterDbCategorized = {};
  for (const entry of entries) {
    const key = entry.object_type.toLowerCase();
    if (!result[key]) result[key] = [];
    result[key].push({
      name: entry.name,
      aliases: entry.tags,
      tags: entry.tags, // Added for UI consumption
      object_type: entry.object_type,
      metadata: entry.metadata,
      thumbnail_path: entry.thumbnail_path,
      custom_skins: entry.custom_skins, // Added for UI consumption
    });
  }
  return result;
}

/**
 * Simple fuzzy match score (0..1) using longest common subsequence ratio.
 * Returns higher score for better matches. Falls back to includes() check first.
 */
function fuzzyScore(query: string, target: string): number {
  if (!query || !target) return 0;
  const q = query.toLowerCase();
  const t = target.toLowerCase();

  // Exact substring match → high score
  if (t.includes(q) || q.includes(t)) return 1;

  // LCS-based fuzzy score
  const m = q.length;
  const n = t.length;
  if (m === 0 || n === 0) return 0;

  // Simple LCS length via two-row DP
  let prev = new Array(n + 1).fill(0);
  let curr = new Array(n + 1).fill(0);
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (q[i - 1] === t[j - 1]) {
        curr[j] = prev[j - 1] + 1;
      } else {
        curr[j] = Math.max(prev[j], curr[j - 1]);
      }
    }
    [prev, curr] = [curr, prev];
    curr.fill(0);
  }
  const lcs = prev[n];
  // Normalize by the shorter string length to get a 0..1 ratio
  return lcs / Math.min(m, n);
}

const DEFAULT_OPTION_LIMIT = 120;
const SEARCH_RESULT_LIMIT = 80;

export function useMasterDbSync(objectType: string | undefined, originalName?: string) {
  const { activeGame } = useActiveGame();

  const [isSyncMode, setIsSyncMode] = useState(false);
  const [dbSearch, setDbSearch] = useState('');
  const [isDbOpen, setIsDbOpen] = useState(false);

  // Fetch MasterDB for Sync (flat array canonical format)
  // Use game_type (e.g. "GIMI") not game_id (UUID)
  const {
    data: masterDb,
    isLoading: isQueryLoading,
    error,
  } = useQuery({
    queryKey: ['master-db', activeGame?.game_type],
    queryFn: async () => {
      if (!activeGame) return null;
      try {
        const json = await invoke<string>('get_master_db', { gameType: activeGame.game_type });
        const entries = JSON.parse(json) as DbEntry[];
        return categorizeMasterDb(entries);
      } catch (e) {
        console.error('[useMasterDbSync] Failed to load MasterDB:', e);
        throw e;
      }
    },
    enabled: !!activeGame && isSyncMode,
    staleTime: 1000 * 60 * 60, // 1 hour
    retry: 1,
  });

  // Smart Suggestions (Top 4 matches for originalName)
  const suggestions = useMemo(() => {
    if (!masterDb || !originalName) return [];
    // Search GLOBAL DB for suggestions to handle miscategorized items
    const source = Object.values(masterDb).flat();
    const searchLower = originalName.toLowerCase();
    const FUZZY_THRESHOLD = 0.25; // Lowered from 0.4 to ensure matches appear

    const matches = source
      .map((item) => {
        const nameScore = fuzzyScore(searchLower, item.name);
        const aliasScore = Math.max(
          0,
          ...(item.aliases?.map((a) => fuzzyScore(searchLower, a)) ?? []),
        );
        return { item, score: Math.max(nameScore, aliasScore) };
      })
      .filter(({ score }) => score >= FUZZY_THRESHOLD)
      .sort((a, b) => b.score - a.score)
      .slice(0, 4) // Max 4 suggestions
      .map(({ item, score }) => ({ ...item, score })); // Include score for UI confidence

    return matches;
  }, [masterDb, originalName]);

  // Filter DB options — dynamically maps objectType to categorized DB keys
  // Uses fuzzy matching for better search tolerance (typos, partial names)
  const dbOptions = useMemo(() => {
    if (!masterDb) return [];

    let source: DbEntryFull[];

    // Look up by exact object_type key (lowercase)
    const typeLower = objectType?.toLowerCase() || '';

    const searchText = dbSearch.trim().toLowerCase();

    if (searchText) {
      // If searching, search GLOBAL DB to allow changing category/finding correct item
      source = Object.values(masterDb).flat();
    } else if (typeLower && masterDb[typeLower]) {
      // If no search, show current category items as context
      source = masterDb[typeLower];
    } else {
      // Fallback: Show everything
      source = Object.values(masterDb).flat();
    }

    if (!searchText) {
      return source.slice(0, DEFAULT_OPTION_LIMIT);
    }

    const directMatches = source
      .filter((item) => {
        if (item.name.toLowerCase().includes(searchText)) {
          return true;
        }

        return item.aliases?.some((alias) => alias.toLowerCase().includes(searchText)) ?? false;
      })
      .slice(0, SEARCH_RESULT_LIMIT);

    if (directMatches.length > 0 || searchText.length < 3) {
      return directMatches;
    }

    const FUZZY_THRESHOLD = 0.2;
    const scored = source
      .map((item) => {
        const nameScore = fuzzyScore(searchText, item.name);
        const aliasScore = Math.max(
          0,
          ...(item.aliases?.map((alias) => fuzzyScore(searchText, alias)) ?? []),
        );

        return { item, score: Math.max(nameScore, aliasScore) };
      })
      .filter(({ score }) => score >= FUZZY_THRESHOLD)
      .sort((a, b) => b.score - a.score)
      .slice(0, SEARCH_RESULT_LIMIT)
      .map(({ item }) => item);

    return scored;
  }, [masterDb, dbSearch, objectType]);

  return {
    isSyncMode,
    setIsSyncMode,
    dbSearch,
    setDbSearch,
    isDbOpen,
    setIsDbOpen,
    dbOptions,
    suggestions,
    masterDb, // Exported to allow Exact-Match hydration in EditObjectModal
    isLoading: isQueryLoading && isSyncMode, // Only load if sync mode active
    error,
  };
}
