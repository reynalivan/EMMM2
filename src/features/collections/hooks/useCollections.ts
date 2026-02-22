import { invoke } from '@tauri-apps/api/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import type {
  ApplyCollectionResult,
  Collection,
  CollectionDetails,
  CollectionPreviewMod,
  CreateCollectionInput,
  UpdateCollectionInput,
} from '../../../types/collection';

export const collectionKeys = {
  all: ['collections'] as const,
  list: (gameId: string) => [...collectionKeys.all, gameId] as const,
  preview: (collectionId: string) => [...collectionKeys.all, 'preview', collectionId] as const,
};

export function useCollections(gameId?: string | null) {
  return useQuery<Collection[]>({
    queryKey: collectionKeys.list(gameId ?? ''),
    queryFn: () => invoke<Collection[]>('list_collections', { gameId }),
    enabled: !!gameId,
    staleTime: 10_000,
  });
}

export function useCollectionPreview(collectionId: string | null, gameId: string | null) {
  return useQuery<CollectionPreviewMod[]>({
    queryKey: collectionKeys.preview(collectionId ?? ''),
    queryFn: () =>
      invoke<CollectionPreviewMod[]>('get_collection_preview', { collectionId, gameId }),
    enabled: !!collectionId && !!gameId,
    staleTime: 60_000, // Previews don't change unless collection is updated
  });
}

export function useCreateCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateCollectionInput) =>
      invoke<CollectionDetails>('create_collection', { input }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
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
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      toast.success(`Saved current state as: ${result.collection.name}`);
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
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
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
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      toast.success('Collection deleted');
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useApplyCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ collectionId, gameId }: { collectionId: string; gameId: string }) =>
      invoke<ApplyCollectionResult>('apply_collection', { collectionId, gameId }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: ['mods'] });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      toast.success(`Applied collection (${result.changed_count} changes)`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}
