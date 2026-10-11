import type { QueryClient } from '@tanstack/react-query';
import { publishQueryInvalidations, runtimeQueryKeys } from '@/shared/lib/queryRefresh';

const catalogReadKeys = [
  runtimeQueryKeys.catalogPack,
  ['master-db'],
  ['search-master-db'],
  ['suggest-master-db'],
  ['object-identity-suggestions'],
  runtimeQueryKeys.thumbnails,
  runtimeQueryKeys.objectRows,
  runtimeQueryKeys.workspaceViewModel,
  runtimeQueryKeys.previewDetails,
] as const;

export async function refreshCatalogQueries(queryClient: QueryClient): Promise<void> {
  // IPC cannot be aborted, but cancelled query results must not repopulate old catalog data.
  await Promise.all(catalogReadKeys.map((queryKey) => queryClient.cancelQueries({ queryKey })));
  await publishQueryInvalidations(queryClient, [...catalogReadKeys], 'active');
}
