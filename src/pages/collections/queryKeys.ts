// ---------------------------------------------------------------------------
// v2 Query Key Factory — Single source of truth for all v2 query cache keys
// ---------------------------------------------------------------------------

export const collectionRuntimeKeys = {
  all: ['v2-collection-runtime'] as const,
  state: (gameId: string) => [...collectionRuntimeKeys.all, 'state', gameId] as const,
  descriptor: (gameId: string) => [...collectionRuntimeKeys.all, 'descriptor', gameId] as const,
};

export const collectionKeys = {
  all: ['v2-collections'] as const,
  list: (gameId: string) => [...collectionKeys.all, 'list', gameId] as const,
  preview: (collectionId: string) => [...collectionKeys.all, 'preview', collectionId] as const,
  previewApply: (collectionId: string) =>
    [...collectionKeys.all, 'previewApply', collectionId] as const,
  applyProgress: (gameId: string) => [...collectionKeys.all, 'apply-progress', gameId] as const,
};
