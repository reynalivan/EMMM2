import { useEffect, useRef, useState } from 'react';
import { formatAppError } from '../../../shared/lib/appError';
import {
  commands,
  type FolderConflictSummary,
  type FolderNameConflictGroup,
} from '../../../shared/api/tauri/bindings';

export function useFolderConflictDetails(
  gameId: string | null,
  group: FolderNameConflictGroup | null,
  enabled = true,
) {
  const [details, setDetails] = useState<Record<string, FolderConflictSummary>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [requestVersion, setRequestVersion] = useState(0);
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (!enabled || !gameId || !group) {
      setDetails({});
      setError(null);
      setLoading(false);
      return;
    }

    const requestId = ++requestIdRef.current;
    setDetails({});
    setError(null);
    setLoading(true);
    commands
      .getFolderConflictDetails(
        gameId,
        group.candidates.map((candidate) => candidate.path),
      )
      .then((result) => {
        if (requestIdRef.current === requestId) {
          setDetails(Object.fromEntries(result.map((detail) => [detail.path, detail])));
        }
      })
      .catch((requestError) => {
        if (requestIdRef.current === requestId) setError(formatAppError(requestError));
      })
      .finally(() => {
        if (requestIdRef.current === requestId) setLoading(false);
      });

    return () => {
      if (requestIdRef.current === requestId) requestIdRef.current += 1;
    };
  }, [enabled, gameId, group, requestVersion]);

  return {
    details,
    loading,
    error,
    retry: () => setRequestVersion((version) => version + 1),
  };
}
