import { useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { useSettings } from '@/entities/settings';
import { useAppStore } from '@/app/store';
import { commands } from '@/shared/api/tauri/bindings';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { buildHotkeyNoticeView, HOTKEY_NOTICE_EVENT } from '@/shared/lib/hotkeyNotice';
import { useHotkeyNoticeStore } from './store';

export function HotkeyNoticeHost() {
  const { t } = useTranslation('settings');
  const { settings } = useSettings();
  const requestedGameId = useAppStore((state) => state.requestedGameId);
  const gameId = requestedGameId ? null : (settings?.active_game_id ?? null);
  const modsPath = settings?.games.find((game) => game.id === gameId)?.mod_path ?? null;
  const notice = useHotkeyNoticeStore((state) => state.notice);
  const runtime = useAppStore((state) => (gameId ? state.runtimeSyncByGame[gameId] : undefined));

  useEffect(() => {
    useHotkeyNoticeStore.getState().dismiss();
    if (!gameId || !modsPath) return;
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    const receive = (candidate: HotkeyNotice) => {
      if (!disposed) useHotkeyNoticeStore.getState().receive(candidate, gameId, modsPath);
    };
    void listen<HotkeyNotice>(HOTKEY_NOTICE_EVENT, (event) => receive(event.payload))
      .then(async (stop) => {
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        try {
          const snapshot = await commands.getHotkeyNotice();
          if (snapshot) receive(snapshot);
        } catch (error: unknown) {
          if (!disposed)
            console.warn(
              'Hotkey feedback snapshot unavailable; live notifications remain subscribed.',
              error,
            );
        }
      })
      .catch((error: unknown) => {
        if (!disposed)
          console.error(
            'Hotkey feedback unavailable; restart EMMM to retry the notification listener.',
            error,
          );
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [gameId, modsPath]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(
      () => useHotkeyNoticeStore.getState().dismiss(notice.id),
      Math.max(0, notice.expires_at_ms - Date.now()),
    );
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (
        event.key !== 'Escape' ||
        event.defaultPrevented ||
        document.querySelector('dialog[open], [aria-modal="true"]')
      )
        return;
      useHotkeyNoticeStore.getState().dismiss(notice.id);
    };
    document.addEventListener('keydown', dismissOnEscape);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('keydown', dismissOnEscape);
    };
  }, [notice]);

  if (!notice || notice.game_id !== gameId || notice.mods_path !== modsPath) return null;
  const view = buildHotkeyNoticeView(
    notice,
    t,
    {
      hotkeys:
        (settings?.revision ?? 0) >= notice.settings_revision
          ? (settings?.hotkeys ?? notice.hotkeys)
          : notice.hotkeys,
      keyviewerEnabled:
        (settings?.revision ?? 0) >= notice.settings_revision
          ? (settings?.keyviewer?.enabled ?? notice.keyviewer_enabled)
          : notice.keyviewer_enabled,
    },
    runtime?.generation === notice.runtime_generation ? runtime : undefined,
  );

  return (
    <div
      className="pointer-events-none fixed inset-x-4 top-20 z-40 flex justify-center"
      data-testid="hotkey-notice-host"
    >
      <section
        className="pointer-events-auto relative w-full max-w-2xl max-h-[calc(100dvh-6rem)] overflow-y-auto rounded-box border border-base-content/20 bg-base-200 p-6 text-base-content shadow-xl"
        role={notice.kind === 'failed' ? 'alert' : 'status'}
        aria-live={notice.kind === 'failed' ? 'assertive' : 'polite'}
        aria-atomic="true"
      >
        <button
          type="button"
          aria-label={t('hotkeys.feedback.dismiss')}
          className="btn btn-ghost absolute right-2 top-2 size-11 p-0"
          onClick={() => useHotkeyNoticeStore.getState().dismiss(notice.id)}
        >
          <X className="size-5" aria-hidden="true" />
        </button>
        <p className="pr-10 text-sm font-medium">
          {notice.game_name} · {view.title}
        </p>
        <p className="mt-2 break-words text-3xl font-semibold text-primary">{view.primary}</p>
        {view.origin && <p className="mt-1 break-words text-sm">{view.origin}</p>}
        <p className="mt-3 break-words text-sm">{view.detail}</p>
        <p className="mt-1 text-sm font-medium">{view.reload}</p>
        <h3 className="mt-5 border-t border-base-content/20 pt-3 text-xs font-semibold">
          {t('hotkeys.feedback.controls_heading')}
        </h3>
        <dl className="mt-2 grid grid-cols-1 gap-x-6 gap-y-2 sm:grid-cols-2">
          {view.hints.map((hint) => (
            <div
              key={hint.id}
              className="flex min-w-0 flex-wrap items-baseline justify-between gap-2 text-xs"
            >
              <dt>{hint.label}</dt>
              <dd className="break-words font-mono">
                {hint.binding}
                {!hint.enabled && (
                  <span className="ml-2 font-sans">{t('hotkeys.feedback.inactive')}</span>
                )}
              </dd>
            </div>
          ))}
        </dl>
      </section>
    </div>
  );
}
