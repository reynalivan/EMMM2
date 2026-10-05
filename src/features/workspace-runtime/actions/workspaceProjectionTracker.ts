interface ProjectionSnapshot {
  game_id: string;
  source_epoch: string;
  projected_revision: number;
  projection_repair_reason?: string | null;
}

interface ProjectionWaiter {
  revision: number;
  resolve: () => void;
  reject: (error: Error) => void;
}

interface ProjectionTracker {
  epoch?: string;
  projected: number;
  waiters: Set<ProjectionWaiter>;
  wake?: () => void;
}

const SNAPSHOT_TIMEOUT_MS = 2_000;
const INITIAL_RETRY_MS = 250;
const MAX_RETRY_MS = 30_000;

export class WorkspaceRootEpochChangedError extends Error {
  constructor() {
    super('Workspace root epoch changed');
  }
}

export class WorkspaceProjectionNeedsRepairError extends Error {
  constructor(reason: string) {
    super(reason);
  }
}

/** One polling owner per game; root replacement cannot acknowledge old receipts. */
export class WorkspaceProjectionTracker {
  private readonly trackers = new Map<string, ProjectionTracker>();
  private readonly checkpoints = new Map<string, { epoch?: string; revision: number }>();
  private readonly epochs = new Map<string, string>();
  private readonly epochListeners = new Set<(gameId: string, epoch: string) => void>();
  private readonly priming = new Map<string, Promise<void>>();
  private readonly lifecycleVersions = new Map<string, number>();
  private readonly snapshotsInFlight = new Map<
    string,
    {
      version: number;
      promise: Promise<ProjectionSnapshot>;
    }
  >();

  constructor(
    private readonly snapshot: (gameId: string) => Promise<ProjectionSnapshot>,
    private readonly isCurrent: (gameId: string) => boolean,
  ) {}

  onEpochChange(listener: (gameId: string, epoch: string) => void): () => void {
    this.epochListeners.add(listener);
    return () => this.epochListeners.delete(listener);
  }

  acceptsEpoch(gameId: string, epoch: string | undefined): boolean {
    const known = this.epochs.get(gameId);
    return epoch === undefined || known === undefined || known === epoch;
  }

  async verifyEpoch(gameId: string, epoch: string): Promise<void> {
    if (this.epochs.get(gameId) === epoch) return;
    const snapshot = await this.readSnapshot(gameId);
    if (!this.isCurrent(gameId)) return;
    if (snapshot.game_id !== gameId) throw new WorkspaceRootEpochChangedError();
    this.observeEpoch(gameId, snapshot.source_epoch);
    if (snapshot.source_epoch !== epoch) throw new WorkspaceRootEpochChangedError();
    this.record(gameId, snapshot.projected_revision, epoch);
  }

  invalidate(gameId: string): void {
    this.lifecycleVersions.set(gameId, (this.lifecycleVersions.get(gameId) ?? 0) + 1);
    this.priming.delete(gameId);
    const tracker = this.trackers.get(gameId);
    if (tracker) {
      for (const waiter of tracker.waiters) waiter.reject(new WorkspaceRootEpochChangedError());
      tracker.waiters.clear();
      tracker.wake?.();
      this.trackers.delete(gameId);
    }
    this.checkpoints.delete(gameId);
    this.epochs.delete(gameId);
  }

  prime(gameId: string): Promise<void> {
    const current = this.priming.get(gameId);
    if (current) return current;
    if (this.trackers.has(gameId)) return Promise.resolve();
    const version = this.lifecycleVersions.get(gameId) ?? 0;
    const priming = this.readSnapshot(gameId)
      .then((snapshot) => {
        if (
          this.isCurrent(gameId) &&
          snapshot.game_id === gameId &&
          version === (this.lifecycleVersions.get(gameId) ?? 0)
        ) {
          this.observeEpoch(gameId, snapshot.source_epoch);
        }
      })
      .finally(() => {
        if (this.priming.get(gameId) === priming) this.priming.delete(gameId);
      });
    this.priming.set(gameId, priming);
    return priming;
  }

  private observeEpoch(gameId: string, epoch: string): void {
    const previous = this.epochs.get(gameId);
    this.epochs.set(gameId, epoch);
    if (previous !== undefined && previous !== epoch) {
      this.checkpoints.delete(gameId);
      for (const listener of this.epochListeners) listener(gameId, epoch);
    }
  }

  private readSnapshot(gameId: string): Promise<ProjectionSnapshot> {
    let inFlight = this.snapshotsInFlight.get(gameId);
    if (!inFlight) {
      const version = this.lifecycleVersions.get(gameId) ?? 0;
      const promise = this.snapshot(gameId).finally(() => {
        if (this.snapshotsInFlight.get(gameId)?.promise === promise) {
          this.snapshotsInFlight.delete(gameId);
        }
      });
      inFlight = { version, promise };
      this.snapshotsInFlight.set(gameId, inFlight);
    }
    const request = inFlight;
    return request.promise.then((snapshot) =>
      request.version === (this.lifecycleVersions.get(gameId) ?? 0)
        ? snapshot
        : this.readSnapshot(gameId),
    );
  }

  record(gameId: string, revision: number, epoch?: string): void {
    if (!gameId || !Number.isSafeInteger(revision) || revision < 0) return;
    if (!this.acceptsEpoch(gameId, epoch)) return;
    const previous = this.checkpoints.get(gameId);
    this.checkpoints.set(gameId, {
      epoch,
      revision:
        previous && previous.epoch === epoch ? Math.max(previous.revision, revision) : revision,
    });
    const tracker = this.trackers.get(gameId);
    if (!tracker || (tracker.epoch !== undefined && tracker.epoch !== epoch)) return;
    tracker.projected = Math.max(tracker.projected, revision);
    for (const waiter of tracker.waiters) {
      if (waiter.revision <= tracker.projected) {
        tracker.waiters.delete(waiter);
        waiter.resolve();
      }
    }
    tracker.wake?.();
  }

  wait(gameId: string, revision: number, epoch?: string): Promise<void> {
    if (!this.isCurrent(gameId)) return Promise.resolve();
    let tracker = this.trackers.get(gameId);
    if (tracker && tracker.epoch !== epoch) {
      for (const waiter of tracker.waiters) waiter.reject(new WorkspaceRootEpochChangedError());
      tracker.waiters.clear();
      tracker.wake?.();
      this.trackers.delete(gameId);
      tracker = undefined;
    }
    const checkpoint = this.checkpoints.get(gameId);
    const projected = checkpoint && checkpoint.epoch === epoch ? checkpoint.revision : 0;
    if (projected >= revision) return Promise.resolve();
    const isNew = !tracker;
    tracker ??= { epoch, projected, waiters: new Set() };
    this.trackers.set(gameId, tracker);
    const completion = new Promise<void>((resolve, reject) => {
      tracker.waiters.add({ revision, resolve, reject });
    });
    if (isNew) void this.poll(gameId, tracker);
    return completion;
  }

  private async poll(gameId: string, tracker: ProjectionTracker): Promise<void> {
    let retryMs = INITIAL_RETRY_MS;
    while (tracker.waiters.size > 0 && this.trackers.get(gameId) === tracker) {
      if (!this.isCurrent(gameId)) {
        for (const waiter of tracker.waiters) waiter.resolve();
        tracker.waiters.clear();
        break;
      }
      let timeout: ReturnType<typeof setTimeout> | undefined;
      try {
        const snapshot = await Promise.race([
          this.readSnapshot(gameId),
          new Promise<null>((resolve) => {
            timeout = setTimeout(() => resolve(null), SNAPSHOT_TIMEOUT_MS);
          }),
        ]);
        if (snapshot && this.trackers.get(gameId) === tracker) {
          if (snapshot.game_id === gameId) this.observeEpoch(gameId, snapshot.source_epoch);
          if (
            snapshot.game_id !== gameId ||
            (tracker.epoch !== undefined && snapshot.source_epoch !== tracker.epoch)
          ) {
            if (snapshot.game_id === gameId) {
              for (const listener of this.epochListeners) listener(gameId, snapshot.source_epoch);
            }
            for (const waiter of tracker.waiters)
              waiter.reject(new WorkspaceRootEpochChangedError());
            tracker.waiters.clear();
            break;
          }
          // Legacy receipts lack an epoch, but still share a single poll owner.
          this.record(gameId, snapshot.projected_revision, tracker.epoch);
          if (snapshot.projection_repair_reason) {
            const error = new WorkspaceProjectionNeedsRepairError(
              snapshot.projection_repair_reason,
            );
            for (const waiter of tracker.waiters) waiter.reject(error);
            tracker.waiters.clear();
            break;
          }
        }
      } catch (error) {
        console.warn('[WorkspaceSwitch] Projection snapshot unavailable; retrying', error);
      } finally {
        if (timeout !== undefined) clearTimeout(timeout);
      }
      if (tracker.waiters.size === 0) break;
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, retryMs);
        tracker.wake = () => {
          clearTimeout(timer);
          resolve();
        };
      });
      tracker.wake = undefined;
      retryMs = Math.min(retryMs * 2, MAX_RETRY_MS);
    }
    if (this.trackers.get(gameId) === tracker) this.trackers.delete(gameId);
  }
}
