import type { QueryClient } from '@tanstack/react-query';
import type { DiskReconcileResult } from '../../lib/bindings';
import { publishRuntimeDescriptor } from '../runtime-sync/queryRefresh';
import {
  buildRuntimeMutationDescriptor,
  type RuntimeMutationClass,
} from '../workspace-runtime/optimistic/descriptorBuilders';

export function publishDiskReconcileRefresh(
  queryClient: QueryClient,
  result: DiskReconcileResult,
  previewAffected: boolean,
): void {
  const thumbnailsChanged = result.thumbnail_roots.length > 0;
  const dashboardAffected =
    result.objects_changed ||
    result.folders_changed ||
    result.runtime_file_changed ||
    result.collections_changed ||
    thumbnailsChanged;

  // ponytail: one publish. The refresh bus coalesces scopes on a microtask, so
  // eight separate calls only ever produced one invalidation pass anyway.
  const kinds: Array<[boolean, RuntimeMutationClass]> = [
    [
      result.objects_changed || result.folders_changed || result.path_updates.length > 0,
      'objectRows',
    ],
    [result.folders_changed, 'folderStructureOnly'],
    [thumbnailsChanged, 'thumbnailOnly'],
    [result.runtime_file_changed, 'folderMetadataPreview'],
    [result.collections_changed, 'collectionsCatalog'],
    [dashboardAffected, 'dashboardKeybindings'],
    [previewAffected, 'previewOnly'],
    [result.folders_changed || result.objects_changed, 'conflictsOnly'],
  ];

  const active = kinds.filter(([applies]) => applies).map(([, kind]) => kind);
  if (active.length === 0) {
    return;
  }

  void publishRuntimeDescriptor(queryClient, buildRuntimeMutationDescriptor(active), 'active');
}
