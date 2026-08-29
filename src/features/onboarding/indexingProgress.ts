export interface IndexingProgress {
  completed: number;
  total: number;
  currentGame: string | null;
  completedDurationsMs: number[];
}

export function estimatedRemainingMs(progress: IndexingProgress): number | null {
  if (progress.completedDurationsMs.length === 0 || progress.completed >= progress.total) {
    return null;
  }
  const average =
    progress.completedDurationsMs.reduce((total, duration) => total + duration, 0) /
    progress.completedDurationsMs.length;
  return Math.round(average * (progress.total - progress.completed));
}

export function formatEstimatedDuration(milliseconds: number): string {
  const seconds = Math.max(1, Math.ceil(milliseconds / 1000));
  return seconds < 60 ? `${seconds}s` : `${Math.ceil(seconds / 60)}m`;
}
