import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  commands,
  type OnboardingIndexingBackgroundGameStatus,
  type OnboardingIndexingBackgroundStatus,
} from '@/shared/api/tauri/bindings';

export interface BackgroundIndexingStatusState {
  isLoaded: boolean;
  loadError: boolean;
  sessions: OnboardingIndexingBackgroundStatus[];
  gamesById: ReadonlyMap<string, OnboardingIndexingBackgroundGameStatus>;
  refresh: () => Promise<void>;
}

export function useBackgroundIndexingStatus(): BackgroundIndexingStatusState {
  const [sessions, setSessions] = useState<OnboardingIndexingBackgroundStatus[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);
  const [loadError, setLoadError] = useState(false);
  const mountedRef = useRef(false);

  const refresh = useCallback(async () => {
    try {
      const statuses = await commands.getOnboardingIndexingBackgroundStatus();
      if (!mountedRef.current) return;
      setSessions(Array.isArray(statuses) ? statuses : []);
      setLoadError(false);
    } catch {
      if (!mountedRef.current) return;
      setLoadError(true);
    } finally {
      if (mountedRef.current) {
        setIsLoaded(true);
      }
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;
    mountedRef.current = true;

    void refresh();

    void listen<OnboardingIndexingBackgroundStatus>(
      'onboarding_indexing:background_status',
      ({ payload }) => {
        if (!mounted) return;
        setSessions((current) => {
          const otherSessions = current.filter((session) => session.session_id !== payload.session_id);
          return [...otherSessions, payload];
        });
        setLoadError(false);
        setIsLoaded(true);
      },
    ).then((stop) => {
      if (mounted) {
        unlisten = stop;
      } else {
        stop();
      }
    });

    return () => {
      mounted = false;
      mountedRef.current = false;
      unlisten?.();
    };
  }, [refresh]);

  const gamesById = useMemo(() => {
    const games = new Map<string, OnboardingIndexingBackgroundGameStatus>();
    for (const session of sessions) {
      for (const game of session.games) {
        games.set(game.game_id, game);
      }
    }
    return games;
  }, [sessions]);

  return { isLoaded, loadError, sessions, gamesById, refresh };
}
