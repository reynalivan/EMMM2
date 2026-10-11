import type { DiskReconcileReason, GameActivationPhase } from './bindings.gen.js';
export type { RuntimeSyncPhase, RuntimeSyncStatus } from './bindings.gen.js';

// Event-only payloads remain local; runtime status is generated through notice snapshots.
export type DiskReconcilePhase =
  'DiscoveringRoots' | 'ScanningRoots' | 'Projecting' | 'Finalizing' | 'Completed' | 'Failed';

export type DiskReconcileProgress = {
  game_id: string;
  run_id: string;
  reason: DiskReconcileReason;
  phase: DiskReconcilePhase;
  // Units belong to the current phase: roots during scan, mod rows during projection.
  completed_units: number;
  total_units: number | null;
  current_root: string | null;
  elapsed_ms: number;
  eta_ms: number | null;
};

export type GameActivationStatus = {
  game_id: string | null;
  generation: number;
  phase: GameActivationPhase;
  reconcile_revision: number | null;
  runtime_sync_generation: number | null;
  error: string | null;
};

export type OnboardingIndexingSnapshotProgress = {
  session_id: string;
  game_id: string;
  phase: 'Metadata' | 'Classifying' | 'Ready' | 'Rechecking';
  completed_games: number;
  total_games: number;
  completed_roots: number;
  total_roots: number;
  folders_classified: number;
  current_root: string | null;
  elapsed_ms: number;
};
