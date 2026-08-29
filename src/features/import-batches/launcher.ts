import type { ImportFlow, TargetMode } from '../../shared/api/tauri/bindings';

export type ImportBatchLaunchRequest =
  | {
      kind: 'sources';
      gameId: string;
      flow: ImportFlow;
      targetMode: TargetMode;
      targetObjectId?: string | null;
      targetSubpath?: string | null;
      paths: string[];
    }
  | { kind: 'existing'; batchId: string };

type Listener = (request: ImportBatchLaunchRequest) => void;

const listeners = new Set<Listener>();

export function openImportBatchWizard(request: ImportBatchLaunchRequest): void {
  for (const listener of listeners) listener(request);
}

export function subscribeImportBatchWizard(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
