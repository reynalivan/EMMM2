import { Monitor, Languages, Database, LogOut, Plus, Trash2, Download } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { useSettings } from '../../../hooks/useSettings';
import {
  THEME_OPTIONS,
  BUILTIN_THEMES,
  normalizeThemeSetting,
  type ThemeSetting,
} from '../theme/themeOptions';
import { useTranslation } from 'react-i18next';
import { useCustomThemes } from '../theme/useCustomThemes';
import { commands, type CustomTheme } from '../../../lib/bindings';
import { open, save } from '@tauri-apps/plugin-dialog';
// Tauri v2 plugin-fs exports. If text-specific ones are missing in the IDE, we handle conversion.
import { readFile, writeFile } from '@tauri-apps/plugin-fs';
import { useToastStore } from '../../../stores/useToastStore';

export default function GeneralTab() {
  const { autoCloseLauncher, setAutoCloseLauncher } = useAppStore();
  const { settings, updateTheme, updateLanguage } = useSettings();
  const { customThemes, refreshCustomThemes } = useCustomThemes();
  const { addToast } = useToastStore();
  const { t } = useTranslation('settings');

  const selectedTheme = normalizeThemeSetting(settings?.theme);

  const handleThemeChange = (value: string) => {
    updateTheme.mutate(normalizeThemeSetting(value as ThemeSetting));
  };

  const handleLanguageChange = (value: string) => {
    updateLanguage.mutate(value);
  };

  const handleImportTheme = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'JSON Theme',
            extensions: ['json'],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        const data = await readFile(selected);
        const content = new TextDecoder().decode(data);
        const themeData = JSON.parse(content) as CustomTheme;

        if (!themeData.id || !themeData.label || !themeData.config) {
          throw new Error('Invalid theme format: Missing required fields (id, label, config).');
        }

        await commands.saveCustomTheme({ theme: themeData });
        await refreshCustomThemes();
        addToast('success', t('general.appearance.import_success', { name: themeData.label }));
      }
    } catch (err) {
      console.error('Failed to import theme:', err);
      addToast('error', t('general.appearance.import_failed', { error: String(err) }));
    }
  };

  const handleExportTheme = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const themeData = await commands.loadCustomTheme({ id });
      const fileName = `${themeData.id}.json`;

      const savePath = await save({
        defaultPath: fileName,
        filters: [
          {
            name: 'JSON Theme',
            extensions: ['json'],
          },
        ],
      });

      if (savePath) {
        const content = JSON.stringify(themeData, null, 2);
        const data = new TextEncoder().encode(content);
        await writeFile(savePath, data);
        addToast('success', t('general.appearance.export_success', { name: fileName }));
      }
    } catch (err) {
      console.error('Failed to export theme:', err);
      addToast('error', t('general.appearance.export_failed', { error: String(err) }));
    }
  };

  const handleDeleteTheme = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm(t('general.appearance.delete_confirm'))) {
      try {
        await commands.deleteCustomTheme({ id });
        await refreshCustomThemes();
        if (selectedTheme === id) {
          handleThemeChange('onyx');
        }
        addToast('success', t('general.appearance.delete_success'));
      } catch (err) {
        addToast('error', t('general.appearance.delete_failed', { error: String(err) }));
      }
    }
  };

  const isCustom = !(BUILTIN_THEMES as readonly string[]).includes(selectedTheme);

  return (
    <div className="space-y-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body p-5">
          <div className="flex items-center justify-between">
            <h3 className="card-title text-lg flex items-center gap-2">
              <Monitor size={20} className="text-primary" />
              {t('general.appearance.title')}
            </h3>
            <button className="btn btn-ghost btn-sm gap-2 text-primary" onClick={handleImportTheme}>
              <Plus size={16} />
              {t('general.appearance.import')}
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mt-4">
            <div className="form-control w-full">
              <label className="label" htmlFor="theme-select">
                <span className="label-text">{t('general.appearance.theme_select')}</span>
              </label>
              <div className="flex flex-col gap-2">
                <select
                  id="theme-select"
                  className="select select-bordered w-full theme-controller"
                  value={selectedTheme}
                  onChange={(e) => handleThemeChange(e.target.value)}
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
                      {customThemes.map((ct) => (
                        <option key={ct.id} value={ct.id}>
                          {ct.label}
                        </option>
                      ))}
                    </optgroup>
                  )}
                </select>

                {isCustom && (
                  <div className="flex items-center gap-4 mt-1">
                    <span className="text-xs badge badge-ghost">
                      {t('general.appearance.custom_active')}
                    </span>
                    <div className="flex items-center gap-2">
                      <button
                        className="btn btn-ghost btn-xs text-info p-0 h-auto min-h-0"
                        onClick={(e) => handleExportTheme(selectedTheme, e)}
                      >
                        <Download size={12} className="mr-1" /> {t('general.appearance.export')}
                      </button>
                      <button
                        className="btn btn-ghost btn-xs text-error p-0 h-auto min-h-0"
                        onClick={(e) => handleDeleteTheme(selectedTheme, e)}
                      >
                        <Trash2 size={12} className="mr-1" /> {t('general.appearance.remove')}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="text-xs text-base-content/60 bg-base-300/30 p-3 rounded-lg flex flex-col justify-center">
              <p className="font-medium text-base-content/80 mb-1">
                {t('general.appearance.info.title')}
              </p>
              <p>{t('general.appearance.info.system')}</p>
              <p className="mt-1">{t('general.appearance.info.builtin')}</p>
              <p className="mt-1">{t('general.appearance.info.custom')}</p>
            </div>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body p-5">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Languages size={20} className="text-primary" />
            {t('general.language.title')}
          </h3>

          <div className="form-control max-w-xs mt-2">
            <label className="label">
              <span className="label-text">{t('general.language.label')}</span>
            </label>
            <select
              className="select select-bordered w-full"
              value={settings?.language || 'en'}
              onChange={(e) => handleLanguageChange(e.target.value)}
              disabled={updateLanguage.isPending || !settings}
            >
              <option value="en">{t('general.language.options.en')}</option>
              <option value="id">{t('general.language.options.id')}</option>
              <option value="zh">{t('general.language.options.zh')}</option>
            </select>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body p-5">
          <h3 className="card-title text-lg flex items-center gap-2">
            <LogOut size={20} className="text-secondary" />
            {t('general.behavior.title')}
          </h3>

          <div className="form-control max-w-sm mt-2">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={autoCloseLauncher}
                onChange={(e) => setAutoCloseLauncher(e.target.checked)}
              />
              <span className="label-text font-medium">{t('general.behavior.auto_close')}</span>
            </label>
            <p className="text-sm text-base-content/70 mt-1 pl-13">
              {t('general.behavior.auto_close_desc')}
            </p>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body p-5">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Database size={20} className="text-accent" />
            {t('general.system.title')}
          </h3>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-2 text-xs opacity-80">
            <div>
              <span className="font-semibold block text-base-content/50 uppercase tracking-tighter">
                {t('general.system.app_version')}
              </span>
              <span className="font-mono">v0.1.0-alpha</span>
            </div>
            <div>
              <span className="font-semibold block text-base-content/50 uppercase tracking-tighter">
                {t('general.system.tauri_version')}
              </span>
              <span className="font-mono">v2.0.0</span>
            </div>
            <div>
              <span className="font-semibold block text-base-content/50 uppercase tracking-tighter">
                {t('general.system.database')}
              </span>
              <span className="font-mono">{t('general.system.db_val')}</span>
            </div>
            <div>
              <span className="font-semibold block text-base-content/50 uppercase tracking-tighter">
                {t('general.system.engine')}
              </span>
              <span className="font-mono">{t('general.system.engine_val')}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
