import { useShallow } from 'zustand/react/shallow';
import { useState, useCallback, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { Webview } from '@tauri-apps/api/webview';
import { useBrowserStore } from '@/entities/browser';
import { useDownloads } from '../hooks/useDownloads';
import { useWebviewSync } from '../hooks/useWebviewSync';
import { normalizeBrowserUrl } from '../utils/browserUrl';
import { BrowserTabBar } from './BrowserTabBar';
import { BrowserToolbar } from './BrowserToolbar';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import { BrowserLibraryPanel } from './BrowserLibraryPanel';
import { AlertTriangle, Download, Globe, MoreVertical } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  commands,
  type BrowserBookmark,
  type BrowserHistoryEntry,
  type BrowserPrivacySummary,
} from '@/shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import ConfirmDialog from '@/shared/ui/components/ui/ConfirmDialog';
import { isDemoMode } from '@/shared/lib/appMode';
import { useAppStore } from '@/app/store';
import { LiquidSurface } from '@/shared/ui/liquid';
import { TopBarActionsPortal } from '@/widgets/top-bar';

type ConfirmationRequest = {
  message: string;
  onConfirm: () => void | Promise<void>;
};

function navigationWarningKey(
  url: string,
): 'tabs.http_warning' | 'tabs.suspicious_url_warning' | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol === 'http:') return 'tabs.http_warning';
    if (
      parsed.username ||
      parsed.password ||
      parsed.hostname.includes('xn--') ||
      /^\d{1,3}(\.\d{1,3}){3}$/.test(parsed.hostname)
    ) {
      return 'tabs.suspicious_url_warning';
    }
  } catch {
    return 'tabs.suspicious_url_warning';
  }
  return null;
}

export function BrowserPage() {
  const { t } = useTranslation(['browser', 'layout']);
  const [urlInput, setUrlInput] = useState('');
  const [isNavigating, setIsNavigating] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [adblockEnabled, setAdblockEnabled] = useState(true);
  const [isRestoringSession, setIsRestoringSession] = useState(true);
  const [isLibraryOpen, setIsLibraryOpen] = useState(false);
  const [bookmarks, setBookmarks] = useState<BrowserBookmark[]>([]);
  const [history, setHistory] = useState<BrowserHistoryEntry[]>([]);
  const [privacySummary, setPrivacySummary] = useState<BrowserPrivacySummary | null>(null);
  const [navigationError, setNavigationError] = useState<{
    label: string;
    url: string;
    status: number;
  } | null>(null);
  const [isFindOpen, setIsFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [confirmation, setConfirmation] = useState<ConfirmationRequest | null>(null);
  const [isTopBarMenuOpen, setIsTopBarMenuOpen] = useState(false);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const sessionRestoreRequest = useRef(0);

  // Container that the Webview will be placed over
  const containerRef = useRef<HTMLDivElement>(null);

  // Select only the browser state used by this page.
  const {
    openDownloadPanel,
    isDownloadPanelOpen,
    isDownloadConfirmationOpen,
    tabs,
    activeTabId,
    addTab,
    removeTab,
    setActiveTab,
    setGameContext,
  } = useBrowserStore(
    useShallow((state) => ({
      openDownloadPanel: state.openDownloadPanel,
      isDownloadPanelOpen: state.isDownloadPanelOpen,
      isDownloadConfirmationOpen: state.isDownloadConfirmationOpen,
      tabs: state.tabs,
      activeTabId: state.activeTabId,
      addTab: state.addTab,
      removeTab: state.removeTab,
      setActiveTab: state.setActiveTab,
      setGameContext: state.setGameContext,
    })),
  );

  // Native webviews always paint above the DOM, so any overlay that must sit
  // on top of the page content requires hiding them while it's open.
  const overlayOpen =
    isDownloadPanelOpen ||
    isDownloadConfirmationOpen ||
    isLibraryOpen ||
    isFindOpen ||
    navigationError?.label === activeTabId;

  const activeGameId = useAppStore((state) => state.activeGameId);

  const { activeCount, queuedCount } = useDownloads(activeGameId, {
    showFeedback: true,
    onOpenDownloads: openDownloadPanel,
  });

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const activeBookmark = activeTab
    ? bookmarks.find((bookmark) => bookmark.url === activeTab.url)
    : undefined;

  const requestConfirmation = useCallback(
    (message: string, onConfirm: ConfirmationRequest['onConfirm']) => {
      setConfirmation({ message, onConfirm });
    },
    [],
  );

  const loadLibrary = useCallback(async () => {
    try {
      const [nextBookmarks, nextHistory, nextPrivacySummary] = await Promise.all([
        commands.browserListBookmarks(),
        commands.browserListHistory(100),
        commands.browserGetPrivacySummary(),
      ]);
      setBookmarks(nextBookmarks);
      setHistory(nextHistory);
      setPrivacySummary(nextPrivacySummary);
    } catch (error) {
      console.error('Failed to load Discover library:', error);
      toast.error(t('tabs.operation_failed'));
    }
  }, [t]);

  const performNavigate = useCallback(
    async (url: string, asNewTab: boolean = false) => {
      setNavigationError(null);
      setIsNavigating(true);
      try {
        if (asNewTab || tabs.length === 0) {
          const label = await commands.browserOpenTab(url, null);

          addTab({
            id: label,
            title: t('tabs.loading'),
            url,
          });
        } else if (activeTabId) {
          await commands.browserNavigate(activeTabId, url);
          useBrowserStore.getState().updateTab(activeTabId, { url });
        }
      } catch (err) {
        console.error('Failed to navigate browser:', err);
        toast.error(t('tabs.operation_failed'));
      } finally {
        setIsNavigating(false);
      }
    },
    [tabs.length, activeTabId, addTab, t],
  );

  const handleNavigate = useCallback(
    async (url: string, asNewTab: boolean = false) => {
      const normalized = normalizeBrowserUrl(url);
      const warning = navigationWarningKey(normalized);
      if (warning) {
        requestConfirmation(t(warning), () => performNavigate(normalized, asNewTab));
        return;
      }
      await performNavigate(normalized, asNewTab);
    },
    [performNavigate, requestConfirmation, t],
  );

  // Synchronize URL input with active tab
  useEffect(() => {
    if (activeTab) {
      setUrlInput(activeTab.url);
    } else {
      setUrlInput('');
    }
  }, [activeTab]);

  useEffect(() => {
    commands
      .browserGetAdblockEnabled()
      .then(setAdblockEnabled)
      .catch((error) => console.error('Failed to load Discover ad-block setting:', error));
  }, []);

  useEffect(() => {
    void loadLibrary();
  }, [loadLibrary]);

  useEffect(() => {
    if (!activeGameId) {
      setGameContext(null);
      setIsRestoringSession(false);
      return;
    }

    const currentBrowserState = useBrowserStore.getState();
    if (currentBrowserState.gameId === activeGameId) {
      setIsRestoringSession(false);
      return;
    }

    if (currentBrowserState.gameId && currentBrowserState.gameId !== activeGameId) {
      void commands.browserSaveSessionTabs(
        currentBrowserState.gameId,
        currentBrowserState.tabs.slice(-12).map((tab, position) => ({
          position,
          url: tab.url,
          title: tab.title,
          active: tab.id === currentBrowserState.activeTabId,
        })),
      );
      void Promise.all(
        currentBrowserState.tabs.map(async (tab) => {
          const webview = await Webview.getByLabel(tab.id);
          await webview?.close();
        }),
      ).catch(() => undefined);
    }
    setGameContext(activeGameId);
    setIsRestoringSession(true);
    const requestId = ++sessionRestoreRequest.current;
    let cancelled = false;
    commands
      .browserGetSessionTabs(activeGameId)
      .then(async (savedTabs) => {
        for (const savedTab of savedTabs) {
          if (cancelled) return;
          const label = await commands.browserOpenTab(savedTab.url, null);
          if (cancelled) return;
          addTab({ id: label, url: savedTab.url, title: savedTab.title || t('tabs.loading') });
          if (savedTab.active) setActiveTab(label);
        }
      })
      .catch((error) => console.error('Failed to restore Discover tabs:', error))
      .finally(() => {
        if (!cancelled && requestId === sessionRestoreRequest.current) {
          setIsRestoringSession(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeGameId, addTab, setActiveTab, setGameContext, t]);

  useEffect(() => {
    if (isRestoringSession || !activeGameId) return;
    const timer = window.setTimeout(() => {
      void commands
        .browserSaveSessionTabs(
          activeGameId,
          tabs.slice(-12).map((tab, position) => ({
            position,
            url: tab.url,
            title: tab.title,
            active: tab.id === activeTabId,
          })),
        )
        .catch((error) => {
          console.error('Failed to save Discover session:', error);
          toast.warning(t('tabs.session_save_failed'));
        });
    }, 400);
    return () => window.clearTimeout(timer);
  }, [activeGameId, activeTabId, isRestoringSession, t, tabs]);

  // Handle resizing and positioning of the Tauri Webviews
  useWebviewSync(containerRef, tabs, activeTabId, overlayOpen);

  // Listen for navigation changes from the backend (run ONCE on mount)
  useEffect(() => {
    if (isDemoMode) {
      return;
    }

    const unlistenUrlPromise = listen<{ label: string; url: string; title?: string }>(
      'browser:url-changed',
      (event) => {
        const { label, url, title } = event.payload;
        const updates: { url: string; title?: string } = { url };
        if (title?.trim()) {
          updates.title = title;
        }
        useBrowserStore.getState().updateTab(label, updates);
        setNavigationError((current) => (current?.label === label ? null : current));
      },
    );

    const unlistenFaviconPromise = listen<{ label: string; favicon: string }>(
      'browser:favicon-changed',
      (event) => {
        useBrowserStore
          .getState()
          .updateTab(event.payload.label, { favicon: event.payload.favicon });
      },
    );

    const unlistenLoadingPromise = listen<{ label: string; loading: boolean }>(
      'browser:loading-changed',
      (event) =>
        useBrowserStore
          .getState()
          .updateTab(event.payload.label, { isLoading: event.payload.loading }),
    );

    const unlistenAdblockUpdateFailure = listen('browser:adblock-update-failed', () => {
      toast.warning(t('tabs.adblock_update_failed'));
    });

    const unlistenNavigationError = listen<{ label: string; url: string; status: number }>(
      'browser:navigation-error',
      (event) => {
        const tab = useBrowserStore
          .getState()
          .tabs.find((candidate) => candidate.id === event.payload.label);
        if (tab?.url === event.payload.url) setNavigationError(event.payload);
      },
    );

    return () => {
      unlistenUrlPromise.then((f) => f());
      unlistenFaviconPromise.then((f) => f());
      unlistenLoadingPromise.then((f) => f());
      unlistenAdblockUpdateFailure.then((f) => f());
      unlistenNavigationError.then((f) => f());
    };
  }, [t]);

  const handleReload = useCallback(async () => {
    if (!activeTabId) return;
    setNavigationError(null);
    setIsRefreshing(true);
    try {
      await commands.browserReloadTab(activeTabId);
    } catch (err) {
      console.error('Failed to reload:', err);
      toast.error(t('tabs.operation_failed'));
    } finally {
      setIsRefreshing(false);
    }
  }, [activeTabId, t]);

  const handleGoBack = async () => {
    if (!activeTabId) return;
    try {
      await commands.browserGoBack(activeTabId);
    } catch (err) {
      console.error('Failed to go back:', err);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleGoForward = async () => {
    if (!activeTabId) return;
    try {
      await commands.browserGoForward(activeTabId);
    } catch (err) {
      console.error('Failed to go forward:', err);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleNewTab = useCallback(async () => {
    let homepage = 'https://www.google.com';
    try {
      if (!activeGameId) return;
      homepage = await commands.browserGetHomepage(activeGameId);
    } catch (err) {
      console.error('Failed to load homepage setting:', err);
      toast.error(t('tabs.operation_failed'));
    }
    await handleNavigate(homepage, true);
  }, [activeGameId, handleNavigate, t]);

  const handleToggleAdblock = async () => {
    const nextEnabled = !adblockEnabled;
    try {
      await commands.browserSetAdblockEnabled(nextEnabled);
      setAdblockEnabled(nextEnabled);
      await handleReload();
      toast.success(t(nextEnabled ? 'tabs.adblock_enabled' : 'tabs.adblock_disabled'));
    } catch (error) {
      console.error('Failed to update Discover ad-block setting:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleClearCookiesAndSiteData = async () => {
    if (!activeTabId) return;
    requestConfirmation(t('tabs.clear_cookies_and_site_data_confirm'), async () => {
      try {
        await commands.browserClearCookiesAndSiteData(activeTabId);
        toast.success(t('tabs.cookies_and_site_data_cleared'));
      } catch (err) {
        console.error('Failed to clear Discover cookies and site data:', err);
        toast.error(t('tabs.operation_failed'));
      }
    });
  };

  const handleClearCache = async () => {
    if (!activeTabId) return;
    requestConfirmation(t('tabs.clear_cache_confirm'), async () => {
      try {
        await commands.browserClearCache(activeTabId);
        toast.success(t('tabs.cache_cleared'));
      } catch (err) {
        console.error('Failed to clear Discover cache:', err);
        toast.error(t('tabs.operation_failed'));
      }
    });
  };

  const handleToggleBookmark = async () => {
    if (!activeTab) return;
    try {
      if (activeBookmark) {
        await commands.browserDeleteBookmark(activeBookmark.id);
      } else {
        await commands.browserAddBookmark(
          activeTab.url,
          activeTab.title,
          activeTab.favicon ?? null,
        );
      }
      await loadLibrary();
    } catch (error) {
      console.error('Failed to update Discover bookmark:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleDeleteBookmark = async (id: string) => {
    try {
      await commands.browserDeleteBookmark(id);
      await loadLibrary();
    } catch (error) {
      console.error('Failed to delete Discover bookmark:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleClearHistory = async () => {
    requestConfirmation(t('library.clear_history_confirm'), async () => {
      try {
        await commands.browserClearHistory();
        await loadLibrary();
        toast.success(t('library.history_cleared'));
      } catch (error) {
        console.error('Failed to clear Discover history:', error);
        toast.error(t('tabs.operation_failed'));
      }
    });
  };

  const handleOpenExternally = async () => {
    if (!activeTab) return;
    try {
      await commands.browserOpenExternally(activeTab.url);
    } catch (error) {
      console.error('Failed to open external browser:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleChangeZoom = async (zoom: number) => {
    if (!activeTabId) return;
    try {
      await commands.browserSetZoom(activeTabId, zoom);
      useBrowserStore.getState().updateTab(activeTabId, { zoom });
    } catch (error) {
      console.error('Failed to change Discover zoom:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleFind = async () => {
    if (!activeTabId || !findQuery.trim()) return;
    try {
      await commands.browserFindInPage(activeTabId, findQuery.trim());
      setIsFindOpen(false);
    } catch (error) {
      console.error('Failed to find text in Discover tab:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleCloseTab = useCallback(
    async (id: string) => {
      try {
        const webview = await Webview.getByLabel(id);
        if (webview) await webview.close();
      } catch {
        // A tab can already be closed by the native WebView lifecycle.
      }
      removeTab(id);
    },
    [removeTab],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key.toLowerCase() === 'f' && activeTabId) {
        event.preventDefault();
        setIsFindOpen(true);
      }
      if (event.key.toLowerCase() === 'l') {
        event.preventDefault();
        document.getElementById('browser-url-input')?.focus();
      }
      if (event.key.toLowerCase() === 't') {
        event.preventDefault();
        void handleNewTab();
      }
      if (event.key.toLowerCase() === 'r' && activeTabId) {
        event.preventDefault();
        void handleReload();
      }
      if (event.key.toLowerCase() === 'w' && activeTabId) {
        event.preventDefault();
        void handleCloseTab(activeTabId);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [activeTabId, handleCloseTab, handleNewTab, handleReload]);

  const handleUrlSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const url = urlInput.trim();
    if (url) {
      // Logic fix: if we have an active tab, navigate it.
      // If we have no tabs, open a new one.
      handleNavigate(url, tabs.length === 0);
    }
  };

  return (
    <div className="flex flex-col h-full relative overflow-hidden bg-base-100/85">
      <TopBarActionsPortal>
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square"
          onClick={openDownloadPanel}
          title={t('tabs.open_downloads')}
          aria-label={t('tabs.open_downloads')}
        >
          <Download size={18} />
        </button>
        <div className="relative">
          <button
            type="button"
            className="btn btn-ghost btn-sm btn-square"
            onClick={() => setIsTopBarMenuOpen((open) => !open)}
            title={t('tabs.browser_menu')}
            aria-label={t('tabs.browser_menu')}
          >
            <MoreVertical size={18} />
          </button>
          {isTopBarMenuOpen && (
            <LiquidSurface
              liquidRole="overlay"
              className="absolute right-0 top-full z-[var(--workspace-layer-popover)] mt-2 w-48 rounded-xl shadow-lg"
              contentClassName="p-2"
            >
              <button
                type="button"
                className="btn btn-ghost btn-sm w-full justify-start"
                onClick={() => setWorkspaceView('downloads')}
              >
                <Download size={16} />
                {t('downloads.title')}
              </button>
              <button
                type="button"
                className="btn btn-ghost btn-sm w-full justify-start"
                onClick={() => setWorkspaceView('settings')}
              >
                <Globe size={16} />
                {t('layout:nav.settings')}
              </button>
            </LiquidSurface>
          )}
        </div>
      </TopBarActionsPortal>
      <BrowserTabBar
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={setActiveTab}
        onCloseTab={(id, event) => {
          event.stopPropagation();
          void handleCloseTab(id);
        }}
        onNewTab={handleNewTab}
      />

      <BrowserToolbar
        urlInput={urlInput}
        onUrlInputChange={setUrlInput}
        onUrlSubmit={handleUrlSubmit}
        activeTabId={activeTabId}
        activeTabUrl={activeTab?.url ?? null}
        isBookmarked={Boolean(activeBookmark)}
        activeZoom={activeTab?.zoom ?? 1}
        isNavigating={isNavigating || Boolean(activeTab?.isLoading)}
        isRefreshing={isRefreshing}
        activeDownloadCount={activeCount}
        queuedDownloadCount={queuedCount}
        onGoBack={handleGoBack}
        onGoForward={handleGoForward}
        onReload={handleReload}
        onOpenDiscover={() => handleNavigate('https://gamebanana.com', true)}
        onToggleBookmark={handleToggleBookmark}
        onOpenLibrary={() => setIsLibraryOpen(true)}
        onOpenExternally={handleOpenExternally}
        onChangeZoom={handleChangeZoom}
        onOpenFind={() => setIsFindOpen(true)}
        adblockEnabled={adblockEnabled}
        onToggleAdblock={handleToggleAdblock}
        onClearCookiesAndSiteData={handleClearCookiesAndSiteData}
        onClearCache={handleClearCache}
        onOpenDownloads={openDownloadPanel}
      />

      {isFindOpen && (
        <form
          className="absolute right-4 top-24 z-20 flex gap-2 rounded-box border border-base-200 bg-base-100 p-2 shadow-lg"
          onSubmit={(event) => {
            event.preventDefault();
            void handleFind();
          }}
        >
          <input
            autoFocus
            className="input input-sm input-bordered w-56"
            placeholder={t('tabs.find_in_page')}
            value={findQuery}
            onChange={(event) => setFindQuery(event.target.value)}
          />
          <button className="btn btn-primary btn-sm" type="submit">
            {t('tabs.find')}
          </button>
          <button
            className="btn btn-ghost btn-sm"
            type="button"
            onClick={() => setIsFindOpen(false)}
          >
            {t('tabs.close')}
          </button>
        </form>
      )}

      {/* ── Main Content / Webview Container ──────────────────────────── */}
      {/* This div acts as the reference for where the native Webview will be placed. */}
      {/* It must span the remaining height. */}
      <div ref={containerRef} className="flex-1 w-full bg-base-100 relative">
        {navigationError?.label === activeTabId && (
          <div className="absolute inset-0 z-50 flex items-center justify-center bg-base-100 p-8">
            <div className="max-w-md text-center">
              <AlertTriangle className="mx-auto mb-4 text-warning" size={42} />
              <h2 className="text-lg font-semibold">{t('error.title')}</h2>
              <p className="mt-2 text-sm text-base-content/60">{t('error.description')}</p>
              <p className="mt-2 truncate font-mono text-xs text-base-content/50">
                {navigationError.url}
              </p>
              <div className="mt-5 flex justify-center gap-2">
                <button className="btn btn-primary btn-sm" onClick={handleReload}>
                  {t('error.retry')}
                </button>
                <button className="btn btn-ghost btn-sm" onClick={handleOpenExternally}>
                  {t('error.open_externally')}
                </button>
              </div>
            </div>
          </div>
        )}
        {/* Placeholder UI shown when the container is empty or webview is loading */}
        {tabs.length === 0 && (
          <div className="absolute inset-0 z-50 grid place-items-center bg-base-100 p-6 pointer-events-none">
            <div className="flex max-w-xs flex-col items-center gap-4 text-center">
              <Globe size={40} className="text-base-content/25" />
              <div>
                <h2 className="text-base font-semibold text-base-content">{t('welcome.title')}</h2>
                <p className="mt-1 text-sm text-base-content/60">{t('welcome.description')}</p>
              </div>
              <div className="flex flex-wrap justify-center gap-2 pointer-events-auto">
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
            className={`fixed inset-0 z-[var(--workspace-layer-overlay)] bg-overlay-mask backdrop-blur-sm transition-opacity duration-300 ${
              isDownloadPanelOpen
                ? 'opacity-100 pointer-events-auto'
                : 'opacity-0 pointer-events-none'
            }`}
            onClick={() => useBrowserStore.getState().closeDownloadPanel()}
          />

          {/* Download Manager Panel */}
          <div className="relative z-[calc(var(--workspace-layer-overlay)+1)]">
            <DownloadManagerPanel />
          </div>

          {isLibraryOpen && (
            <BrowserLibraryPanel
              bookmarks={bookmarks}
              history={history}
              privacy={privacySummary}
              onClose={() => setIsLibraryOpen(false)}
              onNavigate={(url) => {
                setIsLibraryOpen(false);
                void handleNavigate(url);
              }}
              onDeleteBookmark={handleDeleteBookmark}
              onClearHistory={handleClearHistory}
            />
          )}
        </>,
        document.body,
      )}
      <ConfirmDialog
        open={confirmation !== null}
        title={t('tabs.confirm_action')}
        message={confirmation?.message ?? ''}
        danger
        onCancel={() => setConfirmation(null)}
        onConfirm={() => {
          const action = confirmation?.onConfirm;
          setConfirmation(null);
          void action?.();
        }}
      />
    </div>
  );
}
