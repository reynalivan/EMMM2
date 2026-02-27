import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

interface MetadataSyncResult {
  updated: boolean;
  version: number | null;
}

export const metadataSyncKeys = {
  sync: ['metadata-sync'] as const,
};

/**
 * Trigger a metadata sync check on mount. Fires once and caches the result.
 * Fails silently — network errors are logged but never surface to the user.
 */
export function useMetadataSyncQuery() {
  return useQuery<MetadataSyncResult>({
    queryKey: metadataSyncKeys.sync,
    queryFn: () => invoke<MetadataSyncResult>('check_metadata_update'),
    staleTime: 1000 * 60 * 60, // 1 hour — only check once per session
    retry: false, // Backend already handles retries
    refetchOnWindowFocus: false,
  });
}

/**
 * Manual trigger for metadata sync (e.g., "Sync Now" button).
 */
export function useMetadataSyncMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => invoke<MetadataSyncResult>('check_metadata_update'),
    onSuccess: (data) => {
      queryClient.setQueryData(metadataSyncKeys.sync, data);
    },
  });
}

/**
 * Fetch a missing asset file from the CDN on demand.
 */
export function useAssetFetch() {
  return useMutation({
    mutationFn: (assetName: string) => invoke<string | null>('fetch_missing_asset', { assetName }),
  });
}
