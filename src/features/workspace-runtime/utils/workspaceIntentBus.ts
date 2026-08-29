export type WorkspaceIntent = { type: 'autoOrganizePaths'; paths: string[] };

type WorkspaceIntentListener = (intent: WorkspaceIntent) => void;

const listeners = new Set<WorkspaceIntentListener>();

export function publishWorkspaceIntent(intent: WorkspaceIntent): void {
  for (const listener of listeners) {
    listener(intent);
  }
}

export function subscribeWorkspaceIntent(listener: WorkspaceIntentListener): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}
