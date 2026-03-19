import { Monitor, Languages, Database, LogOut } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { useSettings } from '../../../hooks/useSettings';
import { THEME_OPTIONS, normalizeThemeSetting, type ThemeSetting } from '../theme/themeOptions';

export default function GeneralTab() {
  const { autoCloseLauncher, setAutoCloseLauncher } = useAppStore();
  const { settings, updateTheme } = useSettings();
  const selectedTheme = normalizeThemeSetting(settings?.theme);

  const handleThemeChange = (value: string) => {
    updateTheme.mutate(normalizeThemeSetting(value as ThemeSetting));
  };

  return (
    <div className="space-y-6">
      {/* Visual / Theme */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Monitor size={20} className="text-primary" />
            Appearance
          </h3>

          <div className="form-control max-w-xs mt-2">
            <label className="label" htmlFor="theme-select">
              <span className="label-text">Theme</span>
            </label>
            <select
              id="theme-select"
              className="select select-bordered w-full theme-controller"
              value={selectedTheme}
              onChange={(e) => handleThemeChange(e.target.value)}
              disabled={updateTheme.isPending || !settings}
            >
              {THEME_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          <p className="text-xs text-base-content/50 mt-2">
            System default will match your OS color scheme preference.
          </p>
        </div>
      </div>

      {/* Language */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Languages size={20} className="text-primary" />
            Language
          </h3>

          <div className="form-control max-w-xs mt-2">
            <label className="label">
              <span className="label-text">Interface Language</span>
            </label>
            <select className="select select-bordered w-full" disabled defaultValue="en">
              <option value="en">English (US)</option>
              <option value="id">Bahasa Indonesia (Coming Soon)</option>
              <option value="zh">Chinese (Simplified) (Coming Soon)</option>
            </select>
            <label className="label">
              <span className="label-text-alt text-warning">
                Only English is currently supported.
              </span>
            </label>
          </div>
        </div>
      </div>

      {/* App Behavior */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <LogOut size={20} className="text-secondary" />
            App Behavior
          </h3>

          <div className="form-control max-w-sm mt-2">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={autoCloseLauncher}
                onChange={(e) => setAutoCloseLauncher(e.target.checked)}
              />
              <span className="label-text font-medium">Auto-Close on Launch</span>
            </label>
            <p className="text-sm text-base-content/70 mt-1 pl-13">
              Automatically close EMMM2 completely when you successfully launch a game.
            </p>
          </div>
        </div>
      </div>

      {/* Advanced System Info */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Database size={20} className="text-secondary" />
            System Information
          </h3>

          <div className="grid grid-cols-2 gap-4 mt-2 text-sm opacity-80">
            <div>
              <span className="font-semibold block">App Version</span>
              <span>v0.1.0 (Alpha)</span>
            </div>
            <div>
              <span className="font-semibold block">Tauri Version</span>
              <span>v2.0.0</span>
            </div>
            <div>
              <span className="font-semibold block">Database</span>
              <span>SQLite (SQLx)</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
