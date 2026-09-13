import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { ExternalLink, FileCog, TriangleAlert, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { useToastStore } from '@/shared/ui/toast';
import { SettingsSection } from '../SettingsLayout';

const MOD_VIEWER_RELEASE_URL = 'https://github.com/drelymk/mod_viewer/releases/latest';

export default function IntegrationsTab() {
  const { t } = useTranslation('settings');
  const { addToast } = useToastStore();
  const { settings, setModViewerExecutable } = useSettings();
  const [pendingExecutable, setPendingExecutable] = useState<string | null>(null);
  const executable = settings?.external_tools?.mod_viewer_executable ?? null;
  const executablePresence = useQuery({
    queryKey: ['mod-viewer-executable-exists', executable],
    queryFn: () => commands.checkPathExistsCmd(executable ?? ''),
    enabled: executable !== null,
    retry: false,
  });

  const handleViewLatestRelease = async () => {
    try {
      await commands.browserOpenExternally(MOD_VIEWER_RELEASE_URL);
    } catch (error) {
      addToast(
        'error',
        t('integrations.mod_viewer.release_error', { error: formatAppError(error) }),
      );
    }
  };

  const handleSelectExecutable = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Executable', extensions: ['exe'] }],
      });

      if (typeof selected === 'string') {
        setPendingExecutable(selected);
      }
    } catch (error) {
      addToast(
        'error',
        t('integrations.mod_viewer.picker_error', { error: formatAppError(error) }),
      );
    }
  };

  const handleConfirmExecutable = async () => {
    if (!pendingExecutable) {
      return;
    }

    try {
      await setModViewerExecutable.mutateAsync(pendingExecutable);
      setPendingExecutable(null);
    } catch {
      // The mutation already presents an actionable error toast.
    }
  };

  const isMissing = executablePresence.data === false;
  const status = executable
    ? isMissing
      ? t('integrations.mod_viewer.status.missing')
      : executablePresence.isLoading
        ? t('integrations.mod_viewer.status.checking')
        : t('integrations.mod_viewer.status.configured')
    : t('integrations.mod_viewer.status.not_configured');

  return (
    <div>
      <SettingsSection id="integrations-heading" title={t('integrations.title')}>
        <article>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h3 className="flex items-center gap-2 font-semibold">
                <FileCog size={18} />
                {t('integrations.mod_viewer.title')}
              </h3>
              <p className="mt-1 max-w-2xl text-xs leading-5 text-base-content/60">
                {t('integrations.mod_viewer.description')}
              </p>
              <p className="mt-1 text-xs text-base-content/60">
                {t('integrations.mod_viewer.recommended_version')}
              </p>
            </div>
            <span
              className={`badge shrink-0 ${isMissing ? 'badge-warning' : executable ? 'badge-success' : 'badge-ghost'}`}
              data-testid="mod-viewer-status"
            >
              {status}
            </span>
          </div>

          {executable && (
            <div className="mt-3 border-l-2 border-base-300 pl-3">
              <p className="text-xs font-medium text-base-content/60">
                {t('integrations.mod_viewer.path')}
              </p>
              <p
                className="mt-1 break-all font-mono text-xs"
                data-testid="mod-viewer-executable-path"
              >
                {executable}
              </p>
            </div>
          )}

          {isMissing && (
            <div className="alert alert-warning mt-4 text-sm" role="status">
              <TriangleAlert size={18} />
              <span>{t('integrations.mod_viewer.status.missing')}</span>
            </div>
          )}

          <div className="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              className="btn btn-outline btn-sm gap-2"
              onClick={() => void handleViewLatestRelease()}
            >
              <ExternalLink size={15} />
              {t('integrations.mod_viewer.view_latest_release')}
            </button>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={() => void handleSelectExecutable()}
              disabled={setModViewerExecutable.isPending}
            >
              {executable
                ? t('integrations.mod_viewer.replace')
                : t('integrations.mod_viewer.select_executable')}
            </button>
            {executable && (
              <button
                type="button"
                className="btn btn-ghost btn-sm gap-2 text-error hover:bg-error/10 hover:text-error"
                onClick={() => setModViewerExecutable.mutate(null)}
                disabled={setModViewerExecutable.isPending}
              >
                <Trash2 size={15} />
                {t('integrations.mod_viewer.remove')}
              </button>
            )}
          </div>
        </article>
      </SettingsSection>

      {pendingExecutable && (
        <dialog
          open
          className="modal modal-open bg-overlay-mask backdrop-blur-sm"
          aria-modal="true"
          aria-labelledby="mod-viewer-disclosure-title"
          onCancel={() => setPendingExecutable(null)}
        >
          <section className="modal-box max-w-xl">
            <h3 id="mod-viewer-disclosure-title" className="text-base font-semibold">
              {t('integrations.mod_viewer.disclosure.title')}
            </h3>
            <p className="mt-4 text-sm leading-6 text-base-content/75">
              {t('integrations.mod_viewer.disclosure.body')}
            </p>
            <p className="mt-3 text-sm font-medium leading-6 text-warning">
              {t('integrations.mod_viewer.disclosure.official_source')}
            </p>
            <p className="mt-4 break-all rounded-lg bg-base-200 px-3 py-2 font-mono text-xs">
              {pendingExecutable}
            </p>
            <div className="modal-action">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setPendingExecutable(null)}
                disabled={setModViewerExecutable.isPending}
              >
                {t('integrations.mod_viewer.disclosure.cancel')}
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void handleConfirmExecutable()}
                disabled={setModViewerExecutable.isPending}
              >
                {t('integrations.mod_viewer.disclosure.continue')}
              </button>
            </div>
          </section>
          <form method="dialog" className="modal-backdrop" aria-hidden="true">
            <button onClick={() => setPendingExecutable(null)}>
              {t('integrations.mod_viewer.disclosure.cancel')}
            </button>
          </form>
        </dialog>
      )}
    </div>
  );
}
