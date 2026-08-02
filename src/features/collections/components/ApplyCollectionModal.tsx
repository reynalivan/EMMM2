import { useState } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle, ArrowRight, CheckCircle2, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import {
  useApplyCollectionPreview,
  useApplyCollection,
  useApplyProgress,
  useReplaceCollectionWithCurrentState,
} from '../hooks/useCollections';
import { useAppStore } from '../../../stores/useAppStore';
import { CollectionTreeView } from './CollectionTreeView';
import type { PreviewTreeNode } from '../../../types/collection';
import { ApplyCollectionActions } from './ApplyCollectionActions';
import { extractMissingModsPayload, formatAppError } from '../../../lib/appError';
import type { ApplyResult } from '../../../types/collection';

interface ApplyCollectionModalProps {
  collectionId: string;
  onClose: () => void;
}

function SummaryStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-xl border border-base-content/8 bg-base-300/20 px-3 py-2">
      <div className="text-[10px] uppercase tracking-[0.18em] text-base-content/45">{label}</div>
      <div className="mt-1 text-sm font-semibold text-base-content/85">{value}</div>
    </div>
  );
}

interface StatePanelProps {
  containerClass: string;
  heading: string;
  title: string;
  titleClass: string;
  summary: { active_root_count: number; enabled_object_count: number; object_count: number };
  nodes?: PreviewTreeNode[];
  colorClass: string;
  emptyMessage: string;
  t: TFunction;
}

/** One side of the before/after comparison. Both sides render identically. */
function StatePanel({
  containerClass,
  heading,
  title,
  titleClass,
  summary,
  nodes,
  colorClass,
  emptyMessage,
  t,
}: StatePanelProps) {
  return (
    <div className={`flex-1 flex flex-col max-h-full overflow-hidden ${containerClass}`}>
      <div className="p-4 bg-base-300/30 border-b border-base-content/5 shrink-0">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-base-content/45">
              {heading}
            </div>
            <div className={`mt-1 text-lg font-semibold ${titleClass}`}>{title}</div>
          </div>
          <SummaryStat
            label={t('collections:apply.summary.mods', 'Active Roots')}
            value={summary.active_root_count}
          />
        </div>
      </div>
      <div className="p-4 grid grid-cols-2 gap-3 border-b border-base-content/5 bg-base-100/30">
        <SummaryStat
          label={t('collections:apply.summary.objects_on', 'Objects On')}
          value={summary.enabled_object_count}
        />
        <SummaryStat
          label={t('collections:apply.summary.objects', 'Objects')}
          value={summary.object_count}
        />
      </div>
      <div className="flex-1 overflow-y-auto custom-scrollbar p-4">
        <CollectionTreeView nodes={nodes} colorClass={colorClass} emptyMessage={emptyMessage} />
      </div>
    </div>
  );
}

export function ApplyCollectionModal({ collectionId, onClose }: ApplyCollectionModalProps) {
  const { t } = useTranslation(['collections', 'layout', 'common']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const [missingPaths, setMissingPaths] = useState<string[] | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);
  const applyMutation = useApplyCollection();
  const replaceMutation = useReplaceCollectionWithCurrentState();
  const previewQuery = useApplyCollectionPreview(activeGameId, collectionId);
  const progressQuery = useApplyProgress(activeGameId, applyMutation.isPending);
  const preview = previewQuery.data;

  const currentStateLabel = preview
    ? preview.current_state_name || t('collections:list.item.current_runtime', 'Current Runtime')
    : t('common:status.loading');

  const confirmApplyAction = async (ignoreMissing: boolean) => {
    if (!activeGameId) {
      return;
    }

    try {
      const applyResult = await applyMutation.mutateAsync({
        gameId: activeGameId,
        collectionId,
        ignoreMissing,
      });
      setMissingPaths(null);
      setResult(applyResult);
    } catch (error) {
      const missingMods = extractMissingModsPayload(error);
      if (missingMods) {
        setMissingPaths(missingMods.paths);
      }
    }
  };

  const updateOriginalCollection = () => {
    if (!activeGameId || !result?.partial_apply) {
      return;
    }

    replaceMutation.mutate(
      {
        gameId: activeGameId,
        collectionId,
      },
      {
        onSuccess: onClose,
      },
    );
  };

  if (previewQuery.isError) {
    return createPortal(
      <dialog className="modal modal-open z-100">
        <div className="modal-box bg-base-200 border border-error/20 shadow-2xl max-w-lg">
          <h3 className="font-bold text-lg text-error flex items-center gap-2">
            <AlertTriangle size={20} /> {t('collections:apply.failed.title')}
          </h3>
          <p className="py-4 text-sm text-base-content/80">
            {formatAppError(previewQuery.error)}
          </p>
          <div className="modal-action">
            <button className="btn btn-neutral" onClick={onClose}>
              {t('collections:apply.failed.close')}
            </button>
          </div>
        </div>
      </dialog>,
      document.body,
    );
  }

  return createPortal(
    <dialog className="modal modal-open z-100">
      <div className="modal-box bg-base-200 border border-base-content/10 shadow-2xl max-w-6xl w-11/12 flex flex-col max-h-[85vh] p-0">
        <div className="p-6 border-b border-base-content/5 shrink-0 bg-base-300 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-warning/20 text-warning flex items-center justify-center shrink-0">
              {result ? <CheckCircle2 size={20} /> : <AlertTriangle size={20} />}
            </div>
            <div>
              <h3 className="font-bold text-lg">
                {result
                  ? t('collections:apply.success.title', 'Collection Applied')
                  : preview
                    ? t('collections:apply.title', { name: preview.collection_name })
                    : t('collections:apply.title_loading')}
              </h3>
              <p className="text-sm text-base-content/70">
                {result
                  ? t('collections:apply.success.desc', 'The corridor state has been refreshed.')
                  : t('collections:apply.desc')}
              </p>
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-hidden bg-base-100 flex min-h-[50vh]">
          {previewQuery.isLoading ? (
            <div className="flex flex-col h-full items-center justify-center w-full text-base-content/50">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-primary" />
              <p>{t('collections:apply.actions.loading')}</p>
            </div>
          ) : missingPaths ? (
            <div className="w-full p-6 flex flex-col gap-4 justify-center">
              <div className="rounded-2xl border border-error/20 bg-error/8 p-4">
                <div className="text-sm font-semibold text-error/85">
                  {t('collections:apply.missing.title', 'Missing Mods')}
                </div>
                <div className="mt-1 text-xs text-base-content/65">
                  {t(
                    'collections:apply.missing.desc',
                    'Some saved mod folders no longer exist on disk.',
                  )}
                </div>
              </div>
              <div className="rounded-2xl border border-base-content/8 bg-base-200/40 p-4">
                <ul className="space-y-1 text-sm font-mono text-base-content/75">
                  {missingPaths.map((path) => (
                    <li key={path}>{path}</li>
                  ))}
                </ul>
              </div>
            </div>
          ) : result ? (
            <div className="w-full p-6 flex flex-col gap-4 justify-center">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                <SummaryStat
                  label={t('collections:apply.summary.enabled', 'Enabled')}
                  value={result.mods_enabled}
                />
                <SummaryStat
                  label={t('collections:apply.summary.disabled', 'Disabled')}
                  value={result.mods_disabled}
                />
                <SummaryStat
                  label={t('collections:apply.summary.objects', 'Objects Updated')}
                  value={result.objects_toggled}
                />
              </div>
              <div className="rounded-2xl border border-success/20 bg-success/8 p-4">
                <div className="text-sm font-semibold text-success/85">
                  {result.final_state_name ?? preview?.collection_name}
                </div>
                <div className="mt-1 text-xs text-base-content/60">
                  {result.final_mode ?? t('collections:apply.success.runtime', 'Runtime snapshot')}
                </div>
              </div>
              {result.partial_apply && (
                <div className="rounded-2xl border border-warning/20 bg-warning/8 p-4">
                  <div className="text-sm font-semibold text-warning/85">
                    {t('collections:apply.partial.title', 'Applied Available Files')}
                  </div>
                  <div className="mt-1 text-xs text-base-content/65">
                    {t(
                      'collections:apply.partial.desc',
                      'Some saved files were missing. The current state is unsaved until you update the original collection.',
                    )}
                  </div>
                  <ul className="mt-3 space-y-1 text-xs font-mono text-base-content/70">
                    {result.skipped_missing_paths.map((path) => (
                      <li key={path}>{path}</li>
                    ))}
                  </ul>
                </div>
              )}
              {result.warnings.length > 0 && (
                <div className="rounded-2xl border border-warning/20 bg-warning/8 p-4">
                  <div className="text-sm font-semibold text-warning/85">
                    {t('collections:apply.missing.title', 'Warnings')}
                  </div>
                  <ul className="mt-2 space-y-1 text-xs text-base-content/70">
                    {result.warnings.map((warning) => (
                      <li key={warning}>{warning}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          ) : preview ? (
            <div className="flex w-full divide-x divide-base-content/5">
              <StatePanel
                containerClass="bg-base-100/30"
                heading={t('collections:apply.panels.before', 'Current State')}
                title={currentStateLabel}
                titleClass="text-base-content/85"
                summary={preview.current_projected_state.summary}
                nodes={preview.current_tree_nodes}
                colorClass="text-error/70"
                emptyMessage={t('collections:apply.panels.empty_before')}
                t={t}
              />

              <div className="flex items-center justify-center -mx-3 z-10 w-0">
                <div className="w-8 h-8 rounded-full bg-base-300 border border-base-content/10 flex items-center justify-center shadow-lg text-base-content/50">
                  <ArrowRight size={16} />
                </div>
              </div>

              <StatePanel
                containerClass="bg-base-200/20"
                heading={t('collections:apply.panels.after', 'Target State')}
                title={preview.collection_name}
                titleClass="text-primary"
                summary={preview.target_projected_state.summary}
                nodes={preview.target_tree_nodes}
                colorClass="text-success/70"
                emptyMessage={t('collections:apply.panels.empty_after')}
                t={t}
              />
            </div>
          ) : null}
        </div>

        {applyMutation.isPending && (
          <div className="border-t border-base-content/5 bg-base-300/50 px-6 py-4">
            <div className="flex items-center gap-3">
              <Loader2 size={16} className="animate-spin text-warning" />
              <div className="flex-1">
                <div className="text-sm font-medium text-base-content/85">
                  {progressQuery.data?.phase ?? 'preparing'}
                </div>
                <div className="text-xs text-base-content/55">
                  {progressQuery.data?.current_item ?? t('collections:apply.actions.loading')}
                </div>
              </div>
              <div className="text-xs font-mono text-base-content/55">
                {progressQuery.data
                  ? `${progressQuery.data.completed}/${progressQuery.data.total || 0}`
                  : ''}
              </div>
            </div>
          </div>
        )}

        <ApplyCollectionActions
          result={result}
          missingPaths={missingPaths}
          isApplying={applyMutation.isPending}
          isReplacing={replaceMutation.isPending}
          isPreviewLoading={previewQuery.isLoading}
          onClose={onClose}
          onDismissMissing={() => setMissingPaths(null)}
          onConfirm={() => {
            void confirmApplyAction(!!missingPaths);
          }}
          onUpdateOriginal={updateOriginalCollection}
        />
      </div>

      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose} disabled={applyMutation.isPending}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>,
    document.body,
  );
}
