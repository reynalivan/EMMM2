import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { commands } from '../../lib/bindings';
import type {
  DestinationSuggestion,
  GameSchema,
  ImportBatch,
  ImportBatchReport,
  ImportDecision,
  ImportItem,
  JsonValue,
  StableCategory,
} from '../../lib/bindings.gen';
import type { ObjectSummary } from '../../types/object';
import { formatAppError } from '../../lib/appError';
import { toast } from '../../stores/useToastStore';
import { useAppStore } from '../../stores/useAppStore';
import { ImportBatchWizard } from '../match-wizard/ImportBatchWizard';
import { subscribeImportBatchWizard, type ImportBatchLaunchRequest } from './launcher';
import { needsSourceAnalysis, selectLatestResumableBatch } from './resume';

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

  const launch = useCallback(
    async (request: ImportBatchLaunchRequest) => {
      setLoading(true);
      setReport(null);
      try {
        let next: ImportBatch | null;
        if (request.kind === 'existing') {
          next = await commands.getImportBatch(request.batchId);
          if (needsSourceAnalysis(next)) next = await commands.analyzeImportBatch(next.id);
        } else {
          next = await commands.createImportBatch({
            gameId: request.gameId,
            flow: request.flow,
            targetMode: request.targetMode,
            targetObjectId: request.targetObjectId ?? null,
            targetSubpath: request.targetSubpath ?? null,
            sources: request.paths.map((path) => ({ path, sourceKind: null })),
          });
          next = await commands.analyzeImportBatch(next.id);
        }
        setBatch(next);
        await loadContext(next);
      } catch (error) {
        toast.error(t('errors.launch', { error: formatAppError(error) }));
      } finally {
        setLoading(false);
      }
    },
    [loadContext, t],
  );

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

  if (!batch) {
    if (loading)
      return (
        <div className="fixed inset-0 z-[100] grid place-items-center bg-black/30">
          <span className="loading loading-spinner loading-lg text-primary" />
        </div>
      );
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
          if (renamed.matchCategory) await commands.refreshImportItemSuggestions(item.id);
        })
      }
      onRetry={(item) =>
        updateItem(item.id, () => {
          if (item.matchCategory) return commands.refreshImportItemSuggestions(item.id);
          return commands.analyzeImportBatch(batch.id);
        })
      }
      onOpenInExplorer={(item) =>
        commands.openInExplorer(batch.gameId, item.stagingPath ?? item.sourcePath)
      }
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
      }}
      onClose={() => setBatch(null)}
    />
  );
}

export type { DestinationSuggestion, ImportDecision, ImportItem, JsonValue, StableCategory };
