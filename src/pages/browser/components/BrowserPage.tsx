import { useShallow } from 'zustand/react/shallow';
import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { Webview } from '@tauri-apps/api/webview';
import { createNewBrowserTab, useBrowserStore, type BrowserTab } from '@/entities/browser';
import { useDownloads } from '../hooks/useDownloads';
import { useBrowserLibrary } from '../hooks/useBrowserLibrary';
import { useWebviewSync } from '../hooks/useWebviewSync';
import { normalizeBrowserUrl } from '../utils/browserUrl';
import { BrowserTabBar, BrowserTabContextMenu } from './BrowserTabBar';
import { BrowserToolbar, BrowserToolbarMenu } from './BrowserToolbar';
import { DownloadManagerPanel } from './DownloadManagerPanel';
import { BrowserLibraryPanel } from './BrowserLibraryPanel';
import { BookmarkEditorDialog } from './BookmarkEditorDialog';
import { BrowserDecodedTextDialog } from './BrowserDecodedTextDialog';
import { BrowserImagePreviewDialog } from './BrowserImagePreviewDialog';
import { BrowserNewTabPage } from './BrowserNewTabPage';
import { AlertTriangle, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { commands, type BrowserBookmark } from '@/shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import ConfirmDialog from '@/shared/ui/components/ui/ConfirmDialog';
import { isDemoMode } from '@/shared/lib/appMode';
import { useAppStore } from '@/app/store';
import { TopBarActionsPortal } from '@/widgets/top-bar';
import {
  getBrowserSidePanelLayout,
  getBrowserSurfacePresentation,
  type BrowserSidePanel,
} from '../browserSurfacePresentation';
import { getChromeTrayAnchorOffset } from '../browserChromeTray';

type ConfirmationRequest = {
  message: string;
  onConfirm: () => void | Promise<void>;
};

type BrowserChromeTray =
  | { kind: 'none' }
  | { kind: 'find' }
  | { kind: 'toolbar-menu' }
  | { kind: 'tab-context-menu'; tab: BrowserTab; anchorLeftPx: number };

const MAX_DECODED_TEXT_CHARS = 750_000;

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
  const [navigationError, setNavigationError] = useState<{
    label: string;
    url: string;
    status: number;
  } | null>(null);
  const [findQuery, setFindQuery] = useState('');
  const [confirmation, setConfirmation] = useState<ConfirmationRequest | null>(null);
  const [chromeTray, setChromeTray] = useState<BrowserChromeTray>({ kind: 'none' });
  const [libraryTab, setLibraryTab] = useState<'bookmarks' | 'history'>('bookmarks');
  const [editingBookmark, setEditingBookmark] = useState<BrowserBookmark | null>(null);
  const [previewImageUrl, setPreviewImageUrl] = useState<string | null>(null);
  const [decodedText, setDecodedText] = useState<string | null>(null);
  const sessionRestoreRequest = useRef(0);

  // Container that the Webview will be placed over
  const browserPageRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const browserContentRef = useRef<HTMLDivElement>(null);
  const [browserContentWidth, setBrowserContentWidth] = useState(0);

  // Select only the browser state used by this page.
  const {
    openDownloadPanel,
    closeDownloadPanel,
    isDownloadPanelOpen,
    isDownloadConfirmationOpen,
    tabs,
    activeTabId,
    recentlyClosedTabs,
    addTab,
    replaceTab,
    removeTab,
    recordClosedTab,
    removeLastClosedTab,
    setActiveTab,
    setGameContext,
  } = useBrowserStore(
    useShallow((state) => ({
      openDownloadPanel: state.openDownloadPanel,
      closeDownloadPanel: state.closeDownloadPanel,
      isDownloadPanelOpen: state.isDownloadPanelOpen,
      isDownloadConfirmationOpen: state.isDownloadConfirmationOpen,
      tabs: state.tabs,
      activeTabId: state.activeTabId,
      recentlyClosedTabs: state.recentlyClosedTabs,
      addTab: state.addTab,
      replaceTab: state.replaceTab,
      removeTab: state.removeTab,
      recordClosedTab: state.recordClosedTab,
      removeLastClosedTab: state.removeLastClosedTab,
      setActiveTab: state.setActiveTab,
      setGameContext: state.setGameContext,
    })),
  );

  const activeSidePanel: BrowserSidePanel = isDownloadPanelOpen
    ? 'downloads'
    : isLibraryOpen
      ? 'library'
      : null;
  const sidePanelLayout = getBrowserSidePanelLayout(activeSidePanel, browserContentWidth);
  const hasBlockingOverlay =
    isDownloadConfirmationOpen ||
    confirmation !== null ||
    editingBookmark !== null ||
    previewImageUrl !== null ||
    decodedText !== null ||
    navigationError?.label === activeTabId;

  const activeGameId = useAppStore((state) => state.activeGameId);
  const isAppMenuOpen = useAppStore((state) => state.isAppMenuOpen);
  const browserSurfacePresentation = useMemo(
    () =>
      getBrowserSurfacePresentation({
        activeSidePanel,
        availableWidth: browserContentWidth,
        hasBlockingOverlay,
        isAppMenuOpen,
        isChromeTrayOpen: chromeTray.kind !== 'none',
      }),
    [activeSidePanel, browserContentWidth, chromeTray.kind, hasBlockingOverlay, isAppMenuOpen],
  );

  const { activeCount, queuedCount } = useDownloads(activeGameId, {
    showFeedback: true,
    onOpenDownloads: openDownloadPanel,
  });
  const pendingDownloadCount = activeCount + queuedCount;
  const {
    bookmarks,
    history,
    privacySummary,
    isLoading: isLibraryLoading,
    addBookmark,
    deleteBookmark,
    updateBookmark,
    clearHistory,
  } = useBrowserLibrary();

  useEffect(() => {
    if (isDownloadPanelOpen) {
      setIsLibraryOpen(false);
    }
  }, [isDownloadPanelOpen]);

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

  const performNavigate = useCallback(
    async (url: string, asNewTab: boolean = false) => {
      setChromeTray({ kind: 'none' });
      setNavigationError(null);
      setIsNavigating(true);
      try {
        const currentTab = tabs.find((tab) => tab.id === activeTabId);
        if (asNewTab || !currentTab || currentTab.isNewTab) {
          const label = await commands.browserOpenTab(url, null);
          const nextTab: BrowserTab = {
            id: label,
            title: t('tabs.loading'),
            url,
          };
          if (!asNewTab && currentTab?.isNewTab) {
            replaceTab(currentTab.id, nextTab);
          } else {
            addTab(nextTab);
          }
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
    [tabs, activeTabId, addTab, replaceTab, t],
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
    if (!activeGameId) {
      setGameContext(null);
      if (useBrowserStore.getState().tabs.length === 0) {
        addTab(createNewBrowserTab());
      }
      setIsRestoringSession(false);
      return;
    }

    const currentBrowserState = useBrowserStore.getState();
    if (currentBrowserState.gameId === activeGameId) {
      if (currentBrowserState.tabs.length === 0) {
        addTab(createNewBrowserTab());
      }
      setIsRestoringSession(false);
      return;
    }

    if (currentBrowserState.gameId && currentBrowserState.gameId !== activeGameId) {
      const sessionTabs = currentBrowserState.tabs.filter((tab) => !tab.isNewTab).slice(-12);
      if (sessionTabs.length > 0) {
        void commands.browserSaveSessionTabs(
          currentBrowserState.gameId,
          sessionTabs.map((tab, position) => ({
            position,
            url: tab.url,
            title: tab.title,
            active: tab.id === currentBrowserState.activeTabId,
          })),
        );
      }
      void Promise.all(
        currentBrowserState.tabs
          .filter((tab) => !tab.isNewTab)
          .map(async (tab) => {
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
        if (savedTabs.length === 0) {
          addTab(createNewBrowserTab());
          return;
        }
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
    const sessionTabs = tabs.filter((tab) => !tab.isNewTab).slice(-12);
    if (sessionTabs.length === 0) return;
    const timer = window.setTimeout(() => {
      void commands
        .browserSaveSessionTabs(
          activeGameId,
          sessionTabs.map((tab, position) => ({
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

  useEffect(() => {
    const content = browserContentRef.current;
    if (!content) return;

    const updateWidth = () => setBrowserContentWidth(content.getBoundingClientRect().width);
    updateWidth();

    const observer = new ResizeObserver(updateWidth);
    observer.observe(content);
    return () => observer.disconnect();
  }, []);

  useWebviewSync(containerRef, tabs, activeTabId, browserSurfacePresentation);

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

    const unlistenImagePreview = listen<{ label: string; url: string }>(
      'browser:preview-image',
      (event) => {
        if (event.payload.label !== activeTabId) return;
        try {
          const imageUrl = new URL(event.payload.url);
          if (imageUrl.protocol === 'http:' || imageUrl.protocol === 'https:') {
            setPreviewImageUrl(imageUrl.toString());
          }
        } catch {
          // Ignore malformed URLs from the untrusted browser surface.
        }
      },
    );

    const unlistenBase64Decoded = listen<{ label: string; text: string }>(
      'browser:base64-decoded',
      (event) => {
        if (
          event.payload.label !== activeTabId ||
          typeof event.payload.text !== 'string' ||
          event.payload.text.length > MAX_DECODED_TEXT_CHARS
        ) {
          return;
        }
        setDecodedText(event.payload.text);
      },
    );

    return () => {
      unlistenUrlPromise.then((f) => f());
      unlistenFaviconPromise.then((f) => f());
      unlistenLoadingPromise.then((f) => f());
      unlistenAdblockUpdateFailure.then((f) => f());
      unlistenNavigationError.then((f) => f());
      unlistenImagePreview.then((f) => f());
      unlistenBase64Decoded.then((f) => f());
    };
  }, [activeTabId, t]);

  const handleReloadTab = useCallback(
    async (id: string) => {
      const tab = tabs.find((candidate) => candidate.id === id);
      if (!tab || tab.isNewTab) return;
      setNavigationError(null);
      setIsRefreshing(true);
      try {
        await commands.browserReloadTab(id);
      } catch (err) {
        console.error('Failed to reload:', err);
        toast.error(t('tabs.operation_failed'));
      } finally {
        setIsRefreshing(false);
      }
    },
    [t, tabs],
  );

  const handleReload = useCallback(() => {
    if (activeTabId) void handleReloadTab(activeTabId);
  }, [activeTabId, handleReloadTab]);

  const handleGoBack = async () => {
    if (!activeTabId || activeTab?.isNewTab) return;
    try {
      await commands.browserGoBack(activeTabId);
    } catch (err) {
      console.error('Failed to go back:', err);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleGoForward = async () => {
    if (!activeTabId || activeTab?.isNewTab) return;
    try {
      await commands.browserGoForward(activeTabId);
    } catch (err) {
      console.error('Failed to go forward:', err);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleNewTab = useCallback(() => {
    setChromeTray({ kind: 'none' });
    addTab(createNewBrowserTab());
    setUrlInput('');
  }, [addTab]);

  const handleDuplicateTab = useCallback(
    async (id: string) => {
      const sourceTab = tabs.find((tab) => tab.id === id);
      if (!sourceTab || sourceTab.isNewTab || !sourceTab.url) {
        handleNewTab();
        return;
      }

      try {
        const label = await commands.browserOpenTab(sourceTab.url, null);
        addTab({
          id: label,
          title: sourceTab.title || t('tabs.loading'),
          url: sourceTab.url,
        });
      } catch (error) {
        console.error('Failed to duplicate Discover tab:', error);
        toast.error(t('tabs.operation_failed'));
      }
    },
    [addTab, handleNewTab, t, tabs],
  );

  const handleRestoreLastClosedTab = useCallback(async () => {
    const lastClosedTab = recentlyClosedTabs[0];
    if (!lastClosedTab?.url) return;

    try {
      const label = await commands.browserOpenTab(lastClosedTab.url, null);
      addTab({
        id: label,
        title: lastClosedTab.title || t('tabs.loading'),
        url: lastClosedTab.url,
      });
      removeLastClosedTab();
    } catch (error) {
      console.error('Failed to restore Discover tab:', error);
      toast.error(t('tabs.operation_failed'));
    }
  }, [addTab, recentlyClosedTabs, removeLastClosedTab, t]);

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
    if (!activeTabId || activeTab?.isNewTab) return;
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
    if (!activeTabId || activeTab?.isNewTab) return;
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
    if (!activeTab || activeTab.isNewTab) return;
    try {
      if (activeBookmark) {
        await deleteBookmark.mutateAsync(activeBookmark.id);
      } else {
        await addBookmark.mutateAsync({
          url: activeTab.url,
          title: activeTab.title,
          favicon: activeTab.favicon ?? null,
        });
      }
    } catch (error) {
      console.error('Failed to update Discover bookmark:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleDeleteBookmark = async (id: string) => {
    try {
      await deleteBookmark.mutateAsync(id);
    } catch (error) {
      console.error('Failed to delete Discover bookmark:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleClearHistory = async () => {
    requestConfirmation(t('library.clear_history_confirm'), async () => {
      try {
        await clearHistory.mutateAsync();
        toast.success(t('library.history_cleared'));
      } catch (error) {
        console.error('Failed to clear Discover history:', error);
        toast.error(t('tabs.operation_failed'));
      }
    });
  };

  const handleUpdateBookmark = async (input: {
    id: string;
    url: string;
    title: string;
  }): Promise<boolean> => {
    try {
      await updateBookmark.mutateAsync(input);
      return true;
    } catch (error) {
      console.error('Failed to update Discover bookmark:', error);
      toast.error(t('tabs.operation_failed'));
      return false;
    }
  };

  const handleOpenExternally = async () => {
    if (!activeTab || activeTab.isNewTab) return;
    try {
      await commands.browserOpenExternally(activeTab.url);
    } catch (error) {
      console.error('Failed to open external browser:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleChangeZoom = async (zoom: number) => {
    if (!activeTabId || activeTab?.isNewTab) return;
    try {
      await commands.browserSetZoom(activeTabId, zoom);
      useBrowserStore.getState().updateTab(activeTabId, { zoom });
    } catch (error) {
      console.error('Failed to change Discover zoom:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleFind = async () => {
    if (!activeTabId || activeTab?.isNewTab || !findQuery.trim()) return;
    try {
      await commands.browserFindInPage(activeTabId, findQuery.trim());
      setChromeTray({ kind: 'none' });
    } catch (error) {
      console.error('Failed to find text in Discover tab:', error);
      toast.error(t('tabs.operation_failed'));
    }
  };

  const handleCloseTab = useCallback(
    async (id: string) => {
      const tab = tabs.find((candidate) => candidate.id === id);
      if (!tab?.isNewTab) {
        try {
          const webview = await Webview.getByLabel(id);
          if (webview) await webview.close();
        } catch (error) {
          console.debug('[Browser] Webview was already closed:', error);
        }
      }
      if (tab) recordClosedTab(tab);
      removeTab(id);
    },
    [recordClosedTab, removeTab, tabs],
  );

  const closeChromeTray = useCallback(
    (restoreFocus: boolean = true) => {
      if (chromeTray.kind === 'none') return;

      const triggerId =
        chromeTray.kind === 'toolbar-menu'
          ? 'browser-toolbar-menu-trigger'
          : chromeTray.kind === 'tab-context-menu'
            ? `browser-tab-trigger-${chromeTray.tab.id}`
            : null;

      if (restoreFocus && triggerId) {
        document.getElementById(triggerId)?.focus();
      }
      setChromeTray({ kind: 'none' });
    },
    [chromeTray],
  );

  useEffect(() => {
    if (chromeTray.kind === 'none') return;

    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closeChromeTray();
      }
    };

    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [chromeTray.kind, closeChromeTray]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key.toLowerCase() === 'f' && activeTabId && !activeTab?.isNewTab) {
        event.preventDefault();
        setChromeTray({ kind: 'find' });
      }
      if (event.key.toLowerCase() === 'l') {
        event.preventDefault();
        document.getElementById('browser-url-input')?.focus();
      }
      if (event.key.toLowerCase() === 't') {
        event.preventDefault();
        void handleNewTab();
      }
      if (event.key.toLowerCase() === 'r' && activeTabId && !activeTab?.isNewTab) {
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
  }, [activeTab?.isNewTab, activeTabId, handleCloseTab, handleNewTab, handleReload]);

  const handleUrlSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const url = urlInput.trim();
    if (url) {
      void handleNavigate(url);
    }
  };

  const handleSearchGoogle = useCallback(
    (query: string) => {
      void handleNavigate(`https://www.google.com/search?q=${encodeURIComponent(query)}`);
    },
    [handleNavigate],
  );

  const handleBrowserMenuOpenChange = useCallback(
    (isOpen: boolean) => {
      if (isOpen) {
        setChromeTray({ kind: 'toolbar-menu' });
        return;
      }
      closeChromeTray();
    },
    [closeChromeTray],
  );

  const handleOpenDownloads = useCallback(() => {
    setIsLibraryOpen(false);
    openDownloadPanel();
  }, [openDownloadPanel]);

  const handleOpenLibrary = useCallback(
    (tab: 'bookmarks' | 'history') => {
      closeDownloadPanel();
      setLibraryTab(tab);
      setIsLibraryOpen(true);
    },
    [closeDownloadPanel],
  );

  const sidePanel =
    activeSidePanel === 'downloads' ? (
      <DownloadManagerPanel layout={sidePanelLayout} />
    ) : activeSidePanel === 'library' ? (
      <BrowserLibraryPanel
        layout={sidePanelLayout}
        bookmarks={bookmarks}
        history={history}
        privacy={privacySummary}
        isLoading={isLibraryLoading}
        activeTab={libraryTab}
        onTabChange={setLibraryTab}
        onClose={() => setIsLibraryOpen(false)}
        onNavigate={(url) => {
          setIsLibraryOpen(false);
          void handleNavigate(url);
        }}
        onDeleteBookmark={handleDeleteBookmark}
        onEditBookmark={setEditingBookmark}
        onClearHistory={handleClearHistory}
      />
    ) : null;

  return (
    <div
      ref={browserPageRef}
      className="flex flex-col h-full relative overflow-hidden bg-base-100/85"
    >
      <TopBarActionsPortal>
        <button
          type="button"
          className="btn btn-ghost btn-sm btn-square relative"
          onClick={handleOpenDownloads}
          title={t('tabs.open_downloads')}
          aria-label={t('tabs.open_downloads')}
        >
          <Download size={18} />
          {pendingDownloadCount > 0 && (
            <span className="badge badge-primary badge-xs absolute -right-1 -top-1">
              {pendingDownloadCount}
            </span>
          )}
        </button>
      </TopBarActionsPortal>
      <BrowserTabBar
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={(id) => {
          setActiveTab(id);
          setChromeTray({ kind: 'none' });
        }}
        onCloseTab={(id) => void handleCloseTab(id)}
        onNewTab={handleNewTab}
        onOpenContextMenu={(tab, clientX) => {
          const surface = browserPageRef.current?.getBoundingClientRect();
          setChromeTray({
            kind: 'tab-context-menu',
            tab,
            anchorLeftPx: getChromeTrayAnchorOffset({
              clientX,
              surfaceLeft: surface?.left ?? 0,
              surfaceWidth: surface?.width ?? 0,
            }),
          });
        }}
      />

      <BrowserToolbar
        urlInput={urlInput}
        onUrlInputChange={setUrlInput}
        onUrlSubmit={handleUrlSubmit}
        activeTabId={activeTabId}
        activeTabUrl={activeTab?.isNewTab ? null : (activeTab?.url ?? null)}
        isNewTab={Boolean(activeTab?.isNewTab)}
        isBookmarked={Boolean(activeBookmark)}
        isNavigating={isNavigating || Boolean(activeTab?.isLoading)}
        isRefreshing={isRefreshing}
        isMoreMenuOpen={chromeTray.kind === 'toolbar-menu'}
        onMoreMenuOpenChange={handleBrowserMenuOpenChange}
        onGoBack={handleGoBack}
        onGoForward={handleGoForward}
        onReload={handleReload}
        onToggleBookmark={handleToggleBookmark}
      />

      {chromeTray.kind !== 'none' && (
        <div className="shrink-0 border-b border-base-300 bg-base-200/70 p-2">
          <div className="max-h-[min(24rem,50dvh)] overflow-y-auto">
            {chromeTray.kind === 'find' && (
              <div className="flex justify-end">
                <form
                  className="flex flex-wrap justify-end gap-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void handleFind();
                  }}
                >
                  <input
                    id="browser-find-input"
                    autoFocus
                    className="input input-sm input-bordered w-56 max-w-full"
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
                    onClick={() => closeChromeTray()}
                  >
                    {t('tabs.close')}
                  </button>
                </form>
              </div>
            )}
            {chromeTray.kind === 'toolbar-menu' && (
              <div className="flex justify-end">
                <BrowserToolbarMenu
                  activeTabUrl={activeTab?.isNewTab ? null : (activeTab?.url ?? null)}
                  activeZoom={activeTab?.zoom ?? 1}
                  adblockEnabled={adblockEnabled}
                  hasActiveWebview={Boolean(activeTabId) && !activeTab?.isNewTab}
                  onChangeZoom={(zoom) => void handleChangeZoom(zoom)}
                  onClearCache={() => void handleClearCache()}
                  onClearCookiesAndSiteData={() => void handleClearCookiesAndSiteData()}
                  onClose={() => closeChromeTray()}
                  onNewTab={handleNewTab}
                  onOpenExternally={() => void handleOpenExternally()}
                  onOpenFind={() => setChromeTray({ kind: 'find' })}
                  onOpenLibrary={handleOpenLibrary}
                  onToggleAdblock={() => void handleToggleAdblock()}
                />
              </div>
            )}
            {chromeTray.kind === 'tab-context-menu' && (
              <div style={{ marginLeft: chromeTray.anchorLeftPx }}>
                <BrowserTabContextMenu
                  tab={chromeTray.tab}
                  canRestoreLastClosedTab={recentlyClosedTabs.length > 0}
                  onClose={() => closeChromeTray()}
                  onCloseTab={(id) => void handleCloseTab(id)}
                  onDuplicateTab={(id) => void handleDuplicateTab(id)}
                  onReloadTab={(id) => void handleReloadTab(id)}
                  onRestoreLastClosedTab={() => void handleRestoreLastClosedTab()}
                />
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Main Content / Webview Container ──────────────────────────── */}
      {/* This div acts as the reference for where the native Webview will be placed. */}
      {/* It must span the remaining height. */}
      <div
        ref={browserContentRef}
        className="relative flex min-h-0 flex-1 overflow-hidden bg-base-100"
      >
        {sidePanelLayout === 'full' && sidePanel ? (
          <div className="min-h-0 flex-1">{sidePanel}</div>
        ) : (
          <>
            <div
              className="min-w-0 flex-1"
              style={
                browserSurfacePresentation.leftInsetPx > 0
                  ? { marginLeft: browserSurfacePresentation.leftInsetPx }
                  : undefined
              }
            >
              <div ref={containerRef} className="relative h-full w-full bg-base-100">
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
                {(!activeTab || activeTab.isNewTab) && (
                  <BrowserNewTabPage
                    bookmarks={bookmarks}
                    onNavigate={(url) => void handleNavigate(url)}
                    onSearchGoogle={handleSearchGoogle}
                    onEditBookmark={setEditingBookmark}
                    onOpenBookmarks={() => handleOpenLibrary('bookmarks')}
                  />
                )}
              </div>
            </div>
            {sidePanelLayout === 'docked' && sidePanel}
          </>
        )}
      </div>

      {/* ── Overlays (Rendered via Portal to avoid clipping) ──────────────────────── */}
      {createPortal(
        <>
          <BookmarkEditorDialog
            bookmark={editingBookmark}
            isSaving={updateBookmark.isPending}
            onClose={() => setEditingBookmark(null)}
            onSave={handleUpdateBookmark}
          />
          <BrowserImagePreviewDialog
            imageUrl={previewImageUrl}
            onClose={() => setPreviewImageUrl(null)}
          />
          <BrowserDecodedTextDialog
            decodedText={decodedText}
            onOpenLink={(url) => {
              setDecodedText(null);
              void handleNavigate(url, true);
            }}
            onClose={() => setDecodedText(null)}
          />
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
