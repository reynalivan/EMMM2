import { useState } from 'react';
import { Keyboard, Eye, AlertTriangle } from 'lucide-react';
import { useSettings } from '../../../hooks/useSettings';
import type { HotkeyConfig, KeyViewerConfig } from '../../../hooks/useSettings';
import { useToastStore } from '../../../stores/useToastStore';

/** Default hotkey config values for comparison / reset. */
const DEFAULT_HOTKEYS: HotkeyConfig = {
  enabled: true,
  game_focus_only: true,
  cooldown_ms: 500,
  toggle_safe_mode: 'F5',
  next_preset: 'F6',
  prev_preset: 'Shift+F6',
  next_variant: 'F8',
  prev_variant: 'Shift+F8',
  toggle_overlay: 'F7',
};

const DEFAULT_KEYVIEWER: KeyViewerConfig = {
  enabled: true,
  status_ttl_seconds: 3.0,
  overlay_toggle_key: 'F7',
  keybinds_dir: 'EMM2/keybinds/active',
};

interface KeyBindingRowProps {
  label: string;
  value: string;
  defaultValue: string;
  onChange: (value: string) => void;
}

function KeyBindingRow({ label, value, defaultValue, onChange }: KeyBindingRowProps) {
  return (
    <div className="flex items-center justify-between py-2">
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
            className="btn btn-ghost btn-xs"
            onClick={() => onChange(defaultValue)}
            title="Reset to default"
          >
            ↺
          </button>
        )}
      </div>
    </div>
  );
}

/** Detect conflicts: same key string used for multiple actions. */
function detectConflicts(config: HotkeyConfig): string[] {
  const bindings: [string, string][] = [
    ['Safe Mode Toggle', config.toggle_safe_mode],
    ['Next Preset', config.next_preset],
    ['Prev Preset', config.prev_preset],
    ['Next Variant', config.next_variant],
    ['Prev Variant', config.prev_variant],
    ['Toggle Overlay', config.toggle_overlay],
  ];

  const conflicts: string[] = [];
  for (let i = 0; i < bindings.length; i++) {
    for (let j = i + 1; j < bindings.length; j++) {
      if (bindings[i][1].toLowerCase() === bindings[j][1].toLowerCase()) {
        conflicts.push(`${bindings[i][0]} and ${bindings[j][0]} share key "${bindings[i][1]}"`);
      }
    }
  }
  return conflicts;
}

export default function HotkeyTab() {
  const { settings, saveSettings } = useSettings();
  const { addToast } = useToastStore();
  const [isSaving, setIsSaving] = useState(false);

  if (!settings) return null;

  const hotkeys = settings.hotkeys ?? DEFAULT_HOTKEYS;
  const keyviewer = settings.keyviewer ?? DEFAULT_KEYVIEWER;

  const conflicts = detectConflicts(hotkeys);

  const persistHotkeys = async (patch: Partial<HotkeyConfig>) => {
    if (!settings) return;
    setIsSaving(true);
    try {
      saveSettings({
        ...settings,
        hotkeys: { ...hotkeys, ...patch },
      });
    } catch (err) {
      addToast('error', `Failed to save hotkey settings: ${String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const persistKeyViewer = async (patch: Partial<KeyViewerConfig>) => {
    if (!settings) return;
    setIsSaving(true);
    try {
      saveSettings({
        ...settings,
        keyviewer: { ...keyviewer, ...patch },
      });
    } catch (err) {
      addToast('error', `Failed to save KeyViewer settings: ${String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleResetAll = () => {
    saveSettings({
      ...settings,
      hotkeys: { ...DEFAULT_HOTKEYS },
      keyviewer: { ...DEFAULT_KEYVIEWER },
    });
    addToast('success', 'Hotkey settings reset to defaults.');
  };

  return (
    <div className="space-y-6">
      {/* ─── Hotkeys Section ─── */}
      <div className="card bg-base-200">
        <div className="card-body">
          <h3 className="card-title text-lg gap-2">
            <Keyboard className="w-5 h-5" />
            Global Hotkeys
          </h3>
          <p className="text-sm text-base-content/60">
            System-wide keyboard shortcuts for quick actions while gaming.
          </p>

          {/* Master toggle */}
          <div className="form-control">
            <label className="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={hotkeys.enabled}
                onChange={() => persistHotkeys({ enabled: !hotkeys.enabled })}
                disabled={isSaving}
              />
              <span className="label-text font-medium">Enable Global Hotkeys</span>
            </label>
          </div>

          {hotkeys.enabled && (
            <>
              {/* Game focus only */}
              <div className="form-control">
                <label className="label cursor-pointer justify-start gap-3">
                  <input
                    type="checkbox"
                    className="toggle toggle-sm"
                    checked={hotkeys.game_focus_only}
                    onChange={() => persistHotkeys({ game_focus_only: !hotkeys.game_focus_only })}
                    disabled={isSaving}
                  />
                  <span className="label-text text-sm">
                    Only trigger when game window is focused
                  </span>
                </label>
              </div>

              {/* Cooldown */}
              <div className="flex items-center justify-between py-2">
                <span className="text-sm font-medium">Cooldown (ms)</span>
                <input
                  type="number"
                  className="input input-bordered input-sm w-24 text-center"
                  value={hotkeys.cooldown_ms}
                  min={100}
                  max={5000}
                  step={100}
                  onChange={(e) => persistHotkeys({ cooldown_ms: parseInt(e.target.value) || 500 })}
                  disabled={isSaving}
                />
              </div>

              <div className="divider my-1 text-xs text-base-content/40">Key Bindings</div>

              {/* Conflict warning */}
              {conflicts.length > 0 && (
                <div className="alert alert-warning text-sm">
                  <AlertTriangle className="w-4 h-4" />
                  <div>
                    <p className="font-semibold">Key Conflicts Detected</p>
                    {conflicts.map((c, i) => (
                      <p key={i}>{c}</p>
                    ))}
                  </div>
                </div>
              )}

              {/* Key bindings */}
              <div className="space-y-1">
                <KeyBindingRow
                  label="Toggle Safe Mode"
                  value={hotkeys.toggle_safe_mode}
                  defaultValue={DEFAULT_HOTKEYS.toggle_safe_mode}
                  onChange={(v) => persistHotkeys({ toggle_safe_mode: v })}
                />
                <KeyBindingRow
                  label="Next Preset"
                  value={hotkeys.next_preset}
                  defaultValue={DEFAULT_HOTKEYS.next_preset}
                  onChange={(v) => persistHotkeys({ next_preset: v })}
                />
                <KeyBindingRow
                  label="Previous Preset"
                  value={hotkeys.prev_preset}
                  defaultValue={DEFAULT_HOTKEYS.prev_preset}
                  onChange={(v) => persistHotkeys({ prev_preset: v })}
                />
                <KeyBindingRow
                  label="Next Variant Folder"
                  value={hotkeys.next_variant}
                  defaultValue={DEFAULT_HOTKEYS.next_variant}
                  onChange={(v) => persistHotkeys({ next_variant: v })}
                />
                <KeyBindingRow
                  label="Previous Variant Folder"
                  value={hotkeys.prev_variant}
                  defaultValue={DEFAULT_HOTKEYS.prev_variant}
                  onChange={(v) => persistHotkeys({ prev_variant: v })}
                />
                <KeyBindingRow
                  label="Toggle KeyViewer Overlay"
                  value={hotkeys.toggle_overlay}
                  defaultValue={DEFAULT_HOTKEYS.toggle_overlay}
                  onChange={(v) => persistHotkeys({ toggle_overlay: v })}
                />
              </div>
            </>
          )}
        </div>
      </div>

      {/* ─── KeyViewer Section ─── */}
      <div className="card bg-base-200">
        <div className="card-body">
          <h3 className="card-title text-lg gap-2">
            <Eye className="w-5 h-5" />
            KeyViewer Overlay
          </h3>
          <p className="text-sm text-base-content/60">
            In-game overlay showing keybinds for the detected character.
          </p>

          {/* Master toggle */}
          <div className="form-control">
            <label className="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={keyviewer.enabled}
                onChange={() => persistKeyViewer({ enabled: !keyviewer.enabled })}
                disabled={isSaving}
              />
              <span className="label-text font-medium">Enable KeyViewer</span>
            </label>
          </div>

          {keyviewer.enabled && (
            <div className="space-y-3">
              {/* Status TTL */}
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Status Banner Duration (seconds)</span>
                <input
                  type="number"
                  className="input input-bordered input-sm w-20 text-center"
                  value={keyviewer.status_ttl_seconds}
                  min={1}
                  max={30}
                  step={0.5}
                  onChange={(e) =>
                    persistKeyViewer({ status_ttl_seconds: parseFloat(e.target.value) || 3.0 })
                  }
                  disabled={isSaving}
                />
              </div>

              {/* Overlay toggle key */}
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Overlay Toggle Key</span>
                <input
                  type="text"
                  className="input input-bordered input-sm w-20 text-center font-mono"
                  value={keyviewer.overlay_toggle_key}
                  onChange={(e) => persistKeyViewer({ overlay_toggle_key: e.target.value })}
                  placeholder="F7"
                  disabled={isSaving}
                />
              </div>

              {/* Keybinds directory */}
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Keybinds Directory</span>
                <input
                  type="text"
                  className="input input-bordered input-sm w-56 font-mono text-xs"
                  value={keyviewer.keybinds_dir}
                  onChange={(e) => persistKeyViewer({ keybinds_dir: e.target.value })}
                  disabled={isSaving}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ─── Reset ─── */}
      <div className="flex justify-end">
        <button className="btn btn-outline btn-sm" onClick={handleResetAll} disabled={isSaving}>
          Reset All to Defaults
        </button>
      </div>
    </div>
  );
}
