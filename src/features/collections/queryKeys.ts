// ---------------------------------------------------------------------------
// v2 Query Key Factory — Single source of truth for all v2 query cache keys
// ---------------------------------------------------------------------------

export const corridorKeys = {
  all: ['v2-corridor'] as const,
  state: (gameId: string) => [...corridorKeys.all, 'state', gameId] as const,
};

export const collectionKeys = {
  all: ['v2-collections'] as const,
  list: (gameId: string) => [...collectionKeys.all, 'list', gameId] as const,
  preview: (collectionId: string) => [...collectionKeys.all, 'preview', collectionId] as const,
  previewApply: (collectionId: string) =>
    [...collectionKeys.all, 'previewApply', collectionId] as const,
  applyProgress: (gameId: string) => [...collectionKeys.all, 'apply-progress', gameId] as const,
};
