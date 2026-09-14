import {
  RefreshCw,
  MoreVertical,
  PanelRightClose,
  PanelRightOpen,
  Shield,
  ShieldAlert,
  ShieldCheck,
} from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { useToastStore } from '@/shared/ui/toast';
import { useActiveGame } from '@/entities/game';
import { applyDiskReconcileResult } from '@/features/file-watcher';
import { LiquidSurface } from '@/shared/ui/liquid';
import type { SafetyFilter } from '@/shared/ui/components/ui/SafetyFilterControl';

interface GlobalActionsProps {
  launchBar?: ReactNode;
  safetyFilter?: SafetyFilter;
  onSafetyFilterChange?: (value: SafetyFilter) => void;
}

const SAFETY_FILTERS = [
  { value: 'all', icon: Shield, className: 'text-base-content/55' },
  { value: 'safe', icon: ShieldCheck, className: 'text-success' },
  { value: 'unsafe', icon: ShieldAlert, className: 'text-warning' },
] as const;

export default function GlobalActions({
  launchBar,
  safetyFilter,
  onSafetyFilterChange,
}: GlobalActionsProps) {
  const { t } = useTranslation(['layout', 'common']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const workspaceView = useAppStore((state) => state.workspaceView);
  const isPreviewOpen = useAppStore((state) => state.isPreviewOpen);
  const togglePreview = useAppStore((state) => state.togglePreview);
  const reconcileInProgress = useAppStore((state) =>
    activeGame ? Boolean(state.diskReconcileByGame[activeGame.id]?.progress) : false,
  );
  const [isRefreshing, setIsRefreshing] = useState(false);
  const refreshInFlight = useRef(false);

  const runFullReconcile = async () => {
    if (!activeGame || refreshInFlight.current) {
      return;
    }

    refreshInFlight.current = true;
    setIsRefreshing(true);
    const toastStore = useToastStore.getState();
    const pendingToastId = toastStore.addToast(
      'info',
      t('actions.refresh_started', { name: activeGame.name }),
      0,
    );

    try {
      const result = await commands.reconcileDiskStateCmd(
        activeGame.id,
        'ManualRepair',
        null,
        true,
      );
      applyDiskReconcileResult(result, queryClient, activeGame);
      toastStore.removeToast(pendingToastId);

      switch (result.status) {
        case 'Applied':
          toastStore.addToast(
            'success',
            t(
              result.change_summary.has_user_visible_changes
                ? 'actions.refresh_complete'
                : 'actions.refresh_already_synced',
            ),
          );
          break;
        case 'AppliedWithFolderConflicts':
          toastStore.addToast(
            'warning',
            t('actions.refresh_conflicts', { count: result.folder_conflicts.length }),
          );
          break;
        case 'NeedsRenameConfirmation':
          toastStore.addToast(
            'warning',
            t('actions.refresh_rename_confirmation', {
              count: result.rename_confirmations.length,
            }),
          );
          break;
        case 'SourceUnavailable':
          toastStore.addToast(
            'error',
            t('actions.refresh_source_unavailable', {
              error: result.error_message ?? activeGame.mod_path,
            }),
          );
          break;
      }
    } catch (error) {
      toastStore.removeToast(pendingToastId);
      toastStore.addToast('error', t('actions.refresh_failed', { error: formatAppError(error) }));
    } finally {
      refreshInFlight.current = false;
      setIsRefreshing(false);
    }
  };

  const refreshDisabled = !activeGame || isRefreshing || reconcileInProgress;
  const showSync = workspaceView === 'mods';
  const showMobileMenu = workspaceView === 'mods';

  return (
    <div className="flex items-center gap-2 md:gap-3">
      {/* Desktop Tools */}
      <div className="hidden items-center gap-1 xl:flex">
        {showSync && (
          <button
            type="button"
            className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-primary hover:bg-base-content/10 disabled:text-base-content/20"
            title={t('actions.refresh')}
            aria-label={t('actions.refresh')}
            disabled={refreshDisabled}
            onClick={() => void runFullReconcile()}
          >
            <RefreshCw
              size={18}
              className={isRefreshing ? 'animate-spin motion-reduce:animate-none' : undefined}
            />
          </button>
        )}
        {/* Launch Bar (Epic 10) */}
        {launchBar}
      </div>

      {/* Mobile Menu Dropdown */}
      {showMobileMenu && (
        <div className="dropdown dropdown-end xl:hidden">
          <button
            type="button"
            className="btn btn-sm btn-ghost btn-square text-base-content/70"
            aria-label={t('actions.more', 'More actions')}
            title={t('actions.more', 'More actions')}
          >
            <MoreVertical size={18} aria-hidden="true" />
          </button>
          <LiquidSurface
            liquidRole="overlay"
            className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-64 rounded-xl shadow-lg"
          >
            <ul
              tabIndex={0}
              className="menu max-h-[calc(100vh-5rem)] w-full overflow-x-hidden overflow-y-auto p-2"
            >
              {workspaceView === 'mods' && safetyFilter && onSafetyFilterChange && (
                <li className="block w-full min-w-0 max-w-full">
                  <div className="!block w-full min-w-0 max-w-full px-2 pb-2">
                    <span className="mb-1.5 block px-0 py-0 text-[10px] font-medium uppercase tracking-widest text-base-content/45">
                      {t('common:safety_filter.label', 'Mod safety filter')}
                    </span>
                    <div
                      className="grid w-full min-w-0 grid-cols-3 gap-1 rounded-lg bg-base-content/5 p-1"
                      role="group"
                      aria-label={t('common:safety_filter.label', 'Mod safety filter')}
                    >
                      {SAFETY_FILTERS.map((filter) => {
                        const FilterIcon = filter.icon;
                        const isActive = safetyFilter === filter.value;

                        return (
                          <button
                            key={filter.value}
                            type="button"
                            className={`flex min-h-8 min-w-0 items-center justify-center gap-1 rounded-md px-2 text-xs font-medium transition-colors ${
                              isActive
                                ? 'bg-base-100 text-base-content shadow-sm'
                                : 'text-base-content/60 hover:bg-base-content/5 hover:text-base-content'
                            }`}
                            aria-pressed={isActive}
                            onClick={(event) => {
                              onSafetyFilterChange(filter.value);
                              event.currentTarget.blur();
                            }}
                          >
                            <FilterIcon size={13} className={filter.className} aria-hidden="true" />
                            {t(`common:safety_filter.${filter.value}`, filter.value)}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </li>
              )}

              {workspaceView === 'mods' && (
                <li
                  id="topbar-more-collection-portal"
                  className="block w-full min-w-0 max-w-full empty:hidden"
                />
              )}

              <li
                id="topbar-more-launch-portal"
                className="block w-full min-w-0 max-w-full empty:hidden"
              />

              <li className="my-1 h-px bg-base-content/10" aria-hidden="true" />
              {showSync && (
                <li>
                  <button
                    type="button"
                    className="gap-2 hover:bg-base-content/10"
                    disabled={refreshDisabled}
                    onClick={(event) => {
                      event.currentTarget.blur();
                      void runFullReconcile();
                    }}
                  >
                    <RefreshCw
                      size={16}
                      className={
                        isRefreshing ? 'animate-spin motion-reduce:animate-none' : undefined
                      }
                    />{' '}
                    {t('actions.refresh')}
                  </button>
                </li>
              )}
              {workspaceView === 'mods' && (
                <li>
                  <button
                    type="button"
                    className="gap-2 hover:bg-base-content/10"
                    onClick={(event) => {
                      event.currentTarget.blur();
                      togglePreview();
                    }}
                  >
                    {isPreviewOpen ? <PanelRightClose size={16} /> : <PanelRightOpen size={16} />}
                    {isPreviewOpen ? t('actions.hide_preview') : t('actions.show_preview')}
                  </button>
                </li>
              )}
            </ul>
          </LiquidSurface>
        </div>
      )}

      <div className="mx-1 hidden h-6 w-px bg-base-content/10 xl:block" />

      {/* Desktop Toggle Preview */}
      {workspaceView === 'mods' && (
        <button
          onClick={togglePreview}
          className={`btn btn-sm btn-square ml-1 hidden transition-[background-color,color] duration-150 xl:flex ${isPreviewOpen ? 'btn-ghost bg-primary/10 text-primary' : 'btn-ghost text-base-content/30 hover:text-primary'}`}
          title={isPreviewOpen ? t('actions.hide_preview') : t('actions.show_preview')}
        >
          {isPreviewOpen ? <PanelRightClose size={18} /> : <PanelRightOpen size={18} />}
        </button>
      )}
    </div>
  );
}
