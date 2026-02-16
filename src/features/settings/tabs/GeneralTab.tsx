import { Monitor, Languages, Database } from 'lucide-react';

export default function GeneralTab() {
  // Theme logic would hook into a ThemeContext or useTheme store.
  // For now, these are UI placeholders as per plan.

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
            <label className="label">
              <span className="label-text">Theme</span>
            </label>
            <select className="select select-bordered w-full" defaultValue="dark">
              <option value="system">System Default</option>
              <option value="dark">Dark (Dracula)</option>
              <option value="light">Light</option>
              <option value="cyberpunk">Cyberpunk</option>
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
