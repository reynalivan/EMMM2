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
import type { ModInboxSnapshot, ProcessedModInboxDestination } from './types';
import {
  WorkspacePageContent,
  WorkspacePageFrame,
} from '@/shared/ui/components/layout/WorkspacePageFrame';

export default function ModInboxPage() {
  const { t } = useTranslation('mod_inbox');
  const activeGameId = useAppStore((state) => state.activeGameId);
  const [snapshot, setSnapshot] = useState<ModInboxSnapshot | null>(null);
  const [activeTab, setActiveTab] = useState<'ready' | 'processed'>('ready');
  const [readySelection, setReadySelection] = useState<Set<string>>(() => new Set());
  const [processedSelection, setProcessedSelection] = useState<Set<string>>(() => new Set());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const refreshSequence = useRef(0);
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
      setSnapshot(null);
      return;
    }

    const requestedGameId = activeGameId;
    const requestSequence = ++refreshSequence.current;
    setLoading(true);
    setError(null);
    try {
      const next = await modInboxCommands.getModInbox(requestedGameId);
      if (requestSequence !== refreshSequence.current) {
        return;
      }
      setSnapshot(next);
      syncReadySelection(next);
    } catch (cause) {
      if (requestSequence !== refreshSequence.current) {
        return;
      }
      setError(formatAppError(cause));
    } finally {
      if (requestSequence === refreshSequence.current) {
        setLoading(false);
      }
    }
  }, [activeGameId, syncReadySelection]);

  useEffect(() => {
    if (!activeGameId) return;

    void refresh();

    if (isDemoMode) {
      return;
    }

    const unlistenPromise = listen('mod-inbox://changed', () => void refresh());
    return () => {
      refreshSequence.current += 1;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGameId, refresh]);

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
      const next = await modInboxCommands.createModInboxFolder(activeGameId);
      setSnapshot(next);
      syncReadySelection(next);
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
        <div className="grid flex-1 place-items-center">
          <span className="loading loading-spinner loading-lg text-primary" />
        </div>
      ) : snapshot?.rootState === 'missing' ? (
        <MissingInboxState
          rootPath={snapshot.rootPath}
          creating={busyAction === 'create-folder'}
          onCreate={() => void createFolder()}
          onChooseLocation={() => void chooseInboxLocation()}
        />
      ) : snapshot ? (
        <>
          <WorkspacePageContent>
            {activeTab === 'ready' ? (
              <section className="mx-auto max-w-6xl space-y-3">
                {snapshot.readyEntries.length === 0 ? (
                  <EmptyState
                    icon={<PackageOpen size={34} />}
                    title={t('ready.empty_title')}
                    description={t('ready.empty_description')}
                  />
                ) : (
                  snapshot.readyEntries.map((entry) => (
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
                  ))
                )}
              </section>
            ) : (
              <section className="mx-auto max-w-6xl space-y-3">
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
                ) : (
                  snapshot.processedSources.map((source) => (
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
                  ))
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
