import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  commands,
  type OnboardingIndexingBackgroundGameStatus,
  type OnboardingIndexingBackgroundStatus,
  type OnboardingIndexingSnapshotProgress,
} from '@/shared/api/tauri/bindings';

export interface BackgroundIndexingStatusState {
  isLoaded: boolean;
  loadError: boolean;
  sessions: OnboardingIndexingBackgroundStatus[];
  gamesById: ReadonlyMap<string, OnboardingIndexingBackgroundGameStatus>;
  snapshotProgressByGame: ReadonlyMap<string, OnboardingIndexingSnapshotProgress>;
  refresh: () => Promise<void>;
}

function snapshotKey(sessionId: string, gameId: string): string {
  return JSON.stringify([sessionId, gameId]);
}

export function useBackgroundIndexingStatus(): BackgroundIndexingStatusState {
  const [sessions, setSessions] = useState<OnboardingIndexingBackgroundStatus[]>([]);
  const [snapshotProgressBySession, setSnapshotProgressBySession] = useState(
    new Map<string, OnboardingIndexingSnapshotProgress>(),
  );
  const [isLoaded, setIsLoaded] = useState(false);
  const [loadError, setLoadError] = useState(false);
  const mountedRef = useRef(false);
  const eventSequenceRef = useRef(0);
  const refreshSequenceRef = useRef(0);
  const latestEventsRef = useRef(
    new Map<string, { sequence: number; status: OnboardingIndexingBackgroundStatus }>(),
  );

  const refresh = useCallback(async () => {
    const refreshSequence = ++refreshSequenceRef.current;
    const eventSequenceAtStart = eventSequenceRef.current;
    try {
      const statuses = await commands.getOnboardingIndexingBackgroundStatus();
      if (!mountedRef.current || refreshSequence !== refreshSequenceRef.current) return;
      const sessionsById = new Map(
        (Array.isArray(statuses) ? statuses : []).map((status) => [status.session_id, status]),
      );
      for (const { sequence, status } of latestEventsRef.current.values()) {
        if (sequence > eventSequenceAtStart) {
          sessionsById.set(status.session_id, status);
        }
      }
      setSessions([...sessionsById.values()]);
      setLoadError(false);
    } catch {
      if (!mountedRef.current || refreshSequence !== refreshSequenceRef.current) return;
      setLoadError(true);
    } finally {
      if (mountedRef.current && refreshSequence === refreshSequenceRef.current) {
        setIsLoaded(true);
      }
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    mountedRef.current = true;

    void listen<OnboardingIndexingSnapshotProgress>(
      'onboarding_indexing:snapshot_progress',
      ({ payload }) => {
        if (!mounted) return;
        setSnapshotProgressBySession((current) => {
          const key = snapshotKey(payload.session_id, payload.game_id);
          const previous = current.get(key);
          if (previous) {
            if (previous.elapsed_ms > payload.elapsed_ms) return current;
            if (
              previous.elapsed_ms === payload.elapsed_ms &&
              previous.completed_roots > payload.completed_roots
            ) {
              return current;
            }
          }
          const next = new Map(current);
          next.set(key, payload);
          return next;
        });
      },
    )
      .then((stop) => {
        if (mounted) unlistenProgress = stop;
        else stop();
      })
      .catch((error: unknown) => {
        if (mounted) console.error('Failed to subscribe to onboarding indexing progress', error);
      });

    void listen<OnboardingIndexingBackgroundStatus>(
      'onboarding_indexing:background_status',
      ({ payload }) => {
        if (!mounted) return;
        latestEventsRef.current.set(payload.session_id, {
          sequence: ++eventSequenceRef.current,
          status: payload,
        });
        setSessions((current) => {
          const otherSessions = current.filter(
            (session) => session.session_id !== payload.session_id,
          );
          return [...otherSessions, payload];
        });
        setLoadError(false);
        setIsLoaded(true);
      },
    )
      .then((stop) => {
        if (mounted) {
          unlisten = stop;
          void refresh();
        } else {
          stop();
        }
      })
      .catch((error: unknown) => {
        if (!mounted) return;
        console.error('Failed to subscribe to background indexing status', error);
        void refresh();
      });

    return () => {
      mounted = false;
      mountedRef.current = false;
      unlisten?.();
      unlistenProgress?.();
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

  const snapshotProgressByGame = useMemo(() => {
    const progress = new Map<string, OnboardingIndexingSnapshotProgress>();
    const gamesWithSession = new Set(gamesById.keys());
    for (const snapshot of snapshotProgressBySession.values()) {
      if (!gamesWithSession.has(snapshot.game_id)) {
        progress.set(snapshot.game_id, snapshot);
      }
    }
    for (const session of sessions) {
      for (const game of session.games) {
        const snapshot = snapshotProgressBySession.get(
          snapshotKey(session.session_id, game.game_id),
        );
        if (snapshot) {
          progress.set(game.game_id, snapshot);
        } else {
          progress.delete(game.game_id);
        }
      }
    }
    return progress;
  }, [gamesById, sessions, snapshotProgressBySession]);

  return { isLoaded, loadError, sessions, gamesById, snapshotProgressByGame, refresh };
}
