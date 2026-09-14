import type { DiskReconcilePhase, DiskReconcileProgress } from '../../../shared/api/tauri/bindings';

export interface IndexingProgress {
  completed: number;
  total: number;
  currentGame: string | null;
  completedDurationsMs: number[];
}

export interface OverallIndexingProgress {
  percent: number;
  gameIndex: number;
  gameTotal: number;
  step: number;
  folderName: string | null;
}

export interface IndexingWorkPlan {
  game_id: string;
  work_units: number;
  roots: Array<{ root_name: string; work_units: number }>;
}

export const INDEXING_STEP_COUNT = 4;

export function calculateOverallIndexingProgress(
  progress: DiskReconcileProgress,
  gameIds: string[],
  workPlans: IndexingWorkPlan[] = [],
  completedRootsByGame: Record<string, string[]> = {},
): OverallIndexingProgress | null {
  const gameIndex = gameIds.indexOf(progress.game_id);
  if (gameIndex < 0 || gameIds.length === 0) return null;

  const plansByGame = new Map(workPlans.map((plan) => [plan.game_id, plan]));
  const gameWeights = gameIds.map((gameId) => totalGameWork(plansByGame.get(gameId)));
  const totalWork = gameWeights.reduce((total, work) => total + work, 0);
  const gameCompletion = completionWithinGame(
    progress,
    plansByGame.get(progress.game_id),
    completedRootsByGame[progress.game_id] ?? [],
  );
  const completedWork =
    gameWeights.slice(0, gameIndex).reduce((total, work) => total + work, 0) +
    gameWeights[gameIndex] * gameCompletion;
  return {
    percent: Math.round((completedWork / totalWork) * 100),
    gameIndex,
    gameTotal: gameIds.length,
    step: stepForPhase(progress.phase),
    folderName: humanizeFolderName(progress.current_root),
  };
}

function totalGameWork(plan: IndexingWorkPlan | undefined): number {
  const scanWork = Math.max(1, plan?.work_units ?? 1);
  return scanWork / 0.8;
}

function completionWithinGame(
  progress: DiskReconcileProgress,
  plan: IndexingWorkPlan | undefined,
  completedRoots: string[],
): number {
  switch (progress.phase) {
    case 'DiscoveringRoots':
      return 0;
    case 'ScanningRoots': {
      const scanCompletion = plan
        ? completedWorkFraction(plan, completedRoots)
        : completedFraction(progress.completed_units, progress.total_units);
      return 0.05 + scanCompletion * 0.8;
    }
    case 'Projecting':
      return 0.85;
    case 'Finalizing':
      return 0.95;
    case 'Completed':
      return 1;
    case 'Failed':
      return 0;
  }
}

function completedWorkFraction(plan: IndexingWorkPlan, completedRoots: string[]): number {
  if (plan.work_units <= 0) return 1;
  const completedRootNames = new Set(completedRoots);
  const completedWork = plan.roots
    .filter((root) => completedRootNames.has(root.root_name))
    .reduce((total, root) => total + root.work_units, 0);
  return Math.min(1, Math.max(0, completedWork / plan.work_units));
}

function completedFraction(completed: number, total: number | null): number {
  if (!total || total <= 0) return 0;
  return Math.min(1, Math.max(0, completed / total));
}

function stepForPhase(phase: DiskReconcilePhase): number {
  switch (phase) {
    case 'DiscoveringRoots':
      return 1;
    case 'ScanningRoots':
      return 2;
    case 'Projecting':
      return 3;
    case 'Finalizing':
    case 'Completed':
    case 'Failed':
      return INDEXING_STEP_COUNT;
  }
}

function humanizeFolderName(path: string | null): string | null {
  const segment = path
    ?.trim()
    .replace(/[\\/]+$/, '')
    .split(/[\\/]/)
    .pop();
  if (!segment) return null;
  return segment.replace(/^#+/, '') || segment;
}

export function estimatedRemainingMs(
  progress: IndexingProgress,
  activeScanRemainingMs: number | null | undefined = null,
): number | null {
  if (progress.completed >= progress.total) {
    return null;
  }

  const average = progress.completedDurationsMs.length
    ? progress.completedDurationsMs.reduce((total, duration) => total + duration, 0) /
      progress.completedDurationsMs.length
    : null;

  if (activeScanRemainingMs !== null && activeScanRemainingMs !== undefined) {
    const laterGames = Math.max(0, progress.total - progress.completed - 1);
    return Math.round(activeScanRemainingMs + (average ?? 0) * laterGames);
  }

  if (average === null) {
    return null;
  }

  return Math.round(average * (progress.total - progress.completed));
}

export function formatEstimatedDuration(milliseconds: number): string {
  const seconds = Math.max(1, Math.ceil(milliseconds / 1000));
  return seconds < 60 ? `${seconds}s` : `${Math.ceil(seconds / 60)}m`;
}
