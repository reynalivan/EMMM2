import { describe, expect, it } from 'vitest';
import type { DiskReconcileProgress } from '../../../shared/api/tauri/bindings';
import { calculateOverallIndexingProgress, estimatedRemainingMs } from './indexingProgress';

function progress(overrides: Partial<DiskReconcileProgress>): DiskReconcileProgress {
  return {
    game_id: 'game-3',
    run_id: 'game-3-run',
    reason: 'OnboardingCompleted',
    phase: 'ScanningRoots',
    completed_units: 3,
    total_units: 4,
    current_root: '#UI',
    elapsed_ms: 1_000,
    eta_ms: 500,
    ...overrides,
  };
}

describe('calculateOverallIndexingProgress', () => {
  it('maps the active game scan into one global percentage', () => {
    const result = calculateOverallIndexingProgress(progress({}), [
      'game-1',
      'game-2',
      'game-3',
      'game-4',
      'game-5',
    ]);

    expect(result).toMatchObject({
      percent: 53,
      gameIndex: 2,
      gameTotal: 5,
      step: 2,
      folderName: 'UI',
    });
  });

  it('keeps a completed game contribution in the global progress', () => {
    const result = calculateOverallIndexingProgress(
      progress({ phase: 'Completed', completed_units: 0, total_units: null, current_root: null }),
      ['game-1', 'game-2', 'game-3', 'game-4', 'game-5'],
    );

    expect(result).toMatchObject({ percent: 60, step: 4, folderName: null });
  });

  it('weights completed roots by their planned file-size work', () => {
    const result = calculateOverallIndexingProgress(
      progress({ completed_units: 1, total_units: 2, current_root: '#UI' }),
      ['game-1', 'game-2', 'game-3', 'game-4', 'game-5'],
      [
        { game_id: 'game-1', work_units: 100, roots: [] },
        { game_id: 'game-2', work_units: 900, roots: [] },
        {
          game_id: 'game-3',
          work_units: 100,
          roots: [
            { root_name: 'UI', work_units: 75 },
            { root_name: 'Other', work_units: 25 },
          ],
        },
        { game_id: 'game-4', work_units: 100, roots: [] },
        { game_id: 'game-5', work_units: 100, roots: [] },
      ],
      { 'game-3': ['UI'] },
    );

    expect(result?.percent).toBe(82);
  });
});

describe('estimatedRemainingMs', () => {
  it('uses the active scan ETA before the first game completes', () => {
    expect(
      estimatedRemainingMs(
        {
          completed: 0,
          total: 1,
          currentGame: 'New Game',
          completedDurationsMs: [],
        },
        1_500,
      ),
    ).toBe(1_500);
  });
});
