import { FolderInput, FolderOpen, Inbox, RefreshCw, Settings } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModInboxSnapshot } from './types';

export function NoGameState() {
  const { t } = useTranslation('mod_inbox');

  return (
    <div className="grid h-full place-items-center bg-base-100 p-8">
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
}: {
  snapshot: ModInboxSnapshot | null;
  loading: boolean;
  onSettings: () => void;
  onOpen: () => void;
  onRefresh: () => void;
}) {
  const { t } = useTranslation('mod_inbox');

  return (
    <header className="border-b border-base-300 bg-base-200/60 px-5 py-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <div className="rounded-xl bg-primary/15 p-2 text-primary">
              <Inbox size={22} />
            </div>
            <div>
              <h1 className="text-xl font-bold">{t('title')}</h1>
              <p className="text-xs text-base-content/55">{t('subtitle')}</p>
            </div>
          </div>
          {snapshot?.rootPath && (
            <p className="mt-2 max-w-2xl truncate font-mono text-[11px] text-base-content/45">
              {snapshot.rootPath}
            </p>
          )}
        </div>

        <div className="flex flex-wrap gap-2">
          <button type="button" className="btn btn-ghost btn-sm gap-2" onClick={onSettings}>
            <FolderInput size={16} /> {t('actions.choose_location')}
          </button>
          <button
            type="button"
            className="btn btn-ghost btn-sm gap-2"
            disabled={!snapshot || snapshot.rootState !== 'ready'}
            onClick={onOpen}
          >
            <FolderOpen size={16} /> {t('actions.open_inbox')}
          </button>
          <button
            type="button"
            className="btn btn-outline btn-sm gap-2"
            disabled={loading}
            onClick={onRefresh}
          >
            <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
            {t('actions.refresh')}
          </button>
        </div>
      </div>
    </header>
  );
}

export function ModInboxTabs({
  activeTab,
  readyCount,
  processedCount,
  onChange,
}: {
  activeTab: 'ready' | 'processed';
  readyCount: number;
  processedCount: number;
  onChange: (tab: 'ready' | 'processed') => void;
}) {
  const { t } = useTranslation('mod_inbox');

  return (
    <div className="tabs tabs-border border-b border-base-300 px-5" role="tablist">
      <button
        type="button"
        role="tab"
        aria-label={t('tabs.ready')}
        aria-selected={activeTab === 'ready'}
        className={`tab gap-2 ${activeTab === 'ready' ? 'tab-active' : ''}`}
        onClick={() => onChange('ready')}
      >
        {t('tabs.ready')} <span className="badge badge-sm">{readyCount}</span>
      </button>
      <button
        type="button"
        role="tab"
        aria-label={t('tabs.processed')}
        aria-selected={activeTab === 'processed'}
        className={`tab gap-2 ${activeTab === 'processed' ? 'tab-active' : ''}`}
        onClick={() => onChange('processed')}
      >
        {t('tabs.processed')} <span className="badge badge-sm">{processedCount}</span>
      </button>
    </div>
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
      <section className="max-w-lg rounded-3xl border border-dashed border-base-300 bg-base-200/40 p-10 text-center">
        <FolderInput className="mx-auto h-14 w-14 text-primary/60" />
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
