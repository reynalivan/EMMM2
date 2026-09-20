import { useEffect, useRef, type RefObject } from 'react';
import { Webview } from '@tauri-apps/api/webview';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
import type { BrowserTab } from '@/entities/browser';
import { isDemoMode } from '@/shared/lib/appMode';
import type { BrowserSurfacePresentation } from '../browserSurfacePresentation';

/**
 * Keeps the native Tauri webviews positioned over `containerRef`, showing only
 * the active tab. The presentation policy controls whether browser UI retains
 * the native surface and which layer owns keyboard focus.
 */
export function useWebviewSync(
  containerRef: RefObject<HTMLDivElement | null>,
  tabs: BrowserTab[],
  activeTabId: string | null,
  presentation: BrowserSurfacePresentation,
): void {
  const latestTabsRef = useRef(tabs);

  useEffect(() => {
    latestTabsRef.current = tabs;
  }, [tabs]);

  useEffect(() => {
    if (isDemoMode) return;

    return () => {
      latestTabsRef.current
        .filter((tab) => !tab.isNewTab)
        .forEach((tab) => {
          Webview.getByLabel(tab.id)
            .then((webview) => webview?.hide().catch(() => undefined))
            .catch(() => undefined);
        });
    };
  }, []);

  useEffect(() => {
    if (isDemoMode) {
      return;
    }

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
        if (presentation.visibility === 'hidden') {
          for (const tab of tabs) {
            if (!isMounted) break;
            if (tab.isNewTab) continue;

            try {
              const webview = await Webview.getByLabel(tab.id);
              if (!isMounted) return;
              await webview?.hide();
            } catch (hideErr) {
              console.error(`[Browser] Error hiding webview ${tab.id}:`, hideErr);
            }
          }
          return;
        }

        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return;

        for (const tab of tabs) {
          if (!isMounted) break;
          if (tab.isNewTab) continue;
          try {
            const webview = await Webview.getByLabel(tab.id);
            if (!isMounted) return;
            if (webview) {
              if (tab.id === activeTabId) {
                try {
                  await webview.setSize(new LogicalSize(rect.width, rect.height));
                  if (!isMounted) return;
                  await webview.setPosition(new LogicalPosition(rect.left, rect.top));
                  if (!isMounted) return;
                  await webview.show();
                  if (isMounted && presentation.focus === 'browser') {
                    await webview.setFocus();
                  }
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
    };
  }, [containerRef, tabs, activeTabId, presentation]);
}
