/**
 * US-3.6: Hook for Web Worker search with synchronous fallback.
 * Manages worker lifecycle and provides an imperative `search()` API.
 * Falls back to main-thread filtering when Worker is unavailable (e.g., test env).
 */

import { useRef, useEffect, useState, useCallback } from 'react';

interface SearchableItem {
  id: string;
  name: string;
}

interface WorkerResponse {
  ids: string[] | null;
}

export function useSearchWorker() {
  const workerRef = useRef<Worker | null>(null);
  const [filteredIds, setFilteredIds] = useState<Set<string> | null>(null);

  useEffect(() => {
    try {
      workerRef.current = new Worker(new URL('../../../workers/searchWorker.ts', import.meta.url), {
        type: 'module',
      });
      workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
        const { ids } = e.data;
        setFilteredIds(ids ? new Set(ids) : null);
      };
    } catch {
      // Worker not available (test env, SSR, etc.) — sync fallback used
      workerRef.current = null;
    }

    return () => {
      workerRef.current?.terminate();
    };
  }, []);

  const search = useCallback((items: SearchableItem[], query: string) => {
    if (!query || query.trim().length === 0) {
      setFilteredIds(null);
      return;
    }

    if (workerRef.current) {
      workerRef.current.postMessage({ objects: items, query });
    } else {
      // Synchronous fallback (test env)
      const tokens = query.toLowerCase().trim().split(/\s+/);
      const ids = items
        .filter((i) => {
          const name = i.name.toLowerCase();
          return tokens.every((t) => name.includes(t));
        })
        .map((i) => i.id);
      setFilteredIds(new Set(ids));
    }
  }, []);

  return { filteredIds, search };
}
