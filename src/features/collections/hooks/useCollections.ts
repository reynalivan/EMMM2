/**
 * v2 Collection hooks — Query + Mutation hooks for the greenfield collection system.
 *
 * Replaces: useCollections, useCollectionRuntimePreview, useCreateCollection,
 *           useSaveCurrentAsCollection, useSaveSnapshotCollectionAsNamed,
 *           useUpdateCollection, useDeleteCollection, useApplyCollection, useUndoCollection.
 *
 * Key simplification: Backend now handles corridor filtering, so list queries
 * don't need isSafe — the backend reads it from config.
 */

import { useMutation, useQuery, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import { collectionKeys, corridorKeys } from '../queryKeys';
import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import type {
  CollectionSummary,
  CollectionPreview,
  ApplyPreview,
  ApplyResult,
} from '../../../types/collection';

// ── Query Hooks ────────────────────────────────────────────────────────────

/** List all collections for the current game + corridor. */
export function useCollections(gameId: string | null, isSafe: boolean) {
  return useQuery<CollectionSummary[]>({
    queryKey: collectionKeys.list(gameId ?? '', isSafe),
    queryFn: () => commands.listCollections({ gameId: gameId ?? '' }),
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
    queryKey: [...collectionKeys.previewApply(collectionId ?? ''), isSafe],
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
    mutationFn: ({ gameId, name }: { gameId: string; name: string }) =>
      commands.createCollection({ gameId, name }),

    onSuccess: (result: CollectionSummary) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      toast.success(`Created collection: ${result.name}`);
    },

    onError: (err: unknown) => {
      toast.error(String(err));
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

    onSuccess: (result: CollectionSummary) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      toast.success(`Updated collection: ${result.name}`);
    },

    onError: (err: unknown) => {
      toast.error(String(err));
    },
  });
}

/** Delete a collection. */
export function useDeleteCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, id }: { gameId: string; id: string }) =>
      commands.deleteCollection({ gameId, id }),

    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      toast.success('Collection deleted');
    },

    onError: (err: unknown) => {
      toast.error(String(err));
    },
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
    }) =>
      commands.applyCollection({
        gameId,
        collectionId,
        ignoreMissing: ignoreMissing ?? false,
      }),

    onSuccess: (result: ApplyResult) => {
      // Invalidate everything that might have changed
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard'] });

      const total = result.mods_enabled + result.mods_disabled;
      toast.success(`Applied collection (${total} changes)`);
    },

    onError: (err: unknown, variables) => {
      const errStr = String(err);
      if (errStr.includes('"type":"FileInUse"')) {
        try {
          const body = JSON.parse(errStr);
          const payload = body.payload;
          useAppStore
            .getState()
            .openFileInUseDialog(payload.path, payload.processes, () =>
              mutation.mutate(variables),
            );
          return;
        } catch {
          /* parse failed */
        }
      }
      toast.error(errStr);
    },
  });

  return mutation;
}


/** Undo the last collection application. */
export function useUndoCollection() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ gameId }: { gameId: string }) => commands.undoCollection({ gameId }),


    onSuccess: (result: ApplyResult) => {
      // Invalidate everything
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });

      const total = result.mods_enabled + result.mods_disabled;
      toast.success(`Undid collection application (${total} changes reverted)`);
    },

    onError: (err: unknown, variables) => {
      const errStr = String(err);
      if (errStr.includes('"type":"FileInUse"')) {
        try {
          const body = JSON.parse(errStr);
          const payload = body.payload;
          useAppStore
            .getState()
            .openFileInUseDialog(payload.path, payload.processes, () =>
              mutation.mutate(variables),
            );
          return;
        } catch {
          /* parse failed */
        }
      }
      toast.error(errStr);
    },
  });

  return mutation;
}
