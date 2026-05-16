/**
 * v2 Collection hooks — Query + Mutation hooks for the greenfield collection system.
 *
 * Replaces: useCollections, useCollectionRuntimePreview, useCreateCollection,
 *           useSaveCurrentAsCollection, useSaveSnapshotCollectionAsNamed,
 *           useUpdateCollection, useDeleteCollection, useApplyCollection.
 *
 * Collection queries are corridor-explicit so frontend cache keys match the
 * backend command inputs.
 */

import { useMutation, useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import { collectionKeys, corridorKeys } from '../queryKeys';
import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import {
  extractFileInUsePayload,
  extractMissingModsPayload,
  formatAppError,
} from '../../../lib/appError';
import {
  publishQueryInvalidations,
  publishRuntimeDescriptor,
} from '../../runtime-sync/queryRefresh';
import {
  buildRuntimeMutationDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../../workspace-runtime/optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../../workspace-runtime/optimistic/descriptor';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import { openWorkspaceFileInUseDialog } from '../../workspace-runtime/state/workspaceDialogs';
import type {
  CollectionSummary,
  CollectionPreview,
  ApplyPreview,
  ApplyResult,
  ApplyProgressSnapshot,
  CorridorSnapshot,
} from '../../../types/collection';
import type { CollectionSaveMode } from '../types';

async function refetchStrictCorridorState(
  queryClient: ReturnType<typeof useQueryClient>,
  gameId: string,
  isSafe: boolean,
): Promise<CorridorSnapshot> {
  const snapshot = await queryClient.fetchQuery({
    queryKey: corridorKeys.state(gameId, isSafe),
    queryFn: () =>
      commands.getCorridorState({
        gameId,
        isSafe,
      }),
    staleTime: 0,
  });
  queryClient.setQueryData(corridorKeys.state(gameId, isSafe), snapshot);
  return snapshot;
}

async function refetchCollectionList(
  queryClient: ReturnType<typeof useQueryClient>,
  gameId: string,
  isSafe: boolean,
): Promise<void> {
  await publishQueryInvalidations(queryClient, [collectionKeys.list(gameId, isSafe)], 'active');
}

async function refetchCollectionPreview(
  queryClient: ReturnType<typeof useQueryClient>,
  collectionId: string,
  gameId: string,
): Promise<void> {
  await publishQueryInvalidations(
    queryClient,
    [[...collectionKeys.preview(collectionId), gameId]],
    'active',
  );
}

// ── Query Hooks ────────────────────────────────────────────────────────────

/** List all collections for the current game + corridor. */
export function useCollections(gameId: string | null, isSafe: boolean) {
  return useQuery<CollectionSummary[]>({
    queryKey: collectionKeys.list(gameId ?? '', isSafe),
    queryFn: () => commands.listCollections({ gameId: gameId ?? '', isSafe }),
    enabled: !!gameId,
    placeholderData: keepPreviousData,
    staleTime: 10_000,
  });
}

/** Get detailed preview for a specific collection. */
export function useCollectionPreview(collectionId: string | null, gameId: string | null) {
  return useQuery<CollectionPreview>({
    queryKey: [...collectionKeys.preview(collectionId ?? ''), gameId],
    queryFn: () =>
      commands.getCollectionPreview({
        collectionId: collectionId ?? '',
        gameId: gameId ?? '',
      }),
    enabled: !!collectionId && !!gameId,
    staleTime: 30_000,
  });
}

/** Get before/after preview for applying a collection. */
export function useApplyCollectionPreview(
  gameId: string | null,
  collectionId: string | null,
  isSafe: boolean,
) {
  return useQuery<ApplyPreview>({
    queryKey: [...collectionKeys.previewApply(collectionId ?? ''), gameId ?? '', isSafe],
    queryFn: () =>
      commands.previewApplyCollection({
        gameId: gameId ?? '',
        collectionId: collectionId ?? '',
        isSafe,
      }),
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
    }) => commands.createCollection({ gameId, name, saveMode, sourceCollectionId }),

    onSuccess: async (result: CollectionSummary, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsCatalog'),
        'none',
      );
      await Promise.all([
        refetchStrictCorridorState(queryClient, variables.gameId, result.is_safe),
        refetchCollectionList(queryClient, variables.gameId, result.is_safe),
        refetchCollectionPreview(queryClient, result.id, variables.gameId),
      ]);
      toast.success(`Created collection: ${result.name}`);
    },

    onError: (err: unknown) => {
      toast.error(formatAppError(err));
    },
  });
}

export function useApplyProgress(gameId: string | null, enabled: boolean) {
  return useQuery<ApplyProgressSnapshot | null>({
    queryKey: collectionKeys.applyProgress(gameId ?? ''),
    queryFn: () =>
      commands.getApplyProgress({
        gameId: gameId ?? '',
      }),
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
      commands.updateCollection({
        gameId,
        id,
        name,
      }),

    onSuccess: async (result: CollectionSummary) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsOnly'),
        'none',
      );
      toast.success(`Updated collection: ${result.name}`);
    },

    onError: (err: unknown) => {
      toast.error(formatAppError(err));
    },
  });
}

/** Replace an existing named collection with the current live corridor state. */
export function useReplaceCollectionWithCurrentState() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, collectionId }: { gameId: string; collectionId: string }) =>
      commands.replaceCollectionWithCurrentState({ gameId, collectionId }),

    onSuccess: async (result: CollectionSummary, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsState'),
        'active',
      );
      await Promise.all([
        refetchStrictCorridorState(queryClient, variables.gameId, result.is_safe),
        refetchCollectionList(queryClient, variables.gameId, result.is_safe),
        refetchCollectionPreview(queryClient, result.id, variables.gameId),
      ]);
      toast.success(`Updated collection: ${result.name}`);
    },

    onError: (err: unknown) => {
      toast.error(formatAppError(err));
    },
  });
}

/** Delete a collection. */
export function useDeleteCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId: _gameId, id }: { gameId: string; id: string }) =>
      commands.deleteCollection({ id }),

    onSuccess: async (_result, variables) => {
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('collectionsCatalog'),
        'none',
      );
      await Promise.all([
        refetchStrictCorridorState(queryClient, variables.gameId, useAppStore.getState().safeMode),
        refetchCollectionList(queryClient, variables.gameId, useAppStore.getState().safeMode),
      ]);
      toast.success('Collection deleted');
    },

    onError: (err: unknown) => {
      toast.error(formatAppError(err));
    },
  });
}

/** Apply a collection (enable/disable mods to match the snapshot). */
export function useApplyCollection() {
  const queryClient = useQueryClient();
  const safeMode = useAppStore((state) => state.safeMode);

  const mutation = useMutation({
    mutationFn: ({
      gameId,
      collectionId,
      ignoreMissing,
    }: {
      gameId: string;
      collectionId: string;
      ignoreMissing?: boolean;
    }) =>
      commands.applyCollection({
        gameId,
        collectionId,
        ignoreMissing: ignoreMissing ?? false,
      }),

    onSuccess: async (result: ApplyResult, variables) => {
      const descriptor = mergeRuntimeEffectDescriptors(
        buildRuntimeMutationDescriptor('collectionsState'),
        buildWorkspacePathRewritesDescriptor(result.runtime_path_rewrites ?? [], []),
      );
      applyRuntimeEffects(queryClient, descriptor);
      await publishRuntimeDescriptor(queryClient, descriptor, 'active');
      await Promise.all([
        refetchStrictCorridorState(queryClient, variables.gameId, safeMode),
        refetchCollectionList(queryClient, variables.gameId, safeMode),
        refetchCollectionPreview(queryClient, variables.collectionId, variables.gameId),
      ]);

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

      toast.error(formatAppError(err));
    },
  });

  return mutation;
}
