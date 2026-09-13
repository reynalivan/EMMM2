import { AlertTriangle, ArrowRight, CheckCircle2, Loader2, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { formatAppError } from '../../../shared/lib/appError';
import { commands, type FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import { closeWorkspaceDialog } from '@/features/workspace-runtime';
import { useWorkspaceRuntimeSelector } from '@/features/workspace-runtime';
import { validateFolderConflictDrafts } from './folderConflictValidation';
import {
  createFolderConflictDraftState,
  reconcileFolderConflictDraftState,
  type FolderConflictCandidateAction,
  type FolderConflictDraftState,
} from './folderConflictDrafts';
import FolderConflictCandidateCard from './FolderConflictCandidateCard';
import FolderConflictCompletion from './FolderConflictCompletion';
import { useApplyFolderConflictActionResult } from './useApplyFolderConflictActionResult';
import { useFolderConflictDetails } from './useFolderConflictDetails';
import {
  reconcileFolderConflictQueue,
  selectNextFolderConflictGroup,
  type CompletedFolderConflict,
} from './folderConflictQueue';

const EMPTY_GROUPS: never[] = [];
export default function FolderConflictManager() {
  const { t } = useTranslation('folder_grid');
  const activeGameId = useAppStore((state) => state.activeGameId);
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const conflictsByGame = useAppStore((state) => state.folderConflictsByGame);
  const reportsByGame = useAppStore((state) => state.folderConflictReportsByGame);
  const report = activeGameId ? (reportsByGame[activeGameId] ?? null) : null;
  const groups = activeGameId
    ? (report?.groups ?? conflictsByGame[activeGameId] ?? EMPTY_GROUPS)
    : EMPTY_GROUPS;
  const resolvedExternally = report?.status === 'resolvedExternally';
  const applyConflictActionResult = useApplyFolderConflictActionResult();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const inputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const [selectedId, setSelectedId] = useState<string | null>(groups[0]?.group_id ?? null);
  const [completedGroups, setCompletedGroups] = useState<CompletedFolderConflict[]>([]);
  const previousGroupsRef = useRef(groups);
  const queueGameIdRef = useRef(activeGameId);
  const resolvedGroupIdRef = useRef<string | null>(null);
  const [queueTotal, setQueueTotal] = useState(groups.length);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const draftsRef = useRef(drafts);
  const [keepPath, setKeepPath] = useState<string | null>(null);
  const keepPathRef = useRef<string | null>(null);
  const [actions, setActions] = useState<Record<string, FolderConflictCandidateAction>>({});
  const actionsRef = useRef(actions);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const errorsRef = useRef(errors);
  const draftGroupKeyRef = useRef<string | null>(null);
  const draftStatesByGroupRef = useRef<Record<string, FolderConflictDraftState>>({});
  const [submitting, setSubmitting] = useState(false);

  const isDialogOpen = dialogState.kind === 'folderConflicts';
  const selected = groups.find((group) => group.group_id === selectedId) ?? groups[0] ?? null;
  const {
    details,
    loading: loadingDetails,
    error: detailsError,
    retry: retryDetails,
  } = useFolderConflictDetails(activeGameId, selected, isDialogOpen);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog && !dialog.open && isDialogOpen) dialog.showModal();
    return () => {
      if (dialog?.open) dialog.close();
    };
  }, [isDialogOpen]);

  useEffect(() => {
    if (queueGameIdRef.current !== activeGameId) {
      queueGameIdRef.current = activeGameId;
      previousGroupsRef.current = groups;
      resolvedGroupIdRef.current = null;
      setQueueTotal(groups.length);
      setCompletedGroups([]);
      setSelectedId(groups[0]?.group_id ?? null);
      return;
    }

    const previous = previousGroupsRef.current;
    const resolvedGroupId = resolvedGroupIdRef.current;
    setCompletedGroups((current) =>
      reconcileFolderConflictQueue(previous, groups, current, resolvedGroupId),
    );
    resolvedGroupIdRef.current = null;
    if (previous.length === 0 && groups.length > 0) {
      setQueueTotal(groups.length);
    }
    setSelectedId((current) => selectNextFolderConflictGroup(previous, groups, current));
    previousGroupsRef.current = groups;

    if (groups.length === 0 && previous.length === 0 && !resolvedExternally) {
      closeWorkspaceDialog('folderConflicts');
    }
  }, [activeGameId, groups, resolvedExternally]);

  useEffect(() => {
    draftsRef.current = drafts;
    actionsRef.current = actions;
    errorsRef.current = errors;
    const draftGroupKey = draftGroupKeyRef.current;
    if (draftGroupKey) {
      draftStatesByGroupRef.current[draftGroupKey] = { drafts, keepPath, errors, actions };
    }
  }, [actions, drafts, errors, keepPath]);

  useEffect(() => {
    if (!selected || !activeGameId) {
      draftGroupKeyRef.current = null;
      keepPathRef.current = null;
      return;
    }
    const draftGroupKey = `${activeGameId}:${selected.group_id}`;
    if (draftGroupKeyRef.current !== draftGroupKey) {
      draftGroupKeyRef.current = draftGroupKey;
      const cached = draftStatesByGroupRef.current[draftGroupKey];
      const next = cached
        ? reconcileFolderConflictDraftState(
            selected.candidates,
            cached.drafts,
            cached.keepPath,
            cached.errors,
            undefined,
            cached.actions,
          )
        : createFolderConflictDraftState(selected.candidates);
      keepPathRef.current = next.keepPath;
      setDrafts(next.drafts);
      setKeepPath(next.keepPath);
      setActions(next.actions);
      setErrors(next.errors);
    } else {
      const next = reconcileFolderConflictDraftState(
        selected.candidates,
        draftsRef.current,
        keepPathRef.current,
        errorsRef.current,
        undefined,
        actionsRef.current,
      );
      keepPathRef.current = next.keepPath;
      setDrafts(next.drafts);
      setKeepPath(next.keepPath);
      setActions(next.actions);
      setErrors(next.errors);
    }
  }, [activeGameId, selected]);

  const totalGroups = Math.max(queueTotal, completedGroups.length + groups.length);
  const progress = useMemo(
    () =>
      t('conflict_manager.queue_progress', {
        resolved: completedGroups.length,
        remaining: groups.length,
        total: totalGroups,
      }),
    [completedGroups.length, groups.length, t, totalGroups],
  );
  const progressValue = totalGroups === 0 ? 0 : (completedGroups.length / totalGroups) * 100;
  const isComplete =
    groups.length === 0 && completedGroups.length > 0 && completedGroups.length >= totalGroups;

  const buildValidationErrors = (
    candidates: FolderNameConflictCandidate[],
    currentDrafts: Record<string, string>,
    currentKeepPath: string | null,
  ) => {
    const validationCodes = validateFolderConflictDrafts(
      candidates,
      currentDrafts,
      currentKeepPath,
    );
    return Object.fromEntries(
      Object.entries(validationCodes).map(([path, code]) => [
        path,
        t(`conflict_manager.validation.${code}`),
      ]),
    );
  };

  const validateRenameDrafts = () => {
    if (!selected) return;
    const renamePlanCandidates = selected.candidates.filter(
      (candidate) => candidate.path === keepPath || actions[candidate.path] !== 'trash',
    );
    setErrors(buildValidationErrors(renamePlanCandidates, drafts, keepPath));
  };

  const renameCandidates =
    selected?.candidates.filter(
      (candidate) => candidate.path !== keepPath && actions[candidate.path] !== 'trash',
    ) ?? [];
  const trashCandidates =
    selected?.candidates.filter(
      (candidate) => candidate.path !== keepPath && actions[candidate.path] === 'trash',
    ) ?? [];
  const actionCount = renameCandidates.length + trashCandidates.length;
  const hasValidationErrors = Object.keys(errors).length > 0;
  const actionButtonLabel =
    actionCount === 0
      ? t('conflict_manager.apply_no_changes')
      : renameCandidates.length > 0 && trashCandidates.length > 0
        ? t('conflict_manager.apply_mixed', { count: actionCount })
        : renameCandidates.length > 0
          ? t(
              renameCandidates.length === 1
                ? 'conflict_manager.apply_rename_one'
                : 'conflict_manager.apply_rename_other',
              { count: renameCandidates.length },
            )
          : t(
              trashCandidates.length === 1
                ? 'conflict_manager.apply_trash_one'
                : 'conflict_manager.apply_trash_other',
              { count: trashCandidates.length },
            );
  const hasNextGroup = groups.length > 1;

  const applySelectedActions = async () => {
    if (!selected || !activeGameId) return;
    const selectedGroup = selected;
    const selectedGameId = activeGameId;
    const selectedKeepPath = keepPath;
    const selectedDrafts = drafts;
    const selectedActions = actions;
    const renamePlanCandidates = selectedGroup.candidates.filter(
      (candidate) =>
        candidate.path === selectedKeepPath || selectedActions[candidate.path] !== 'trash',
    );
    const pendingTrashCandidates = selectedGroup.candidates.filter(
      (candidate) =>
        candidate.path !== selectedKeepPath && selectedActions[candidate.path] === 'trash',
    );
    const nextErrors = buildValidationErrors(
      renamePlanCandidates,
      selectedDrafts,
      selectedKeepPath,
    );
    setErrors(nextErrors);
    const firstInvalid = renamePlanCandidates.find(
      (candidate) => candidate.path !== selectedKeepPath && nextErrors[candidate.path],
    );
    if (firstInvalid) {
      inputRefs.current[firstInvalid.path]?.focus();
      return;
    }

    setSubmitting(true);
    try {
      for (const candidate of pendingTrashCandidates) {
        const result = await commands.trashFolderConflictCandidate(selectedGameId, candidate.path);
        notifyCommittedMutationSyncWarning(result);
        if (
          result.reconcile &&
          applyConflictActionResult(result.reconcile, selectedGroup.group_id)
        ) {
          if (result.reconcile.status === 'Applied') {
            resolvedGroupIdRef.current = selectedGroup.group_id;
          }
        }
      }

      if (renamePlanCandidates.length > 1) {
        const result = await commands.resolveFolderNameConflict(
          selectedGameId,
          selectedGroup.group_id,
          renamePlanCandidates.map((candidate) => ({
            path: candidate.path,
            base_name: selectedDrafts[candidate.path],
          })),
        );
        if (applyConflictActionResult(result, selectedGroup.group_id)) {
          resolvedGroupIdRef.current = selectedGroup.group_id;
        }
      }
      setErrors({});
    } catch (error) {
      toast.error(t('conflict_manager.apply_failed', { error: formatAppError(error) }));
    } finally {
      setSubmitting(false);
    }
  };

  if (!isDialogOpen) return null;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom lg:modal-middle"
      aria-labelledby="folder-conflict-manager-title"
      onClose={() => closeWorkspaceDialog('folderConflicts')}
    >
      <div className="modal-box flex h-[min(88vh,760px)] max-w-6xl flex-col overflow-hidden border border-base-content/10 bg-base-100 p-0 shadow-2xl">
        <header className="flex items-start gap-3 border-b border-base-content/10 p-4">
          <span className="rounded-lg bg-warning/10 p-2 text-warning">
            <AlertTriangle size={20} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="folder-conflict-manager-title" className="font-semibold text-base-content">
              {t('conflict_manager.title')}
            </h2>
            <div className="mt-2 flex items-center gap-2">
              <progress
                className="progress progress-success h-1.5 w-full max-w-56"
                value={progressValue}
                max="100"
                aria-label={progress}
              />
              <span className="shrink-0 text-xs tabular-nums text-base-content/50">{progress}</span>
            </div>
          </div>
          <button
            className="btn btn-sm btn-circle btn-ghost"
            aria-label={t('conflict_manager.close')}
            onClick={() => closeWorkspaceDialog('folderConflicts')}
          >
            <X size={18} />
          </button>
        </header>

        <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-[240px_1fr]">
          <nav
            className="max-h-48 overflow-y-auto border-b border-base-content/10 bg-base-200/40 p-2 lg:max-h-none lg:border-b-0 lg:border-r"
            aria-label={t('conflict_manager.groups')}
          >
            <div className="space-y-0.5">
              {completedGroups.map((group) => (
                <div
                  key={group.fingerprint}
                  className="flex min-h-8 items-center gap-2 px-3 py-2 text-left text-base-content/40"
                >
                  <CheckCircle2 size={14} className="shrink-0 text-success/60" aria-hidden="true" />
                  <span className="truncate text-xs line-through">{group.display_name}</span>
                </div>
              ))}
              {groups.map((group) => {
                const isActive = group.group_id === selected?.group_id;
                return (
                  <button
                    key={group.group_id}
                    className={`flex min-h-9 w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors ${
                      isActive
                        ? 'bg-base-content/10 text-base-content font-medium'
                        : 'text-base-content/70 hover:bg-base-content/5'
                    }`}
                    onClick={() => setSelectedId(group.group_id)}
                  >
                    <span className="truncate text-sm flex-1">{group.display_name}</span>
                    <span
                      className={`text-[10px] px-1.5 py-0.5 rounded-full ${isActive ? 'bg-base-content/20' : 'bg-base-content/10'}`}
                    >
                      {group.candidates.length}
                    </span>
                  </button>
                );
              })}
            </div>
          </nav>

          <section className="min-h-0 overflow-y-auto p-4">
            {isComplete && (
              <FolderConflictCompletion
                kind="action"
                resolved={completedGroups.length}
                total={totalGroups}
                onClose={() => closeWorkspaceDialog('folderConflicts')}
              />
            )}
            {resolvedExternally && !isComplete && (
              <FolderConflictCompletion
                kind="external"
                onClose={() => closeWorkspaceDialog('folderConflicts')}
              />
            )}
            {loadingDetails && (
              <div className="flex items-center justify-center gap-2 py-12 text-base-content/50">
                <Loader2 className="animate-spin motion-reduce:animate-none" size={20} />
                {t('conflict_manager.loading')}
              </div>
            )}
            {!loadingDetails && detailsError && (
              <div className="grid place-items-center gap-3 py-12 text-center text-sm text-error">
                <p>{t('conflict_manager.details_failed', { error: detailsError })}</p>
                <button className="btn btn-sm btn-outline" onClick={retryDetails}>
                  {t('conflict_manager.retry')}
                </button>
              </div>
            )}
            {!resolvedExternally && !loadingDetails && !detailsError && selected && (
              <div className="flex min-h-0 flex-1 flex-col">
                <div className="mb-4">
                  <p className="text-sm font-medium text-base-content/80">
                    {t('conflict_manager.keep_name_instruction')}
                  </p>
                  <p className="text-xs text-base-content/50">
                    {t('conflict_manager.remaining_instruction')}
                  </p>
                </div>

                <div className="flex flex-col gap-3 overflow-y-auto pb-4">
                  {selected.candidates.map((candidate) => (
                    <FolderConflictCandidateCard
                      key={candidate.path}
                      candidate={candidate}
                      detail={details[candidate.path]}
                      isKeep={candidate.path === keepPath}
                      action={actions[candidate.path] ?? 'rename'}
                      value={drafts[candidate.path] ?? candidate.base_name}
                      error={errors[candidate.path]}
                      disabled={submitting}
                      inputRef={(element) => {
                        inputRefs.current[candidate.path] = element;
                      }}
                      onKeep={() => {
                        const next = reconcileFolderConflictDraftState(
                          selected.candidates,
                          draftsRef.current,
                          keepPathRef.current,
                          errorsRef.current,
                          candidate.path,
                          actionsRef.current,
                        );
                        keepPathRef.current = next.keepPath;
                        setKeepPath(next.keepPath);
                        setDrafts(next.drafts);
                        setActions(next.actions);
                        setErrors(next.errors);
                      }}
                      onActionChange={(action) => {
                        setActions((current) => ({ ...current, [candidate.path]: action }));
                        if (action === 'trash') {
                          setErrors((current) => {
                            const next = { ...current };
                            delete next[candidate.path];
                            return next;
                          });
                        }
                      }}
                      onChange={(value) =>
                        setDrafts((current) => ({ ...current, [candidate.path]: value }))
                      }
                      onBlur={validateRenameDrafts}
                    />
                  ))}
                </div>

                {!isComplete && (
                  <div className="mt-auto flex flex-col gap-3 border-t border-base-content/10 pt-4 sm:flex-row sm:items-center sm:justify-between">
                    <p className="text-xs text-base-content/50">
                      {t('conflict_manager.action_summary', {
                        renameCount: renameCandidates.length,
                        trashCount: trashCandidates.length,
                      })}
                    </p>
                    <button
                      className="btn btn-warning btn-sm shrink-0"
                      disabled={submitting || !selected || actionCount === 0 || hasValidationErrors}
                      onClick={applySelectedActions}
                    >
                      {submitting && (
                        <Loader2 size={15} className="animate-spin motion-reduce:animate-none" />
                      )}
                      {actionButtonLabel}
                      {hasNextGroup && (
                        <span className="inline-flex items-center gap-1">
                          <ArrowRight size={14} aria-hidden="true" />
                          {t('conflict_manager.next')}
                        </span>
                      )}
                    </button>
                  </div>
                )}
              </div>
            )}
          </section>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask">
        <button>{t('conflict_manager.close')}</button>
      </form>
    </dialog>
  );
}
