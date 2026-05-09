import type { QueryClient } from '@tanstack/react-query';
import type { RuntimeEffectDescriptor } from '../workspace-runtime/optimistic/descriptor';

// FE runtime refresh bus only.
// Disk Reconcile event handlers or explicit non-filesystem mutations publish into this layer.
export type QueryRefetchType = 'active' | 'inactive' | 'all' | 'none';

export const runtimeQueryKeys = {
  workspaceViewModel: ['workspace', 'mods'] as const,
  folderStructure: ['mod-folders'] as const,
  folderMetadata: ['mod-folders'] as const,
  objectRows: ['objects', 'list'] as const,
  objectCounts: ['objects', 'counts'] as const,
  corridorState: ['v2-corridor'] as const,
  collections: ['v2-collections'] as const,
  dashboard: ['dashboard-stats'] as const,
  activeKeybindings: ['active-keybindings'] as const,
  previewDetails: ['details'] as const,
  thumbnails: ['thumbnails'] as const,
  conflicts: ['conflicts'] as const,
  trash: ['trash'] as const,
  settings: ['settings'] as const,
  browserDownloads: ['browser-downloads'] as const,
  browserImportQueue: ['import-queue'] as const,
  browserHomepage: ['browser_homepage'] as const,
  dedupAll: ['dedup'] as const,
  dedupReport: ['dedup', 'report'] as const,
  pins: ['v2-pin'] as const,
};

export type RuntimeRefreshScope = keyof typeof runtimeQueryKeys;
export type RuntimeRefreshEvent =
  | 'workspaceChanged'
  | 'objectRowsChanged'
  | 'folderStructureChanged'
  | 'folderMetadataChanged'
  | 'previewChanged'
  | 'thumbnailChanged'
  | 'conflictsChanged'
  | 'corridorChanged'
  | 'collectionsChanged'
  | 'dashboardChanged'
  | 'activeKeybindingsChanged'
  | 'trashChanged'
  | 'settingsChanged'
  | 'browserDownloadsChanged'
  | 'browserImportQueueChanged'
  | 'browserHomepageChanged'
  | 'dedupChanged'
  | 'dedupReportChanged'
  | 'scannerChanged'
  | 'pinsChanged';

interface RefreshRuntimeQueriesOptions {
  refetchType?: QueryRefetchType;
  scopes: RuntimeRefreshScope[];
}

interface PublishRuntimeEventsOptions {
  refetchType?: QueryRefetchType;
  events: RuntimeRefreshEvent[];
}

interface PendingRuntimeRefresh {
  promise: Promise<void>;
  resolve: () => void;
  reject: (error: unknown) => void;
  refetchType: QueryRefetchType;
  scopes: Set<RuntimeRefreshScope>;
}

const runtimeEventScopes: Record<RuntimeRefreshEvent, RuntimeRefreshScope[]> = {
  workspaceChanged: ['workspaceViewModel', 'objectRows', 'objectCounts'],
  objectRowsChanged: ['workspaceViewModel', 'objectRows', 'objectCounts'],
  folderStructureChanged: [
    'workspaceViewModel',
    'folderStructure',
    'objectRows',
    'objectCounts',
    'previewDetails',
  ],
  folderMetadataChanged: ['workspaceViewModel', 'folderMetadata', 'previewDetails'],
  previewChanged: ['workspaceViewModel', 'previewDetails'],
  thumbnailChanged: ['workspaceViewModel', 'thumbnails', 'previewDetails'],
  conflictsChanged: ['conflicts'],
  corridorChanged: ['corridorState', 'workspaceViewModel', 'objectRows', 'objectCounts'],
  collectionsChanged: ['collections', 'corridorState'],
  dashboardChanged: ['dashboard'],
  activeKeybindingsChanged: ['activeKeybindings'],
  trashChanged: ['trash'],
  settingsChanged: ['settings'],
  browserDownloadsChanged: ['browserDownloads'],
  browserImportQueueChanged: ['browserImportQueue'],
  browserHomepageChanged: ['browserHomepage'],
  dedupChanged: ['dedupAll'],
  dedupReportChanged: ['dedupReport'],
  scannerChanged: ['folderStructure', 'trash', 'conflicts', 'dedupAll', 'dedupReport'],
  pinsChanged: ['pins'],
};

const pendingRuntimeRefreshes = new WeakMap<QueryClient, PendingRuntimeRefresh>();

function mergeRefetchType(current: QueryRefetchType, next: QueryRefetchType): QueryRefetchType {
  if (current === next) {
    return current;
  }

  if (current === 'all' || next === 'all') {
    return 'all';
  }

  if (
    (current === 'active' && next === 'inactive') ||
    (current === 'inactive' && next === 'active')
  ) {
    return 'all';
  }

  if (current === 'none') {
    return next;
  }

  if (next === 'none') {
    return current;
  }

  if (current === 'active' || next === 'active') {
    return 'active';
  }

  return 'inactive';
}

async function refreshRuntimeQueriesNow(
  queryClient: QueryClient,
  options: RefreshRuntimeQueriesOptions,
): Promise<void> {
  const refetchType = options.refetchType ?? 'active';
  const uniqueScopes = [...new Set(options.scopes)];
  const tasks = uniqueScopes.map((scope) =>
    queryClient.invalidateQueries({
      queryKey: runtimeQueryKeys[scope],
      refetchType,
    }),
  );

  await Promise.all(tasks);
}

function scheduleRuntimeRefresh(
  queryClient: QueryClient,
  options: RefreshRuntimeQueriesOptions,
): Promise<void> {
  const nextRefetchType = options.refetchType ?? 'active';
  const current = pendingRuntimeRefreshes.get(queryClient);
  if (current) {
    current.refetchType = mergeRefetchType(current.refetchType, nextRefetchType);
    for (const scope of options.scopes) {
      current.scopes.add(scope);
    }

    return current.promise;
  }

  let resolvePromise!: () => void;
  let rejectPromise!: (error: unknown) => void;
  const pending: PendingRuntimeRefresh = {
    promise: new Promise<void>((resolve, reject) => {
      resolvePromise = resolve;
      rejectPromise = reject;
    }),
    resolve: resolvePromise,
    reject: rejectPromise,
    refetchType: nextRefetchType,
    scopes: new Set(options.scopes),
  };

  pendingRuntimeRefreshes.set(queryClient, pending);

  queueMicrotask(() => {
    const latest = pendingRuntimeRefreshes.get(queryClient);
    if (!latest || latest !== pending) {
      return;
    }

    pendingRuntimeRefreshes.delete(queryClient);
    void refreshRuntimeQueriesNow(queryClient, {
      scopes: [...latest.scopes],
      refetchType: latest.refetchType,
    })
      .then(() => {
        latest.resolve();
      })
      .catch((error: unknown) => {
        latest.reject(error);
      });
  });

  return pending.promise;
}

async function publishRuntimeEvents(
  queryClient: QueryClient,
  options: PublishRuntimeEventsOptions,
): Promise<void> {
  const scopes = options.events.flatMap((event) => runtimeEventScopes[event]);
  await scheduleRuntimeRefresh(queryClient, {
    scopes,
    refetchType: options.refetchType,
  });
}

export async function publishQueryScopes(
  queryClient: QueryClient,
  scopes: RuntimeRefreshScope[],
  refetchType: QueryRefetchType = 'active',
): Promise<void> {
  if (scopes.length === 0) {
    return;
  }

  await scheduleRuntimeRefresh(queryClient, {
    scopes,
    refetchType,
  });
}

export async function publishQueryInvalidations(
  queryClient: QueryClient,
  queryKeys: Array<readonly unknown[]>,
  refetchType: QueryRefetchType,
): Promise<void> {
  if (queryKeys.length === 0) {
    return;
  }

  const uniqueKeys = new Map<string, readonly unknown[]>();
  for (const queryKey of queryKeys) {
    uniqueKeys.set(JSON.stringify(queryKey), queryKey);
  }

  await Promise.all(
    [...uniqueKeys.values()].map((queryKey) =>
      queryClient.invalidateQueries({
        queryKey,
        refetchType,
      }),
    ),
  );
}

export async function publishRuntimeDescriptor(
  queryClient: QueryClient,
  descriptor: RuntimeEffectDescriptor,
  refetchType: QueryRefetchType = 'active',
): Promise<void> {
  if (descriptor.refreshEvents.length === 0) {
    return;
  }

  await publishRuntimeEvents(queryClient, {
    events: descriptor.refreshEvents,
    refetchType,
  });
}
