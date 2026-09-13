import { useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Check, ListChecks, Search, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  DestinationSuggestion,
  GameSchema,
  ImportBatch,
  ImportBatchReport,
  ImportDecision,
  ImportItem,
  ImportSourcePreview,
  JsonValue,
  StableCategory,
} from '../../shared/api/tauri/bindings.gen';
import type { ObjectSummary } from '@/entities/game-object';
import { destinationDecision } from './utils/importBatchDecision';
import { ImportBatchWizardItemRow } from './components/ImportBatchWizardItemRow';

type ConfidenceFilter = 'all' | 'high' | 'medium' | 'low' | 'no_match' | 'errors';
type SortMode = 'review' | 'confidence_desc' | 'name';

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
  onOpenDestination?: (item: ImportItem) => Promise<void>;
  onLoadSourcePreview?: (item: ImportItem) => Promise<ImportSourcePreview>;
  onCommit: () => Promise<void>;
  onCancel: () => Promise<void>;
  onClose: () => void;
};

const FILTERS: ConfidenceFilter[] = ['all', 'high', 'medium', 'low', 'no_match', 'errors'];

function isUnresolved(item: ImportItem): boolean {
  return ['awaiting_category', 'awaiting_destination', 'discovered', 'staged'].includes(
    item.status,
  );
}

function needsMetadataRecovery(item: ImportItem): boolean {
  return [
    'committing',
    'reconciling',
    'finalizing_metadata',
    'partial',
    'metadata_pending',
  ].includes(item.status);
}

function filterMatches(item: ImportItem, filter: ConfidenceFilter): boolean {
  if (filter === 'all') return true;
  if (filter === 'errors') {
    return (item.error !== null && !needsMetadataRecovery(item)) || item.status === 'failed';
  }
  return item.error === null && item.confidenceTier === filter;
}

function reviewWeight(item: ImportItem): number {
  if (needsMetadataRecovery(item)) return -2;
  if (item.error || item.status === 'failed') return -1;
  if (
    item.identityMatchStatus === 'needs_review' ||
    item.reviewGate.reasons.length > 0 ||
    item.diagnostics.length > 0 ||
    item.targetComparison !== null
  ) {
    return 0;
  }
  if (item.confidenceTier === 'no_match') return 1;
  if (item.confidenceTier === 'low') return 2;
  if (item.confidenceTier === 'medium') return 3;
  return 4;
}

export function ImportBatchWizard({
  batch,
  objects,
  busyItemId,
  report,
  onCancel,
  onChooseDestination,
  onChooseManualTarget,
  onClose,
  onCommit,
  onLoadSourcePreview,
  onOpenDestination,
  onOpenInExplorer,
  onRename,
  onRetry,
  onSkip,
}: Props) {
  // TanStack Virtual exposes imperative functions; React Compiler must not memoize this component.
  'use no memo';
  const { t } = useTranslation(['match_wizard', 'common']);
  const [filter, setFilter] = useState<ConfidenceFilter>('all');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<SortMode>('review');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const listRef = useRef<HTMLDivElement>(null);
  const processing = batch.status === 'draft' || batch.status === 'analyzing';
  const terminal = ['done', 'cancelled'].includes(batch.status);
  const readyItems = batch.items.filter((item) => item.status === 'ready');
  const recoveryItems = batch.items.filter(needsMetadataRecovery);
  const actionableItems = recoveryItems.length > 0 ? recoveryItems : readyItems;
  const unresolvedItems = batch.items.filter(isUnresolved);
  const skippedCount = batch.items.filter((item) => item.status === 'skipped').length;
  const errorCount = batch.items.filter(
    (item) => (item.error && !needsMetadataRecovery(item)) || item.status === 'failed',
  ).length;
  const normalizedSearch = search.trim().toLocaleLowerCase();

  const visibleItems = useMemo(() => {
    const items = batch.items
      .filter((item) => filterMatches(item, filter))
      .filter((item) => {
        if (!normalizedSearch) return true;
        return [item.plannedName, item.sourcePath]
          .join(' ')
          .toLocaleLowerCase()
          .includes(normalizedSearch);
      });
    return [...items].sort((left, right) => {
      if (sort === 'confidence_desc') {
        return right.confidencePercentage - left.confidencePercentage;
      }
      if (sort === 'name') return left.plannedName.localeCompare(right.plannedName);
      return (
        reviewWeight(left) - reviewWeight(right) ||
        left.confidencePercentage - right.confidencePercentage
      );
    });
  }, [batch.items, filter, normalizedSearch, sort]);

  const visibleIds = useMemo(() => visibleItems.map((item) => item.id), [visibleItems]);
  const rowVirtualizer = useVirtualizer({
    count: visibleItems.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 112,
    overscan: 8,
  });
  const virtualRows = rowVirtualizer.getVirtualItems();
  const firstVirtualRow = virtualRows[0];
  const lastVirtualRow = virtualRows[virtualRows.length - 1];
  const bottomSpacer = lastVirtualRow ? rowVirtualizer.getTotalSize() - lastVirtualRow.end : 0;
  const allVisibleSelected =
    visibleIds.length > 0 && visibleIds.every((itemId) => selected.has(itemId));
  const headerSummary = [
    t('summary.items', { count: batch.items.length }),
    unresolvedItems.length > 0 && t('summary.needs_review', { count: unresolvedItems.length }),
    readyItems.length > 0 && t('summary.ready', { count: readyItems.length }),
    skippedCount > 0 && t('summary.skipped', { count: skippedCount }),
    errorCount > 0 && t('filters.errors', { count: errorCount }),
  ]
    .filter((part): part is string => Boolean(part))
    .join(' · ');

  useEffect(() => {
    const currentIds = new Set(batch.items.map((item) => item.id));
    setSelected((previous) => new Set([...previous].filter((itemId) => currentIds.has(itemId))));
  }, [batch.items]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        (target instanceof Element &&
          target.matches('input, textarea, select, [contenteditable="true"]')) ||
        !(event.ctrlKey || event.metaKey) ||
        event.key.toLocaleLowerCase() !== 'a'
      ) {
        return;
      }
      event.preventDefault();
      if (event.shiftKey) setSelected(new Set());
      else setSelected(new Set(visibleIds));
    };
    window.addEventListener('keydown', handleShortcut);
    return () => window.removeEventListener('keydown', handleShortcut);
  }, [visibleIds]);

  const selectAllVisible = () => setSelected(new Set(visibleIds));
  const selectNone = () => setSelected(new Set());

  const bulkProceed = async () => {
    for (const item of batch.items.filter((candidate) => selected.has(candidate.id))) {
      if (item.status === 'ready' || !['pending', 'skip'].includes(item.decision)) continue;
      if (item.reviewGate.reasons.length > 0) continue;
      if (item.canonicalSuggestions[0]?.matchStatus !== 'auto_matched') continue;
      const suggestion = item.destinationSuggestions[0];
      if (suggestion) {
        await onChooseDestination(item, suggestion, destinationDecision(batch, suggestion));
      }
    }
    selectNone();
  };

  const bulkSkip = async () => {
    for (const item of batch.items.filter((candidate) => selected.has(candidate.id))) {
      if (item.decision !== 'skip') await onSkip(item);
    }
    selectNone();
  };

  return (
    <dialog open className="modal modal-open" aria-labelledby="match-wizard-title">
      <div className="modal-box flex max-h-[min(92vh,58rem)] max-w-[88rem] flex-col bg-base-200 p-0">
        <header className="flex items-start justify-between gap-4 border-b border-base-300 px-5 py-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
                <ListChecks size={18} aria-hidden="true" />
              </span>
              <h2 id="match-wizard-title" className="truncate text-lg font-bold">
                {t('review_title')}
              </h2>
            </div>
            <p className="mt-0.5 text-xs text-base-content/60">{headerSummary}</p>
          </div>
          <button
            type="button"
            className="btn btn-ghost btn-square btn-sm"
            onClick={onClose}
            disabled={processing}
            aria-label={t('common:actions.close')}
          >
            <X size={18} />
          </button>
        </header>

        {processing ? (
          <div className="grid min-h-72 flex-1 place-items-center">
            <div className="text-center">
              <span className="loading loading-spinner loading-lg text-primary" />
              <p className="mt-3 text-sm text-base-content/70">{t('analyzing')}</p>
            </div>
          </div>
        ) : (
          <>
            <div className="border-b border-base-300 px-5 py-3">
              <div className="flex flex-wrap items-center gap-2">
                <label className="relative min-w-64 flex-1">
                  <Search
                    size={14}
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40"
                  />
                  <input
                    className="input input-sm input-bordered w-full pl-8"
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                    placeholder={t('search_placeholder')}
                  />
                </label>
                <select
                  className="select select-sm select-bordered"
                  value={sort}
                  onChange={(event) => setSort(event.target.value as SortMode)}
                  aria-label={t('sort.label')}
                >
                  <option value="review">{t('sort.review')}</option>
                  <option value="confidence_desc">{t('sort.confidence_desc')}</option>
                  <option value="name">{t('sort.name')}</option>
                </select>
              </div>

              <div className="mt-3 flex flex-wrap items-center gap-1.5">
                {FILTERS.map((candidate) => {
                  const count = batch.items.filter((item) => filterMatches(item, candidate)).length;
                  return (
                    <button
                      key={candidate}
                      type="button"
                      className={`btn btn-xs ${filter === candidate ? 'btn-primary' : 'btn-ghost'}`}
                      onClick={() => setFilter(candidate)}
                    >
                      {t(`filters.${candidate}`)} <span className="opacity-60">{count}</span>
                    </button>
                  );
                })}
                <div className="ml-auto flex flex-wrap items-center justify-end gap-2">
                  <div className="flex items-center rounded-lg border border-base-300 bg-base-100 p-0.5">
                    <span className="pl-2 text-xs text-base-content/55">
                      {t('selection.count', { count: selected.size })}
                    </span>
                    <span className="mx-1 h-4 w-px bg-base-300" />
                    <button
                      type="button"
                      className="btn btn-ghost btn-xs"
                      onClick={selectAllVisible}
                      aria-keyshortcuts="Control+A Meta+A"
                    >
                      {t('selection.all')}
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost btn-xs"
                      onClick={selectNone}
                      aria-keyshortcuts="Control+Shift+A Meta+Shift+A"
                    >
                      {t('selection.none')}
                    </button>
                  </div>
                  {selected.size > 0 && (
                    <div className="flex items-center gap-1">
                      <button
                        type="button"
                        className="btn btn-success btn-xs"
                        disabled={busyItemId !== null}
                        onClick={() => void bulkProceed()}
                      >
                        {t('actions.set_proceed')}
                      </button>
                      <button
                        type="button"
                        className="btn btn-warning btn-xs"
                        disabled={busyItemId !== null}
                        onClick={() => void bulkSkip()}
                      >
                        {t('actions.set_skip')}
                      </button>
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div
              ref={listRef}
              className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-5 py-3"
            >
              <div className="overflow-visible rounded-xl border border-base-300 bg-base-100">
                <table className="table table-fixed table-sm w-full">
                  <colgroup>
                    <col className="w-10" />
                    <col className="w-[30%]" />
                    <col className="w-[36%]" />
                    <col className="w-[18%]" />
                    <col className="w-32" />
                  </colgroup>
                  <thead className="sticky top-0 z-20 bg-base-100 shadow-[0_1px_0_hsl(var(--bc)/0.1)]">
                    <tr className="text-[10px] uppercase tracking-wider text-base-content/55">
                      <th className="w-10 text-center">
                        <input
                          type="checkbox"
                          className="checkbox checkbox-xs"
                          checked={allVisibleSelected}
                          onChange={() => (allVisibleSelected ? selectNone() : selectAllVisible())}
                          aria-label={t('selection.all')}
                        />
                      </th>
                      <th>{t('import_as_column')}</th>
                      <th>{t('columns.destination')}</th>
                      <th>{t('match_column')}</th>
                      <th>{t('columns.decision')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {firstVirtualRow && (
                      <tr aria-hidden="true">
                        <td colSpan={5} className="p-0" style={{ height: firstVirtualRow.start }} />
                      </tr>
                    )}
                    {virtualRows.map((virtualRow) => {
                      const item = visibleItems[virtualRow.index];
                      return (
                        <ImportBatchWizardItemRow
                          key={item.id}
                          batch={batch}
                          item={item}
                          objects={objects}
                          busy={busyItemId === item.id}
                          selected={selected.has(item.id)}
                          virtualIndex={virtualRow.index}
                          measureElement={(element) => rowVirtualizer.measureElement(element)}
                          onToggleSelected={() =>
                            setSelected((previous) => {
                              const next = new Set(previous);
                              if (next.has(item.id)) next.delete(item.id);
                              else next.add(item.id);
                              return next;
                            })
                          }
                          onChooseDestination={onChooseDestination}
                          onChooseManualTarget={onChooseManualTarget}
                          onSkip={onSkip}
                          onRename={onRename}
                          onRetry={onRetry}
                          onRevealSource={onOpenInExplorer}
                          onRevealDestination={onOpenDestination}
                          onLoadSourcePreview={onLoadSourcePreview}
                        />
                      );
                    })}
                    {bottomSpacer > 0 && (
                      <tr aria-hidden="true">
                        <td colSpan={5} className="p-0" style={{ height: bottomSpacer }} />
                      </tr>
                    )}
                    {visibleItems.length === 0 && (
                      <tr>
                        <td colSpan={5} className="py-12 text-center text-sm text-base-content/45">
                          {t('no_filtered_items')}
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>

              {report && (
                <div className="alert mt-4 border border-success/20 bg-success/10 text-sm">
                  <Check size={18} className="shrink-0 text-success" aria-hidden="true" />
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
            </div>
          </>
        )}

        <footer className="flex items-center justify-between gap-3 border-t border-base-300 px-5 py-3">
          <p className="text-xs text-base-content/60">
            {terminal
              ? t('footer.finished')
              : unresolvedItems.length > 0
                ? t('footer.resolve_first', { count: unresolvedItems.length })
                : actionableItems.length > 0
                  ? recoveryItems.length > 0
                    ? t('footer.recovery', { count: recoveryItems.length })
                    : t('footer.ready', { count: actionableItems.length })
                  : t('footer.no_items')}
          </p>
          <div className="flex gap-2">
            {!terminal && (
              <button
                type="button"
                className="btn btn-ghost"
                disabled={processing}
                onClick={() => void onCancel()}
              >
                {t('common:actions.cancel')}
              </button>
            )}
            {terminal ? (
              <button type="button" className="btn btn-primary" onClick={onClose}>
                {t('common:actions.close')}
              </button>
            ) : (
              <button
                type="button"
                className="btn btn-primary"
                disabled={processing || actionableItems.length === 0 || unresolvedItems.length > 0}
                onClick={() => void onCommit()}
              >
                {recoveryItems.length > 0
                  ? t('actions.resume')
                  : t('actions.import', { count: actionableItems.length })}
              </button>
            )}
          </div>
        </footer>
      </div>
    </dialog>
  );
}
