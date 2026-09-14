import { listen } from '@tauri-apps/api/event';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Archive, PackageOpen, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import { formatAppError } from '../../shared/lib/appError';
import { commands } from '../../shared/api/tauri/bindings';
import { isDemoMode } from '@/shared/lib/appMode';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import { openImportBatchWizard } from '@/features/import-batches';
import { modInboxCommands } from './api';
import {
  DeleteProcessedDialog,
  MissingInboxState,
  ModInboxHeader,
  ModInboxTabs,
  NoGameState,
} from './ModInboxChrome';
import { EmptyState, ProcessedSourceRow, ReadyEntryRow } from './ModInboxRows';
import { openProcessedDestinationInApp } from './navigation';
import type {
  ModInboxEntry,
  ModInboxSnapshot,
  ProcessedModInboxDestination,
  ProcessedModInboxSource,
} from './types';
import {
  WorkspacePageContent,
  WorkspacePageFrame,
} from '@/shared/ui/components/layout/WorkspacePageFrame';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';

const WATCHER_REFRESH_DEBOUNCE_MS = 600;
const VIRTUAL_LIST_THRESHOLD = 80;

interface InboxRefreshSlot {
  pending: boolean;
  promise: Promise<void>;
}

const getReadyEntryKey = (entry: ModInboxEntry) => entry.entryKey;
const getProcessedSourceKey = (source: ProcessedModInboxSource) => source.sourceId;

export default function ModInboxPage() {
  const { t } = useTranslation('mod_inbox');
  const activeGameId = useAppStore((state) => state.activeGameId);
  const [snapshot, setSnapshot] = useState<ModInboxSnapshot | null>(null);
  const [activeTab, setActiveTab] = useState<'ready' | 'processed'>('ready');
  const [readySelection, setReadySelection] = useState<Set<string>>(() => new Set());
  const [processedSelection, setProcessedSelection] = useState<Set<string>>(() => new Set());
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refreshSequence = useRef(0);
  const refreshSlots = useRef(new Map<string, InboxRefreshSlot>());
  const watcherRefreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const readyScrollOffset = useRef(0);
  const processedScrollOffset = useRef(0);
  const readySelectionScope = useRef<string | null>(null);
  const watcherTransition = useRef(Promise.resolve());
  const watcherErrorPrefix = t('watcher_failed', { error: '' });

  const syncReadySelection = useCallback((next: ModInboxSnapshot) => {
    if (next.rootState !== 'ready') {
      readySelectionScope.current = null;
      setReadySelection(new Set());
      return;
    }

    const scope = `${next.gameId}:${next.rootPath}`;
    const selectableEntryKeys = new Set(
      next.readyEntries.filter((entry) => !entry.pendingBatchId).map((entry) => entry.entryKey),
    );
    const selectAll = readySelectionScope.current !== scope;
    readySelectionScope.current = scope;
    setReadySelection((current) => {
      if (selectAll) return selectableEntryKeys;
      return new Set([...current].filter((entryKey) => selectableEntryKeys.has(entryKey)));
    });
  }, []);

  const refresh = useCallback(async () => {
    if (!activeGameId) {
      refreshSequence.current += 1;
      refreshSlots.current.clear();
      setSnapshot(null);
      setLoading(false);
      setError(null);
      return;
    }

    const requestedGameId = activeGameId;
    const existingSlot = refreshSlots.current.get(requestedGameId);
    if (existingSlot) {
      existingSlot.pending = true;
      return existingSlot.promise;
    }

    const slot: InboxRefreshSlot = {
      pending: false,
      promise: Promise.resolve(),
    };
    refreshSlots.current.set(requestedGameId, slot);

    const run = async () => {
      do {
        slot.pending = false;
        const requestSequence = ++refreshSequence.current;
        setLoading(true);
        setError(null);
        try {
          const next = await modInboxCommands.getModInbox(requestedGameId);
          if (requestSequence !== refreshSequence.current) continue;
          setSnapshot(next);
          syncReadySelection(next);
        } catch (cause) {
          if (requestSequence !== refreshSequence.current) continue;
          setError(formatAppError(cause));
        } finally {
          if (requestSequence === refreshSequence.current) setLoading(false);
        }
      } while (slot.pending);
    };

    slot.promise = run().finally(() => {
      if (refreshSlots.current.get(requestedGameId) === slot) {
        refreshSlots.current.delete(requestedGameId);
      }
    });
    return slot.promise;
  }, [activeGameId, syncReadySelection]);

  const scheduleWatcherRefresh = useCallback(() => {
    if (watcherRefreshTimer.current !== null) {
      clearTimeout(watcherRefreshTimer.current);
    }
    watcherRefreshTimer.current = setTimeout(() => {
      watcherRefreshTimer.current = null;
      void refresh();
    }, WATCHER_REFRESH_DEBOUNCE_MS);
  }, [refresh]);

  useEffect(() => {
    if (!activeGameId) return;
    void refresh();
    if (isDemoMode) return;

    const unlistenPromise = listen('mod-inbox://changed', scheduleWatcherRefresh);
    return () => {
      refreshSequence.current += 1;
      if (watcherRefreshTimer.current !== null) {
        clearTimeout(watcherRefreshTimer.current);
        watcherRefreshTimer.current = null;
      }
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGameId, refresh, scheduleWatcherRefresh]);

  useEffect(() => {
    readyScrollOffset.current = 0;
    processedScrollOffset.current = 0;
  }, [activeGameId]);

  const inboxWatcherRoot =
    snapshot?.gameId === activeGameId && snapshot.rootState === 'ready' ? snapshot.rootPath : null;

  useEffect(() => {
    if (isDemoMode || !activeGameId || !inboxWatcherRoot) {
      return;
    }

    watcherTransition.current = watcherTransition.current.then(async () => {
      try {
        await modInboxCommands.startModInboxWatcher(activeGameId);
      } catch (cause) {
        toast.warning(`${watcherErrorPrefix}${formatAppError(cause)}`);
      }
    });
    return () => {
      watcherTransition.current = watcherTransition.current.then(async () => {
        try {
          await modInboxCommands.stopModInboxWatcher(activeGameId, inboxWatcherRoot);
        } catch (cause) {
          console.warn('Could not stop Mod Inbox watcher', cause);
        }
      });
    };
  }, [activeGameId, inboxWatcherRoot, watcherErrorPrefix]);

  const selectableReadyEntries = useMemo(
    () => snapshot?.readyEntries.filter((entry) => !entry.pendingBatchId) ?? [],
    [snapshot?.readyEntries],
  );
  const retainedProcessedSources = useMemo(
    () =>
      snapshot?.processedSources.filter(
        (source) => Boolean(source.processedPath) && !source.sourceDeletedAt,
      ) ?? [],
    [snapshot?.processedSources],
  );
  const readyUsesVirtualList = (snapshot?.readyEntries.length ?? 0) > VIRTUAL_LIST_THRESHOLD;
  const processedUsesVirtualList =
    (snapshot?.processedSources.length ?? 0) > VIRTUAL_LIST_THRESHOLD;
  const activeTabUsesVirtualList =
    (activeTab === 'ready' && readyUsesVirtualList) ||
    (activeTab === 'processed' && processedUsesVirtualList);
  const saveReadyScrollOffset = useCallback((offset: number) => {
    readyScrollOffset.current = offset;
  }, []);
  const saveProcessedScrollOffset = useCallback((offset: number) => {
    processedScrollOffset.current = offset;
  }, []);
  const getReadyScrollOffset = useCallback(() => readyScrollOffset.current, []);
  const getProcessedScrollOffset = useCallback(() => processedScrollOffset.current, []);

  const allReadySelected =
    selectableReadyEntries.length > 0 &&
    selectableReadyEntries.every((entry) => readySelection.has(entry.entryKey));
  const allProcessedSelected =
    retainedProcessedSources.length > 0 &&
    retainedProcessedSources.every((source) => processedSelection.has(source.sourceId));

  const toggleSelection = (id: string, setSelected: Dispatch<SetStateAction<Set<string>>>) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const createFolder = async () => {
    if (!activeGameId) return;
    setBusyAction('create-folder');
    try {
      const nextSnapshot = await modInboxCommands.createModInboxFolder(activeGameId);
      setSnapshot(nextSnapshot);
      syncReadySelection(nextSnapshot);
    } catch (cause) {
      toast.error(t('errors.create_folder', { error: formatAppError(cause) }));
    } finally {
      setBusyAction(null);
    }
  };

  const createBatch = async () => {
    if (!activeGameId || !snapshot) return;
    const entryKeys = snapshot.readyEntries
      .filter((entry) => readySelection.has(entry.entryKey) && !entry.pendingBatchId)
      .map((entry) => entry.entryKey);
    if (entryKeys.length === 0) return;

    setBusyAction('create-batch');
    try {
      const batch = await modInboxCommands.createModInboxBatch({
        gameId: activeGameId,
        entryKeys,
      });
      setReadySelection(new Set());
      openImportBatchWizard({ kind: 'existing', batchId: batch.id });
    } catch (cause) {
      toast.error(t('errors.create_batch', { error: formatAppError(cause) }));
    } finally {
      setBusyAction(null);
    }
  };

  const deleteProcessedSources = async () => {
    if (!activeGameId || !snapshot) return;
    const sourceIds = snapshot.processedSources
      .filter(
        (source) =>
          processedSelection.has(source.sourceId) &&
          Boolean(source.processedPath) &&
          !source.sourceDeletedAt,
      )
      .map((source) => source.sourceId);
    if (sourceIds.length === 0) return;

    setBusyAction('delete-sources');
    try {
      setSnapshot(
        await modInboxCommands.deleteProcessedModInboxSources({
          gameId: activeGameId,
          sourceIds,
        }),
      );
      setProcessedSelection(new Set());
      setConfirmDelete(false);
    } catch (cause) {
      toast.error(t('errors.delete_sources', { error: formatAppError(cause) }));
    } finally {
      setBusyAction(null);
    }
  };

  const openDestinationInApp = async (destination: ProcessedModInboxDestination) => {
    try {
      await openProcessedDestinationInApp(destination);
    } catch (cause) {
      toast.error(t('errors.open_destination', { error: formatAppError(cause) }));
    }
  };

  const chooseInboxLocation = async () => {
    if (isDemoMode || !activeGameId) return;
    try {
      const selectedPath = await openDialog({
        directory: true,
        multiple: false,
        title: t('actions.choose_location'),
      });
      if (selectedPath && typeof selectedPath === 'string') {
        const settings = await modInboxCommands.getSettings();
        const game = settings.games.find((g) => g.id === activeGameId);
        if (game) {
          game.ready_to_move_path = selectedPath;
          await modInboxCommands.saveSettings(settings);
          await refresh();
        }
      }
    } catch (cause) {
      toast.error(t('errors.load', { error: formatAppError(cause) }));
    }
  };

  if (!activeGameId) {
    return <NoGameState />;
  }

  return (
    <WorkspacePageFrame
      context={
        snapshot ? (
          <ModInboxTabs
            activeTab={activeTab}
            readyCount={snapshot.readyEntries.length}
            processedCount={snapshot.processedSources.length}
            allSelected={activeTab === 'ready' ? allReadySelected : allProcessedSelected}
            selectionDisabled={
              activeTab === 'ready'
                ? selectableReadyEntries.length === 0
                : retainedProcessedSources.length === 0
            }
            onToggleSelectAll={() => {
              if (activeTab === 'ready') {
                setReadySelection(
                  allReadySelected
                    ? new Set()
                    : new Set(selectableReadyEntries.map((entry) => entry.entryKey)),
                );
                return;
              }
              setProcessedSelection(
                allProcessedSelected
                  ? new Set()
                  : new Set(retainedProcessedSources.map((source) => source.sourceId)),
              );
            }}
            onChange={setActiveTab}
          />
        ) : undefined
      }
    >
      <ModInboxHeader
        snapshot={snapshot}
        loading={loading}
        onSettings={() => void chooseInboxLocation()}
        onOpen={() => {
          if (snapshot) void modInboxCommands.openModInboxFolder(activeGameId);
        }}
        onRefresh={() => void refresh()}
        topBarAction={
          activeTab === 'ready' ? (
            <button
              type="button"
              className="btn btn-primary btn-sm"
              disabled={busyAction === 'create-batch' || readySelection.size === 0}
              onClick={() => void createBatch()}
            >
              {busyAction === 'create-batch' && <span className="loading loading-spinner" />}
              {t('ready.review_selected', { count: readySelection.size })}
            </button>
          ) : null
        }
      />

      {error ? (
        <WorkspacePageContent>
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-error/25 bg-error/8 p-4">
            <span className="text-sm">{t('errors.load', { error })}</span>
            <button
              type="button"
              className="btn btn-error btn-outline btn-sm"
              onClick={() => void refresh()}
            >
              {t('actions.retry')}
            </button>
          </div>
        </WorkspacePageContent>
      ) : !snapshot && loading ? (
        <WorkspacePageContent>
          <WorkspacePanelSkeleton variant="inbox" />
        </WorkspacePageContent>
      ) : snapshot?.rootState === 'missing' ? (
        <MissingInboxState
          rootPath={snapshot.rootPath}
          creating={busyAction === 'create-folder'}
          onCreate={() => void createFolder()}
          onChooseLocation={() => void chooseInboxLocation()}
        />
      ) : snapshot ? (
        <>
          <WorkspacePageContent
            className={
              activeTabUsesVirtualList ? 'flex h-full min-h-0 flex-col overflow-hidden' : undefined
            }
          >
            {activeTab === 'ready' ? (
              <section className="mx-auto flex min-h-0 w-full max-w-6xl flex-1 flex-col">
                {snapshot.readyEntries.length === 0 ? (
                  <EmptyState
                    icon={<PackageOpen size={34} />}
                    title={t('ready.empty_title')}
                    description={t('ready.empty_description')}
                  />
                ) : readyUsesVirtualList ? (
                  <VirtualList
                    items={snapshot.readyEntries}
                    getItemKey={getReadyEntryKey}
                    estimateSize={() => 104}
                    ariaLabel={t('tabs.ready')}
                    initialOffset={getReadyScrollOffset}
                    onScrollOffsetChange={saveReadyScrollOffset}
                    className="pr-1 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20"
                    renderItem={(entry) => (
                      <div className="pb-3">
                        <ReadyEntryRow
                          entry={entry}
                          selected={readySelection.has(entry.entryKey)}
                          onToggle={() => toggleSelection(entry.entryKey, setReadySelection)}
                          onResume={() => {
                            if (entry.pendingBatchId) {
                              openImportBatchWizard({
                                kind: 'existing',
                                batchId: entry.pendingBatchId,
                              });
                            }
                          }}
                        />
                      </div>
                    )}
                  />
                ) : (
                  <div className="space-y-3">
                    {snapshot.readyEntries.map((entry) => (
                      <ReadyEntryRow
                        key={entry.entryKey}
                        entry={entry}
                        selected={readySelection.has(entry.entryKey)}
                        onToggle={() => toggleSelection(entry.entryKey, setReadySelection)}
                        onResume={() => {
                          if (entry.pendingBatchId) {
                            openImportBatchWizard({
                              kind: 'existing',
                              batchId: entry.pendingBatchId,
                            });
                          }
                        }}
                      />
                    ))}
                  </div>
                )}
              </section>
            ) : (
              <section className="mx-auto flex min-h-0 w-full max-w-6xl flex-1 flex-col">
                {processedSelection.size > 0 && (
                  <div className="flex min-h-10 justify-end">
                    <button
                      type="button"
                      className="btn btn-error btn-outline btn-sm gap-2"
                      onClick={() => setConfirmDelete(true)}
                    >
                      <Trash2 size={15} />
                      {t('processed.delete_selected', { count: processedSelection.size })}
                    </button>
                  </div>
                )}

                {snapshot.processedSources.length === 0 ? (
                  <EmptyState
                    icon={<Archive size={34} />}
                    title={t('processed.empty_title')}
                    description={t('processed.empty_description')}
                  />
                ) : processedUsesVirtualList ? (
                  <VirtualList
                    items={snapshot.processedSources}
                    getItemKey={getProcessedSourceKey}
                    estimateSize={() => 190}
                    ariaLabel={t('tabs.processed')}
                    initialOffset={getProcessedScrollOffset}
                    onScrollOffsetChange={saveProcessedScrollOffset}
                    className="pr-1 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20"
                    renderItem={(source) => (
                      <div className="pb-3">
                        <ProcessedSourceRow
                          source={source}
                          selected={processedSelection.has(source.sourceId)}
                          onToggle={() => toggleSelection(source.sourceId, setProcessedSelection)}
                          onOpenInApp={(destination) => void openDestinationInApp(destination)}
                          onOpenInExplorer={(destination) =>
                            void commands.openInExplorer(activeGameId, destination.placedPath)
                          }
                        />
                      </div>
                    )}
                  />
                ) : (
                  <div className="space-y-3">
                    {snapshot.processedSources.map((source) => (
                      <ProcessedSourceRow
                        key={source.sourceId}
                        source={source}
                        selected={processedSelection.has(source.sourceId)}
                        onToggle={() => toggleSelection(source.sourceId, setProcessedSelection)}
                        onOpenInApp={(destination) => void openDestinationInApp(destination)}
                        onOpenInExplorer={(destination) =>
                          void commands.openInExplorer(activeGameId, destination.placedPath)
                        }
                      />
                    ))}
                  </div>
                )}
              </section>
            )}
          </WorkspacePageContent>
        </>
      ) : null}

      {confirmDelete && (
        <DeleteProcessedDialog
          count={processedSelection.size}
          deleting={busyAction === 'delete-sources'}
          onCancel={() => setConfirmDelete(false)}
          onConfirm={() => void deleteProcessedSources()}
        />
      )}
    </WorkspacePageFrame>
  );
}
