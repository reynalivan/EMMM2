import { useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, ArrowRight, FolderOpen, Loader2, Split, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
import { formatAppError } from '../../../shared/lib/appError';
import {
  commands,
  type RenameConfirmationGroup,
  type RenameConfirmationResolution,
} from '../../../shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import { applyDiskReconcileResult } from '@/features/file-watcher';
import { closeWorkspaceDialog } from '@/features/workspace-runtime';
import { useWorkspaceRuntimeSelector } from '@/features/workspace-runtime';

const EMPTY_GROUPS: never[] = [];

type Decision = {
  action: '' | 'Rename' | 'Separate';
  previousPath: string;
  currentPath: string;
};

function initialDecision(group: RenameConfirmationGroup): Decision {
  return {
    action: '',
    previousPath: group.previous_paths[0] ?? '',
    currentPath: group.current_paths[0] ?? '',
  };
}

export default function RenameConfirmationManager() {
  const { t } = useTranslation('folder_grid');
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const activeGameId = useAppStore((state) => state.activeGameId);
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const groupsByGame = useAppStore((state) => state.renameConfirmationsByGame);
  const groups = activeGameId ? (groupsByGame[activeGameId] ?? EMPTY_GROUPS) : EMPTY_GROUPS;
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [selectedId, setSelectedId] = useState<string | null>(groups[0]?.group_id ?? null);
  const [decisions, setDecisions] = useState<Record<string, Decision>>({});
  const [submitting, setSubmitting] = useState(false);
  const selected = groups.find((group) => group.group_id === selectedId) ?? groups[0] ?? null;

  const reportKey = groups
    .map((group) => group.group_id)
    .slice()
    .sort()
    .join('|');

  const isDialogOpen = dialogState.kind === 'renameConfirmations' && groups.length > 0;

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog && !dialog.open && isDialogOpen) dialog.showModal();
    return () => {
      if (dialog?.open) dialog.close();
    };
  }, [isDialogOpen]);

  useEffect(() => {
    if (groups.length === 0) {
      closeWorkspaceDialog('renameConfirmations');
      return;
    }
    setDecisions((current) =>
      Object.fromEntries(
        groups.map((group) => [group.group_id, current[group.group_id] ?? initialDecision(group)]),
      ),
    );
    if (!groups.some((group) => group.group_id === selectedId)) {
      setSelectedId(groups[0].group_id);
    }
  }, [groups, reportKey, selectedId]);

  const completedCount = useMemo(
    () => groups.filter((group) => Boolean(decisions[group.group_id]?.action)).length,
    [decisions, groups],
  );
  const allValid =
    groups.length > 0 &&
    groups.every((group) => {
      const decision = decisions[group.group_id];
      return (
        decision?.action === 'Separate' ||
        (decision?.action === 'Rename' && decision.previousPath && decision.currentPath)
      );
    });

  const updateDecision = (groupId: string, patch: Partial<Decision>) => {
    setDecisions((current) => ({
      ...current,
      [groupId]: {
        ...(current[groupId] ?? { action: '', previousPath: '', currentPath: '' }),
        ...patch,
      },
    }));
  };

  const applyDecisions = async () => {
    if (!activeGameId || !allValid) return;
    const resolutions: RenameConfirmationResolution[] = groups.map((group) => {
      const decision = decisions[group.group_id];
      return {
        group_id: group.group_id,
        action: decision.action === 'Separate' ? 'Separate' : 'Rename',
        previous_path: decision.action === 'Rename' ? decision.previousPath : null,
        current_path: decision.action === 'Rename' ? decision.currentPath : null,
      };
    });

    setSubmitting(true);
    try {
      const result = await commands.resolveRenameConfirmations(activeGameId, resolutions);
      if (useAppStore.getState().activeGameId === result.game_id) {
        applyDiskReconcileResult(
          result,
          queryClient,
          activeGame?.id === result.game_id ? activeGame : null,
        );
        toast.success(t('rename_confirmation.resolved'));
      } else {
        useAppStore.getState().setRenameConfirmations(result.game_id, result.rename_confirmations);
      }
    } catch (error) {
      toast.error(t('rename_confirmation.resolve_failed', { error: formatAppError(error) }));
    } finally {
      setSubmitting(false);
    }
  };

  const decision = selected ? (decisions[selected.group_id] ?? initialDecision(selected)) : null;

  if (!isDialogOpen) return null;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom lg:modal-middle"
      onClose={() => closeWorkspaceDialog('renameConfirmations')}
    >
      <div className="modal-box flex h-[min(86vh,700px)] max-w-4xl flex-col overflow-hidden border border-base-content/10 bg-base-100 p-0 shadow-2xl">
        <header className="flex items-start gap-3 border-b border-base-content/10 p-4">
          <span className="rounded-lg bg-info/10 p-2 text-info">
            <AlertTriangle size={20} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 className="font-semibold text-base-content">{t('rename_confirmation.title')}</h2>
            <p className="text-sm text-base-content/60">{t('rename_confirmation.description')}</p>
          </div>
          <button
            className="btn btn-sm btn-circle btn-ghost"
            aria-label={t('rename_confirmation.close')}
            onClick={() => closeWorkspaceDialog('renameConfirmations')}
          >
            <X size={18} />
          </button>
        </header>

        <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-[240px_1fr]">
          <nav
            className="max-h-48 overflow-y-auto border-b border-base-content/10 bg-base-200/40 p-3 lg:max-h-none lg:border-b-0 lg:border-r"
            aria-label={t('rename_confirmation.groups')}
          >
            <p className="mb-2 px-2 text-xs font-semibold uppercase tracking-wide text-base-content/50">
              {t('rename_confirmation.progress', {
                completed: completedCount,
                total: groups.length,
              })}
            </p>
            <div className="space-y-1">
              {groups.map((group, index) => (
                <button
                  key={group.group_id}
                  className={`btn btn-sm h-auto w-full justify-start py-2 text-left ${group.group_id === selected?.group_id ? 'btn-info' : 'btn-ghost'}`}
                  onClick={() => setSelectedId(group.group_id)}
                >
                  <span className="truncate">
                    {index + 1}. {t(`rename_confirmation.kind.${group.kind}`)}
                  </span>
                  {decisions[group.group_id]?.action && (
                    <span
                      className="badge badge-success badge-xs ml-auto"
                      aria-label={t('rename_confirmation.reviewed')}
                    />
                  )}
                </button>
              ))}
            </div>
          </nav>

          {selected && decision && (
            <section className="min-h-0 overflow-y-auto p-4">
              <div className="mb-4 rounded-lg border border-info/20 bg-info/5 p-3 text-sm text-base-content/70">
                {t(`rename_confirmation.reason.${selected.reason}`)}
              </div>

              <fieldset className="space-y-3">
                <legend className="mb-2 font-semibold text-base-content">
                  {t('rename_confirmation.choose_action')}
                </legend>
                <label className="flex cursor-pointer items-start gap-3 rounded-lg border border-base-content/10 p-3 has-[:checked]:border-info has-[:checked]:bg-info/5">
                  <input
                    type="radio"
                    className="radio radio-info radio-sm mt-0.5"
                    name={`action-${selected.group_id}`}
                    checked={decision.action === 'Rename'}
                    onChange={() => updateDecision(selected.group_id, { action: 'Rename' })}
                  />
                  <span>
                    <span className="flex items-center gap-2 font-medium">
                      <ArrowRight size={16} /> {t('rename_confirmation.rename_action')}
                    </span>
                    <span className="mt-1 block text-xs text-base-content/60">
                      {t('rename_confirmation.rename_help')}
                    </span>
                  </span>
                </label>

                {decision.action === 'Rename' && (
                  <div className="grid grid-cols-1 gap-3 rounded-lg bg-base-200/50 p-3 md:grid-cols-[1fr_auto_1fr] md:items-end">
                    <label className="form-control min-w-0">
                      <span className="label-text mb-1 text-xs">
                        {t('rename_confirmation.previous_path')}
                      </span>
                      <select
                        className="select select-bordered select-sm w-full"
                        value={decision.previousPath}
                        onChange={(event) =>
                          updateDecision(selected.group_id, { previousPath: event.target.value })
                        }
                      >
                        {selected.previous_paths.map((path) => (
                          <option key={path} value={path}>
                            {path}
                          </option>
                        ))}
                      </select>
                    </label>
                    <ArrowRight className="hidden text-base-content/40 md:block" size={18} />
                    <label className="form-control min-w-0">
                      <span className="label-text mb-1 text-xs">
                        {t('rename_confirmation.current_path')}
                      </span>
                      <div className="join w-full">
                        <select
                          className="select select-bordered select-sm join-item min-w-0 flex-1"
                          value={decision.currentPath}
                          onChange={(event) =>
                            updateDecision(selected.group_id, { currentPath: event.target.value })
                          }
                        >
                          {selected.current_paths.map((path) => (
                            <option key={path} value={path}>
                              {path}
                            </option>
                          ))}
                        </select>
                        <button
                          type="button"
                          className="btn btn-sm join-item"
                          aria-label={t('rename_confirmation.open_folder')}
                          onClick={() =>
                            commands
                              .openInExplorer(activeGameId as string, decision.currentPath)
                              .catch((error) => toast.error(formatAppError(error)))
                          }
                        >
                          <FolderOpen size={15} />
                        </button>
                      </div>
                    </label>
                  </div>
                )}

                <label className="flex cursor-pointer items-start gap-3 rounded-lg border border-base-content/10 p-3 has-[:checked]:border-info has-[:checked]:bg-info/5">
                  <input
                    type="radio"
                    className="radio radio-info radio-sm mt-0.5"
                    name={`action-${selected.group_id}`}
                    checked={decision.action === 'Separate'}
                    onChange={() => updateDecision(selected.group_id, { action: 'Separate' })}
                  />
                  <span>
                    <span className="flex items-center gap-2 font-medium">
                      <Split size={16} /> {t('rename_confirmation.separate_action')}
                    </span>
                    <span className="mt-1 block text-xs text-base-content/60">
                      {t('rename_confirmation.separate_help')}
                    </span>
                  </span>
                </label>
              </fieldset>
            </section>
          )}
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-base-content/10 p-4">
          <span className="text-xs text-base-content/50">
            {allValid ? t('rename_confirmation.ready') : t('rename_confirmation.review_all')}
          </span>
          <button
            className="btn btn-info btn-sm"
            disabled={!allValid || submitting}
            onClick={applyDecisions}
          >
            {submitting && (
              <Loader2 size={15} className="animate-spin motion-reduce:animate-none" />
            )}
            {t('rename_confirmation.apply')}
          </button>
        </footer>
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask">
        <button>{t('rename_confirmation.close')}</button>
      </form>
    </dialog>
  );
}
