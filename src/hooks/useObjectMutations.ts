import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createObject, deleteObject, updateObject } from '../lib/services/objectService';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import {
  buildObjectListRefreshDescriptor,
  objectKeys,
  patchObjectListQueries,
  patchObjectSummary,
  restoreObjectListQueries,
  snapshotObjectListQueries,
  type CreateObjectInput,
  type UpdateObjectInput,
} from './objectQueryCache';

export function useUpdateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: UpdateObjectInput }) =>
      updateObject(id, updates),
    onMutate: async ({ id, updates }) => {
      await queryClient.cancelQueries({ queryKey: objectKeys.lists() });
      const previousQueries = snapshotObjectListQueries(queryClient);
      patchObjectListQueries(queryClient, id, (object) => patchObjectSummary(object, updates));

      return { previousQueries };
    },
    onError: (_error, _variables, context) => {
      if (!context?.previousQueries) {
        return;
      }

      restoreObjectListQueries(queryClient, context.previousQueries);
    },
    onSuccess: async () => {
      await publishRuntimeDescriptor(
        queryClient,
        buildObjectListRefreshDescriptor({}),
        'active',
      );
    },
  });
}

export function useDeleteObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, force }: { id: string; force: boolean }) => deleteObject(id, force),
    onSuccess: async () => {
      await publishRuntimeDescriptor(
        queryClient,
        buildObjectListRefreshDescriptor({}),
        'active',
      );
    },
  });
}

export function useCreateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateObjectInput) => createObject(input),
    onSuccess: async () => {
      await publishRuntimeDescriptor(
        queryClient,
        buildObjectListRefreshDescriptor({}),
        'active',
      );
    },
  });
}
