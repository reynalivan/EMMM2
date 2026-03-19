import { invoke } from '@tauri-apps/api/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import { useAppStore } from '../../../stores/useAppStore';
import {
  applyProgressKeys,
  collectionKeys,
  corridorRuntimeKeys,
} from '../queryKeys';
import { invalidateCorridorRuntime } from '../utils/invalidateCorridorRuntime';
import { invalidateCollectionRuntime } from '../utils/invalidateCollectionRuntime';
import { refetchCollectionRuntime } from '../utils/refetchCollectionRuntime';
import type {
  ApplyCollectionProgress,
  ApplyCollectionResult,
  Collection,
  CollectionDetails,
  CollectionRuntimePreview,
  CorridorRuntimeSnapshot,
  CreateCollectionInput,
  UpdateCollectionInput,
} from '../../../types/collection';

function upsertCollectionListEntry(
  queryClient: ReturnType<typeof useQueryClient>,
  collection: Collection,
) {
  queryClient.setQueryData<Collection[]>(collectionKeys.list(collection.game_id), (current) => {
    if (!current) {
      return [collection];
    }

    const withoutCollection = current.filter((item) => item.id !== collection.id);
    return [...withoutCollection, collection];
  });
}

export function useCollections(gameId?: string | null) {
  return useQuery<Collection[]>({
    queryKey: collectionKeys.list(gameId ?? ''),
    queryFn: () => invoke<Collection[]>('list_collections', { gameId }),
    enabled: !!gameId,
    staleTime: 10_000,
  });
}

export function useCollectionRuntimePreview(collectionId: string | null, gameId: string | null) {
  return useQuery<CollectionRuntimePreview>({
    queryKey: collectionKeys.runtimePreview(collectionId ?? ''),
    queryFn: () =>
      invoke<CollectionRuntimePreview>('get_collection_runtime_preview', { collectionId, gameId }),
    enabled: !!collectionId && !!gameId,
    staleTime: 30_000,
  });
}

export function useCorridorRuntimeSnapshot(gameId: string | null, safeMode: boolean) {
  return useQuery<CorridorRuntimeSnapshot>({
    queryKey: corridorRuntimeKeys.snapshot(gameId ?? '', safeMode),
    queryFn: () =>
      invoke<CorridorRuntimeSnapshot>('get_corridor_runtime_snapshot', {
        gameId,
        isSafe: safeMode,
      }),
    enabled: !!gameId,
    placeholderData: (previousData) => previousData,
    staleTime: 5000,
  });
}

export function useApplyProgress(gameId: string | null, enabled: boolean) {
  return useQuery<ApplyCollectionProgress>({
    queryKey: applyProgressKeys.detail(gameId ?? ''),
    queryFn: () => invoke<ApplyCollectionProgress>('get_apply_progress', { gameId }),
    enabled: enabled && !!gameId,
    refetchInterval: enabled ? 200 : false,
    staleTime: 0,
  });
}

export function useCreateCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateCollectionInput) =>
      invoke<CollectionDetails>('create_collection', { input }),
    onSuccess: async (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      await invalidateCorridorRuntime(queryClient);
      toast.success(`Created collection: ${result.collection.name}`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useSaveCurrentAsCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: Omit<CreateCollectionInput, 'mod_ids' | 'auto_snapshot'>) =>
      invoke<CollectionDetails>('create_collection', {
        input: { ...input, mod_ids: [], auto_snapshot: true },
      }),
    onSuccess: async (result, input) => {
      upsertCollectionListEntry(queryClient, result.collection);
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      await invalidateCorridorRuntime(queryClient);
      await refetchCollectionRuntime(queryClient, {
        gameId: input.game_id,
        isSafe: input.is_safe_context,
        collectionId: result.collection.id,
      });
      toast.success(`Saved current state as: ${result.collection.name}`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

interface SaveSnapshotCollectionInput {
  source_collection_id: string;
  game_id: string;
  name: string;
}

export function useSaveSnapshotCollectionAsNamed() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: SaveSnapshotCollectionInput) =>
      invoke<CollectionDetails>('save_snapshot_collection_as_named', {
        sourceCollectionId: input.source_collection_id,
        gameId: input.game_id,
        name: input.name,
      }),
    onSuccess: async (result) => {
      upsertCollectionListEntry(queryClient, result.collection);
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({
        queryKey: collectionKeys.runtimePreview(result.collection.id),
      });
      await invalidateCorridorRuntime(queryClient);
      await refetchCollectionRuntime(queryClient, {
        gameId: result.collection.game_id,
        isSafe: result.collection.is_safe_context,
        collectionId: result.collection.id,
      });
      toast.success(`Saved snapshot as: ${result.collection.name}`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useUpdateCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: UpdateCollectionInput) =>
      invoke<CollectionDetails>('update_collection', { input }),
    onSuccess: async (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      await invalidateCorridorRuntime(queryClient);
      await refetchCollectionRuntime(queryClient, {
        gameId: result.collection.game_id,
        isSafe: result.collection.is_safe_context,
        collectionId: result.collection.id,
      });
      toast.success(`Updated collection: ${result.collection.name}`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useDeleteCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, gameId }: { id: string; gameId: string }) =>
      invoke<void>('delete_collection', { id, gameId }),
    onSuccess: async () => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      await invalidateCorridorRuntime(queryClient);
      toast.success('Collection deleted');
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useUndoCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId }: { gameId: string }) =>
      invoke<ApplyCollectionResult>('undo_collection', { gameId }),
    onSuccess: async (result, variables) => {
      await invalidateCollectionRuntime(queryClient, { includeMods: true });
      const { activeGameId, safeMode } = useAppStore.getState();
      const nextGameId = variables.gameId || activeGameId;
      if (nextGameId) {
        await refetchCollectionRuntime(queryClient, {
          gameId: nextGameId,
          isSafe: safeMode,
        });
      }
      toast.success(`Undid collection application (${result.changed_count} changes reverted)`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}
export function useApplyCollection() {
  const queryClient = useQueryClient();
  const undoCollection = useUndoCollection();

  return useMutation({
    mutationFn: ({
      collectionId,
      gameId,
      safeMode,
    }: {
      collectionId: string;
      gameId: string;
      safeMode: boolean;
      targetPreview?: CollectionRuntimePreview | null;
    }) =>
      invoke<ApplyCollectionResult>('apply_collection', { collectionId, gameId }),
    onSuccess: async (result, variables) => {
      if (variables.targetPreview && !variables.targetPreview.collection.is_last_unsaved) {
        queryClient.setQueryData<CorridorRuntimeSnapshot>(
          corridorRuntimeKeys.snapshot(variables.gameId, variables.safeMode),
          {
            game_id: variables.gameId,
            is_safe: variables.safeMode,
            active_collection_id: variables.collectionId,
            state_name: variables.targetPreview.collection.name,
            state_kind: 'named',
            roots: variables.targetPreview.roots,
            object_states: variables.targetPreview.object_states,
            signature: variables.targetPreview.signature,
            snapshot_source: 'apply_result',
            reconciled_count: 0,
          },
        );
      }

      await invalidateCollectionRuntime(queryClient, { includeMods: true });
      void refetchCollectionRuntime(queryClient, {
        gameId: variables.gameId,
        isSafe: variables.safeMode,
        collectionId: variables.collectionId,
      });

      toast.withAction(
        'success',
        `Applied collection (${result.changed_count} changes)`,
        {
          label: 'Undo',
          onClick: () => {
            undoCollection.mutate({ gameId: variables.gameId });
          },
        },
        7000,
      );
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}
