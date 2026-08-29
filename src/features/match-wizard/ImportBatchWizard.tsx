import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  DestinationSuggestion,
  GameSchema,
  ImportBatch,
  ImportBatchReport,
  ImportDecision,
  ImportItem,
  JsonValue,
  StableCategory,
} from '../../shared/api/tauri/bindings.gen';
import type { ObjectSummary } from '@/entities/game-object/model/object';
import { destinationDecision } from './utils/importBatchDecision';
import { ImportBatchWizardItemRow } from './components/ImportBatchWizardItemRow';

type Props = {
  batch: ImportBatch;
  schema: GameSchema | null;
  objects: ObjectSummary[];
  busyItemId: string | null;
  report: ImportBatchReport | null;
  onClassify: (
    item: ImportItem,
    category: StableCategory,
    subCategory: string | null,
    metadata: JsonValue,
  ) => Promise<void>;
  onChooseDestination: (
    item: ImportItem,
    suggestion: DestinationSuggestion,
    decision: ImportDecision,
  ) => Promise<void>;
  onChooseManualTarget: (item: ImportItem, objectId: string) => Promise<void>;
  onSkip: (item: ImportItem) => Promise<void>;
  onRename: (item: ImportItem, plannedName: string) => Promise<void>;
  onRetry: (item: ImportItem) => Promise<void>;
  onOpenInExplorer: (item: ImportItem) => Promise<void>;
  onCommit: () => Promise<void>;
  onCancel: () => Promise<void>;
  onClose: () => void;
};

function isUnresolved(item: ImportItem): boolean {
  return ['awaiting_category', 'awaiting_destination', 'discovered', 'staged'].includes(
    item.status,
  );
}

export function ImportBatchWizard({
  batch,
  schema,
  objects,
  busyItemId,
  report,
  onCancel,
  onChooseDestination,
  onChooseManualTarget,
  onClassify,
  onClose,
  onCommit,
  onOpenInExplorer,
  onRename,
  onRetry,
  onSkip,
}: Props) {
  const { t } = useTranslation(['match_wizard', 'common']);
  const readyItems = useMemo(
    () => batch.items.filter((item) => item.status === 'ready'),
    [batch.items],
  );
  const recoveryItems = useMemo(
    () =>
      batch.items.filter((item) =>
        [
          'committing',
          'reconciling',
          'finalizing_metadata',
          'partial',
          'metadata_pending',
        ].includes(item.status),
      ),
    [batch.items],
  );
  const actionableItems = recoveryItems.length > 0 ? recoveryItems : readyItems;
  const unresolvedItems = batch.items.filter(isUnresolved);
  const processing = batch.status === 'draft' || batch.status === 'analyzing';
  const terminal = ['done', 'cancelled'].includes(batch.status);

  const confirmHigh = async () => {
    for (const item of batch.items) {
      const suggestion = item.destinationSuggestions[0];
      if (item.status === 'awaiting_destination' && suggestion?.confidenceTier === 'high') {
        await onChooseDestination(item, suggestion, destinationDecision(batch, suggestion));
      }
    }
  };

  const skipMediumLow = async () => {
    for (const item of batch.items) {
      if (item.status === 'awaiting_destination' && item.confidenceTier !== 'high') {
        await onSkip(item);
      }
    }
  };

  return (
    <dialog open className="modal modal-open" aria-label={t('title')}>
      <div className="modal-box max-w-7xl h-[88vh] flex flex-col">
        <header className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-xl font-bold">{t('title')}</h2>
            <p className="text-sm opacity-60">
              {t('batch_summary', { count: batch.items.length, flow: batch.flow })}
            </p>
          </div>
          <button className="btn btn-ghost btn-sm" onClick={onClose} disabled={processing}>
            {t('common:actions.close')}
          </button>
        </header>

        <ol className="steps steps-horizontal w-full my-4 text-xs">
          {(['sources', 'category', 'destination', 'validation', 'result'] as const).map((step) => (
            <li key={step} className="step">
              {t('steps.' + step)}
            </li>
          ))}
        </ol>

        {processing ? (
          <div className="flex-1 grid place-items-center">
            <div className="text-center">
              <span className="loading loading-spinner loading-lg text-primary" />
              <p className="mt-3">{t('analyzing')}</p>
            </div>
          </div>
        ) : (
          <>
            {!terminal && (
              <div className="flex flex-wrap gap-2 mb-3">
                <button className="btn btn-primary btn-sm" onClick={() => void confirmHigh()}>
                  {t('actions.confirm_high')}
                </button>
                <button className="btn btn-outline btn-sm" onClick={() => void skipMediumLow()}>
                  {t('actions.skip_medium_low')}
                </button>
              </div>
            )}

            <div className="overflow-auto flex-1 border border-base-300 rounded-box">
              <table className="table table-sm table-pin-rows">
                <thead>
                  <tr>
                    <th>{t('columns.source')}</th>
                    <th>{t('columns.category')}</th>
                    <th>{t('columns.canonical')}</th>
                    <th>{t('columns.destination')}</th>
                    <th>{t('columns.confidence')}</th>
                    <th>{t('columns.decision')}</th>
                    <th>{t('columns.actions')}</th>
                  </tr>
                </thead>
                <tbody>
                  {batch.items.map((item) => (
                    <ImportBatchWizardItemRow
                      key={item.id}
                      batch={batch}
                      item={item}
                      schema={schema}
                      objects={objects}
                      busy={busyItemId === item.id}
                      onClassify={onClassify}
                      onChooseDestination={onChooseDestination}
                      onChooseManualTarget={onChooseManualTarget}
                      onSkip={onSkip}
                      onRename={onRename}
                      onRetry={onRetry}
                      onOpenInExplorer={onOpenInExplorer}
                    />
                  ))}
                </tbody>
              </table>
            </div>

            {report && (
              <div className="alert mt-3">
                <span>
                  {t('result_summary', {
                    moved: report.moved,
                    reallocated: report.reallocated,
                    created: report.createdCanonicalFolders,
                    skipped: report.skipped,
                    collisions: report.collisions,
                    pending: report.metadataPending,
                    failed: report.failed,
                  })}
                </span>
              </div>
            )}
          </>
        )}

        <footer className="modal-action">
          {!terminal && (
            <button className="btn btn-ghost" disabled={processing} onClick={() => void onCancel()}>
              {t('common:actions.cancel')}
            </button>
          )}
          {terminal ? (
            <button className="btn btn-primary" onClick={onClose}>
              {t('common:actions.close')}
            </button>
          ) : (
            <button
              className="btn btn-primary"
              disabled={processing || actionableItems.length === 0 || unresolvedItems.length > 0}
              onClick={() => void onCommit()}
            >
              {t('actions.commit', { count: actionableItems.length })}
            </button>
          )}
        </footer>
      </div>
    </dialog>
  );
}
