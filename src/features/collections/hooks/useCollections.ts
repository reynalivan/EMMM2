import { invoke } from '@tauri-apps/api/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import type {
  ApplyCollectionResult,
  Collection,
  CollectionDetails,
  CreateCollectionInput,
  ExportCollectionPayload,
  ImportCollectionResult,
  UndoCollectionResult,
  UpdateCollectionInput,
} from '../../../types/collection';

export const collectionKeys = {
  all: ['collections'] as const,
  list: (gameId: string) => [...collectionKeys.all, gameId] as const,
};

export function useCollections(gameId?: string | null) {
  return useQuery<Collection[]>({
    queryKey: collectionKeys.list(gameId ?? ''),
    queryFn: () => invoke<Collection[]>('list_collections', { gameId }),
    enabled: !!gameId,
    staleTime: 10_000,
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
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      toast.success(`Applied collection (${result.changed_count} changes)`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useUndoCollectionApply() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (gameId: string) =>
      invoke<UndoCollectionResult>('undo_collection_apply', { gameId }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      toast.success(`Undo complete (${result.restored_count} restored)`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useExportCollection() {
  return useMutation({
    mutationFn: ({ collectionId, gameId }: { collectionId: string; gameId: string }) =>
      invoke<ExportCollectionPayload>('export_collection', { collectionId, gameId }),
    onError: (err) => {
      toast.error(String(err));
    },
  });
}

export function useImportCollection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, payload }: { gameId: string; payload: ExportCollectionPayload }) =>
      invoke<ImportCollectionResult>('import_collection', { gameId, payload }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      if (result.missing.length > 0) {
        toast.warning(`Imported ${result.imported_count} items, ${result.missing.length} missing`);
        return;
      }
      toast.success(`Imported collection (${result.imported_count} items)`);
    },
    onError: (err) => {
      toast.error(String(err));
    },
  });
}
