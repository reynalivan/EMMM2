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
import { useEffect, useRef, useState } from 'react';
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
  isNavigating: boolean;
  isRefreshing: boolean;
  isMoreMenuOpen: boolean;
  onMoreMenuOpenChange: (isOpen: boolean) => void;
  onGoBack: () => void;
  onGoForward: () => void;
  onReload: () => void;
  onToggleBookmark: () => void;
}

export function BrowserToolbar({
  urlInput,
  onUrlInputChange,
  onUrlSubmit,
  activeTabId,
  activeTabUrl,
  isNewTab,
  isBookmarked,
  isNavigating,
  isRefreshing,
  isMoreMenuOpen,
  onMoreMenuOpenChange,
  onGoBack,
  onGoForward,
  onReload,
  onToggleBookmark,
}: BrowserToolbarProps) {
  const { t } = useTranslation(['browser']);
  const addressInputRef = useRef<HTMLInputElement>(null);
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
        <button
          id="browser-toolbar-menu-trigger"
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
      </div>
    </div>
  );
}

interface BrowserToolbarMenuProps {
  activeTabUrl: string | null;
  activeZoom: number;
  adblockEnabled: boolean;
  hasActiveWebview: boolean;
  onChangeZoom: (zoom: number) => void;
  onClearCache: () => void;
  onClearCookiesAndSiteData: () => void;
  onClose: () => void;
  onNewTab: () => void;
  onOpenExternally: () => void;
  onOpenFind: () => void;
  onOpenLibrary: (tab: 'bookmarks' | 'history') => void;
  onToggleAdblock: () => void;
}

export function BrowserToolbarMenu({
  activeTabUrl,
  activeZoom,
  adblockEnabled,
  hasActiveWebview,
  onChangeZoom,
  onClearCache,
  onClearCookiesAndSiteData,
  onClose,
  onNewTab,
  onOpenExternally,
  onOpenFind,
  onOpenLibrary,
  onToggleAdblock,
}: BrowserToolbarMenuProps) {
  const { t } = useTranslation(['browser']);

  return (
    <div data-testid="browser-toolbar-menu" className="w-full max-w-sm">
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
                onClose();
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
                onClose();
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
                onClose();
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
                onClose();
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
                onClose();
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
                onClose();
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
                onClose();
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
                onClose();
                onClearCache();
              }}
            >
              <RotateCcw size={16} />
              {t('tabs.clear_cache')}
            </button>
          </li>
        </ul>
      </LiquidSurface>
    </div>
  );
}
