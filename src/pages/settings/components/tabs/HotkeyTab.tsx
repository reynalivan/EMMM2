import { formatAppError } from '../../../../shared/lib/appError';
import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { HotkeyConfig, KeyViewerConfig } from '@/entities/settings';
import { useSettings } from '@/entities/settings';
import { commands } from '@/shared/api/tauri/bindings';
import { useToastStore } from '@/shared/ui/toast';
import { detectConflicts, type ReservedBinding } from '../../utils/hotkeyConflicts';
import { SettingsRow, SettingsSection } from '../SettingsLayout';

const DEFAULT_HOTKEYS: HotkeyConfig = {
  enabled: true,
  safe_mode: 'F5',
  next_preset: 'Ctrl+F6',
  prev_preset: 'Shift+F6',
  toggle_overlay: 'F7',
};

const DEFAULT_KEYVIEWER: KeyViewerConfig = { enabled: true };

type ResolvedHotkeyConfig = Omit<
  HotkeyConfig,
  'safe_mode' | 'next_preset' | 'prev_preset' | 'toggle_overlay'
> & {
  safe_mode: string;
  next_preset: string;
  prev_preset: string;
  toggle_overlay: string;
};

function normalizeHotkeys(config?: HotkeyConfig): ResolvedHotkeyConfig {
  return { ...DEFAULT_HOTKEYS, ...config } as ResolvedHotkeyConfig;
}

interface KeyBindingRowProps {
  label: string;
  value: string;
  defaultValue: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}

function KeyBindingRow({ label, value, defaultValue, disabled, onChange }: KeyBindingRowProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <div className="flex items-center justify-between gap-4 border-b border-base-300/70 py-2.5 last:border-0">
      <label className="text-sm font-medium">{label}</label>
      <div className="flex items-center gap-2">
        <input
          aria-label={label}
          type="text"
          className="input input-bordered input-sm w-32 text-center font-mono"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={defaultValue}
          disabled={disabled}
        />
        {value !== defaultValue && (
          <button
            type="button"
            className="btn btn-ghost btn-xs text-base-content/60 hover:text-primary"
            onClick={() => onChange(defaultValue)}
            title={t('settings:hotkeys.reset_tip')}
            disabled={disabled}
          >
            ↺
          </button>
        )}
      </div>
    </div>
  );
}

function formatRuntimeSyncTime(value: number | null | undefined): string | null {
  if (value === null || value === undefined) return null;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'medium',
  }).format(new Date(value));
}

export default function HotkeyTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, saveHotkeyConfiguration } = useSettings();
  const { addToast } = useToastStore();
  const [isSaving, setIsSaving] = useState(false);
  const [draftHotkeys, setDraftHotkeys] = useState<ResolvedHotkeyConfig>(
    normalizeHotkeys(DEFAULT_HOTKEYS),
  );
  const [draftKeyviewer, setDraftKeyviewer] = useState<KeyViewerConfig>(DEFAULT_KEYVIEWER);
  const activeGameId = settings?.active_game_id ?? null;
  const runtimeDiagnosticsQuery = useQuery({
    queryKey: ['keyviewerRuntimeDiagnostics', activeGameId, settings?.revision],
    queryFn: () => commands.getKeyviewerRuntimeDiagnostics(activeGameId!),
    enabled: activeGameId !== null,
  });

  useEffect(() => {
    if (!settings) return;
    setDraftHotkeys(normalizeHotkeys(settings.hotkeys));
    setDraftKeyviewer({ ...DEFAULT_KEYVIEWER, ...settings.keyviewer });
  }, [settings]);

  if (!settings) return null;

  const runtime = runtimeDiagnosticsQuery.data;
  const publication = runtime?.publication
    ? t(`settings:hotkeys.runtime.publication.${runtime.publication}`)
    : runtimeDiagnosticsQuery.isError
      ? t('settings:hotkeys.runtime.unavailable')
      : runtimeDiagnosticsQuery.isLoading
        ? t('settings:hotkeys.runtime.loading')
        : t('settings:hotkeys.runtime.not_synced');
  const reload = runtime?.reload
    ? t(`settings:hotkeys.runtime.reload.${runtime.reload}`, {
        binding: runtime.reload_binding ?? t('settings:hotkeys.runtime.binding_unavailable'),
      })
    : runtimeDiagnosticsQuery.isError
      ? t('settings:hotkeys.runtime.unavailable')
      : runtimeDiagnosticsQuery.isLoading
        ? t('settings:hotkeys.runtime.loading')
        : t('settings:hotkeys.runtime.not_attempted');

  const reserved: ReservedBinding[] = [
    { label: t('settings:hotkeys.reserved.package_toggle'), key: 'F6' },
    { label: t('settings:hotkeys.reserved.frame_analysis'), key: 'F8' },
  ];
  const conflicts = detectConflicts(draftHotkeys, reserved, t);
  const isDirty =
    JSON.stringify(draftHotkeys) !== JSON.stringify(normalizeHotkeys(settings.hotkeys)) ||
    JSON.stringify(draftKeyviewer) !==
      JSON.stringify({ ...DEFAULT_KEYVIEWER, ...settings.keyviewer });

  const updateHotkey = (patch: Partial<ResolvedHotkeyConfig>) => {
    setDraftHotkeys((current) => ({ ...current, ...patch }));
  };

  const save = async () => {
    if (conflicts.length > 0) return;
    setIsSaving(true);
    try {
      await saveHotkeyConfiguration(settings.revision ?? 0, draftHotkeys, draftKeyviewer);
      await runtimeDiagnosticsQuery.refetch();
      addToast('success', t('settings:hotkeys.save_success'));
    } catch (error) {
      addToast('error', t('settings:hotkeys.save_failed', { error: formatAppError(error) }));
    } finally {
      setIsSaving(false);
    }
  };

  const reset = () => {
    setDraftHotkeys(normalizeHotkeys(DEFAULT_HOTKEYS));
    setDraftKeyviewer(DEFAULT_KEYVIEWER);
  };

  return (
    <div>
      <SettingsSection
        id="hotkeys-settings-heading"
        title={t('settings:hotkeys.title')}
        description={t('settings:hotkeys.desc')}
        action={
          <div className="form-control">
            <label className="label cursor-pointer gap-3">
              <span className="label-text font-medium">{t('settings:hotkeys.enabled')}</span>
              <input
                type="checkbox"
                className="toggle toggle-primary toggle-sm"
                checked={draftHotkeys.enabled}
                onChange={() => updateHotkey({ enabled: !draftHotkeys.enabled })}
                disabled={isSaving}
              />
            </label>
          </div>
        }
      >
        <div className="space-y-3">
          <p className="text-xs text-base-content/60">{t('settings:hotkeys.os_hotkeys_hint')}</p>
          {conflicts.length > 0 && (
            <div
              className="alert alert-warning text-sm py-2 px-3 border-none bg-warning/10 text-warning-content"
              role="alert"
            >
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <div>
                <p className="font-bold">{t('settings:hotkeys.conflicts_title')}</p>
                {conflicts.map((conflict) => (
                  <p key={conflict} className="opacity-80">
                    {conflict}
                  </p>
                ))}
              </div>
            </div>
          )}
          <div>
            <KeyBindingRow
              label={t('settings:hotkeys.labels.safe_mode')}
              value={draftHotkeys.safe_mode}
              defaultValue={DEFAULT_HOTKEYS.safe_mode ?? 'F5'}
              disabled={isSaving || !draftHotkeys.enabled}
              onChange={(value) => updateHotkey({ safe_mode: value })}
            />
            <KeyBindingRow
              label={t('settings:hotkeys.labels.prev_preset')}
              value={draftHotkeys.prev_preset}
              defaultValue={DEFAULT_HOTKEYS.prev_preset ?? 'Shift+F6'}
              disabled={isSaving || !draftHotkeys.enabled}
              onChange={(value) => updateHotkey({ prev_preset: value })}
            />
            <KeyBindingRow
              label={t('settings:hotkeys.labels.next_preset')}
              value={draftHotkeys.next_preset}
              defaultValue={DEFAULT_HOTKEYS.next_preset ?? 'Ctrl+F6'}
              disabled={isSaving || !draftHotkeys.enabled}
              onChange={(value) => updateHotkey({ next_preset: value })}
            />
          </div>
        </div>
      </SettingsSection>

      <SettingsSection
        id="keyviewer-settings-heading"
        title={t('settings:hotkeys.viewer_title')}
        description={t('settings:hotkeys.viewer_desc', { key: draftHotkeys.toggle_overlay })}
        action={
          <div className="form-control">
            <label className="label cursor-pointer gap-3">
              <span className="label-text font-medium">
                {t('settings:hotkeys.overlay_enabled')}
              </span>
              <input
                type="checkbox"
                className="toggle toggle-secondary toggle-sm"
                checked={draftKeyviewer.enabled}
                onChange={() =>
                  setDraftKeyviewer((current) => ({ ...current, enabled: !current.enabled }))
                }
                disabled={isSaving}
              />
            </label>
          </div>
        }
      >
        <KeyBindingRow
          label={t('settings:hotkeys.labels.toggle_overlay')}
          value={draftHotkeys.toggle_overlay}
          defaultValue={DEFAULT_HOTKEYS.toggle_overlay ?? 'F7'}
          disabled={isSaving || !draftKeyviewer.enabled}
          onChange={(value) => updateHotkey({ toggle_overlay: value })}
        />
        <div className="border-l-2 border-info/40 pl-3 text-xs text-base-content/60">
          <p className="font-medium text-base-content">
            {t('settings:hotkeys.infrastructure_title')}
          </p>
          <p className="opacity-80">{t('settings:hotkeys.infrastructure_desc')}</p>
        </div>
      </SettingsSection>

      {activeGameId && (
        <SettingsSection
          id="keyviewer-runtime-heading"
          title={t('settings:hotkeys.runtime.title')}
          description={t('settings:hotkeys.runtime.desc')}
        >
          <div aria-live="polite">
            <SettingsRow
              label={t('settings:hotkeys.runtime.last_sync')}
              control={
                <span className="text-sm text-base-content/70">
                  {runtimeDiagnosticsQuery.isLoading
                    ? t('settings:hotkeys.runtime.loading')
                    : (formatRuntimeSyncTime(runtime?.last_sync_unix_ms) ??
                      t('settings:hotkeys.runtime.not_synced'))}
                </span>
              }
            />
            <SettingsRow
              label={t('settings:hotkeys.runtime.snapshot')}
              control={<span className="text-sm text-base-content/70">{publication}</span>}
            />
            <SettingsRow
              label={t('settings:hotkeys.runtime.reload_status')}
              control={<span className="text-sm text-base-content/70">{reload}</span>}
            />
            <SettingsRow
              label={t('settings:hotkeys.runtime.reload_binding')}
              control={
                <span className="font-mono text-sm text-base-content/70">
                  {runtimeDiagnosticsQuery.isLoading
                    ? t('settings:hotkeys.runtime.loading')
                    : (runtime?.reload_binding ??
                      t('settings:hotkeys.runtime.binding_unavailable'))}
                </span>
              }
            />
            <SettingsRow
              label={t('settings:hotkeys.runtime.generation_cleanup')}
              description={
                runtime?.cleanup_automatic_disabled
                  ? t('settings:hotkeys.runtime.cleanup_disabled_desc')
                  : undefined
              }
              control={
                <span className="text-sm text-base-content/70">
                  {runtimeDiagnosticsQuery.isLoading
                    ? t('settings:hotkeys.runtime.loading')
                    : runtime?.cleanup_automatic_disabled
                      ? t('settings:hotkeys.runtime.cleanup_disabled')
                      : runtimeDiagnosticsQuery.isError
                        ? t('settings:hotkeys.runtime.unavailable')
                        : t('settings:hotkeys.runtime.cleanup_enabled')}
                </span>
              }
            />
          </div>
        </SettingsSection>
      )}

      <div className="flex justify-end gap-3 pt-6">
        <button
          className="btn btn-ghost btn-sm text-base-content/60 hover:text-error"
          type="button"
          onClick={reset}
          disabled={isSaving || !isDirty}
        >
          {t('settings:hotkeys.reset')}
        </button>
        <button
          className="btn btn-primary btn-sm"
          type="button"
          onClick={() => void save()}
          disabled={isSaving || !isDirty || conflicts.length > 0}
        >
          {t('settings:hotkeys.save')}
        </button>
      </div>
    </div>
  );
}
