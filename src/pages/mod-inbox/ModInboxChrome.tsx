import { useEffect, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { FolderInput, FolderOpen, Inbox, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModInboxSnapshot } from './types';
import { LiquidSurface } from '@/shared/ui/liquid';

export function NoGameState() {
  const { t } = useTranslation('mod_inbox');

  return (
    <div className="grid h-full place-items-center bg-base-100 p-8 pt-[calc(var(--workspace-topbar-height)+2rem)]">
      <div className="max-w-md text-center">
        <Inbox className="mx-auto mb-4 h-12 w-12 text-base-content/30" />
        <h1 className="text-xl font-bold">{t('no_game.title')}</h1>
        <p className="mt-2 text-sm text-base-content/60">{t('no_game.description')}</p>
      </div>
    </div>
  );
}

export function ModInboxHeader({
  snapshot,
  loading,
  onSettings,
  onOpen,
  onRefresh,
  topBarAction,
}: {
  snapshot: ModInboxSnapshot | null;
  loading: boolean;
  onSettings: () => void;
  onOpen: () => void;
  onRefresh: () => void;
  topBarAction?: ReactNode;
}) {
  const { t } = useTranslation('mod_inbox');
  const [portalTarget, setPortalTarget] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setPortalTarget(document.getElementById('topbar-actions-portal'));
  }, []);

  if (!portalTarget) return null;

  return createPortal(
    <>
      {snapshot?.rootPath && (
        <span
          className="text-[10px] text-base-content/40 font-mono max-w-[150px] 2xl:max-w-[300px] truncate mr-2 hidden xl:block"
          title={snapshot.rootPath}
        >
          {snapshot.rootPath}
        </span>
      )}
      <button
        type="button"
        className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-primary hover:bg-base-content/5 tooltip tooltip-bottom"
        data-tip={t('actions.choose_location')}
        aria-label={t('actions.choose_location')}
        onClick={onSettings}
      >
        <FolderInput size={18} />
      </button>
      <button
        type="button"
        className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-primary hover:bg-base-content/5 tooltip tooltip-bottom"
        data-tip={t('actions.open_inbox')}
        aria-label={t('actions.open_inbox')}
        disabled={!snapshot || snapshot.rootState !== 'ready'}
        onClick={onOpen}
      >
        <FolderOpen size={18} />
      </button>
      <button
        type="button"
        className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-primary hover:bg-base-content/5 tooltip tooltip-bottom"
        data-tip={t('actions.refresh')}
        aria-label={t('actions.refresh')}
        disabled={loading}
        onClick={onRefresh}
      >
        <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
      </button>
      {topBarAction}
    </>,
    portalTarget,
  );
}

export function ModInboxTabs({
  activeTab,
  readyCount,
  processedCount,
  allSelected,
  selectionDisabled,
  onToggleSelectAll,
  onChange,
}: {
  activeTab: 'ready' | 'processed';
  readyCount: number;
  processedCount: number;
  allSelected: boolean;
  selectionDisabled: boolean;
  onToggleSelectAll: () => void;
  onChange: (tab: 'ready' | 'processed') => void;
}) {
  const { t } = useTranslation('mod_inbox');

  return (
    <LiquidSurface liquidRole="nav" className="block w-full" contentClassName="h-auto">
      <div className="flex items-center justify-between gap-3 px-5 pb-3 pt-[calc(var(--workspace-topbar-height)+0.75rem)] lg:pt-[calc(var(--workspace-topbar-height)+0.75rem)]">
        <div className="min-w-0" role="tablist" aria-label={t('tabs.label', 'Inbox status')}>
          <LiquidSurface
            liquidRole="control"
            className="rounded-[var(--radius-box)]"
            contentClassName="flex gap-1 p-1"
          >
            <InboxTab
              active={activeTab === 'ready'}
              label={t('tabs.ready')}
              count={readyCount}
              onClick={() => onChange('ready')}
            />
            <InboxTab
              active={activeTab === 'processed'}
              label={t('tabs.processed')}
              count={processedCount}
              onClick={() => onChange('processed')}
            />
          </LiquidSurface>
        </div>
        <label className="flex shrink-0 cursor-pointer items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox checkbox-sm"
            aria-label={t(activeTab === 'ready' ? 'ready.select_all' : 'processed.select_all')}
            checked={allSelected}
            disabled={selectionDisabled}
            onChange={onToggleSelectAll}
          />
          {t('actions.select_all')}
        </label>
      </div>
    </LiquidSurface>
  );
}

function InboxTab({
  active,
  label,
  count,
  onClick,
}: {
  active: boolean;
  label: string;
  count: number;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-label={label}
      aria-selected={active}
      className={`flex min-h-8 items-center justify-center gap-2 rounded-[calc(var(--radius-box)-0.25rem)] px-3 text-sm font-medium transition-[background-color,color] duration-150 ${
        active
          ? 'bg-base-content/[0.08] text-base-content'
          : 'text-base-content/55 hover:bg-base-content/[0.05] hover:text-base-content'
      }`}
      onClick={onClick}
    >
      {label} <span className="text-xs tabular-nums text-base-content/45">{count}</span>
    </button>
  );
}

export function MissingInboxState({
  rootPath,
  creating,
  onCreate,
  onChooseLocation,
}: {
  rootPath: string;
  creating: boolean;
  onCreate: () => void;
  onChooseLocation: () => void;
}) {
  const { t } = useTranslation('mod_inbox');

  return (
    <main className="grid flex-1 place-items-center overflow-auto p-6">
      <section className="max-w-lg rounded-xl border border-dashed border-base-300 bg-base-200/40 p-8 text-center">
        <FolderInput className="mx-auto h-12 w-12 text-base-content/40" />
        <h2 className="mt-5 text-xl font-bold">{t('missing.title')}</h2>
        <p className="mt-2 text-sm text-base-content/60">{t('missing.description')}</p>
        <code className="mt-4 block break-all rounded-xl bg-base-300/60 p-3 text-xs">
          {rootPath}
        </code>
        <div className="mt-6 flex flex-wrap justify-center gap-2">
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={creating}
            onClick={onCreate}
          >
            {creating && <span className="loading loading-spinner" />}
            {t('actions.create_folder')}
          </button>
          <button type="button" className="btn btn-ghost btn-sm" onClick={onChooseLocation}>
            {t('actions.choose_location')}
          </button>
        </div>
      </section>
    </main>
  );
}

export function DeleteProcessedDialog({
  count,
  deleting,
  onCancel,
  onConfirm,
}: {
  count: number;
  deleting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation('mod_inbox');

  return (
    <dialog open className="modal modal-open" aria-labelledby="mod-inbox-delete-title">
      <div className="modal-box">
        <h2 id="mod-inbox-delete-title" className="text-lg font-bold">
          {t('delete.title', { count })}
        </h2>
        <p className="mt-3 text-sm text-base-content/70">{t('delete.description')}</p>
        <div className="modal-action">
          <button type="button" className="btn btn-ghost" disabled={deleting} onClick={onCancel}>
            {t('actions.cancel')}
          </button>
          <button type="button" className="btn btn-error" disabled={deleting} onClick={onConfirm}>
            {deleting && <span className="loading loading-spinner" />}
            {t('delete.confirm')}
          </button>
        </div>
      </div>
    </dialog>
  );
}
