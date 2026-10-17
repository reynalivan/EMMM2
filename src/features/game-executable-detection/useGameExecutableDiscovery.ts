import { useCallback, useEffect, useRef, useState } from 'react';
import { commands } from '@/shared/api/tauri/bindings';
import type {
  GameExecutableCandidate,
  GameExecutableDiscoveryIssue,
} from '@/shared/api/tauri/bindings.gen';
import { formatAppError } from '@/shared/lib/appError';

export interface ExecutableDiscoveryController {
  candidates: GameExecutableCandidate[];
  issues: GameExecutableDiscoveryIssue[];
  pendingTypes: number[];
  completedTypes: number[];
  error: string | null;
  scan: (gameTypes: number[]) => Promise<void>;
  cancel: () => void;
}

const EMPTY_ROOTS: string[] = [];

export function useGameExecutableDiscovery(
  xxmiRoots: string[] = EMPTY_ROOTS,
): ExecutableDiscoveryController {
  const [candidates, setCandidates] = useState<GameExecutableCandidate[]>([]);
  const [issues, setIssues] = useState<GameExecutableDiscoveryIssue[]>([]);
  const [pendingTypes, setPendingTypes] = useState<number[]>([]);
  const [completedTypes, setCompletedTypes] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const rootsKey = JSON.stringify([...new Set(xxmiRoots.filter(Boolean))].sort());

  const cancel = useCallback(() => {
    generation.current += 1;
    setPendingTypes([]);
  }, []);

  useEffect(() => {
    generation.current += 1;
    setCandidates([]);
    setIssues([]);
    setPendingTypes([]);
    setCompletedTypes([]);
    setError(null);
    return () => {
      generation.current += 1;
    };
  }, [rootsKey]);

  const scan = useCallback(
    async (gameTypes: number[]) => {
      const types = [...new Set(gameTypes)];
      if (types.length === 0) return;
      const request = ++generation.current;
      setPendingTypes(types);
      setCandidates((current) =>
        current.filter((candidate) => !types.includes(candidate.game_type)),
      );
      setCompletedTypes((current) => current.filter((type) => !types.includes(type)));
      setIssues([]);
      setError(null);
      try {
        const result = await commands.discoverGameExecutables({
          game_types: types,
          xxmi_roots: JSON.parse(rootsKey) as string[],
        });
        if (request !== generation.current) return;
        setCandidates((current) => [
          ...current.filter((candidate) => !types.includes(candidate.game_type)),
          ...result.candidates,
        ]);
        setIssues(result.issues);
        setCompletedTypes((current) => [...new Set([...current, ...types])]);
      } catch (scanError) {
        if (request === generation.current) setError(formatAppError(scanError));
      } finally {
        if (request === generation.current) setPendingTypes([]);
      }
    },
    [rootsKey],
  );

  return { candidates, issues, pendingTypes, completedTypes, error, scan, cancel };
}
