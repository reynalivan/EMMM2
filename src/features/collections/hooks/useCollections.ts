/**
 * v2 Collection hooks — Query + Mutation hooks for the greenfield collection system.
 *
 * Replaces: useCollections, useCollectionRuntimePreview, useCreateCollection,
 *           useSaveCurrentAsCollection, useSaveSnapshotCollectionAsNamed,
 *           useUpdateCollection, useDeleteCollection, useApplyCollection.
 *
 * Collection queries are game-scoped presets. Safe/unsafe is metadata on
 * each collection, not a runtime cache dimension.
 */

import {
  useMutation,
  useQuery,
  useQueryClient,
  keepPreviousData,
  type QueryClient,
} from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import { collectionKeys } from '../queryKeys';
import { commands } from '../../../lib/bindings';
import {
  extractFileInUsePayload,
  extractMissingModsPayload,
  formatAppError,
} from '../../../lib/appError';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import {
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../../workspace-runtime/optimistic/descriptor';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import { openWorkspaceFileInUseDialog } from '../../workspace-runtime/state/workspaceDialogs';
import type { RuntimeEffectDescriptor } from '../../../lib/runtimeEffects';
import type {
  CollectionSummary,
  CollectionPreview,
  ApplyPreview,
  ApplyResult,
  ApplyProgressSnapshot,
} from '../../../types/collection';
import type { CollectionSaveMode } from '../types';

/**
 * Every collection mutation refreshes through the runtime bus, so the refreshed
 * key set lives in the descriptor table and nowhere else. `collectionsChanged`
 * already invalidates the whole `collectionKeys.all` prefix — list, preview and
 * apply-preview included.
 */
async function publishCollectionMutation(
  queryClient: QueryClient,
  descriptor: RuntimeEffectDescriptor,
): Promise<void> {
  applyRuntimeEffects(queryClient, descriptor);
  await publishRuntimeDescriptor(queryClient, descriptor, 'active');
}

function toastMutationError(err: unknown): void {
  toast.error(formatAppError(err));
}

// ── Query Hooks ────────────────────────────────────────────────────────────

/** List all named collection presets for the current game. */
export function useCollections(gameId: string | null) {
  return useQuery<CollectionSummary[]>({
    queryKey: collectionKeys.list(gameId ?? ''),
    queryFn: () => commands.listCollections(gameId ?? '', null),
    enabled: !!gameId,
    placeholderData: keepPreviousData,
    staleTime: 10_000,
  });
}

/** Get detailed preview for a specific collection. */
export function useCollectionPreview(collectionId: string | null, gameId: string | null) {
  return useQuery<CollectionPreview>({
    queryKey: [...collectionKeys.preview(collectionId ?? ''), gameId],
    queryFn: () => commands.getCollectionPreview(collectionId ?? '', gameId ?? ''),
    enabled: !!collectionId && !!gameId,
    staleTime: 30_000,
  });
}

/** Get before/after preview for applying a collection. */
export function useApplyCollectionPreview(gameId: string | null, collectionId: string | null) {
  return useQuery<ApplyPreview>({
    queryKey: [...collectionKeys.previewApply(collectionId ?? ''), gameId ?? ''],
    queryFn: () => commands.previewApplyCollection(gameId ?? '', collectionId ?? '', null),
    enabled: !!gameId && !!collectionId,
    // Don't cache this long, we want fresh disk state when viewing the modal
    staleTime: 0,
  });
}

// ── Mutation Hooks ─────────────────────────────────────────────────────────

/** Create a new named collection. */
export function useCreateCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      gameId,
      name,
      saveMode,
      sourceCollectionId,
    }: {
      gameId: string;
      name: string;
      saveMode?: CollectionSaveMode;
      sourceCollectionId?: string | null;
    }) => commands.createCollection(gameId, name, saveMode ?? null, sourceCollectionId ?? null),

    onSuccess: async (result: CollectionSummary) => {
      await publishCollectionMutation(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsCatalog'),
      );
      toast.success(`Created collection: ${result.name}`);
    },

    onError: toastMutationError,
  });
}

export function useApplyProgress(gameId: string | null, enabled: boolean) {
  return useQuery<ApplyProgressSnapshot | null>({
    queryKey: collectionKeys.applyProgress(gameId ?? ''),
    queryFn: () => commands.getApplyProgress(gameId ?? ''),
    enabled: !!gameId && enabled,
    staleTime: 0,
    refetchInterval: (query) => {
      const snapshot = query.state.data;
      if (!snapshot) {
        return 300;
      }

      if (snapshot.phase === 'done' || snapshot.phase === 'failed') {
        return false;
      }

      return 300;
    },
  });
}

/** Update an existing collection (rename). */
export function useUpdateCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, id, name }: { gameId: string; id: string; name?: string }) =>
      commands.updateCollection(gameId, id, name ?? null),

    onSuccess: async (result: CollectionSummary) => {
      await publishCollectionMutation(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsOnly'),
      );
      toast.success(`Updated collection: ${result.name}`);
    },

    onError: toastMutationError,
  });
}

/** Replace an existing named collection with the current live corridor state. */
export function useReplaceCollectionWithCurrentState() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, collectionId }: { gameId: string; collectionId: string }) =>
      commands.replaceCollectionWithCurrentState(gameId, collectionId),

    onSuccess: async (result: CollectionSummary) => {
      await publishCollectionMutation(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsState'),
      );
      toast.success(`Updated collection: ${result.name}`);
    },

    onError: toastMutationError,
  });
}

/** Delete a collection. */
export function useDeleteCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId: _gameId, id }: { gameId: string; id: string }) =>
      commands.deleteCollection(id),

    onSuccess: async () => {
      await publishCollectionMutation(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsCatalog'),
      );
      toast.success('Collection deleted');
    },

    onError: toastMutationError,
  });
}

/** Apply a collection (enable/disable mods to match the snapshot). */
export function useApplyCollection() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({
      gameId,
      collectionId,
      ignoreMissing,
    }: {
      gameId: string;
      collectionId: string;
      ignoreMissing?: boolean;
    }) => commands.applyCollection(gameId, collectionId, ignoreMissing ?? false),

    onSuccess: async (result: ApplyResult) => {
      await publishCollectionMutation(
        queryClient,
        mergeRuntimeEffectDescriptors(
          buildRuntimeMutationDescriptor('collectionsState'),
          buildWorkspacePathRewritesDescriptor(result.runtime_path_rewrites ?? [], []),
        ),
      );

      const total = result.mods_enabled + result.mods_disabled;
      const suffix = result.final_state_name ? ` -> ${result.final_state_name}` : '';
      toast.success(`Applied collection (${total} changes)${suffix}`);
    },

    onError: (err: unknown, variables) => {
      const fileInUse = extractFileInUsePayload(err);
      if (fileInUse) {
        openWorkspaceFileInUseDialog({
          path: fileInUse.path,
          processes: fileInUse.processes,
          onRetry: () => mutation.mutate(variables),
        });
        return;
      }

      if (extractMissingModsPayload(err)) {
        return;
      }

      toastMutationError(err);
    },
  });

  return mutation;
}
