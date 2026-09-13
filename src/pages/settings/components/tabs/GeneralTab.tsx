import { AlertTriangle, CheckCircle, Download, FileDown, RefreshCw, Upload } from 'lucide-react';
import { useAppStore } from '@/app/store';
import { useSettings } from '@/entities/settings';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import {
  THEME_OPTIONS,
  normalizeThemeSetting,
  type ThemeSetting,
} from '../../../../shared/lib/themeOptions';
import { useTranslation } from 'react-i18next';
import { useCustomThemes } from '../../hooks/useCustomThemes';
import { useAppUpdater } from '../../hooks/useAppUpdater';
import { getVersion } from '@tauri-apps/api/app';
import { useEffect, useState } from 'react';
import { TrustInformationDialog, type TrustDocument } from '../TrustInformationDialog';
import { SettingsRow, SettingsSection } from '../SettingsLayout';
import { formatBytes } from '@/shared/lib/utils/formatters';

const CUSTOM_THEME_TEMPLATE = {
  id: 'midnight-blue',
  label: 'Midnight Blue',
  config: {
    colors: {
      'base-100': '#101827',
      'base-200': '#172033',
      'base-300': '#0b1020',
      'base-content': '#e2e8f0',
      primary: '#60a5fa',
    },
    glass: {
      bg: 'rgba(16, 24, 39, 0.72)',
      border: 'rgba(226, 232, 240, 0.10)',
    },
    liquid: {
      nav: { material: 'regular', tint: '#dbeafe', tint_opacity: 0.08, quality: 'high' },
      control: { material: 'thin', tint: '#bfdbfe', tint_opacity: 0.06, quality: 'high' },
      indicator: { material: 'clear', tint: '#93c5fd', tint_opacity: 0.05, quality: 'high' },
      overlay: { material: 'thick', tint: '#dbeafe', tint_opacity: 0.1, quality: 'high' },
    },
    background: {
      kind: 'gradient',
      value: 'linear-gradient(135deg, #101827, #172554)',
      dim_opacity: 0.62,
    },
  },
} as const;

function downloadCustomThemeTemplate() {
  const blob = new Blob([`${JSON.stringify(CUSTOM_THEME_TEMPLATE, null, 2)}\n`], {
    type: 'application/json',
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = 'custom-theme-template.json';
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export default function GeneralTab() {
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const setAutoCloseLauncher = useAppStore((state) => state.setAutoCloseLauncher);
  const { settings, updateTheme, updateLanguage, setTelemetryEnabled } = useSettings();
  const { customThemes, refreshCustomThemes } = useCustomThemes();
  const { t } = useTranslation(['settings', 'common']);
  const [appVersion, setAppVersion] = useState('');
  const [activeTrustDocument, setActiveTrustDocument] = useState<TrustDocument | null>(null);
  const [isImportingTheme, setIsImportingTheme] = useState(false);
  const [themeImportStatus, setThemeImportStatus] = useState<string | null>(null);
  const {
    update,
    isChecking,
    isInstalling,
    progress,
    error: updateError,
    hasChecked,
    checkForUpdate,
    downloadAndInstall,
    dismiss,
  } = useAppUpdater();

  const selectedTheme = normalizeThemeSetting(settings?.theme);

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion(t('common:status.not_set')));
  }, [t]);

  const handleThemeChange = (value: string) => {
    updateTheme.mutate(normalizeThemeSetting(value as ThemeSetting));
  };

  const handleLanguageChange = (value: string) => {
    updateLanguage.mutate(value);
  };

  const handleImportTheme = async () => {
    setIsImportingTheme(true);
    setThemeImportStatus(null);
    try {
      const theme = await commands.importCustomTheme();
      if (!theme) return;
      await refreshCustomThemes();
      setThemeImportStatus(
        t('general.appearance.import_success', { defaultValue: `Imported ${theme.label}.` }),
      );
    } catch (cause) {
      setThemeImportStatus(
        t('general.appearance.import_error', {
          defaultValue: `Could not import theme: ${formatAppError(cause)}`,
        }),
      );
    } finally {
      setIsImportingTheme(false);
    }
  };

  const progressPercent =
    progress && progress.total ? Math.round((progress.downloaded / progress.total) * 100) : null;

  return (
    <div>
      <SettingsSection id="appearance-heading" title={t('general.appearance.title')}>
        <SettingsRow
          label={t('general.appearance.theme_select')}
          description={t(
            'general.appearance.download_template_hint',
            'JSON templates include liquid and background settings.',
          )}
          control={
            <select
              id="theme-select"
              aria-label={t('general.appearance.theme_select')}
              className="select select-bordered select-sm w-full theme-controller sm:w-72"
              value={selectedTheme}
              onChange={(event) => handleThemeChange(event.target.value)}
              disabled={updateTheme.isPending || !settings}
            >
              <optgroup label={t('general.appearance.groups.builtin')}>
                {THEME_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {t(option.labelKey)}
                  </option>
                ))}
              </optgroup>
              {customThemes.length > 0 && (
                <optgroup label={t('general.appearance.groups.custom')}>
                  {customThemes.map((theme) => (
                    <option key={theme.id} value={theme.id}>
                      {theme.label}
                    </option>
                  ))}
                </optgroup>
              )}
            </select>
          }
        />
        <div className="flex flex-wrap justify-end gap-2 border-t border-base-300/70 pt-3">
          <button
            type="button"
            className="btn btn-sm btn-ghost gap-2"
            onClick={downloadCustomThemeTemplate}
          >
            <FileDown size={15} />
            {t('general.appearance.download_template', 'Download template')}
          </button>
          <button
            type="button"
            className="btn btn-sm btn-outline gap-2"
            disabled={isImportingTheme}
            onClick={() => void handleImportTheme()}
          >
            <Upload size={15} />
            {isImportingTheme
              ? t('common:status.loading')
              : t('general.appearance.import_theme', 'Import theme')}
          </button>
        </div>
        {themeImportStatus && (
          <p className="mt-2 text-right text-xs text-base-content/70" role="status">
            {themeImportStatus}
          </p>
        )}
        <div className="mt-3 border-t border-base-300/70 pt-1">
          <SettingsRow
            label={t('general.language.label')}
            control={
              <select
                aria-label={t('general.language.label')}
                className="select select-bordered select-sm w-full sm:w-72"
                value={settings?.language || 'en'}
                onChange={(event) => handleLanguageChange(event.target.value)}
                disabled={updateLanguage.isPending || !settings}
              >
                <option value="en">{t('general.language.options.en')}</option>
                <option value="id">{t('general.language.options.id')}</option>
                <option value="zh">{t('general.language.options.zh')}</option>
              </select>
            }
          />
        </div>
      </SettingsSection>

      <SettingsSection id="behavior-heading" title={t('general.behavior.title')}>
        <SettingsRow
          label={t('general.behavior.auto_close')}
          description={t('general.behavior.auto_close_desc')}
          control={
            <input
              type="checkbox"
              aria-label={t('general.behavior.auto_close')}
              className="toggle toggle-primary toggle-sm"
              checked={autoCloseLauncher}
              onChange={(event) => setAutoCloseLauncher(event.target.checked)}
            />
          }
        />
      </SettingsSection>

      <SettingsSection id="system-heading" title={t('general.system.title')}>
        <SettingsRow
          label={t('general.system.app_version')}
          control={
            <span className="font-mono text-xs">{appVersion || t('common:status.not_set')}</span>
          }
        />
        <div className="border-t border-base-300/70">
          <SettingsRow
            label={t('general.diagnostics.title')}
            description={t('general.diagnostics.description')}
            control={
              <label className="label cursor-pointer gap-2 py-0" htmlFor="anonymous-diagnostics">
                <span className="label-text text-xs">{t('general.diagnostics.toggle')}</span>
                <input
                  id="anonymous-diagnostics"
                  type="checkbox"
                  className="toggle toggle-sm toggle-primary"
                  checked={settings?.diagnostics?.telemetry_enabled ?? false}
                  disabled={setTelemetryEnabled.isPending || !settings}
                  onChange={(event) => setTelemetryEnabled.mutate(event.target.checked)}
                />
              </label>
            }
          />
        </div>
        <div className="border-t border-base-300/70 py-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className="text-sm font-medium">{t('update.title')}</p>
              <p className="mt-0.5 text-xs text-base-content/60">
                {t('update.current_version', {
                  version: appVersion || t('common:status.not_set'),
                })}
              </p>
            </div>
            <button
              type="button"
              className="btn btn-outline btn-sm gap-2"
              onClick={() => void checkForUpdate()}
              disabled={isChecking || isInstalling}
            >
              <RefreshCw size={15} className={isChecking ? 'animate-spin' : ''} />
              {isChecking ? t('update.checking') : t('update.check_btn')}
            </button>
          </div>

          {update && (
            <div className="mt-3 rounded-md border border-info/25 bg-info/10 p-3 text-sm">
              <div className="flex gap-2">
                <Download size={17} className="mt-0.5 shrink-0 text-info" />
                <div className="min-w-0 flex-1">
                  <p className="font-medium">
                    {t('update.available', { version: update.version })}
                  </p>
                  {update.body && (
                    <p className="mt-1 line-clamp-2 whitespace-pre-wrap text-xs text-base-content/65">
                      {update.body}
                    </p>
                  )}
                </div>
              </div>
              {!isInstalling && (
                <div className="mt-3 flex justify-end">
                  <button
                    type="button"
                    className="btn btn-primary btn-sm gap-2"
                    onClick={() => void downloadAndInstall()}
                  >
                    <Download size={15} />
                    {t('update.install_btn')}
                  </button>
                </div>
              )}
            </div>
          )}

          {progress && (
            <div className="mt-3" role="status">
              <div className="mb-1 flex justify-between gap-3 text-xs text-base-content/70">
                <span>
                  {t('update.downloading', {
                    downloaded: formatBytes(progress.downloaded),
                    total: progress.total ? formatBytes(progress.total) : '?',
                  })}
                </span>
                {progressPercent !== null && <span>{progressPercent}%</span>}
              </div>
              <progress
                className="progress progress-primary w-full"
                value={progressPercent ?? undefined}
                max={100}
              />
            </div>
          )}

          {updateError && (
            <div
              className="mt-3 flex items-center gap-2 rounded-md border border-error/25 bg-error/10 p-3 text-sm text-error"
              role="alert"
            >
              <AlertTriangle size={17} className="shrink-0" />
              <span className="min-w-0 flex-1">{updateError}</span>
              <button type="button" className="btn btn-ghost btn-xs" onClick={dismiss}>
                {t('common:action.dismiss')}
              </button>
            </div>
          )}

          {hasChecked && !update && !isChecking && !updateError && !progress && (
            <p className="mt-3 flex items-center gap-2 text-xs text-success" role="status">
              <CheckCircle size={16} />
              {t('update.latest')}
            </p>
          )}
        </div>
        <div className="divide-y divide-base-300/70 border-t border-base-300/70">
          {(['privacy', 'terms'] as const).map((document) => (
            <button
              key={document}
              type="button"
              className="flex w-full items-center justify-between gap-4 py-3 text-left text-sm transition-colors duration-150 hover:text-primary focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              onClick={() => setActiveTrustDocument(document)}
            >
              <span>
                <span className="block font-medium">
                  {t(`general.trust.${document}.card_title`)}
                </span>
                <span className="mt-0.5 block text-xs text-base-content/60">
                  {t(`general.trust.${document}.card_desc`)}
                </span>
              </span>
              <span aria-hidden="true">›</span>
            </button>
          ))}
        </div>
      </SettingsSection>
      <TrustInformationDialog
        document={activeTrustDocument}
        onClose={() => setActiveTrustDocument(null)}
      />
    </div>
  );
}
