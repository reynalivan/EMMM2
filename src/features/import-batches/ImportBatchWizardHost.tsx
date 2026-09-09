import { useCallback, useEffect, useState } from 'react';
import { Channel } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { commands } from '../../shared/api/tauri/bindings';
import type {
  DestinationSuggestion,
  AppError,
  ExtractionEvent,
  GameSchema,
  ImportBatch,
  ImportBatchReport,
  ImportDecision,
  ImportItem,
  JsonValue,
  ObjectClassificationPreviewItem,
  StableCategory,
} from '../../shared/api/tauri/bindings.gen';
import type { ObjectSummary } from '@/entities/game-object';
import { extractArchiveErrorKind, formatAppError } from '../../shared/lib/appError';
import { toast } from '@/shared/ui/toast';
import { useAppStore } from '@/app/store';
import { ImportBatchWizard } from '@/features/match-wizard/@x/import-batches';
import {
  ImportBatchAnalysisFeedback,
  type ExtractionProgress,
} from './ImportBatchAnalysisFeedback';
import { subscribeImportBatchWizard, type ImportBatchLaunchRequest } from './launcher';
import { openObjectClassificationWizard } from './classificationLauncher';
import { needsSourceAnalysis, selectLatestResumableBatch } from './resume';

const LIBRARY_PREFLIGHT_REMINDER_MS = 24 * 60 * 60 * 1000;

type LibraryPreflight = {
  batch: ImportBatch;
  items: ObjectClassificationPreviewItem[];
  highCount: number;
  mediumCount: number;
};

type ImportWizardPhase =
  | 'idle'
  | 'analyzing'
  | 'checking_library'
  | 'library_prompt'
  | 'classifying_library'
  | 'refreshing_matches'
  | 'reviewing';

function reminderKey(gameId: string): string {
  return `emmm:library-identification-reminder:${gameId}`;
}

function reminderIsActive(gameId: string): boolean {
  try {
    const until = Number.parseInt(localStorage.getItem(reminderKey(gameId)) ?? '', 10);
    return Number.isFinite(until) && until > Date.now();
  } catch {
    return false;
  }
}

function deferReminder(gameId: string): void {
  try {
    localStorage.setItem(reminderKey(gameId), String(Date.now() + LIBRARY_PREFLIGHT_REMINDER_MS));
  } catch {
    // Reminder persistence is optional; import must still continue.
  }
}

type PasswordRequest = {
  batchId: string;
  errorMessage: string;
};

function isArchivePasswordError(error: unknown): error is AppError {
  if (typeof error !== 'object' || error === null || !('type' in error)) return false;
  const type = (error as { type: unknown }).type;
  return type === 'ArchivePasswordRequired' || type === 'ArchivePasswordIncorrect';
}

export function ImportBatchWizardHost() {
  const { t } = useTranslation(['match_wizard']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const [batch, setBatch] = useState<ImportBatch | null>(null);
  const [resumableBatch, setResumableBatch] = useState<ImportBatch | null>(null);
  const [objects, setObjects] = useState<ObjectSummary[]>([]);
  const [schema, setSchema] = useState<GameSchema | null>(null);
  const [busyItemId, setBusyItemId] = useState<string | null>(null);
  const [report, setReport] = useState<ImportBatchReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [extractionProgress, setExtractionProgress] = useState<ExtractionProgress | null>(null);
  const [passwordRequest, setPasswordRequest] = useState<PasswordRequest | null>(null);
  const [libraryPreflight, setLibraryPreflight] = useState<LibraryPreflight | null>(null);
  const [phase, setPhase] = useState<ImportWizardPhase>('idle');

  const formatImportError = useCallback(
    (error: unknown) => {
      const archiveError = extractArchiveErrorKind(error);
      return archiveError ? t(`errors.archive.${archiveError}`) : formatAppError(error);
    },
    [t],
  );

  const loadContext = useCallback(async (next: ImportBatch) => {
    const page = await commands.getObjectsCmd({
      game_id: next.gameId,
      search_query: null,
      object_type: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    });
    setObjects(page.objects);
    const games = await commands.getGames();
    const game = games.find((candidate) => candidate.id === next.gameId);
    setSchema(game ? await commands.getGameSchema(game.game_type) : null);
  }, []);

  const loadBatch = useCallback(
    async (batchId: string) => {
      const next = await commands.getImportBatch(batchId);
      setBatch(next);
      await loadContext(next);
      return next;
    },
    [loadContext],
  );

  const analyzeBatch = useCallback(async (batchId: string, archivePassword: string | null) => {
    const onProgress = new Channel<ExtractionEvent>();
    onProgress.onmessage = (event) => setExtractionProgress(event.data);
    setExtractionProgress(null);

    try {
      const next = await commands.analyzeImportBatchWithOptions(
        { batchId, password: archivePassword, unpackNested: true },
        onProgress,
      );
      setPasswordRequest(null);
      return next;
    } catch (error) {
      const errorMessage = formatAppError(error);
      if (isArchivePasswordError(error)) {
        setPasswordRequest({ batchId, errorMessage });
        return null;
      }
      throw error;
    } finally {
      setExtractionProgress(null);
    }
  }, []);

  const showReview = useCallback(async (next: ImportBatch) => {
    try {
      await commands.markImportBatchReviewStarted(next.id);
    } catch (error) {
      console.warn('Could not persist import review state', error);
    }
    setLibraryPreflight(null);
    setBatch(next);
    setPhase('reviewing');
  }, []);

  const prepareReview = useCallback(
    async (next: ImportBatch) => {
      if (next.targetMode === 'specific' || reminderIsActive(next.gameId)) {
        await showReview(next);
        return;
      }
      setPhase('checking_library');
      try {
        const readiness = await commands.previewImportLibraryReadiness(next.id);
        if (readiness.reviewStarted || readiness.items.length === 0) {
          await showReview(next);
          return;
        }
        setLibraryPreflight({
          batch: next,
          items: readiness.items,
          highCount: readiness.highCount,
          mediumCount: readiness.mediumCount,
        });
        setPhase('library_prompt');
      } catch (error) {
        console.warn('Object Library readiness check skipped', error);
        await showReview(next);
      }
    },
    [showReview],
  );

  const executeLaunch = useCallback(
    async (request: ImportBatchLaunchRequest) => {
      setLoading(true);
      setPhase('analyzing');
      setReport(null);
      setPasswordRequest(null);
      try {
        let next: ImportBatch | null;
        if (request.kind === 'existing') {
          next = await commands.getImportBatch(request.batchId);
          if (needsSourceAnalysis(next)) {
            setBatch(next);
            next = await analyzeBatch(next.id, null);
          }
        } else {
          next = await commands.createImportBatch({
            gameId: request.gameId,
            flow: request.flow,
            targetMode: request.targetMode,
            targetObjectId: request.targetObjectId ?? null,
            targetSubpath: request.targetSubpath ?? null,
            sources: request.paths.map((path) => ({ path, sourceKind: null })),
          });
          setBatch(next);
          next = await analyzeBatch(next.id, null);
        }
        if (!next) return;
        setBatch(next);
        await loadContext(next);
        await prepareReview(next);
      } catch (error) {
        toast.error(t('errors.launch', { error: formatImportError(error) }));
        setPhase('idle');
      } finally {
        setLoading(false);
      }
    },
    [analyzeBatch, formatImportError, loadContext, prepareReview, t],
  );

  const launch = useCallback(
    (request: ImportBatchLaunchRequest) => executeLaunch(request),
    [executeLaunch],
  );

  const retryWithPassword = async (submittedPassword: string) => {
    if (!passwordRequest || loading) return;
    setLoading(true);
    try {
      const next = await analyzeBatch(passwordRequest.batchId, submittedPassword);
      if (!next) return;
      setBatch(next);
      await loadContext(next);
      await prepareReview(next);
    } catch (error) {
      toast.error(t('errors.launch', { error: formatImportError(error) }));
    } finally {
      setLoading(false);
    }
  };

  const cancelPasswordImport = async () => {
    if (!passwordRequest || loading) return;
    setLoading(true);
    try {
      await commands.cancelImportBatch(passwordRequest.batchId);
      setBatch(null);
      setPasswordRequest(null);
      setPhase('idle');
    } catch (error) {
      toast.error(t('errors.update', { error: formatAppError(error) }));
    } finally {
      setLoading(false);
    }
  };

  const refreshResumableBatch = useCallback(async () => {
    if (!activeGameId) {
      setResumableBatch(null);
      return;
    }
    try {
      const batches = await commands.listImportBatches(activeGameId);
      setResumableBatch(selectLatestResumableBatch(batches, activeGameId));
    } catch (error) {
      console.warn('Could not list resumable import batches', error);
    }
  }, [activeGameId]);

  useEffect(() => {
    if (!batch) void refreshResumableBatch();
  }, [batch, refreshResumableBatch]);

  useEffect(() => subscribeImportBatchWizard((request) => void launch(request)), [launch]);

  useEffect(() => {
    const unlisten = listen<{ batch_id: string }>('import:batch-update', (event) => {
      if (batch?.id === event.payload.batch_id) void loadBatch(batch.id);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, [batch?.id, loadBatch]);

  useEffect(() => {
    if (!batch || !['draft', 'analyzing'].includes(batch.status)) return;
    const timer = window.setInterval(() => void loadBatch(batch.id), 1000);
    return () => window.clearInterval(timer);
  }, [batch, loadBatch]);

  const updateItem = async (itemId: string, operation: () => Promise<unknown>) => {
    if (!batch) return;
    setBusyItemId(itemId);
    try {
      await operation();
      await loadBatch(batch.id);
    } catch (error) {
      toast.error(t('errors.update', { error: formatAppError(error) }));
    } finally {
      setBusyItemId(null);
    }
  };

  if (phase === 'analyzing' || phase === 'checking_library' || phase === 'refreshing_matches') {
    return (
      <>
        <div className="fixed inset-0 z-[100] grid place-items-center bg-black/30">
          <div className="text-center">
            <span className="loading loading-spinner loading-lg text-primary" />
            {phase !== 'analyzing' && (
              <p className="mt-3 text-sm text-base-content/70">
                {t(phase === 'checking_library' ? 'checking_library' : 'refreshing_matches')}
              </p>
            )}
          </div>
        </div>
        <ImportBatchAnalysisFeedback
          progress={extractionProgress}
          passwordError={passwordRequest?.errorMessage ?? null}
          loading={loading}
          onPasswordRetry={retryWithPassword}
          onCancel={cancelPasswordImport}
        />
      </>
    );
  }

  if (phase === 'library_prompt' && libraryPreflight) {
    return (
      <dialog open className="modal modal-open" aria-labelledby="library-preflight-title">
        <div className="modal-box max-w-md p-5">
          <h2 id="library-preflight-title" className="text-lg font-bold">
            {t('library_preflight.title', { count: libraryPreflight.items.length })}
          </h2>
          <p className="mt-1 text-sm text-base-content/60">
            {t('library_preflight.summary', {
              high: libraryPreflight.highCount,
              medium: libraryPreflight.mediumCount,
            })}
          </p>
          <div className="modal-action mt-5">
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => {
                const current = libraryPreflight;
                deferReminder(current.batch.gameId);
                void showReview(current.batch);
              }}
            >
              {t('library_preflight.later')}
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                const current = libraryPreflight;
                setLibraryPreflight(null);
                setPhase('classifying_library');
                openObjectClassificationWizard({
                  gameId: current.batch.gameId,
                  objectIds: current.items.map((item) => item.objectId),
                  initialItems: current.items,
                  onComplete: (result) => {
                    if (result === 'cancelled') {
                      void showReview(current.batch);
                      return;
                    }
                    setPhase('refreshing_matches');
                    void (async () => {
                      try {
                        const refreshed = await commands.refreshImportBatchMatches(
                          current.batch.id,
                        );
                        setBatch(refreshed);
                        await loadContext(refreshed);
                        await showReview(refreshed);
                      } catch (error) {
                        toast.warning(
                          t('library_preflight.refresh_failed', {
                            error: formatAppError(error),
                          }),
                        );
                        const fallback = await loadBatch(current.batch.id);
                        await showReview(fallback);
                      }
                    })();
                  },
                });
              }}
            >
              {t('library_preflight.review', { count: libraryPreflight.items.length })}
            </button>
          </div>
        </div>
      </dialog>
    );
  }

  if (phase === 'classifying_library') return null;

  if (!batch || phase === 'idle') {
    return resumableBatch ? (
      <button
        type="button"
        className="btn btn-primary fixed bottom-5 right-5 z-50 shadow-xl"
        onClick={() => void launch({ kind: 'existing', batchId: resumableBatch.id })}
      >
        {t('resume_import', { count: resumableBatch.items.length })}
      </button>
    ) : null;
  }

  return (
    <>
      <ImportBatchWizard
        batch={batch}
        schema={schema}
        objects={objects}
        busyItemId={busyItemId}
        report={report}
        onClassify={(item, category, subCategory, metadata) =>
          updateItem(item.id, async () => {
            await commands.setImportItemClassification({
              itemId: item.id,
              category,
              subCategory,
              metadata,
            });
            await commands.refreshImportItemSuggestions(item.id);
          })
        }
        onChooseDestination={(item, suggestion, decision) =>
          updateItem(item.id, () =>
            commands.setImportItemDecision({
              itemId: item.id,
              decision,
              destinationObjectId: suggestion.objectId,
              destinationPath: suggestion.targetPath,
              canonicalEntryKey: suggestion.canonicalEntryKey,
              matchedAlias: item.canonicalSuggestions[0]?.matchedAlias ?? null,
            }),
          )
        }
        onChooseManualTarget={(item, objectId) =>
          updateItem(item.id, () =>
            commands.setImportItemDecision({
              itemId: item.id,
              decision:
                batch.targetMode === 'specific' && objectId === batch.targetObjectId
                  ? 'keep_specific_target'
                  : 'reallocate',
              destinationObjectId: objectId,
              destinationPath: null,
              canonicalEntryKey: null,
              matchedAlias: null,
            }),
          )
        }
        onSkip={(item) =>
          updateItem(item.id, () =>
            commands.setImportItemDecision({
              itemId: item.id,
              decision: 'skip',
              destinationObjectId: null,
              destinationPath: null,
              canonicalEntryKey: null,
              matchedAlias: null,
            }),
          )
        }
        onRename={(item, plannedName) =>
          updateItem(item.id, async () => {
            const renamed = await commands.renameImportItemPlan({ itemId: item.id, plannedName });
            if (!renamed.matchCategory) {
              const suggestion = renamed.categorySuggestions[0];
              await commands.setImportItemClassification({
                itemId: item.id,
                category: suggestion?.category ?? 'Other',
                subCategory: suggestion?.subCategory ?? null,
                metadata: suggestion?.metadata ?? {},
              });
            }
            await commands.refreshImportItemSuggestions(item.id);
          })
        }
        onRetry={(item) =>
          updateItem(item.id, async () => {
            if (
              [
                'committing',
                'reconciling',
                'finalizing_metadata',
                'partial',
                'metadata_pending',
              ].includes(item.status)
            ) {
              const result = await commands.commitImportBatch({
                batchId: batch.id,
                itemIds: [item.id],
              });
              setReport(result);
              return;
            }
            if (item.status === 'awaiting_category') {
              const suggestion = item.categorySuggestions[0];
              await commands.setImportItemClassification({
                itemId: item.id,
                category: suggestion?.category ?? 'Other',
                subCategory: suggestion?.subCategory ?? null,
                metadata: suggestion?.metadata ?? {},
              });
              return commands.refreshImportItemSuggestions(item.id);
            }
            if (item.matchCategory) return commands.refreshImportItemSuggestions(item.id);
            return analyzeBatch(batch.id, null);
          })
        }
        onOpenInExplorer={(item) => commands.revealImportSource(item.id)}
        onOpenDestination={(item) => commands.revealImportDestination(item.id)}
        onLoadSourcePreview={(item) => commands.getImportSourcePreview(item.id)}
        onCommit={async () => {
          try {
            const recoverableStatuses = new Set([
              'committing',
              'reconciling',
              'finalizing_metadata',
              'partial',
              'metadata_pending',
            ]);
            const recoverableItems = batch.items.filter((item) =>
              recoverableStatuses.has(item.status),
            );
            const commitItems =
              recoverableItems.length > 0
                ? recoverableItems
                : batch.items.filter((item) => item.status === 'ready');
            const result = await commands.commitImportBatch({
              batchId: batch.id,
              itemIds: commitItems.map((item) => item.id),
            });
            setReport(result);
            await loadBatch(batch.id);
          } catch (error) {
            toast.error(t('errors.commit', { error: formatAppError(error) }));
            await loadBatch(batch.id);
          }
        }}
        onCancel={async () => {
          await commands.cancelImportBatch(batch.id);
          setBatch(null);
          setPhase('idle');
        }}
        onClose={() => {
          setBatch(null);
          setPhase('idle');
        }}
      />
      <ImportBatchAnalysisFeedback
        progress={extractionProgress}
        passwordError={passwordRequest?.errorMessage ?? null}
        loading={loading}
        onPasswordRetry={retryWithPassword}
        onCancel={cancelPasswordImport}
      />
    </>
  );
}

export type { DestinationSuggestion, ImportDecision, ImportItem, JsonValue, StableCategory };
