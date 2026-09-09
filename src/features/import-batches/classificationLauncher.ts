import type { ObjectClassificationPreviewItem } from '@/shared/api/tauri/bindings.gen';

export type ObjectClassificationLaunchResult = 'applied' | 'cancelled';

export type ObjectClassificationLaunchRequest = {
  gameId: string;
  objectIds: string[];
  initialItems?: ObjectClassificationPreviewItem[];
  onComplete?: (result: ObjectClassificationLaunchResult) => void;
};

type Listener = (request: ObjectClassificationLaunchRequest) => void;
const listeners = new Set<Listener>();

export function openObjectClassificationWizard(request: ObjectClassificationLaunchRequest): void {
  for (const listener of listeners) listener(request);
}

export function subscribeObjectClassificationWizard(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
