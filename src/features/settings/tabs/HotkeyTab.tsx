import { useState } from 'react';
import { Keyboard, Eye, AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { commands } from '../../../lib/bindings';
import type { HotkeyConfig, KeyViewerConfig } from '../../../types/settings';
import { useSettings } from '../../../hooks/useSettings';
import { useToastStore } from '../../../stores/useToastStore';

/** Default hotkey config values — unified overlay toggle F7. */
const DEFAULT_HOTKEYS: HotkeyConfig = {
  enabled: true,
  cooldown_ms: 500,
  toggle_safe_mode: 'F5',
  next_preset: 'F6',
  prev_preset: 'Shift+F6',
  toggle_overlay: 'F7',
};

const DEFAULT_KEYVIEWER: KeyViewerConfig = {
  enabled: true,
};

interface KeyBindingRowProps {
  label: string;
  value: string;
  defaultValue: string;
  onChange: (value: string) => void;
}

function KeyBindingRow({ label, value, defaultValue, onChange }: KeyBindingRowProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <div className="flex items-center justify-between py-2 border-b border-base-content/5 last:border-0">
      <span className="text-sm font-medium">{label}</span>
      <div className="flex items-center gap-2">
        <input
          type="text"
          className="input input-bordered input-sm w-32 text-center font-mono"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={defaultValue}
        />
        {value !== defaultValue && (
          <button
            className="btn btn-ghost btn-xs text-base-content/40 hover:text-primary"
            onClick={() => onChange(defaultValue)}
            title={t('settings:hotkeys.reset_tip')}
          >
            ↺
          </button>
        )}
      </div>
    </div>
  );
}

/** Detect conflicts: same key string used for multiple actions. */
function detectConflicts(config: HotkeyConfig, t: TFunction): string[] {
  const bindings: [string, string][] = [
    [t('settings:hotkeys.labels.safe_mode'), config.toggle_safe_mode],
    [t('settings:hotkeys.labels.next_preset'), config.next_preset],
    [t('settings:hotkeys.labels.prev_preset'), config.prev_preset],
    [t('settings:hotkeys.labels.toggle_overlay'), config.toggle_overlay],
  ];

  const conflicts: string[] = [];
  for (let i = 0; i < bindings.length; i++) {
    for (let j = i + 1; j < bindings.length; j++) {
      if (bindings[i][1].toLowerCase() === bindings[j][1].toLowerCase()) {
        conflicts.push(
          t('settings:hotkeys.conflicts.message', {
            label1: bindings[i][0],
            label2: bindings[j][0],
            key: bindings[i][1],
            defaultValue: `${bindings[i][0]} ${t('settings:hotkeys.conflicts.and')} ${bindings[j][0]} ${t('settings:hotkeys.conflicts.share_key')} "${bindings[i][1]}"`,
          }),
        );
      }
    }
  }
  return conflicts;
}

export default function HotkeyTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, saveSettingsAsync } = useSettings();
  const { addToast } = useToastStore();
  const [isSaving, setIsSaving] = useState(false);

  if (!settings) return null;

  const hotkeys: HotkeyConfig = (settings.hotkeys ?? DEFAULT_HOTKEYS) as HotkeyConfig;
  const keyviewer: KeyViewerConfig = (settings.keyviewer ?? DEFAULT_KEYVIEWER) as KeyViewerConfig;
  const conflicts = detectConflicts(hotkeys, t);

  const persistHotkeys = async (patch: Partial<HotkeyConfig>) => {
    if (!settings) return;
    setIsSaving(true);
    try {
      await saveSettingsAsync({
        ...settings,
        hotkeys: { ...hotkeys, ...patch },
      });
      await commands.updateHotkeyConfig({});
    } catch (err) {
      addToast('error', t('settings:hotkeys.save_failed', { error: String(err) }));
    } finally {
      setIsSaving(false);
    }
  };

  const persistKeyViewer = async (patch: Partial<KeyViewerConfig>) => {
    if (!settings) return;
    setIsSaving(true);
    try {
      await saveSettingsAsync({
        ...settings,
        keyviewer: { ...keyviewer, ...patch },
      });
    } catch (err) {
      addToast('error', t('settings:hotkeys.viewer_save_failed', { error: String(err) }));
    } finally {
      setIsSaving(false);
    }
  };

  const handleResetAll = () => {
    void (async () => {
      if (!settings) return;
      try {
        await saveSettingsAsync({
          ...settings,
          hotkeys: { ...DEFAULT_HOTKEYS },
          keyviewer: { ...DEFAULT_KEYVIEWER },
        });
        await commands.updateHotkeyConfig({});
        addToast('success', t('settings:hotkeys.reset_success'));
      } catch (err) {
        addToast('error', t('settings:hotkeys.save_failed', { error: String(err) }));
      }
    })();
  };

  return (
    <div className="space-y-6">
      {/* ─── Hotkeys Section ─── */}
      <div className="card bg-base-200 shadow-sm border border-base-content/5">
        <div className="card-body gap-4">
          <div className="flex items-center justify-between">
            <h3 className="card-title text-lg gap-2">
              <Keyboard className="w-5 h-5 text-primary" />
              {t('settings:hotkeys.title')}
            </h3>
            <div className="form-control">
              <label className="label cursor-pointer gap-3">
                <span className="label-text font-medium">{t('settings:hotkeys.enabled')}</span>
                <input
                  type="checkbox"
                  className="toggle toggle-primary toggle-sm"
                  checked={hotkeys.enabled}
                  onChange={() => persistHotkeys({ enabled: !hotkeys.enabled })}
                  disabled={isSaving}
                />
              </label>
            </div>
          </div>

          <p className="text-sm text-base-content/60 leading-relaxed">
            {t('settings:hotkeys.desc')}
          </p>

          {hotkeys.enabled && (
            <div className="space-y-4 pt-2">
              <div className="flex items-center justify-between p-3 bg-base-300/50 rounded-lg">
                <div className="flex flex-col">
                  <span className="text-sm font-semibold">{t('settings:hotkeys.cooldown')}</span>
                  <span className="text-xs text-base-content/40">
                    {t('settings:hotkeys.cooldown_desc')}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    className="input input-bordered input-sm w-24 text-center font-mono"
                    value={hotkeys.cooldown_ms}
                    min={100}
                    max={5000}
                    step={100}
                    onChange={(e) =>
                      persistHotkeys({ cooldown_ms: parseInt(e.target.value) || 500 })
                    }
                    disabled={isSaving}
                  />
                  <span className="text-xs font-medium text-base-content/40 w-6">ms</span>
                </div>
              </div>

              {/* Conflict warning */}
              {conflicts.length > 0 && (
                <div className="alert alert-warning text-sm py-2 px-3 border-none bg-warning/10 text-warning-content">
                  <AlertTriangle className="w-4 h-4 shrink-0" />
                  <div>
                    <p className="font-bold">{t('settings:hotkeys.conflicts_title')}</p>
                    {conflicts.map((c, i) => (
                      <p key={i} className="opacity-80">
                        {c}
                      </p>
                    ))}
                  </div>
                </div>
              )}

              <div className="space-y-0 bg-base-300/30 rounded-lg p-3">
                <KeyBindingRow
                  label={t('settings:hotkeys.labels.safe_mode')}
                  value={hotkeys.toggle_safe_mode}
                  defaultValue={DEFAULT_HOTKEYS.toggle_safe_mode}
                  onChange={(v) => persistHotkeys({ toggle_safe_mode: v })}
                />
                <KeyBindingRow
                  label={t('settings:hotkeys.labels.next_preset')}
                  value={hotkeys.next_preset}
                  defaultValue={DEFAULT_HOTKEYS.next_preset}
                  onChange={(v) => persistHotkeys({ next_preset: v })}
                />
                <KeyBindingRow
                  label={t('settings:hotkeys.labels.prev_preset')}
                  value={hotkeys.prev_preset}
                  defaultValue={DEFAULT_HOTKEYS.prev_preset}
                  onChange={(v) => persistHotkeys({ prev_preset: v })}
                />
                <KeyBindingRow
                  label={t('settings:hotkeys.labels.toggle_overlay')}
                  value={hotkeys.toggle_overlay}
                  defaultValue={DEFAULT_HOTKEYS.toggle_overlay}
                  onChange={(v) => persistHotkeys({ toggle_overlay: v })}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ─── KeyViewer Section ─── */}
      <div className="card bg-base-200 shadow-sm border border-base-content/5">
        <div className="card-body gap-4">
          <div className="flex items-center justify-between">
            <h3 className="card-title text-lg gap-2">
              <Eye className="w-5 h-5 text-secondary" />
              {t('settings:hotkeys.viewer_title')}
            </h3>
            <div className="form-control">
              <label className="label cursor-pointer gap-3">
                <span className="label-text font-medium">{t('settings:hotkeys.auto_reload')}</span>
                <input
                  type="checkbox"
                  className="toggle toggle-secondary toggle-sm"
                  checked={keyviewer.enabled}
                  onChange={() => persistKeyViewer({ enabled: !keyviewer.enabled })}
                  disabled={isSaving}
                />
              </label>
            </div>
          </div>

          <p className="text-sm text-base-content/60 leading-relaxed">
            {t('settings:hotkeys.viewer_desc', { key: hotkeys.toggle_overlay })}
          </p>

          <div className="alert text-xs bg-info/10 border-none text-info-content">
            <div className="flex flex-col gap-1">
              <p className="font-bold">{t('settings:hotkeys.infrastructure_title')}</p>
              <p className="opacity-80">{t('settings:hotkeys.infrastructure_desc')}</p>
            </div>
          </div>
        </div>
      </div>

      {/* ─── Reset ─── */}
      <div className="flex justify-end pt-2">
        <button
          className="btn btn-ghost btn-sm text-base-content/40 hover:text-error"
          onClick={handleResetAll}
          disabled={isSaving}
        >
          {t('settings:hotkeys.reset')}
        </button>
      </div>
    </div>
  );
}
