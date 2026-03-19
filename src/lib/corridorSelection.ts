export type CollectionWorkspaceSource =
  | { kind: 'current_runtime' }
  | { kind: 'stored_collection'; collection_id: string };

export type CorridorSelectionMap = Record<string, CollectionWorkspaceSource>;

export function buildCorridorSelectionKey(gameId: string, isSafe: boolean): string {
  const corridorLabel = isSafe ? 'safe' : 'unsafe';
  return `${gameId}::${corridorLabel}`;
}

export function getWorkspaceSelectionForCorridor(
  selections: CorridorSelectionMap,
  gameId: string | null | undefined,
  isSafe: boolean,
): CollectionWorkspaceSource | null {
  if (!gameId) {
    return null;
  }

  return selections[buildCorridorSelectionKey(gameId, isSafe)] ?? null;
}

export function normalizePersistedWorkspaceSelections(
  persistedValue: unknown,
): CorridorSelectionMap {
  if (!persistedValue || typeof persistedValue !== 'object') {
    return {};
  }

  const entries = Object.entries(persistedValue as Record<string, unknown>);
  const normalizedEntries = entries.flatMap(([key, value]) => {
    if (!value || typeof value !== 'object') {
      return [];
    }

    const maybeSelection = value as Partial<CollectionWorkspaceSource>;
    if (maybeSelection.kind === 'current_runtime') {
      return [[key, { kind: 'current_runtime' } satisfies CollectionWorkspaceSource]];
    }

    if (
      maybeSelection.kind === 'stored_collection' &&
      typeof maybeSelection.collection_id === 'string' &&
      maybeSelection.collection_id.trim() !== ''
    ) {
      return [[
        key,
        {
          kind: 'stored_collection',
          collection_id: maybeSelection.collection_id,
        } satisfies CollectionWorkspaceSource,
      ]];
    }

    return [];
  });

  return Object.fromEntries(normalizedEntries);
}
