import { useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle, Loader2, ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useApplyCollectionPreview, useApplyCollection } from '../hooks/useCollections';
import { useAppStore } from '../../../stores/useAppStore';
import { CollectionTreeView } from './CollectionTreeView';
import type { CollectionMember } from '../../../types/collection';

interface ApplyCollectionModalProps {
  collectionId: string;
  onClose: () => void;
}

export function ApplyCollectionModal({ collectionId, onClose }: ApplyCollectionModalProps) {
  const { t } = useTranslation('collections');
  const { activeGameId, safeMode } = useAppStore();
  const [missingPaths, setMissingPaths] = useState<string[] | null>(null);
  const applyMutation = useApplyCollection();

  const previewQuery = useApplyCollectionPreview(activeGameId, collectionId, safeMode);
  const preview = previewQuery.data;

  const { leavingMembers, targetMembers } = useMemo(() => {
    if (!preview) return { leavingMembers: [] as CollectionMember[], targetMembers: [] as CollectionMember[] };
    return {
      leavingMembers: preview.current_members.filter(
        (m) => m.kind === 'object' || m.kind === 'root' || ((m.kind === 'mod' || m.kind === 'nested') && m.is_enabled),
      ),
      targetMembers: preview.target_members.filter(
        (m) => m.kind === 'object' || m.kind === 'root' || ((m.kind === 'mod' || m.kind === 'nested') && m.is_enabled),
      ),
    };
  }, [preview]);

  if (previewQuery.isError) {
    return createPortal(
      <dialog className="modal modal-open z-100">
        <div className="modal-box bg-base-200 border border-error/20 shadow-2xl max-w-lg">
          <h3 className="font-bold text-lg text-error flex items-center gap-2">
            <AlertTriangle size={20} /> {t('apply.failed.title')}
          </h3>
          <p className="py-4 text-sm text-base-content/80">
            {previewQuery.error instanceof Error
              ? previewQuery.error.message
              : String(previewQuery.error)}
          </p>
          <div className="modal-action">
            <button className="btn btn-neutral" onClick={onClose}>
              {t('apply.failed.close')}
            </button>
          </div>
        </div>
      </dialog>,
      document.body,
    );
  }

  const confirmApplyAction = async (force: boolean = false) => {
    if (!activeGameId) return;

    try {
      await applyMutation.mutateAsync({
        gameId: activeGameId,
        collectionId,
        ignoreMissing: force,
      });
      onClose();
    } catch (err) {
      const msg = String(err);
      if (msg.startsWith('MISSING_MODS:')) {
        const paths = msg.replace('MISSING_MODS:', '').split('|');
        setMissingPaths(paths);
      } else {
        console.error('Failed to apply collection:', err);
      }
    }
  };

  return createPortal(
    <dialog className="modal modal-open z-100">
      <div className="modal-box bg-base-200 border border-base-content/10 shadow-2xl max-w-6xl w-11/12 flex flex-col max-h-[85vh] p-0">
        {/* Header */}
        <div className="p-6 border-b border-base-content/5 shrink-0 bg-base-300 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-warning/20 text-warning flex items-center justify-center shrink-0">
              <AlertTriangle size={20} />
            </div>
            <div>
              <h3 className="font-bold text-lg flex items-center gap-2">
                {preview
                  ? t('apply.title', { name: preview.collection_name })
                  : t('apply.title_loading')}
              </h3>
              <p className="text-sm text-base-content/70">{t('apply.desc')}</p>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden bg-base-100 flex min-h-[50vh]">
          {missingPaths ? (
            <div className="flex flex-col h-full w-full items-center justify-center p-8 text-center bg-error/5 relative">
              <AlertTriangle size={48} className="text-error mb-4" />
              <h4 className="text-xl font-bold mb-2">{t('apply.missing.title')}</h4>
              <p className="text-base-content/70 mb-6 max-w-md">{t('apply.missing.desc')}</p>

              <div className="w-full max-w-2xl bg-base-300 rounded-box p-4 text-left max-h-[30vh] overflow-y-auto mb-8 custom-scrollbar border border-error/20">
                <ul className="list-disc pl-5 space-y-1 font-mono text-sm text-error/80">
                  {missingPaths.map((p, i) => (
                    <li key={i} className="break-all">
                      {p}
                    </li>
                  ))}
                </ul>
              </div>

              <div className="flex flex-col items-center gap-4">
                <p className="text-sm font-semibold">{t('apply.missing.anyway_q')}</p>
                <div className="flex gap-3">
                  <button
                    className="btn btn-ghost"
                    onClick={() => setMissingPaths(null)}
                    disabled={applyMutation.isPending}
                  >
                    {t('apply.missing.back')}
                  </button>
                  <button
                    className="btn btn-error"
                    onClick={() => confirmApplyAction(true)}
                    disabled={applyMutation.isPending}
                  >
                    {applyMutation.isPending ? (
                      <>
                        <Loader2 className="animate-spin" size={16} /> {t('apply.missing.applying')}
                      </>
                    ) : (
                      t('apply.missing.confirm')
                    )}
                  </button>
                </div>
              </div>
            </div>
          ) : previewQuery.isLoading ? (
            <div className="flex flex-col h-full items-center justify-center w-full text-base-content/50">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-primary" />
              <p>{t('apply.actions.loading')}</p>
            </div>
          ) : preview ? (
            <div className="flex w-full divide-x divide-base-content/5">
              {/* Left Panel - Before */}
              <div className="flex-1 flex flex-col max-h-full overflow-hidden bg-base-100/30">
                <div className="p-4 bg-base-300/30 border-b border-base-content/5 shrink-0 flex items-center justify-between">
                  <span className="font-semibold text-sm">{t('apply.panels.before')}</span>
                  <span className="badge badge-sm badge-ghost opacity-60 font-mono">
                    {t('apply.panels.mod_count', {
                      count: leavingMembers.filter((m) => m.kind === 'mod' || m.kind === 'nested').length,
                    })}
                  </span>
                </div>
                <div className="flex-1 overflow-y-auto custom-scrollbar p-4">
                  <CollectionTreeView
                    members={leavingMembers}
                    colorClass="text-error/70"
                    emptyMessage={t('apply.panels.empty_before')}
                  />
                </div>
              </div>

              {/* Middle Arrow */}
              <div className="flex items-center justify-center -mx-3 z-10 w-0">
                <div className="w-8 h-8 rounded-full bg-base-300 border border-base-content/10 flex items-center justify-center shadow-lg text-base-content/50">
                  <ArrowRight size={16} />
                </div>
              </div>

              {/* Right Panel - After */}
              <div className="flex-1 flex flex-col max-h-full overflow-hidden bg-base-200/20">
                <div className="p-4 bg-base-300/30 border-b border-base-content/5 shrink-0 flex items-center justify-between">
                  <div className="flex flex-col">
                    <span className="font-semibold text-sm text-primary">
                      {t('apply.panels.after')}
                    </span>
                  </div>
                  <span className="badge badge-sm badge-primary badge-outline opacity-80 font-mono">
                    {t('apply.panels.mod_count', {
                      count: targetMembers.filter((m) => m.kind === 'mod' || m.kind === 'nested').length,
                    })}
                  </span>
                </div>
                <div className="flex-1 overflow-y-auto custom-scrollbar p-4">
                  <CollectionTreeView
                    members={targetMembers}
                    colorClass="text-success/70"
                    emptyMessage={t('apply.panels.empty_after')}
                  />
                </div>
              </div>
            </div>
          ) : null}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-base-content/5 bg-base-300 shrink-0 flex justify-end gap-2">
          {missingPaths ? null : (
            <>
              <button
                className="btn btn-ghost"
                onClick={onClose}
                disabled={applyMutation.isPending}
              >
                {t('apply.actions.cancel')}
              </button>
              <button
                data-testid="modal-apply-btn"
                className="btn btn-primary min-w-30"
                onClick={() => confirmApplyAction(false)}
                disabled={applyMutation.isPending || previewQuery.isLoading}
              >
                {applyMutation.isPending ? (
                  <Loader2 size={16} className="animate-spin" />
                ) : (
                  t('apply.actions.confirm')
                )}
              </button>
            </>
          )}
        </div>
      </div>

      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose} disabled={applyMutation.isPending}>
          close
        </button>
      </form>
    </dialog>,
    document.body,
  );
}
