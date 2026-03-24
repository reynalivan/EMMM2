/**
 * Hook for lazily fetching mod thumbnails.
 *
 * Thumbnails are resolved per-card AFTER the folder list renders,
 * so the grid appears instantly with skeleton placeholders while
 * thumbnails stream in via the backend `get_mod_thumbnail` command.
 *
 * Works naturally with @tanstack/react-virtual — unmounted cards
 * don't fire queries; mounted cards cache results via TanStack Query.
 */

import { useQuery } from '@tanstack/react-query';
import { convertFileSrc } from '@tauri-apps/api/core';
import { commands } from '../lib/bindings';

/** Query key factory for thumbnail cache */
export const thumbnailKeys = {
  all: ['thumbnails'] as const,
  folder: (folderPath: string) => [...thumbnailKeys.all, folderPath] as const,
};

/**
 * Lazily fetch the cached WebP thumbnail for a single mod folder.
 *
 * @param gameId - The ID of the currently active game
 * @param folderPath - Absolute path to the mod folder
 * @param enabled - Whether to fetch (default true)
 */
export function useThumbnail(gameId: string, folderPath: string, enabled = true) {
  return useQuery<string | null>({
    queryKey: thumbnailKeys.folder(folderPath),
    queryFn: async () => {
      const res = await commands.getModThumbnail({ gameId, folderPath });
      return res ? convertFileSrc(res) : null;
    },
    enabled: enabled && !!gameId,
    staleTime: Infinity, // never auto-refetch; invalidate explicitly
    gcTime: 10 * 60_000, // 10 min garbage collection
    refetchOnWindowFocus: false,
    retry: 1,
  });
}
