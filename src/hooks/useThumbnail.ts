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
import { invoke } from '@tauri-apps/api/core';

/** Query key factory for thumbnail cache */
export const thumbnailKeys = {
  all: ['thumbnails'] as const,
  folder: (folderPath: string) => [...thumbnailKeys.all, folderPath] as const,
};

/**
 * Lazily fetch the cached WebP thumbnail for a single mod folder.
 *
 * @param folderPath - Absolute path to the mod folder
 * @param enabled - Whether to fetch (default true)
 */
export function useThumbnail(folderPath: string, enabled = true) {
  return useQuery<string | null>({
    queryKey: thumbnailKeys.folder(folderPath),
    queryFn: () => invoke<string | null>('get_mod_thumbnail', { folderPath }),
    enabled,
    staleTime: Infinity, // never auto-refetch; invalidate explicitly
    gcTime: 10 * 60_000, // 10 min garbage collection
    refetchOnWindowFocus: false,
    retry: 1,
  });
}
