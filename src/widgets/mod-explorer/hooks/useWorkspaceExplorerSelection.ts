import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { WorkspaceExplorerQuery, WorkspaceExplorerSelectionModel } from '@/entities/workspace';
import {
  isWorkspaceExplorerPathSelected,
  workspaceExplorerQueryScopeKey,
  workspaceExplorerSelectionCount,
} from '@/entities/workspace';

interface UseWorkspaceExplorerSelectionOptions {
  query: WorkspaceExplorerQuery | null;
  /** Changes immediately when an uncommitted search alters selection scope. */
  selectionScopeKey?: string;
  listingRevision: string | null;
  totalMatching: number;
  loadedPaths: readonly string[];
  explicitPaths: Set<string>;
  setExplicitPaths: (paths: Set<string>) => void;
  clearExplicitPaths: () => void;
}

export function useWorkspaceExplorerSelection({
  query,
  selectionScopeKey,
  listingRevision,
  totalMatching,
  loadedPaths,
  explicitPaths,
  setExplicitPaths,
  clearExplicitPaths,
}: UseWorkspaceExplorerSelectionOptions) {
  const [allMatching, setAllMatching] = useState<Extract<
    WorkspaceExplorerSelectionModel,
    { mode: 'all_matching' }
  > | null>(null);
  const scopeKey = query ? workspaceExplorerQueryScopeKey(query) : null;
  const contextKey = JSON.stringify([scopeKey, selectionScopeKey ?? null]);
  const selectionSnapshotKey = JSON.stringify([contextKey, listingRevision]);
  const currentSelectionSnapshotKey = useRef(selectionSnapshotKey);
  const currentContextKey = useRef(contextKey);
  useLayoutEffect(() => {
    currentSelectionSnapshotKey.current = selectionSnapshotKey;
    currentContextKey.current = contextKey;
  }, [contextKey, selectionSnapshotKey]);
  const previousContextKey = useRef(contextKey);
  const previousListingRevision = useRef(listingRevision);

  useEffect(() => {
    if (previousContextKey.current !== contextKey) {
      previousContextKey.current = contextKey;
      setAllMatching(null);
      clearExplicitPaths();
    } else if (previousListingRevision.current !== listingRevision) {
      setAllMatching(null);
    }

    previousListingRevision.current = listingRevision;
  }, [clearExplicitPaths, contextKey, listingRevision]);

  useEffect(() => {
    if (allMatching && explicitPaths.size > 0) {
      setAllMatching(null);
    }
  }, [allMatching, explicitPaths]);

  useEffect(() => {
    setAllMatching((current) => {
      if (!current) {
        return current;
      }
      let excludedPaths = current.excludedPaths;
      if (loadedPaths.length >= totalMatching && excludedPaths.size > 0) {
        const matchingPaths = new Set(loadedPaths);
        const retainedPaths = [...excludedPaths].filter((path) => matchingPaths.has(path));
        if (retainedPaths.length !== excludedPaths.size) {
          excludedPaths = new Set(retainedPaths);
        }
      }
      return current.totalMatching !== totalMatching || excludedPaths !== current.excludedPaths
        ? { ...current, totalMatching, excludedPaths }
        : current;
    });
  }, [loadedPaths, totalMatching]);

  const selection = useMemo<WorkspaceExplorerSelectionModel>(
    () => allMatching ?? { mode: 'explicit', paths: explicitPaths },
    [allMatching, explicitPaths],
  );
  const selectedCount = workspaceExplorerSelectionCount(selection);

  const createAllMatchingCommit = useCallback(
    (
      nextSelection: Extract<WorkspaceExplorerSelectionModel, { mode: 'all_matching' }> | null,
      validateListingRevision = true,
    ) => {
      const capturedSnapshotKey = selectionSnapshotKey;
      const capturedContextKey = contextKey;
      const isCurrent = () =>
        currentSelectionSnapshotKey.current === capturedSnapshotKey ||
        (!validateListingRevision && currentContextKey.current === capturedContextKey);
      return {
        isCurrent,
        onApplied: () => {
          if (isCurrent()) {
            setAllMatching(nextSelection);
          }
        },
      };
    },
    [contextKey, selectionSnapshotKey],
  );

  const clearSelection = useCallback(() => {
    setAllMatching(null);
    clearExplicitPaths();
  }, [clearExplicitPaths]);

  const replaceExplicitPaths = useCallback(
    (paths: Iterable<string>) => {
      setAllMatching(null);
      setExplicitPaths(new Set(paths));
    },
    [setExplicitPaths],
  );

  const addPaths = useCallback(
    (paths: Iterable<string>) => {
      if (allMatching) {
        const nextExcluded = new Set(allMatching.excludedPaths);
        for (const path of paths) {
          nextExcluded.delete(path);
        }
        setAllMatching({ ...allMatching, excludedPaths: nextExcluded });
        return;
      }

      const nextPaths = new Set(explicitPaths);
      for (const path of paths) {
        nextPaths.add(path);
      }
      setExplicitPaths(nextPaths);
    },
    [allMatching, explicitPaths, setExplicitPaths],
  );

  const removePaths = useCallback(
    (paths: Iterable<string>) => {
      if (allMatching) {
        const nextExcluded = new Set(allMatching.excludedPaths);
        for (const path of paths) {
          nextExcluded.add(path);
        }
        setAllMatching({ ...allMatching, excludedPaths: nextExcluded });
        return;
      }

      const nextPaths = new Set(explicitPaths);
      for (const path of paths) {
        nextPaths.delete(path);
      }
      setExplicitPaths(nextPaths);
    },
    [allMatching, explicitPaths, setExplicitPaths],
  );

  const togglePath = useCallback(
    (path: string, multi: boolean) => {
      if (allMatching && multi) {
        const nextExcluded = new Set(allMatching.excludedPaths);
        if (nextExcluded.has(path)) {
          nextExcluded.delete(path);
        } else {
          nextExcluded.add(path);
        }
        setAllMatching({ ...allMatching, excludedPaths: nextExcluded });
        return;
      }

      const nextPaths = new Set(multi ? explicitPaths : []);
      if (nextPaths.has(path)) {
        nextPaths.delete(path);
      } else {
        nextPaths.add(path);
      }
      replaceExplicitPaths(nextPaths);
    },
    [allMatching, explicitPaths, replaceExplicitPaths],
  );

  const selectAllMatching = useCallback(() => {
    if (!query || !listingRevision || totalMatching === 0) {
      clearSelection();
      return;
    }

    setExplicitPaths(new Set());
    setAllMatching({
      mode: 'all_matching',
      query,
      listingRevision,
      excludedPaths: new Set(),
      totalMatching,
    });
  }, [clearSelection, listingRevision, query, setExplicitPaths, totalMatching]);

  const isPathSelected = useCallback(
    (path: string) => isWorkspaceExplorerPathSelected(selection, path),
    [selection],
  );

  return {
    selection,
    selectedCount,
    isPathSelected,
    clearSelection,
    replaceExplicitPaths,
    addPaths,
    removePaths,
    togglePath,
    selectAllMatching,
    createAllMatchingCommit,
  };
}
