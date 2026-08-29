import { AlertTriangle, CheckCircle2, Loader2, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { formatAppError } from '../../../shared/lib/appError';
import { commands, type FolderNameConflictCandidate } from '../../../shared/api/tauri/bindings';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';
import { useAppStore } from '../../../app/store/useAppStore';
import { toast } from '../../../app/store/useToastStore';
import { closeWorkspaceDialog } from '@/features/workspace-runtime/state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '@/features/workspace-runtime/state/workspaceStoreBridge';
import { validateFolderConflictDrafts } from './folderConflictValidation';
import {
  createFolderConflictDraftState,
  reconcileFolderConflictDraftState,
  type FolderConflictDraftState,
} from './folderConflictDrafts';
import FolderConflictActionSummary from './FolderConflictActionSummary';
import FolderConflictCandidateCard from './FolderConflictCandidateCard';
import FolderConflictCompletion from './FolderConflictCompletion';
import FolderConflictTrashDialog from './FolderConflictTrashDialog';
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
  const groups = activeGameId ? (conflictsByGame[activeGameId] ?? EMPTY_GROUPS) : EMPTY_GROUPS;
  const applyConflictActionResult = useApplyFolderConflictActionResult();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const inputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const [selectedId, setSelectedId] = useState<string | null>(groups[0]?.group_id ?? null);
  const [completedGroups, setCompletedGroups] = useState<CompletedFolderConflict[]>([]);
  const previousGroupsRef = useRef(groups);
  const queueGameIdRef = useRef(activeGameId);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const draftsRef = useRef(drafts);
  const [keepPath, setKeepPath] = useState<string | null>(null);
  const keepPathRef = useRef<string | null>(null);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const errorsRef = useRef(errors);
  const draftGroupKeyRef = useRef<string | null>(null);
  const draftStatesByGroupRef = useRef<Record<string, FolderConflictDraftState>>({});
  const [submitting, setSubmitting] = useState(false);
  const [confirmTrash, setConfirmTrash] = useState<{
    candidate: FolderNameConflictCandidate;
    returnFocus: HTMLElement;
  } | null>(null);

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
      setCompletedGroups([]);
      setSelectedId(groups[0]?.group_id ?? null);
      return;
    }

    const previous = previousGroupsRef.current;
    setCompletedGroups((current) => reconcileFolderConflictQueue(previous, groups, current));
    setSelectedId((current) => selectNextFolderConflictGroup(previous, groups, current));
    previousGroupsRef.current = groups;

    if (groups.length === 0 && previous.length === 0) {
      closeWorkspaceDialog('folderConflicts');
    }
  }, [activeGameId, groups]);

  useEffect(() => {
    draftsRef.current = drafts;
    errorsRef.current = errors;
    const draftGroupKey = draftGroupKeyRef.current;
    if (draftGroupKey) {
      draftStatesByGroupRef.current[draftGroupKey] = { drafts, keepPath, errors };
    }
  }, [drafts, errors, keepPath]);

  useEffect(() => {
    setConfirmTrash(null);
  }, [activeGameId]);

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
          )
        : createFolderConflictDraftState(selected.candidates);
      keepPathRef.current = next.keepPath;
      setDrafts(next.drafts);
      setKeepPath(next.keepPath);
      setErrors(next.errors);
    } else {
      const next = reconcileFolderConflictDraftState(
        selected.candidates,
        draftsRef.current,
        keepPathRef.current,
        errorsRef.current,
      );
      keepPathRef.current = next.keepPath;
      setDrafts(next.drafts);
      setKeepPath(next.keepPath);
      setErrors(next.errors);
    }
  }, [activeGameId, selected]);

  const progress = useMemo(
    () =>
      t('conflict_manager.queue_progress', {
        resolved: completedGroups.length,
        remaining: groups.length,
        total: completedGroups.length + groups.length,
      }),
    [completedGroups.length, groups.length, t],
  );
  const totalGroups = completedGroups.length + groups.length;
  const progressValue = totalGroups === 0 ? 0 : (completedGroups.length / totalGroups) * 100;
  const isComplete = groups.length === 0 && completedGroups.length > 0;

  const resolveSelected = async () => {
    if (!selected || !activeGameId) return;
    const validationCodes = validateFolderConflictDrafts(selected.candidates, drafts);
    const nextErrors = Object.fromEntries(
      Object.entries(validationCodes).map(([path, code]) => [
        path,
        t(`conflict_manager.validation.${code}`),
      ]),
    );
    setErrors(nextErrors);
    const firstInvalid = selected.candidates.find(
      (candidate) => candidate.path !== keepPath && nextErrors[candidate.path],
    );
    if (firstInvalid) {
      inputRefs.current[firstInvalid.path]?.focus();
      return;
    }

    setSubmitting(true);
    try {
      const result = await commands.resolveFolderNameConflict(
        activeGameId,
        selected.group_id,
        selected.candidates.map((candidate) => ({
          path: candidate.path,
          base_name: drafts[candidate.path],
        })),
      );
      if (applyConflictActionResult(result)) {
        setErrors({});
      }
    } catch (error) {
      toast.error(t('conflict_manager.resolve_failed', { error: formatAppError(error) }));
    } finally {
      setSubmitting(false);
    }
  };

  const moveToTrash = async () => {
    if (!confirmTrash || !activeGameId) return;
    setSubmitting(true);
    try {
      const result = await commands.trashFolderConflictCandidate(
        activeGameId,
        confirmTrash.candidate.path,
      );
      setConfirmTrash(null);
      notifyCommittedMutationSyncWarning(result);
      if (result.reconcile) {
        applyConflictActionResult(result.reconcile);
      }
    } catch (error) {
      toast.error(t('conflict_manager.trash_failed', { error: formatAppError(error) }));
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
            <p className="text-sm text-base-content/60">{t('conflict_manager.description')}</p>
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
            className="max-h-48 overflow-y-auto border-b border-base-content/10 bg-base-200/40 p-3 lg:max-h-none lg:border-b-0 lg:border-r"
            aria-label={t('conflict_manager.groups')}
          >
            <p className="mb-2 px-2 text-xs font-semibold uppercase tracking-wide text-base-content/50">
              {progress}
            </p>
            <div className="space-y-1">
              {completedGroups.map((group) => (
                <div
                  key={group.fingerprint}
                  className="flex min-h-8 items-center gap-2 px-2 py-1.5 text-left text-base-content/50"
                >
                  <CheckCircle2 size={15} className="shrink-0 text-success" aria-hidden="true" />
                  <span className="truncate text-sm line-through">{group.display_name}</span>
                </div>
              ))}
              {groups.map((group, index) => (
                <button
                  key={group.group_id}
                  className={`btn btn-sm h-auto w-full justify-start py-2 text-left ${group.group_id === selected?.group_id ? 'btn-warning' : 'btn-ghost'}`}
                  onClick={() => setSelectedId(group.group_id)}
                >
                  <span className="truncate">
                    {completedGroups.length + index + 1}. {group.display_name}
                  </span>
                  <span className="badge badge-sm ml-auto">{group.candidates.length}</span>
                </button>
              ))}
            </div>
          </nav>

          <section className="min-h-0 overflow-y-auto p-4">
            {isComplete && (
              <FolderConflictCompletion
                resolved={completedGroups.length}
                total={totalGroups}
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
            {!loadingDetails && !detailsError && selected && (
              <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
                {selected.candidates.map((candidate) => (
                  <FolderConflictCandidateCard
                    key={candidate.path}
                    candidate={candidate}
                    detail={details[candidate.path]}
                    isKeep={candidate.path === keepPath}
                    value={drafts[candidate.path] ?? candidate.base_name}
                    error={errors[candidate.path]}
                    disabled={submitting}
                    inputRef={(element) => {
                      inputRefs.current[candidate.path] = element;
                    }}
                    onKeep={() => {
                      keepPathRef.current = candidate.path;
                      setKeepPath(candidate.path);
                      setDrafts((current) => ({
                        ...current,
                        [candidate.path]: candidate.base_name,
                      }));
                      setErrors({});
                    }}
                    onChange={(value) =>
                      setDrafts((current) => ({ ...current, [candidate.path]: value }))
                    }
                    onTrash={(returnFocus) => setConfirmTrash({ candidate, returnFocus })}
                  />
                ))}
              </div>
            )}
          </section>
        </div>

        {!isComplete && (
          <FolderConflictActionSummary
            group={selected}
            keepPath={keepPath}
            drafts={drafts}
            submitting={submitting}
            onResolve={resolveSelected}
          />
        )}

        {confirmTrash && (
          <FolderConflictTrashDialog
            candidate={confirmTrash.candidate}
            detail={details[confirmTrash.candidate.path]}
            returnFocus={confirmTrash.returnFocus}
            submitting={submitting}
            onCancel={() => setConfirmTrash(null)}
            onConfirm={moveToTrash}
          />
        )}
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask">
        <button>{t('conflict_manager.close')}</button>
      </form>
    </dialog>
  );
}
