const WATCHER_ERROR_DEDUPE_MS = 3_000;
const WATCHER_ERROR_DEDUPE_MAX_ENTRIES = 256;
const watcherErrorSeenAt = new Map<string, number>();

export interface WatchErrorPayload {
  type: string;
  game_id: string;
  error: string;
  path?: string | null;
}

function setBoundedMapEntry<K, V>(map: Map<K, V>, key: K, value: V, maxEntries: number) {
  if (map.has(key)) {
    map.delete(key);
  }
  while (map.size >= maxEntries) {
    const oldestKey = map.keys().next().value as K | undefined;
    if (oldestKey === undefined) {
      break;
    }
    map.delete(oldestKey);
  }
  map.set(key, value);
}

export function isDuplicateWatcherError(payload: WatchErrorPayload, now: number): boolean {
  for (const [key, seenAt] of watcherErrorSeenAt) {
    if (now - seenAt > WATCHER_ERROR_DEDUPE_MS) {
      watcherErrorSeenAt.delete(key);
    }
  }

  const key = `${payload.game_id}\u0000${payload.path ?? ''}\u0000${payload.error}`;
  const previous = watcherErrorSeenAt.get(key);
  setBoundedMapEntry(watcherErrorSeenAt, key, now, WATCHER_ERROR_DEDUPE_MAX_ENTRIES);
  return previous !== undefined && now - previous <= WATCHER_ERROR_DEDUPE_MS;
}
