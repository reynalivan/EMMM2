import { Webview } from '@tauri-apps/api/webview';
import { commands } from '@/shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import { useBrowserStore } from '@/entities/browser';

export async function openBrowserTabForSession({
  url,
  title,
  replaceTabId,
}: {
  url: string;
  title: string;
  replaceTabId?: string;
}): Promise<string | null> {
  const store = useBrowserStore.getState();
  const owner = { gameId: store.gameId, generation: store.generation };
  if (!owner.gameId || useAppStore.getState().activeGameId !== owner.gameId) return null;
  const label = await commands.browserOpenTab(url, null);
  const tab = { id: label, url, title };
  const published =
    useAppStore.getState().activeGameId === owner.gameId &&
    (replaceTabId ? store.replaceTab(replaceTabId, tab, owner) : store.addTab(tab, owner));
  if (!published) {
    const webview = await Webview.getByLabel(label);
    await webview?.close();
    return null;
  }
  return label;
}
