import { useEffect, type RefObject } from 'react';
import { Webview } from '@tauri-apps/api/webview';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
import type { BrowserTab } from '../../../stores/useBrowserStore';

/**
 * Keeps the native Tauri webviews positioned over `containerRef`, showing only
 * the active tab and hiding everything while a DOM overlay is open (native
 * webviews always paint above the DOM).
 */
export function useWebviewSync(
  containerRef: RefObject<HTMLDivElement | null>,
  tabs: BrowserTab[],
  activeTabId: string | null,
  overlayOpen: boolean,
): void {
  useEffect(() => {
    let resizeObserver: ResizeObserver | null = null;
    let isSyncing = false;
    let pendingSync = false;
    let isMounted = true;

    const syncWebviews = async () => {
      if (!isMounted) return;
      if (isSyncing) {
        pendingSync = true;
        return;
      }
      isSyncing = true;

      try {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return;

        for (const tab of tabs) {
          if (!isMounted) break;
          try {
            const webview = await Webview.getByLabel(tab.id);
            if (webview) {
              if (tab.id === activeTabId && !overlayOpen) {
                try {
                  await webview.setSize(new LogicalSize(rect.width, rect.height));
                  await webview.setPosition(new LogicalPosition(rect.left, rect.top));
                  await webview.show();
                  await webview.setFocus();
                } catch (innerErr) {
                  console.error(
                    `[Browser] Error modifying webview properties for ${tab.id}:`,
                    innerErr,
                  );
                }
              } else {
                try {
                  await webview.hide();
                } catch (hideErr) {
                  console.error(`[Browser] Error hiding webview ${tab.id}:`, hideErr);
                }
              }
            }
          } catch (err) {
            console.error(`[Browser] Failed to get/sync webview ${tab.id}:`, err);
          }
        }
      } finally {
        isSyncing = false;
        if (pendingSync && isMounted) {
          pendingSync = false;
          requestAnimationFrame(syncWebviews);
        }
      }
    };

    if (containerRef.current) {
      resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(syncWebviews);
      });
      resizeObserver.observe(containerRef.current);
    }

    const handleWinResize = () => {
      requestAnimationFrame(syncWebviews);
    };
    window.addEventListener('resize', handleWinResize);

    // Initial sync
    syncWebviews();

    return () => {
      isMounted = false;
      if (resizeObserver) resizeObserver.disconnect();
      window.removeEventListener('resize', handleWinResize);

      // We do not await this, just fire and forget hides on unmount
      tabs.forEach((t) => {
        Webview.getByLabel(t.id)
          .then((w) => {
            if (w) w.hide().catch(() => {});
          })
          .catch(() => {});
      });
    };
  }, [containerRef, tabs, activeTabId, overlayOpen]);
}
