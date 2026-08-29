export type ObjectClassificationLaunchRequest = {
  gameId: string;
  objectIds: string[];
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
