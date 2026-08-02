import { useShallow } from 'zustand/react/shallow';
import { useState, useCallback, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { Webview } from '@tauri-apps/api/webview';
import { useBrowserStore } from '../../../stores/useBrowserStore';
import { useDownloads } from '../hooks/useDownloads';
import { useWebviewSync } from '../hooks/useWebviewSync';
import { normalizeBrowserUrl } from '../utils/browserUrl';
import { BrowserTabBar } from './BrowserTabBar';
import { BrowserToolbar } from './BrowserToolbar';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import { GamePickerModal } from './GamePickerModal';
import { ImportQueuePanel } from './ImportQueuePanel';
import { Globe } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';

export function BrowserPage() {
  const { t } = useTranslation(['browser']);
  const [urlInput, setUrlInput] = useState('');
  const [isNavigating, setIsNavigating] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Container that the Webview will be placed over
  const containerRef = useRef<HTMLDivElement>(null);

  // Import selection state
  const [importIds, setImportIds] = useState<string[]>([]);
  const [isGamePickerOpen, setIsGamePickerOpen] = useState(false);

  // Selector-scoped: the browser store also holds persisted settings, so a
  // bare call re-renders this page whenever any of those change.
  const {
    openDownloadPanel,
    isDownloadPanelOpen,
    tabs,
    activeTabId,
    addTab,
    removeTab,
    setActiveTab,
  } = useBrowserStore(
    useShallow((state) => ({
      openDownloadPanel: state.openDownloadPanel,
      isDownloadPanelOpen: state.isDownloadPanelOpen,
      tabs: state.tabs,
      activeTabId: state.activeTabId,
      addTab: state.addTab,
      removeTab: state.removeTab,
      setActiveTab: state.setActiveTab,
    })),
  );

  // Native webviews always paint above the DOM, so any overlay that must sit
  // on top of the page content requires hiding them while it's open.
  const overlayOpen = isDownloadPanelOpen || isGamePickerOpen;

  const { finishedCount } = useDownloads();

  const activeTab = tabs.find((t) => t.id === activeTabId);

  // Navigate in a new tab window
  const handleNavigate = useCallback(
    async (url: string, asNewTab: boolean = false) => {
      const normalized = normalizeBrowserUrl(url);
      setIsNavigating(true);
      try {
        if (asNewTab || tabs.length === 0) {
          const label = await commands.browserOpenTab(normalized, null);

          addTab({
            id: label,
            title: t('tabs.loading'),
            url: normalized,
          });
        } else if (activeTabId) {
          await commands.browserNavigate(activeTabId, normalized);
          useBrowserStore.getState().updateTab(activeTabId, { url: normalized });
        }
      } catch (err) {
        console.error('Failed to navigate browser:', err);
      } finally {
        setIsNavigating(false);
      }
    },
    [tabs.length, activeTabId, addTab, t],
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
  useWebviewSync(containerRef, tabs, activeTabId, overlayOpen);

  // Keep a ref to handleNavigate to avoid listener re-creation loops
  const navigateRef = useRef(handleNavigate);
  useEffect(() => {
    navigateRef.current = handleNavigate;
  }, [handleNavigate]);

  // Listen for navigation changes from the backend (run ONCE on mount)
  useEffect(() => {
    const unlistenUrlPromise = listen<{ label: string; url: string; title: string }>(
      'browser:url-changed',
      (event) => {
        const { label, url, title } = event.payload;
        useBrowserStore.getState().updateTab(label, { url, title: title || url });
      },
    );

    const unlistenNewTabPromise = listen<{ url: string }>('browser:new-tab-requested', (event) => {
      if (navigateRef.current) {
        navigateRef.current(event.payload.url, true);
      }
    });

    return () => {
      unlistenUrlPromise.then((f) => f());
      unlistenNewTabPromise.then((f) => f());
    };
  }, []);

  const handleReload = async () => {
    if (!activeTabId) return;
    setIsRefreshing(true);
    try {
      await commands.browserReloadTab(activeTabId);
    } catch (err) {
      console.error('Failed to reload:', err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleGoBack = async () => {
    if (!activeTabId) return;
    try {
      await commands.browserGoBack(activeTabId);
    } catch (err) {
      console.error('Failed to go back:', err);
    }
  };

  const handleGoForward = async () => {
    if (!activeTabId) return;
    try {
      await commands.browserGoForward(activeTabId);
    } catch (err) {
      console.error('Failed to go forward:', err);
    }
  };

  const handleNewTab = async () => {
    let homepage = 'https://www.google.com';
    try {
      homepage = await commands.browserGetHomepage();
    } catch (err) {
      console.error('Failed to load homepage setting:', err);
    }
    handleNavigate(homepage, true);
  };

  const handleClearData = async () => {
    if (!activeTabId) return;
    const confirmed = await window.confirm(t('tabs.clear_data_confirm'));
    if (!confirmed) return;

    try {
      await commands.browserClearData(activeTabId);
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
      await commands.browserImportSelected(importIds, gameId);
    } catch (err) {
      console.error('Bulk import failed:', err);
    }
    setIsGamePickerOpen(false);
    setImportIds([]);
  };

  return (
    <div className="flex flex-col h-full relative overflow-hidden bg-base-100">
      <BrowserTabBar
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={setActiveTab}
        onCloseTab={handleCloseTab}
        onNewTab={handleNewTab}
      />

      <BrowserToolbar
        urlInput={urlInput}
        onUrlInputChange={setUrlInput}
        onUrlSubmit={handleUrlSubmit}
        activeTabId={activeTabId}
        isNavigating={isNavigating}
        isRefreshing={isRefreshing}
        finishedCount={finishedCount}
        onGoBack={handleGoBack}
        onGoForward={handleGoForward}
        onReload={handleReload}
        onOpenDiscover={() => handleNavigate('https://gamebanana.com', true)}
        onClearData={handleClearData}
        onOpenDownloads={openDownloadPanel}
      />

      {/* ── Main Content / Webview Container ──────────────────────────── */}
      {/* This div acts as the reference for where the native Webview will be placed. */}
      {/* It must span the remaining height. */}
      <div ref={containerRef} className="flex-1 w-full bg-base-100 relative">
        {/* Placeholder UI shown when the container is empty or webview is loading */}
        {tabs.length === 0 && (
          <div className="absolute inset-0 z-50 flex items-center justify-center bg-overlay-mask backdrop-blur-sm pointer-events-none">
            <div className="flex flex-col items-center gap-6 text-center p-8 max-w-md">
              <Globe size={64} className="text-base-300" />
              <div>
                <h2 className="text-xl font-bold text-base-content">{t('welcome.title')}</h2>
                <p className="text-sm text-base-content/60 mt-2">{t('welcome.description')}</p>
              </div>
              <div className="flex flex-wrap gap-2 justify-center pointer-events-auto">
                <button
                  className="btn btn-primary btn-sm gap-2"
                  onClick={() => handleNavigate('https://gamebanana.com', true)}
                >
                  {t('welcome.browse_gb')}
                </button>
                <button
                  className="btn btn-ghost btn-sm"
                  onClick={() => handleNavigate('https://www.google.com', true)}
                >
                  {t('welcome.google')}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* ── Overlays (Rendered via Portal to avoid clipping) ──────────────────────── */}
      {createPortal(
        <>
          {/* Download Manager Panel Backdrop (slide-in) */}
          <div
            className={`fixed inset-0 bg-overlay-mask backdrop-blur-sm z-9998 transition-opacity duration-300 ${
              isDownloadPanelOpen
                ? 'opacity-100 pointer-events-auto'
                : 'opacity-0 pointer-events-none'
            }`}
            onClick={() => useBrowserStore.getState().closeDownloadPanel()}
          />

          {/* Download Manager Panel */}
          <div className="relative z-9999">
            <DownloadManagerPanel onImportSelected={handleImportSelected} />
          </div>

          {/* Import Queue (floating bottom-left) */}
          <div className="relative z-10000">
            <ImportQueuePanel />
          </div>

          {/* Game Picker Modal */}
          <div className="relative z-10010">
            <GamePickerModal
              downloadIds={importIds}
              open={isGamePickerOpen}
              onClose={() => setIsGamePickerOpen(false)}
              onConfirm={handleGameConfirm}
            />
          </div>
        </>,
        document.body,
      )}
    </div>
  );
}
