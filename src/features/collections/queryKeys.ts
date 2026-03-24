// ---------------------------------------------------------------------------
// v2 Query Key Factory — Single source of truth for all v2 query cache keys
// ---------------------------------------------------------------------------

export const corridorKeys = {
  all: ['v2-corridor'] as const,
  state: (gameId: string, safeMode: boolean) =>
    [...corridorKeys.all, 'state', gameId, safeMode] as const,
};

export const collectionKeys = {
  all: ['v2-collections'] as const,
  list: (gameId: string, isSafe: boolean) => [...collectionKeys.all, 'list', gameId, isSafe] as const,
  preview: (collectionId: string) => [...collectionKeys.all, 'preview', collectionId] as const,
  previewApply: (collectionId: string) =>
    [...collectionKeys.all, 'previewApply', collectionId] as const,
};

export const pinKeys = {
  all: ['v2-pin'] as const,
  status: () => [...pinKeys.all, 'status'] as const,
  hasPin: () => [...pinKeys.all, 'has-pin'] as const,
};
