import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createObject, deleteObject, updateObject } from '../lib/services/objectService';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import {
  buildObjectListRefreshDescriptor,
  type CreateObjectInput,
  type UpdateObjectInput,
} from './objectQueryCache';

/** Every object mutation republishes the same object-list scope on success. */
function useObjectListMutation<TVariables, TData>(
  mutationFn: (variables: TVariables) => Promise<TData>,
  options: { publishOnSuccess?: boolean } = {},
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn,
    onSuccess: async () => {
      if (options.publishOnSuccess === false) return;
      await publishRuntimeDescriptor(queryClient, buildObjectListRefreshDescriptor({}), 'active');
    },
  });
}

export function useUpdateObject() {
  return useObjectListMutation(({ id, updates }: { id: string; updates: UpdateObjectInput }) =>
    updateObject(id, updates),
  );
}

export function useDeleteObject(options?: { publishOnSuccess?: boolean }) {
  return useObjectListMutation(
    ({ id, force }: { id: string; force: boolean }) => deleteObject(id, force),
    options,
  );
}

export function useCreateObject() {
  return useObjectListMutation((input: CreateObjectInput) => createObject(input));
}
