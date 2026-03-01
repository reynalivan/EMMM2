import { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Webview } from '@tauri-apps/api/webview';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
import { useBrowserStore } from '../../../stores/useBrowserStore';
import { useDownloads } from '../hooks/useDownloads';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import { GamePickerModal } from './GamePickerModal';
import { ImportQueuePanel } from './ImportQueuePanel';
import { Globe, Download, Plus, X, RotateCcw, Trash2 } from 'lucide-react';

export function BrowserPage() {
  const [urlInput, setUrlInput] = useState('');
  const [isNavigating, setIsNavigating] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Container that the Webview will be placed over
  const containerRef = useRef<HTMLDivElement>(null);

  // Import selection state
  const [importIds, setImportIds] = useState<string[]>([]);
  const [isGamePickerOpen, setIsGamePickerOpen] = useState(false);

  const {
    openDownloadPanel,
    isDownloadPanelOpen,
    tabs,
    activeTabId,
    addTab,
    removeTab,
    setActiveTab,
  } = useBrowserStore();

  const { finishedCount } = useDownloads();

  const activeTab = tabs.find((t) => t.id === activeTabId);

  // Navigate in a new tab window
  const handleNavigate = useCallback(
    async (url: string, asNewTab: boolean = false) => {
      let normalized = url.trim();
      if (!normalized.startsWith('http') && !normalized.startsWith('about:')) {
        normalized = `https://${normalized}`;
      }
      setIsNavigating(true);
      try {
        if (asNewTab || tabs.length === 0) {
          const label = await invoke<string>('browser_open_tab', {
            url: normalized,
            sessionId: null,
          });

          addTab({
            id: label,
            title: 'Loading...',
            url: normalized,
          });
        } else if (activeTabId) {
          await invoke('browser_navigate', {
            label: activeTabId,
            url: normalized,
          });
          useBrowserStore.getState().updateTab(activeTabId, { url: normalized });
        }
      } catch (err) {
        console.error('Failed to navigate browser:', err);
      } finally {
        setIsNavigating(false);
      }
    },
    [tabs.length, activeTabId, addTab],
  );

  // Synchronize URL input with active tab
  useEffect(() => {
    if (activeTab) {
      setUrlInput(activeTab.url);
    } else {
      setUrlInput('');
    }
  }, [activeTab]);

  // Handle resizing and positioning of the Tauri Webviews
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
              if (tab.id === activeTabId) {
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
  }, [tabs, activeTabId]);

  // Keep a ref to handleNavigate to avoid listener re-creation loops
  const navigateRef = useRef(handleNavigate);
  useEffect(() => {
    navigateRef.current = handleNavigate;
  }, [handleNavigate]);

  // Listen for navigation changes from the backend (run ONCE on mount)
  useEffect(() => {
    let unlistenUrl: (() => void) | undefined;
    let unlistenNewTab: (() => void) | undefined;

    listen<{ label: string; url: string; title: string }>('browser:url-changed', (event) => {
      const { label, url, title } = event.payload;
      useBrowserStore.getState().updateTab(label, { url, title: title || url });
    }).then((f) => {
      unlistenUrl = f;
    });

    listen<{ url: string }>('browser:new-tab-requested', (event) => {
      if (navigateRef.current) {
        navigateRef.current(event.payload.url, true);
      }
    }).then((f) => {
      unlistenNewTab = f;
    });

    return () => {
      if (unlistenUrl) unlistenUrl();
      if (unlistenNewTab) unlistenNewTab();
    };
  }, []);

  const handleReload = async () => {
    if (!activeTabId) return;
    setIsRefreshing(true);
    try {
      await invoke('browser_reload_tab', { label: activeTabId });
    } catch (err) {
      console.error('Failed to reload:', err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleClearData = async () => {
    if (!activeTabId) return;
    const confirmed = await window.confirm(
      'Clear all browsing data (cookies, cache, etc.) for this session?',
    );
    if (!confirmed) return;

    try {
      await invoke('browser_clear_data', { label: activeTabId });
      // Reload to apply changes
      handleReload();
    } catch (err) {
      console.error('Failed to clear data:', err);
    }
  };

  const handleUrlSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const url = urlInput.trim();
    if (url) {
      // Logic fix: if we have an active tab, navigate it.
      // If we have no tabs, open a new one.
      handleNavigate(url, tabs.length === 0);
    }
  };

  const handleCloseTab = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const w = await Webview.getByLabel(id);
      if (w) await w.close();
    } catch {
      /* ignore */
    }
    removeTab(id);
  };

  const handleImportSelected = (ids: string[], _gameId: string) => {
    setImportIds(ids);
    setIsGamePickerOpen(true);
  };

  const handleGameConfirm = async (gameId: string) => {
    try {
      await invoke('browser_import_selected', {
        ids: importIds,
        gameId,
      });
    } catch (err) {
      console.error('Bulk import failed:', err);
    }
    setIsGamePickerOpen(false);
    setImportIds([]);
  };

  return (
    <div className="flex flex-col h-full relative overflow-hidden bg-base-100">
      {/* ── Tab Bar ────────────────────────────────────────────────────── */}
      <div className="flex items-end gap-1 px-2 pt-2 bg-base-300 border-b border-base-200 shrink-0 h-12 overflow-x-auto">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`
              group flex items-center gap-2 max-w-48 px-3 py-1.5 rounded-t-lg border border-b-0 text-sm truncate transition-colors
              ${
                activeTabId === tab.id
                  ? 'bg-base-100 border-base-200 text-base-content font-medium opacity-100 relative'
                  : 'bg-base-200/50 border-transparent text-base-content/60 hover:bg-base-200 opacity-80'
              }
            `}
            style={{
              // Cover the bottom border line of the container when active
              marginBottom: activeTabId === tab.id ? '-1px' : '0',
              zIndex: activeTabId === tab.id ? 10 : 1,
            }}
          >
            <span className="truncate flex-1">
              {tab.title && tab.title !== 'Loading...'
                ? tab.title
                : tab.url
                  ? new URL(tab.url).hostname
                  : 'New Tab'}
            </span>
            <div
              className="w-5 h-5 rounded-md hover:bg-base-300 grid place-items-center opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={(e) => handleCloseTab(tab.id, e)}
            >
              <X size={12} />
            </div>
          </button>
        ))}
        <button
          onClick={() => handleNavigate('https://www.google.com', true)}
          className="btn btn-sm btn-ghost btn-square rounded-full mb-1 ml-1"
          title="New Tab"
        >
          <Plus size={16} />
        </button>
      </div>

      {/* ── Top Bar ───────────────────────────────────────────────────── */}
      <div className="flex items-center gap-3 px-4 py-2 bg-base-100 border-b border-base-200 shrink-0 z-10 relative shadow-sm">
        {/* Navigation buttons */}
        <div className="flex items-center gap-1">
          <button
            className="btn btn-ghost btn-xs btn-square"
            title="Refresh"
            onClick={handleReload}
            disabled={!activeTabId || isRefreshing}
          >
            <RotateCcw size={14} className={isRefreshing ? 'animate-spin' : ''} />
          </button>
        </div>

        {/* Discover Hub quick link */}
        <button
          id="browser-gamebanana-btn"
          className="btn btn-ghost btn-sm gap-2"
          title="Open GameBanana"
          onClick={() => handleNavigate('https://gamebanana.com', true)}
        >
          <Globe size={16} className="text-cyan-500" />
          Discover
        </button>

        {/* URL bar */}
        <form onSubmit={handleUrlSubmit} className="flex-1 flex gap-2">
          <input
            id="browser-url-input"
            type="text"
            className="input input-sm input-bordered flex-1 font-mono text-sm bg-base-200 focus:bg-base-100 transition-colors"
            placeholder="Enter URL to open..."
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
          />
          <button
            id="browser-navigate-btn"
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={isNavigating}
          >
            {isNavigating ? <span className="loading loading-spinner loading-xs" /> : 'Go'}
          </button>
        </form>

        {/* Action Buttons */}
        <div className="flex items-center gap-2">
          <button
            className="btn btn-sm btn-ghost text-error"
            onClick={handleClearData}
            title="Clear cookies & cache"
            disabled={!activeTabId}
          >
            <Trash2 size={18} />
          </button>

          <button
            id="browser-downloads-btn"
            className="btn btn-sm btn-ghost relative"
            onClick={openDownloadPanel}
            title="Open Downloads panel"
          >
            <Download size={18} />
            {finishedCount > 0 && (
              <span className="badge badge-primary badge-xs absolute -top-1 -right-1">
                {finishedCount}
              </span>
            )}
          </button>
        </div>
      </div>

      {/* ── Main Content / Webview Container ──────────────────────────── */}
      {/* This div acts as the reference for where the native Webview will be placed. */}
      {/* It must span the remaining height. */}
      <div ref={containerRef} className="flex-1 w-full bg-base-100 relative">
        {/* Placeholder UI shown when the container is empty or webview is loading */}
        {tabs.length === 0 && (
          <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
            <div className="flex flex-col items-center gap-6 text-center p-8 max-w-md">
              <Globe size={64} className="text-base-300" />
              <div>
                <h2 className="text-xl font-bold text-base-content">In-App Browser</h2>
                <p className="text-sm text-base-content/60 mt-2">
                  Browse and download mods directly. Downloads are intercepted automatically and
                  queued for Smart Import.
                </p>
              </div>
              <div className="flex flex-wrap gap-2 justify-center pointer-events-auto">
                <button
                  className="btn btn-primary btn-sm gap-2"
                  onClick={() => handleNavigate('https://gamebanana.com', true)}
                >
                  Browse GameBanana
                </button>
                <button
                  className="btn btn-ghost btn-sm"
                  onClick={() => handleNavigate('https://www.google.com', true)}
                >
                  Google
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* ── Download Manager Panel (slide-in) ─────────────────────────── */}
      {/* Overlay backdrop */}
      {isDownloadPanelOpen && (
        <div
          className="fixed inset-0 bg-black/30 z-60"
          onClick={() => useBrowserStore.getState().closeDownloadPanel()}
        />
      )}
      <div className="z-70 relative">
        <DownloadManagerPanel onImportSelected={handleImportSelected} />
      </div>

      {/* ── Import Queue (floating bottom-left) ───────────────────────── */}
      <div className="z-70 relative">
        <ImportQueuePanel />
      </div>

      {/* ── Game Picker Modal ─────────────────────────────────────────── */}
      <div className="z-80 relative">
        <GamePickerModal
          downloadIds={importIds}
          open={isGamePickerOpen}
          onClose={() => setIsGamePickerOpen(false)}
          onConfirm={handleGameConfirm}
        />
      </div>
    </div>
  );
}
