import type { TFunction } from 'i18next';
import type { HotkeyConfig, HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import type { RuntimeSyncStatus } from '@/shared/api/tauri/events';

export const HOTKEY_NOTICE_EVENT = 'hotkey:notice';
const MAX_NOTICE_DURATION_MS = 5000;
const CLOCK_TOLERANCE_MS = 1000;
const RUNTIME_PHASE_RANK = {
  queued: 0,
  running: 1,
  succeeded: 2,
  needs_manual_reload: 2,
  failed: 2,
};

function forwardRuntimeRefresh(current: HotkeyNotice, candidate: HotkeyNotice): boolean {
  const next = candidate.runtime_status;
  if (
    candidate.id !== current.id ||
    candidate.sequence !== current.sequence ||
    candidate.root_identity !== current.root_identity ||
    candidate.kind !== current.kind ||
    candidate.created_at_ms !== current.created_at_ms ||
    candidate.expires_at_ms !== current.expires_at_ms ||
    candidate.collection !== current.collection ||
    candidate.previous_collection !== current.previous_collection ||
    candidate.safe_mode !== current.safe_mode ||
    !next ||
    next.game_id !== candidate.game_id ||
    next.generation !== candidate.runtime_generation
  )
    return false;
  const previous = current.runtime_status;
  if (!previous) return true;
  return RUNTIME_PHASE_RANK[next.phase] > RUNTIME_PHASE_RANK[previous.phase];
}

function noticeRuntimeStatus(
  notice: HotkeyNotice,
  runtime?: RuntimeSyncStatus,
): RuntimeSyncStatus | null {
  const matches = (status: RuntimeSyncStatus | null | undefined) =>
    status?.game_id === notice.game_id && status.generation === notice.runtime_generation
      ? status
      : null;
  const embedded = matches(notice.runtime_status);
  const observed = matches(runtime);
  if (!embedded) return observed;
  if (!observed) return embedded;
  return RUNTIME_PHASE_RANK[observed.phase] > RUNTIME_PHASE_RANK[embedded.phase]
    ? observed
    : embedded;
}

export function acceptHotkeyNotice(
  current: HotkeyNotice | null,
  candidate: HotkeyNotice,
  context: { gameId: string; modsPath: string; now: number },
): boolean {
  return (
    candidate.game_id === context.gameId &&
    candidate.mods_path === context.modsPath &&
    candidate.id.length > 0 &&
    Number.isSafeInteger(candidate.sequence) &&
    candidate.sequence > 0 &&
    Number.isSafeInteger(candidate.created_at_ms) &&
    Number.isSafeInteger(candidate.expires_at_ms) &&
    candidate.created_at_ms <= context.now + CLOCK_TOLERANCE_MS &&
    candidate.expires_at_ms > context.now &&
    candidate.expires_at_ms > candidate.created_at_ms &&
    candidate.expires_at_ms - candidate.created_at_ms <= MAX_NOTICE_DURATION_MS &&
    (!current || candidate.sequence > current.sequence || forwardRuntimeRefresh(current, candidate))
  );
}

export interface HotkeyHint {
  id: string;
  label: string;
  binding: string;
  enabled: boolean;
}

export function buildHotkeyNoticeView(
  notice: HotkeyNotice,
  t: TFunction,
  controls: { hotkeys: HotkeyConfig; keyviewerEnabled: boolean } = {
    hotkeys: notice.hotkeys,
    keyviewerEnabled: notice.keyviewer_enabled,
  },
  runtime?: RuntimeSyncStatus,
) {
  const key = (name: string) => t(`hotkeys.feedback.${name}`);
  const { hotkeys, keyviewerEnabled } = controls;
  const status = noticeRuntimeStatus(notice, runtime);
  const reloadBinding =
    status?.reload_status === 'manual'
      ? (status.reload_binding ?? null)
      : (status?.reload_binding ?? notice.reload_binding);
  const hints: HotkeyHint[] = [
    {
      id: 'safe_mode',
      label: t('hotkeys.labels.safe_mode'),
      binding: hotkeys.safe_mode ?? '',
      enabled: hotkeys.enabled,
    },
    {
      id: 'prev_preset',
      label: t('hotkeys.labels.prev_preset'),
      binding: hotkeys.prev_preset ?? '',
      enabled: hotkeys.enabled,
    },
    {
      id: 'next_preset',
      label: t('hotkeys.labels.next_preset'),
      binding: hotkeys.next_preset ?? '',
      enabled: hotkeys.enabled,
    },
    {
      id: 'toggle_overlay',
      label: key('keyviewer_control'),
      binding: hotkeys.toggle_overlay ?? '',
      enabled: keyviewerEnabled,
    },
    {
      id: 'reload',
      label: key('reload_control'),
      binding: reloadBinding ?? key('binding_unknown'),
      enabled: reloadBinding !== null,
    },
  ];
  let reload = reloadBinding
    ? t('hotkeys.feedback.reload_hint', { binding: reloadBinding })
    : key('reload_unknown');
  let runtimeDetail = key('runtime_pending');
  if (status?.phase === 'succeeded') {
    runtimeDetail = key('runtime_completed');
    reload =
      status.reload_status === 'sent'
        ? t('hotkeys.feedback.reload_sent', { binding: reloadBinding ?? key('binding_unknown') })
        : status.reload_status === 'not_required'
          ? key('reload_not_required')
          : key('reload_controls_available');
  } else if (status?.phase === 'needs_manual_reload') {
    runtimeDetail = key('runtime_manual');
  } else if (status?.phase === 'failed') {
    runtimeDetail = key('runtime_failed');
    reload = key('runtime_retry_before_reload');
  }

  switch (notice.kind) {
    case 'collection_applied':
      return {
        title: key(
          notice.previous_collection !== notice.collection
            ? 'collection_switched'
            : 'collection_applied',
        ),
        primary: notice.collection ?? key('current_state'),
        origin: notice.previous_collection
          ? t('hotkeys.feedback.from_collection', { name: notice.previous_collection })
          : key('from_current_state'),
        detail: runtimeDetail,
        reload,
        hints,
      };
    case 'safe_mode_changed':
      return {
        title: key('safe_mode_changed'),
        primary: key(notice.safe_mode ? 'safe_on' : 'safe_off'),
        origin: notice.collection
          ? t('hotkeys.feedback.active_collection', { name: notice.collection })
          : null,
        detail: runtimeDetail,
        reload,
        hints,
      };
    case 'no_presets':
      return {
        title: key('no_presets'),
        primary: key('create_collection'),
        origin: null,
        detail: key('no_presets_detail'),
        reload,
        hints,
      };
    case 'failed':
      return {
        title: key('failed'),
        primary: key('not_applied'),
        origin: null,
        detail: notice.message ?? key('failed_detail'),
        reload,
        hints,
      };
  }
}
