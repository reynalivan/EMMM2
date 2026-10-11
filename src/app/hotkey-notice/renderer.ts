import type { i18n } from 'i18next';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { acceptHotkeyNotice, buildHotkeyNoticeView } from '@/shared/lib/hotkeyNotice';

interface NoticeTransport {
  listen: (receive: (notice: HotkeyNotice) => void) => Promise<() => void>;
  get: () => Promise<HotkeyNotice | null>;
  ready: (id: string) => Promise<void>;
}

export async function mountHotkeyNoticeRenderer(
  root: HTMLElement,
  translation: i18n,
  transport: NoticeTransport,
) {
  let current: HotkeyNotice | null = null;
  let disposed = false;
  let acknowledgedId: string | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const text = (tag: string, className: string, value: string) => {
    const node = document.createElement(tag);
    node.className = className;
    node.textContent = value;
    return node;
  };
  const receive = async (candidate: HotkeyNotice) => {
    if (
      disposed ||
      !acceptHotkeyNotice(current, candidate, {
        gameId: candidate.game_id,
        modsPath: candidate.mods_path,
        now: Date.now(),
      })
    )
      return;
    current = candidate;
    await translation.changeLanguage(candidate.language);
    if (disposed || current !== candidate || candidate.expires_at_ms <= Date.now()) return;
    const t = translation.getFixedT(candidate.language, 'settings');
    const view = buildHotkeyNoticeView(candidate, t);
    document.documentElement.dataset.theme = candidate.theme === 'light' ? 'light' : 'onyx';
    document.documentElement.lang = translation.resolvedLanguage ?? 'en';
    root.replaceChildren();
    const panel = document.createElement('section');
    panel.className = 'game-hotkey-notice';
    panel.setAttribute('role', 'status');
    panel.append(text('p', 'notice-title', `${candidate.game_name} · ${view.title}`));
    panel.append(text('p', 'notice-primary', view.primary));
    if (view.origin) panel.append(text('p', 'notice-origin', view.origin));
    panel.append(text('p', 'notice-detail', view.detail));
    panel.append(text('p', 'notice-reload', view.reload));
    panel.append(text('h2', 'notice-helpers-title', t('hotkeys.feedback.controls_heading')));
    const hints = document.createElement('dl');
    hints.className = 'notice-hints';
    for (const hint of view.hints) {
      const row = document.createElement('div');
      row.append(text('dt', '', hint.label));
      row.append(
        text(
          'dd',
          'notice-binding',
          hint.enabled ? hint.binding : `${hint.binding} (${t('hotkeys.feedback.inactive')})`,
        ),
      );
      hints.append(row);
    }
    panel.append(hints);
    root.append(panel);
    clearTimeout(timer);
    timer = setTimeout(
      () => {
        if (current?.id === candidate.id) root.replaceChildren();
      },
      Math.max(0, candidate.expires_at_ms - Date.now()),
    );
    // Rust owns focus, original game/root, expiry and the OS window's visibility.
    if (acknowledgedId !== candidate.id) {
      await transport.ready(candidate.id);
      acknowledgedId = candidate.id;
    }
  };
  const unlisten = await transport.listen((notice) => {
    void receive(notice).catch((error: unknown) =>
      console.error('Game hotkey notification could not be displayed.', error),
    );
  });
  try {
    const snapshot = await transport.get();
    if (snapshot) await receive(snapshot);
  } catch (error) {
    console.error(
      'Initial game notice snapshot unavailable; the subscribed renderer can recover on the next notice.',
      error,
    );
  }
  return () => {
    disposed = true;
    clearTimeout(timer);
    unlisten();
    root.replaceChildren();
  };
}
