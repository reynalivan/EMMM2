import type { QueryClient } from '@tanstack/react-query';
import type { DiskReconcileResult } from '../../lib/bindings';
import { publishRuntimeDescriptor } from '../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../workspace-runtime/optimistic/descriptorBuilders';

export function publishDiskReconcileRefresh(
  queryClient: QueryClient,
  result: DiskReconcileResult,
  previewAffected: boolean,
): void {
  const objectListAffected =
    result.objects_changed || result.folders_changed || result.path_updates.length > 0;

  if (objectListAffected) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('objectRows'),
      'active',
    );
  }

  if (result.folders_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('folderStructureOnly'),
      'active',
    );
  }

  if (result.thumbnail_roots.length > 0) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('thumbnailOnly'),
      'active',
    );
  }

  if (result.collections_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor(['collectionsCatalog', 'dashboardKeybindings']),
      'active',
    );
  }

  if (
    result.objects_changed ||
    result.folders_changed ||
    result.runtime_file_changed ||
    result.thumbnail_roots.length > 0
  ) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('dashboardKeybindings'),
      'active',
    );
  }

  if (previewAffected) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('previewOnly'),
      'active',
    );
  }

  if (result.folders_changed || result.objects_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('conflictsOnly'),
      'active',
    );
  }
}
