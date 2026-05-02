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

function buildIdsSignature(ids: Iterable<string> | null): string | null {
  if (!ids) {
    return null;
  }

  return Array.from(ids).join('\0');
}

export function useSearchWorker() {
  const workerRef = useRef<Worker | null>(null);
  const resultSignatureRef = useRef<string | null>(null);
  const querySignatureRef = useRef<string | null>(null);
  const [filteredIds, setFilteredIds] = useState<Set<string> | null>(null);

  useEffect(() => {
    try {
      workerRef.current = new Worker(new URL('../../../workers/searchWorker.ts', import.meta.url), {
        type: 'module',
      });
      workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
        const { ids } = e.data;
        const nextSignature = buildIdsSignature(ids);
        if (resultSignatureRef.current === nextSignature) {
          return;
        }

        resultSignatureRef.current = nextSignature;
        setFilteredIds((current) => {
          if (!ids) {
            return current === null ? current : null;
          }

          const nextSet = new Set(ids);
          return buildIdsSignature(current) === nextSignature ? current : nextSet;
        });
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
    const normalizedQuery = query.trim().toLowerCase();
    const querySignature =
      normalizedQuery.length === 0
        ? null
        : `${normalizedQuery}\0${items.map((item) => `${item.id}:${item.name}`).join('\0')}`;

    if (querySignatureRef.current === querySignature) {
      return;
    }

    querySignatureRef.current = querySignature;

    if (!normalizedQuery) {
      resultSignatureRef.current = null;
      setFilteredIds((current) => (current === null ? current : null));
      return;
    }

    if (workerRef.current) {
      workerRef.current.postMessage({ objects: items, query: normalizedQuery });
    } else {
      // Synchronous fallback (test env)
      const tokens = normalizedQuery.split(/\s+/);
      const ids = items
        .filter((i) => {
          const name = i.name.toLowerCase();
          return tokens.every((t) => name.includes(t));
        })
        .map((i) => i.id);
      const nextSignature = buildIdsSignature(ids);
      if (resultSignatureRef.current === nextSignature) {
        return;
      }

      resultSignatureRef.current = nextSignature;
      setFilteredIds((current) => {
        if (ids.length === 0) {
          return buildIdsSignature(current) === nextSignature ? current : new Set<string>();
        }

        return buildIdsSignature(current) === nextSignature ? current : new Set(ids);
      });
    }
  }, []);

  return { filteredIds, search };
}
