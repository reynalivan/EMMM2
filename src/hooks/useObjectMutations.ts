import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createObject, deleteObject, updateObject } from '../lib/services/objectService';
import { publishRuntimeDescriptor } from '../features/runtime-sync/queryRefresh';
import {
  buildObjectListRefreshDescriptor,
  type CreateObjectInput,
  type UpdateObjectInput,
} from './objectQueryCache';

export function useUpdateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, updates }: { id: string; updates: UpdateObjectInput }) =>
      updateObject(id, updates),
    onSuccess: async () => {
      await publishRuntimeDescriptor(queryClient, buildObjectListRefreshDescriptor({}), 'active');
    },
  });
}

export function useDeleteObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, force }: { id: string; force: boolean }) => deleteObject(id, force),
    onSuccess: async () => {
      await publishRuntimeDescriptor(queryClient, buildObjectListRefreshDescriptor({}), 'active');
    },
  });
}

export function useCreateObject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateObjectInput) => createObject(input),
    onSuccess: async () => {
      await publishRuntimeDescriptor(queryClient, buildObjectListRefreshDescriptor({}), 'active');
    },
  });
}
