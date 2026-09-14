import {
  ChevronLeft,
  ChevronRight,
  Cookie,
  ExternalLink,
  History,
  LockKeyhole,
  LoaderCircle,
  MoreHorizontal,
  Plus,
  RotateCcw,
  Search,
  ShieldCheck,
  ShieldOff,
  Star,
  TriangleAlert,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { browserAddressParts } from '../utils/browserUrl';
import { LiquidSurface } from '@/shared/ui/liquid';

interface BrowserToolbarProps {
  urlInput: string;
  onUrlInputChange: (value: string) => void;
  onUrlSubmit: (e: React.FormEvent) => void;
  activeTabId: string | null;
  activeTabUrl: string | null;
  isNewTab: boolean;
  isBookmarked: boolean;
  activeZoom: number;
  isNavigating: boolean;
  isRefreshing: boolean;
  isMoreMenuOpen: boolean;
  onMoreMenuOpenChange: (isOpen: boolean) => void;
  onGoBack: () => void;
  onGoForward: () => void;
  onReload: () => void;
  onNewTab: () => void;
  onToggleBookmark: () => void;
  onOpenLibrary: (tab: 'bookmarks' | 'history') => void;
  onOpenExternally: () => void;
  onChangeZoom: (zoom: number) => void;
  onOpenFind: () => void;
  adblockEnabled: boolean;
  onToggleAdblock: () => void;
  onClearCookiesAndSiteData: () => void;
  onClearCache: () => void;
}

export function BrowserToolbar({
  urlInput,
  onUrlInputChange,
  onUrlSubmit,
  activeTabId,
  activeTabUrl,
  isNewTab,
  isBookmarked,
  activeZoom,
  isNavigating,
  isRefreshing,
  isMoreMenuOpen,
  onMoreMenuOpenChange,
  onGoBack,
  onGoForward,
  onReload,
  onNewTab,
  onToggleBookmark,
  onOpenLibrary,
  onOpenExternally,
  onChangeZoom,
  onOpenFind,
  adblockEnabled,
  onToggleAdblock,
  onClearCookiesAndSiteData,
  onClearCache,
}: BrowserToolbarProps) {
  const { t } = useTranslation(['browser']);
  const addressInputRef = useRef<HTMLInputElement>(null);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  const moreMenuTriggerRef = useRef<HTMLButtonElement>(null);
  const [moreMenuPosition, setMoreMenuPosition] = useState({ top: 0, left: 0 });
  const [isEditingAddress, setIsEditingAddress] = useState(false);
  const formattedAddress = browserAddressParts(urlInput);
  const hasActiveWebview = Boolean(activeTabId) && !isNewTab;
  let security: { secure: boolean; domain: string } | null = null;
  if (activeTabUrl) {
    try {
      const parsed = new URL(activeTabUrl);
      security = { secure: parsed.protocol === 'https:', domain: parsed.hostname };
    } catch {
      security = { secure: false, domain: activeTabUrl };
    }
  }

  useEffect(() => {
    if (!isEditingAddress) return;
    addressInputRef.current?.focus();
  }, [isEditingAddress]);

  useEffect(() => {
    if (!isMoreMenuOpen) return;

    const closeWhenLeavingMenu = (event: PointerEvent) => {
      const target = event.target as Node;
      if (moreMenuRef.current?.contains(target) || moreMenuTriggerRef.current?.contains(target)) {
        return;
      }
      onMoreMenuOpenChange(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onMoreMenuOpenChange(false);
      }
    };

    document.addEventListener('pointerdown', closeWhenLeavingMenu);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeWhenLeavingMenu);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [isMoreMenuOpen, onMoreMenuOpenChange]);

  useLayoutEffect(() => {
    if (!isMoreMenuOpen) return;

    const updatePosition = () => {
      const trigger = moreMenuTriggerRef.current;
      if (!trigger) return;

      const rect = trigger.getBoundingClientRect();
      const menuWidth = Math.min(240, window.innerWidth - 16);
      setMoreMenuPosition({
        top: rect.bottom + 8,
        left: Math.max(8, Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 8)),
      });
    };

    updatePosition();
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  }, [isMoreMenuOpen]);

  const startEditingAddress = () => setIsEditingAddress(true);

  return (
    <div className="relative z-10 flex min-h-12 flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-100 px-2 py-1.5 sm:flex-nowrap sm:px-3">
      <div className="flex shrink-0 items-center gap-0.5">
        <button
          id="browser-back-btn"
          className="btn btn-ghost btn-sm btn-square relative"
          title={t('tabs.back')}
          onClick={onGoBack}
          disabled={!hasActiveWebview}
        >
          <ChevronLeft size={17} />
        </button>
        <button
          id="browser-forward-btn"
          className="btn btn-ghost btn-sm btn-square relative"
          title={t('tabs.forward')}
          onClick={onGoForward}
          disabled={!hasActiveWebview}
        >
          <ChevronRight size={17} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square relative"
          title={t('tabs.refresh')}
          onClick={onReload}
          disabled={!hasActiveWebview || isRefreshing}
        >
          <RotateCcw size={16} className={isRefreshing ? 'animate-spin' : ''} />
        </button>
      </div>

      <form
        onSubmit={onUrlSubmit}
        className="order-3 flex basis-full sm:order-none sm:basis-auto sm:flex-1"
      >
        <div className="relative min-w-0 flex-1">
          {security && (
            <span
              className="pointer-events-none absolute inset-y-0 left-3 z-10 flex items-center"
              title={security.domain}
              aria-label={
                security.secure ? t('tabs.secure_connection') : t('tabs.insecure_connection')
              }
            >
              {isNavigating ? (
                <LoaderCircle
                  size={14}
                  className="animate-spin text-primary"
                  aria-label={t('tabs.loading')}
                />
              ) : security.secure ? (
                <LockKeyhole size={14} className="text-success" />
              ) : (
                <TriangleAlert size={14} className="text-warning" />
              )}
            </span>
          )}
          {formattedAddress && !isEditingAddress ? (
            <button
              id="browser-url-input"
              type="button"
              className="input input-sm input-bordered flex w-full items-center gap-0 overflow-hidden pl-9 pr-10 text-left"
              aria-label={urlInput}
              title={urlInput}
              onFocus={startEditingAddress}
              onClick={startEditingAddress}
            >
              <span className="min-w-0 truncate font-medium text-base-content">
                {formattedAddress.host}
              </span>
              <span className="min-w-0 truncate text-base-content/45">
                {formattedAddress.suffix}
              </span>
            </button>
          ) : (
            <input
              ref={addressInputRef}
              id="browser-url-input"
              type="text"
              className="input input-sm input-bordered w-full pl-9 pr-10 font-mono text-sm"
              placeholder={t('tabs.url_placeholder')}
              value={urlInput}
              onBlur={() => setIsEditingAddress(false)}
              onChange={(event) => onUrlInputChange(event.target.value)}
            />
          )}
          <button
            type="button"
            className={`btn btn-ghost btn-xs btn-square absolute inset-y-0 right-1 my-auto ${
              isBookmarked ? 'text-primary' : 'text-base-content/55'
            }`}
            title={t(isBookmarked ? 'tabs.remove_bookmark' : 'tabs.add_bookmark')}
            aria-label={t(isBookmarked ? 'tabs.remove_bookmark' : 'tabs.add_bookmark')}
            aria-pressed={isBookmarked}
            disabled={!hasActiveWebview}
            onClick={onToggleBookmark}
          >
            <Star size={16} className={isBookmarked ? 'fill-current' : ''} />
          </button>
        </div>
      </form>

      <div className="ml-auto flex shrink-0 items-center gap-0.5 sm:ml-0">
        <div className="relative">
          <button
            ref={moreMenuTriggerRef}
            type="button"
            className="btn btn-sm btn-ghost btn-square"
            title={t('tabs.browser_menu')}
            aria-label={t('tabs.browser_menu')}
            aria-expanded={isMoreMenuOpen}
            aria-controls="browser-toolbar-menu"
            onClick={() => onMoreMenuOpenChange(!isMoreMenuOpen)}
          >
            <MoreHorizontal size={18} />
          </button>
          {isMoreMenuOpen &&
            createPortal(
              <div
                ref={moreMenuRef}
                data-testid="browser-toolbar-menu-overlay"
                className="fixed z-[var(--workspace-layer-popover)]"
                style={{
                  top: moreMenuPosition.top,
                  left: moreMenuPosition.left,
                  width: 'min(15rem, calc(100vw - 1rem))',
                }}
              >
                <LiquidSurface liquidRole="overlay" className="w-full rounded-box shadow-lg">
                  <ul
                    id="browser-toolbar-menu"
                    className="menu w-full p-2"
                    aria-label={t('tabs.browser_menu')}
                  >
                    <li>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={adblockEnabled}
                        aria-label={t('tabs.adblock')}
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onToggleAdblock();
                        }}
                        className="flex items-center justify-between gap-3"
                      >
                        <span className="flex items-center gap-2">
                          {adblockEnabled ? <ShieldCheck size={16} /> : <ShieldOff size={16} />}
                          {t('tabs.adblock')}
                        </span>
                        <span className="text-xs text-base-content/60">
                          {adblockEnabled ? t('tabs.on') : t('tabs.off')}
                        </span>
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onNewTab();
                        }}
                      >
                        <Plus size={16} />
                        {t('tabs.new_tab')}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onOpenLibrary('bookmarks');
                        }}
                      >
                        <Star size={16} />
                        {t('library.bookmarks')}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onOpenLibrary('history');
                        }}
                      >
                        <History size={16} />
                        {t('library.history')}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        disabled={!hasActiveWebview || !activeTabUrl}
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onOpenExternally();
                        }}
                      >
                        <ExternalLink size={16} />
                        {t('tabs.open_externally')}
                      </button>
                    </li>
                    <li className="menu-title mt-2 px-2 text-xs">{t('tabs.view')}</li>
                    <li>
                      <button
                        type="button"
                        disabled={!hasActiveWebview}
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onOpenFind();
                        }}
                      >
                        <Search size={16} />
                        {t('tabs.find_in_page')}
                        <span className="ml-auto text-xs text-base-content/50">
                          {t('tabs.find_shortcut')}
                        </span>
                      </button>
                    </li>
                    <li className="flex-row items-center justify-between px-2 py-1">
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs btn-square"
                        aria-label={t('tabs.zoom_out')}
                        disabled={!hasActiveWebview || activeZoom <= 0.5}
                        onClick={() => onChangeZoom(Number((activeZoom - 0.1).toFixed(1)))}
                      >
                        <ZoomOut size={15} />
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs min-h-0 h-7"
                        disabled={!hasActiveWebview || activeZoom === 1}
                        onClick={() => onChangeZoom(1)}
                      >
                        {Math.round(activeZoom * 100)}%
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs btn-square"
                        aria-label={t('tabs.zoom_in')}
                        disabled={!hasActiveWebview || activeZoom >= 3}
                        onClick={() => onChangeZoom(Number((activeZoom + 0.1).toFixed(1)))}
                      >
                        <ZoomIn size={15} />
                      </button>
                    </li>
                    <li className="menu-title mt-2 px-2 text-xs">{t('tabs.privacy')}</li>
                    <li>
                      <button
                        type="button"
                        disabled={!hasActiveWebview}
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onClearCookiesAndSiteData();
                        }}
                      >
                        <Cookie size={16} />
                        {t('tabs.clear_cookies_and_site_data')}
                      </button>
                    </li>
                    <li>
                      <button
                        type="button"
                        disabled={!hasActiveWebview}
                        onClick={() => {
                          onMoreMenuOpenChange(false);
                          onClearCache();
                        }}
                      >
                        <RotateCcw size={16} />
                        {t('tabs.clear_cache')}
                      </button>
                    </li>
                  </ul>
                </LiquidSurface>
              </div>,
              document.body,
            )}
        </div>
      </div>
    </div>
  );
}
