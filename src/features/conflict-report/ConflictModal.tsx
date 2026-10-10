import { useDialogSync } from '../../shared/lib/hooks/useDialogSync';
import { AlertTriangle, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '@/entities/workspace';
import {
  useBulkToggle,
  useSetActiveModConflictGroupsIgnored,
} from '@/features/mod-runtime/@x/conflict-report';
import { commands } from '../../shared/api/tauri/bindings';
import { formatAppError } from '../../shared/lib/appError';
import ConflictGroupCard from './ConflictGroupCard';
import ConflictResolutionSummary from './ConflictResolutionSummary';
import {
  chooseConflictModSetWinner,
  groupConflictsByModSet,
  setModDecision,
  summarizeConflictResolution,
  type ConflictDecisions,
  type ConflictModSet,
} from './conflictResolution';

type ConflictFilter = 'unresolved' | 'ignored';

const EMPTY_IGNORED_GROUP_KEYS: ReadonlySet<string> = new Set();

interface ConflictModalProps {
  open: boolean;
  onClose: () => void;
  conflicts: ConflictInfo[];
  gameId: string;
  ignoredGroupKeys?: ReadonlySet<string>;
  isIgnoredGroupsLoading?: boolean;
  ignoredGroupsError?: unknown;
  onRetryIgnoredGroups?: () => void;
}

export default function ConflictModal({
  open,
  onClose,
  conflicts,
  gameId,
  ignoredGroupKeys = EMPTY_IGNORED_GROUP_KEYS,
  isIgnoredGroupsLoading = false,
  ignoredGroupsError,
  onRetryIgnoredGroups,
}: ConflictModalProps) {
  const { t } = useTranslation(['scanner', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const bulkToggle = useBulkToggle();
  const ignoredGroupsMutation = useSetActiveModConflictGroupsIgnored();
  const [decisions, setDecisions] = useState<ConflictDecisions>(new Map());
  const [reviewing, setReviewing] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [pathErrors, setPathErrors] = useState<ReadonlyMap<string, string>>(new Map());
  const [actionError, setActionError] = useState<string | null>(null);
  const [filter, setFilter] = useState<ConflictFilter>('unresolved');
  const [selectedGroupKeys, setSelectedGroupKeys] = useState<ReadonlySet<string>>(new Set());

  useDialogSync(dialogRef, open);

  useEffect(() => {
    if (open) return;
    setDecisions(new Map());
    setReviewing(false);
    setPathErrors(new Map());
    setActionError(null);
    setFilter('unresolved');
    setSelectedGroupKeys(new Set());
  }, [open]);

  const conflictSets = useMemo(() => groupConflictsByModSet(conflicts), [conflicts]);
  const unresolvedConflictSets = useMemo(
    () => conflictSets.filter((conflictSet) => !ignoredGroupKeys.has(conflictSet.key)),
    [conflictSets, ignoredGroupKeys],
  );
  const ignoredConflictSets = useMemo(
    () => conflictSets.filter((conflictSet) => ignoredGroupKeys.has(conflictSet.key)),
    [conflictSets, ignoredGroupKeys],
  );
  const displayedConflictSets =
    filter === 'unresolved' ? unresolvedConflictSets : ignoredConflictSets;
  const summary = useMemo(
    () =>
      summarizeConflictResolution(
        unresolvedConflictSets.flatMap((conflictSet) => conflictSet.conflicts),
        decisions,
      ),
    [decisions, unresolvedConflictSets],
  );
  const isBusy = isSubmitting || bulkToggle.isPending || ignoredGroupsMutation.isPending;
  const allDisplayedSelected =
    displayedConflictSets.length > 0 &&
    displayedConflictSets.every((conflictSet) => selectedGroupKeys.has(conflictSet.key));

  const handleClose = () => {
    if (isBusy) return;
    setDecisions(new Map());
    setReviewing(false);
    setPathErrors(new Map());
    setActionError(null);
    setFilter('unresolved');
    setSelectedGroupKeys(new Set());
    onClose();
  };

  const handleOpenFolder = async (path: string) => {
    setActionError(null);
    try {
      await commands.openInExplorer(gameId, path);
    } catch (error) {
      setActionError(
        t('scanner:conflict_modal.open_failed', {
          error: formatAppError(error),
        }),
      );
    }
  };

  const handleIgnoreGroups = async (groups: ConflictModSet[], ignored: boolean) => {
    if (isBusy || groups.length === 0) return;

    setActionError(null);
    try {
      await ignoredGroupsMutation.mutateAsync({
        gameId,
        modPathGroups: groups.map((group) => group.modPaths),
        ignored,
      });
      const groupPaths = new Set(groups.flatMap((group) => group.modPaths));
      setDecisions((current) => new Map([...current].filter(([path]) => !groupPaths.has(path))));
      setPathErrors((current) => new Map([...current].filter(([path]) => !groupPaths.has(path))));
      setReviewing(false);
      setSelectedGroupKeys((current) => {
        const next = new Set(current);
        for (const group of groups) {
          next.delete(group.key);
        }
        return next;
      });
    } catch (error) {
      setActionError(
        t('scanner:conflict_modal.ignore_failed', {
          error: formatAppError(error),
        }),
      );
    }
  };

  const handleDisable = async () => {
    if (isBusy || summary.disablePaths.length === 0) return;

    setIsSubmitting(true);
    setActionError(null);
    setPathErrors(new Map());
    try {
      const result = await bulkToggle.mutateAsync({
        gameId,
        paths: summary.disablePaths,
        enable: false,
      });
      const failures = new Map(
        result.failures.map((failure) => [failure.path, formatAppError(failure.error)]),
      );
      setPathErrors(failures);
      setDecisions(new Map(result.failures.map((failure) => [failure.path, 'disable'] as const)));
      setReviewing(false);
    } catch (error) {
      setActionError(
        t('scanner:conflict_modal.submit_failed', {
          error: formatAppError(error),
        }),
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const selectGroups = (groups: ConflictModSet[], selected: boolean) => {
    setSelectedGroupKeys((current) => {
      const next = new Set(current);
      for (const group of groups) {
        if (selected) {
          next.add(group.key);
        } else {
          next.delete(group.key);
        }
      }
      return next;
    });
  };

  const selectedConflictSets = displayedConflictSets.filter((conflictSet) =>
    selectedGroupKeys.has(conflictSet.key),
  );

  return (
    <dialog ref={dialogRef} className="modal bg-overlay-mask" onClose={handleClose}>
      <div className="modal-box flex max-h-[calc(100dvh-2rem)] w-[calc(100%-1rem)] max-w-5xl flex-col rounded-xl border border-warning/30 bg-base-100 p-0 shadow-2xl">
        <header className="flex shrink-0 items-start gap-3 border-b border-base-content/10 px-4 py-4 sm:px-6">
          <AlertTriangle className="mt-0.5 shrink-0 text-warning" aria-hidden="true" />
          <div className="min-w-0 flex-1 pr-2">
            <h3 className="text-lg font-bold text-base-content">
              {t('scanner:conflict_modal.title')}
            </h3>
            <p className="mt-1 text-sm text-base-content/65" aria-live="polite">
              {t('scanner:conflict_modal.queue_status', {
                unresolved: unresolvedConflictSets.length,
                ignored: ignoredConflictSets.length,
              })}
            </p>
          </div>
          <button
            type="button"
            className="btn btn-sm btn-square min-h-11 min-w-11 btn-ghost focus-visible:outline focus-visible:outline-2 focus-visible:outline-warning focus-visible:outline-offset-2"
            onClick={handleClose}
            disabled={isBusy}
            aria-label={t('common:actions.close')}
          >
            <X size={18} />
          </button>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6">
          {actionError && (
            <div className="alert alert-error mb-4 text-sm" role="alert">
              <span>{actionError}</span>
            </div>
          )}

          {Boolean(ignoredGroupsError) && (
            <div className="alert alert-warning mb-4 flex-wrap text-sm" role="alert">
              <span>
                {String(
                  t('scanner:conflict_modal.load_ignored_failed', {
                    error: formatAppError(ignoredGroupsError),
                  }),
                )}
              </span>
              {onRetryIgnoredGroups && (
                <button
                  type="button"
                  className="btn btn-sm min-h-11 btn-ghost"
                  onClick={onRetryIgnoredGroups}
                  disabled={isBusy}
                >
                  {t('scanner:conflict_modal.retry_ignored')}
                </button>
              )}
            </div>
          )}

          {conflicts.length === 0 ? (
            <p className="py-8 text-center text-success" role="status">
              {t('scanner:conflict_modal.empty')}
            </p>
          ) : reviewing ? (
            <ConflictResolutionSummary
              summary={summary}
              isPending={isBusy}
              onBack={() => setReviewing(false)}
              onConfirm={handleDisable}
            />
          ) : (
            <div className="flex flex-col gap-4">
              <div className="alert alert-warning text-sm">
                <span>{t('scanner:conflict_modal.description')}</span>
              </div>

              <div className="flex flex-col gap-3 border-b border-base-content/10 pb-3 sm:flex-row sm:items-center sm:justify-between">
                <div
                  className="join"
                  role="tablist"
                  aria-label={t('scanner:conflict_modal.filter')}
                >
                  {(['unresolved', 'ignored'] as const).map((filterValue) => (
                    <button
                      key={filterValue}
                      type="button"
                      role="tab"
                      aria-selected={filter === filterValue}
                      className={`btn join-item btn-sm min-h-11 px-3 ${
                        filter === filterValue ? 'btn-warning' : 'btn-ghost'
                      }`}
                      onClick={() => {
                        setFilter(filterValue);
                        setSelectedGroupKeys(new Set());
                      }}
                      disabled={isBusy}
                    >
                      {t(`scanner:conflict_modal.filter_${filterValue}`, {
                        count:
                          filterValue === 'unresolved'
                            ? unresolvedConflictSets.length
                            : ignoredConflictSets.length,
                      })}
                    </button>
                  ))}
                </div>
                {isIgnoredGroupsLoading && (
                  <div
                    className="flex items-center gap-2 text-xs text-base-content/65"
                    role="status"
                  >
                    <span className="loading loading-spinner loading-xs" aria-hidden="true" />
                    {t('scanner:conflict_modal.loading_ignored')}
                  </div>
                )}
              </div>

              {displayedConflictSets.length === 0 ? (
                <p className="py-8 text-center text-sm text-base-content/65" role="status">
                  {t(`scanner:conflict_modal.empty_${filter}`)}
                </p>
              ) : (
                <>
                  <div className="flex flex-col gap-2 border-b border-base-content/10 pb-3 sm:flex-row sm:items-center sm:justify-between">
                    <label className="flex min-h-11 cursor-pointer items-center gap-3 text-sm text-base-content/75">
                      <input
                        type="checkbox"
                        className="checkbox checkbox-warning checkbox-sm"
                        checked={allDisplayedSelected}
                        onChange={(event) =>
                          selectGroups(displayedConflictSets, event.target.checked)
                        }
                        disabled={isBusy}
                        aria-label={t('scanner:conflict_modal.select_all')}
                      />
                      {t('scanner:conflict_modal.select_all')}
                    </label>
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-xs text-base-content/65" aria-live="polite">
                        {t('scanner:conflict_modal.selected_groups', {
                          count: selectedConflictSets.length,
                        })}
                      </span>
                      {selectedConflictSets.length > 0 && (
                        <button
                          type="button"
                          className="btn btn-sm min-h-11 btn-ghost"
                          onClick={() => setSelectedGroupKeys(new Set())}
                          disabled={isBusy}
                        >
                          {t('scanner:conflict_modal.clear_selection')}
                        </button>
                      )}
                      <button
                        type="button"
                        className="btn btn-sm min-h-11 btn-ghost"
                        onClick={() =>
                          void handleIgnoreGroups(selectedConflictSets, filter === 'unresolved')
                        }
                        disabled={isBusy || selectedConflictSets.length === 0}
                      >
                        {t(
                          `scanner:conflict_modal.${
                            filter === 'unresolved' ? 'ignore_selected' : 'restore_selected'
                          }`,
                          { count: selectedConflictSets.length },
                        )}
                      </button>
                      {filter === 'unresolved' && (
                        <button
                          type="button"
                          className="btn btn-sm min-h-11 btn-warning"
                          onClick={() => void handleIgnoreGroups(unresolvedConflictSets, true)}
                          disabled={isBusy || unresolvedConflictSets.length === 0}
                        >
                          {t('scanner:conflict_modal.ignore_all', {
                            count: unresolvedConflictSets.length,
                          })}
                        </button>
                      )}
                    </div>
                  </div>

                  {displayedConflictSets.map((conflictSet) => {
                    const ignored = ignoredGroupKeys.has(conflictSet.key);
                    return (
                      <ConflictGroupCard
                        key={conflictSet.key}
                        conflictSet={conflictSet}
                        decisions={decisions}
                        pathErrors={pathErrors}
                        disabled={isBusy}
                        ignored={ignored}
                        selected={selectedGroupKeys.has(conflictSet.key)}
                        onKeep={(path) => {
                          setPathErrors(new Map());
                          setDecisions((current) =>
                            chooseConflictModSetWinner(current, conflictSet, path),
                          );
                        }}
                        onDisable={(path) => {
                          setPathErrors(new Map());
                          setDecisions((current) => setModDecision(current, path, 'disable'));
                        }}
                        onOpenFolder={(path) => void handleOpenFolder(path)}
                        onIgnoreGroup={() => void handleIgnoreGroups([conflictSet], true)}
                        onRestoreGroup={() => void handleIgnoreGroups([conflictSet], false)}
                        onSelectedChange={(selected) => selectGroups([conflictSet], selected)}
                      />
                    );
                  })}
                </>
              )}
            </div>
          )}
        </div>

        {!reviewing && conflicts.length > 0 && (
          <footer className="flex shrink-0 flex-wrap items-center gap-2 border-t border-base-content/10 px-4 py-4 sm:px-6">
            <div className="mr-auto text-xs text-base-content/65">
              {t('scanner:conflict_modal.impact', {
                resolved: summary.resolvedCount,
                total: unresolvedConflictSets.length,
                count: summary.disablePaths.length,
              })}
            </div>
            {summary.disablePaths.length > 0 && (
              <button
                type="button"
                className="btn btn-sm min-h-11 btn-ghost"
                onClick={() => {
                  setDecisions(new Map());
                  setPathErrors(new Map());
                }}
                disabled={isBusy}
              >
                {t('scanner:conflict_modal.clear_choices')}
              </button>
            )}
            <button
              type="button"
              className="btn btn-sm min-h-11 btn-ghost"
              onClick={handleClose}
              disabled={isBusy}
            >
              {t('common:actions.close')}
            </button>
            <button
              type="button"
              className="btn btn-sm min-h-11 btn-warning"
              onClick={() => setReviewing(true)}
              disabled={isBusy || summary.disablePaths.length === 0}
            >
              {t('scanner:conflict_modal.review_changes')}
            </button>
          </footer>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={handleClose} disabled={isBusy} aria-label={t('common:actions.close')} />
      </form>
    </dialog>
  );
}
