import { createInstance } from 'i18next';
import { listen } from '@tauri-apps/api/event';
import { commands } from '@/shared/api/tauri/bindings.gen';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import settingsEn from '@/shared/i18n/locales/en/settings.json';
import settingsId from '@/shared/i18n/locales/id/settings.json';
import settingsZh from '@/shared/i18n/locales/zh/settings.json';
import { HOTKEY_NOTICE_EVENT } from '@/shared/lib/hotkeyNotice';
import { mountHotkeyNoticeRenderer } from './renderer';
import '../entrypoint/App.css';
import './notice.css';

async function start() {
  const root = document.getElementById('notice');
  if (!root) throw new Error('Hotkey notice root is missing');
  const translation = createInstance();
  await translation.init({
    lng: 'en',
    fallbackLng: 'en',
    supportedLngs: ['en', 'id', 'zh'],
    resources: {
      en: { settings: settingsEn },
      id: { settings: settingsId },
      zh: { settings: settingsZh },
    },
    interpolation: { escapeValue: false },
  });
  const stop = await mountHotkeyNoticeRenderer(root, translation, {
    listen: (receive) =>
      listen<HotkeyNotice>(HOTKEY_NOTICE_EVENT, (event) => receive(event.payload)),
    get: async () => {
      const result = await commands.getHotkeyNotice();
      if (result.status === 'error') throw result.error;
      return result.data;
    },
    ready: async (id) => {
      const result = await commands.hotkeyNoticeReady(id);
      if (result.status === 'error') throw result.error;
    },
  });
  window.addEventListener('pagehide', stop, { once: true });
}

void start().catch((error: unknown) =>
  console.error('Native hotkey notification initialization failed.', error),
);
