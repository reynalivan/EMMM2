export const collectionKeys = {
  all: ['collections'] as const,
  list: (gameId: string) => [...collectionKeys.all, gameId] as const,
  runtimePreview: (collectionId: string) =>
    [...collectionKeys.all, 'runtime-preview', collectionId] as const,
};

export const corridorRuntimeKeys = {
  all: ['corridor-runtime'] as const,
  snapshot: (gameId: string, isSafe: boolean) =>
    [...corridorRuntimeKeys.all, gameId, isSafe] as const,
};

export const corridorPreviewKeys = {
  all: ['corridor-preview'] as const,
  detail: (
    gameId: string,
    currentSafeMode: boolean,
    targetSafeMode: boolean,
    currentStateToken: string,
  ) =>
    [
      ...corridorPreviewKeys.all,
      gameId,
      currentSafeMode,
      targetSafeMode,
      currentStateToken,
    ] as const,
};

export const applyProgressKeys = {
  all: ['apply-progress'] as const,
  detail: (gameId: string) => [...applyProgressKeys.all, gameId] as const,
};
