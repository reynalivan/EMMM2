import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { ShieldCheck, ShieldAlert, Loader2, ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '../../stores/useAppStore';
import { CollectionTreeView } from '../collections/components/CollectionTreeView';
import { useCorridor } from '../collections/hooks/useCorridor';
import { commands } from '../../lib/bindings';
import { getCorridorStateName } from '../../lib/corridorLabels';
import type { CollectionMember, CorridorSwitchPreview } from '../../types/collection';



function buildLeavingSubtitle(preview: CorridorSwitchPreview | undefined): string {
  return getCorridorStateName(preview?.leaving_state_name);
}

function buildTargetSubtitle(preview: CorridorSwitchPreview | undefined, t: TFunction): string {
  if (!preview || preview.target_state_kind === 'none') {
    return t('safe_mode:switch.empty', { name: '' }).split('(')[1].replace(')', ''); // Hacky way to get "All Disabled" or just use common
  }

  return getCorridorStateName(preview.target_state_name);
}

function buildTargetDescription(preview: CorridorSwitchPreview | undefined, t: TFunction): string {
  if (!preview || preview.target_state_kind === 'none') {
    return t('safe_mode:switch.target_desc.none');
  }

  return t('safe_mode:switch.target_desc.active');
}

function buildTargetEmptyState(preview: CorridorSwitchPreview | undefined, t: TFunction): string {
  if (!preview || preview.target_state_kind === 'none') {
    return t('safe_mode:switch.missing_target');
  }

  return t('safe_mode:switch.empty', { name: getCorridorStateName(preview.target_state_name) });
}

interface ModeSwitchConfirmModalProps {
  open: boolean;
  targetSafeMode: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export default function ModeSwitchConfirmModal({
  open,
  targetSafeMode,
  onClose,
  onConfirm,
}: ModeSwitchConfirmModalProps) {
  const { t } = useTranslation(['safe_mode', 'common']);
  const { activeGameId, safeMode } = useAppStore();
  const currentRuntimeQuery = useCorridor(activeGameId, safeMode);
  const currentStateToken = useMemo(() => {
    if (!currentRuntimeQuery.data) {
      return 'unknown';
    }

    return [
      currentRuntimeQuery.data.active_collection_id ?? '',
      currentRuntimeQuery.data.active_collection_name ?? '',
      currentRuntimeQuery.data.current_signature,
    ].join(':');
  }, [currentRuntimeQuery.data]);

  const { data: preview, isLoading } = useQuery<CorridorSwitchPreview>({
    queryKey: [
      'v2-corridor-preview',
      activeGameId ?? '',
      safeMode,
      targetSafeMode,
      currentStateToken,
    ],
    queryFn: () =>
      commands.previewCorridorSwitch({
        gameId: activeGameId ?? '',
        targetSafe: targetSafeMode,
      }),
    enabled: open && !!activeGameId && currentRuntimeQuery.status === 'success',
    staleTime: 0,
  });
  const isPreviewLoading = currentRuntimeQuery.status !== 'success' || isLoading;

  const leavingMembers = useMemo((): CollectionMember[] => {
    if (!preview) return [];
    return preview.leaving_members.filter(
      (m) => m.kind === 'object' || m.kind === 'root' || ((m.kind === 'mod' || m.kind === 'nested') && m.is_enabled),
    );
  }, [preview]);

  const targetMembers = useMemo((): CollectionMember[] => {
    if (!preview) return [];
    return preview.target_members.filter(
      (m) => m.kind === 'object' || m.kind === 'root' || ((m.kind === 'mod' || m.kind === 'nested') && m.is_enabled),
    );
  }, [preview]);

  if (!open) return null;

  return createPortal(
    <dialog className="modal modal-open z-1000">
      <div className="modal-box bg-base-200 border border-base-content/10 shadow-2xl max-w-5xl flex flex-col max-h-[85vh] p-0 overflow-hidden">
        {/* Header */}
        <div className="p-6 pb-4 border-b border-base-content/5 shrink-0 bg-base-300/30">
          <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-1">
            {targetSafeMode ? <ShieldCheck size={20} /> : <ShieldAlert size={20} />}
            {targetSafeMode ? t('safe_mode:switch.title_safe') : t('safe_mode:switch.title_unsafe')}
          </h3>
          <p className="text-sm text-base-content/70">{t('safe_mode:switch.desc')}</p>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden flex flex-col bg-base-100/50 min-h-0">
          {isPreviewLoading ? (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-base-content/50 min-h-75">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-warning" />
              <p>{t('safe_mode:switch.loading')}</p>
            </div>
          ) : (
            <div className="flex-1 flex flex-col sm:flex-row min-h-0">
              {/* Left Column: Leaving */}
              <div className="flex-1 border-r border-base-content/5 flex flex-col min-h-0 sm:max-w-[50%]">
                <div
                  className={`p-4 border-b border-base-content/5 ${targetSafeMode ? 'bg-error/5' : 'bg-success/5'} shrink-0`}
                >
                  <h4
                    className={`text-[11px] uppercase tracking-[0.2em] flex justify-between items-center ${targetSafeMode ? 'text-error/70' : 'text-success/70'} mb-2`}
                  >
                    {t('safe_mode:switch.leaving', {
                      mode: targetSafeMode
                        ? t('safe_mode:labels.unsafe')
                        : t('safe_mode:labels.safe'),
                    })}
                    <span
                      className={`badge badge-sm ${targetSafeMode ? 'badge-error' : 'badge-success'} badge-outline`}
                    >
                      {t('safe_mode:switch.snapshot')}
                    </span>
                  </h4>
                  <p
                    className={`text-lg font-semibold break-all leading-tight ${targetSafeMode ? 'text-error/90' : 'text-success/90'}`}
                  >
                    {buildLeavingSubtitle(preview)}
                  </p>
                  <p className="text-xs text-base-content/45 mt-1">
                    {t('safe_mode:switch.active_mods')}
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  <CollectionTreeView
                    members={leavingMembers}
                    colorClass={targetSafeMode ? 'text-error' : 'text-success'}
                  />
                </div>
              </div>

              {/* Center Arrow */}
              <div className="hidden sm:flex items-center justify-center w-8 -mx-4 z-10 text-base-content/30 opacity-50 relative pointer-events-none">
                <div className="bg-base-200 rounded-full p-1 border border-base-content/10">
                  <ArrowRight size={20} />
                </div>
              </div>

              {/* Right Column: Target */}
              <div className="flex-1 flex flex-col min-h-0 sm:max-w-[50%]">
                <div
                  className={`p-4 border-b border-base-content/5 ${targetSafeMode ? 'bg-success/5' : 'bg-error/5'} shrink-0`}
                >
                  <h4
                    className={`text-[11px] uppercase tracking-[0.2em] flex justify-between items-center ${targetSafeMode ? 'text-success/70' : 'text-error/70'} mb-2`}
                  >
                    {t('safe_mode:switch.target', {
                      mode: targetSafeMode
                        ? t('safe_mode:labels.safe')
                        : t('safe_mode:labels.unsafe'),
                    })}
                    <span
                      className={`badge badge-sm ${targetSafeMode ? 'badge-success' : 'badge-error'} badge-outline`}
                    >
                      {t('safe_mode:switch.restore')}
                    </span>
                  </h4>
                  <p
                    className={`text-lg font-semibold break-all leading-tight ${targetSafeMode ? 'text-success/90' : 'text-error/90'}`}
                  >
                    {buildTargetSubtitle(preview, t)}
                  </p>
                  <p className="text-xs text-base-content/45 mt-1">
                    {buildTargetDescription(preview, t)}
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  <CollectionTreeView
                    members={targetMembers}
                    colorClass={targetSafeMode ? 'text-success' : 'text-error'}
                    emptyMessage={buildTargetEmptyState(preview, t)}
                  />
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-base-content/5 shrink-0 bg-base-300/30">
          <div className="modal-action mt-0 gap-2">
            <button
              onClick={onClose}
              className="btn btn-ghost hover:bg-base-content/5"
              disabled={isPreviewLoading}
            >
              {t('common:actions.cancel')}
            </button>
            <button
              onClick={onConfirm}
              className="btn btn-warning shadow-lg shadow-warning/10 font-bold tracking-wide"
              disabled={isPreviewLoading}
            >
              {t('common:actions.confirm')} {'->'}
            </button>
          </div>
        </div>
      </div>
    </dialog>,
    document.body,
  );
}
