import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createObject, deleteObject, updateObject } from '@/entities/game-object';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { openFileInUseRetryDialog } from './fileInUseRetry';
import { useAppStore } from '@/app/store';
import {
  buildObjectListRefreshDescriptor,
  type CreateObjectInput,
  type UpdateObjectInput,
} from './objectMutationCache';

/** Every object mutation republishes the same object-list scope on success. */
function useObjectListMutation<TVariables, TData>(
  mutationFn: (variables: TVariables) => Promise<TData>,
  options: { publishOnSuccess?: boolean; recoverFileLocks?: boolean } = {},
) {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn,
    onMutate: () => ({ gameId: useAppStore.getState().activeGameId }),
    onSuccess: async () => {
      if (options.publishOnSuccess === false) return;
      await publishRuntimeDescriptor(queryClient, buildObjectListRefreshDescriptor({}), 'active');
    },
    onError: (error, variables, context) => {
      if (context?.gameId !== useAppStore.getState().activeGameId) return;
      if (options.recoverFileLocks)
        openFileInUseRetryDialog(error, variables, mutation.mutateAsync);
    },
  });
  return mutation;
}

export function useUpdateObject() {
  return useObjectListMutation(({ id, updates }: { id: string; updates: UpdateObjectInput }) =>
    updateObject(id, updates),
  );
}

export function useDeleteObject(options?: { publishOnSuccess?: boolean }) {
  return useObjectListMutation(
    ({ id, force, recoveryId }: { id: string; force: boolean; recoveryId?: string }) =>
      deleteObject(id, force, recoveryId),
    { ...options, recoverFileLocks: true },
  );
}

export function useCreateObject() {
  return useObjectListMutation((input: CreateObjectInput) => createObject(input));
}
