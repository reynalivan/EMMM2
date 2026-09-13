import {
  ChevronLeft,
  ChevronRight,
  Cookie,
  Download,
  ExternalLink,
  Globe,
  History,
  LockKeyhole,
  LoaderCircle,
  MoreHorizontal,
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
  isBookmarked: boolean;
  activeZoom: number;
  isNavigating: boolean;
  isRefreshing: boolean;
  activeDownloadCount: number;
  queuedDownloadCount: number;
  onGoBack: () => void;
  onGoForward: () => void;
  onReload: () => void;
  onOpenDiscover: () => void;
  onToggleBookmark: () => void;
  onOpenLibrary: () => void;
  onOpenExternally: () => void;
  onChangeZoom: (zoom: number) => void;
  onOpenFind: () => void;
  adblockEnabled: boolean;
  onToggleAdblock: () => void;
  onClearCookiesAndSiteData: () => void;
  onClearCache: () => void;
  onOpenDownloads: () => void;
}

export function BrowserToolbar({
  urlInput,
  onUrlInputChange,
  onUrlSubmit,
  activeTabId,
  activeTabUrl,
  isBookmarked,
  activeZoom,
  isNavigating,
  isRefreshing,
  activeDownloadCount,
  queuedDownloadCount,
  onGoBack,
  onGoForward,
  onReload,
  onOpenDiscover,
  onToggleBookmark,
  onOpenLibrary,
  onOpenExternally,
  onChangeZoom,
  onOpenFind,
  adblockEnabled,
  onToggleAdblock,
  onClearCookiesAndSiteData,
  onClearCache,
  onOpenDownloads,
}: BrowserToolbarProps) {
  const { t } = useTranslation(['browser']);
  const addressInputRef = useRef<HTMLInputElement>(null);
  const [isEditingAddress, setIsEditingAddress] = useState(false);
  const pendingDownloadCount = activeDownloadCount + queuedDownloadCount;
  const formattedAddress = browserAddressParts(urlInput);
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
          disabled={!activeTabId}
        >
          <ChevronLeft size={17} />
        </button>
        <button
          id="browser-forward-btn"
          className="btn btn-ghost btn-sm btn-square relative"
          title={t('tabs.forward')}
          onClick={onGoForward}
          disabled={!activeTabId}
        >
          <ChevronRight size={17} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square relative"
          title={t('tabs.refresh')}
          onClick={onReload}
          disabled={!activeTabId || isRefreshing}
        >
          <RotateCcw size={16} className={isRefreshing ? 'animate-spin' : ''} />
        </button>
      </div>

      <button
        id="browser-gamebanana-btn"
        className="btn btn-ghost btn-sm btn-square shrink-0"
        title={t('tabs.open_gamebanana')}
        aria-label={t('tabs.open_gamebanana')}
        onClick={onOpenDiscover}
      >
        <Globe size={17} className="text-primary" />
      </button>

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
            disabled={!activeTabId}
            onClick={onToggleBookmark}
          >
            <Star size={16} className={isBookmarked ? 'fill-current' : ''} />
          </button>
        </div>
      </form>

      <div className="ml-auto flex shrink-0 items-center gap-0.5 sm:ml-0">
        <div className="dropdown dropdown-end">
          <button
            type="button"
            tabIndex={0}
            className="btn btn-sm btn-ghost btn-square"
            title={t('tabs.browser_menu')}
            aria-label={t('tabs.browser_menu')}
          >
            <MoreHorizontal size={18} />
          </button>
          <LiquidSurface
            liquidRole="overlay"
            className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-60 rounded-box shadow-lg"
          >
            <ul tabIndex={0} className="menu w-full p-2" aria-label={t('tabs.browser_menu')}>
              <li>
                <button
                  type="button"
                  role="switch"
                  aria-checked={adblockEnabled}
                  aria-label={t('tabs.adblock')}
                  onClick={onToggleAdblock}
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
                <button type="button" onClick={onOpenLibrary}>
                  <History size={16} />
                  {t('tabs.bookmarks_and_history')}
                </button>
              </li>
              <li>
                <button type="button" disabled={!activeTabUrl} onClick={onOpenExternally}>
                  <ExternalLink size={16} />
                  {t('tabs.open_externally')}
                </button>
              </li>
              <li className="menu-title mt-2 px-2 text-xs">{t('tabs.view')}</li>
              <li>
                <button type="button" disabled={!activeTabId} onClick={onOpenFind}>
                  <Search size={16} />
                  {t('tabs.find_in_page')}
                  <span className="ml-auto text-xs text-base-content/50">Ctrl+F</span>
                </button>
              </li>
              <li className="flex-row items-center justify-between px-2 py-1">
                <button
                  type="button"
                  className="btn btn-ghost btn-xs btn-square"
                  aria-label={t('tabs.zoom_out')}
                  disabled={!activeTabId || activeZoom <= 0.5}
                  onClick={() => onChangeZoom(Number((activeZoom - 0.1).toFixed(1)))}
                >
                  <ZoomOut size={15} />
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs min-h-0 h-7"
                  disabled={!activeTabId || activeZoom === 1}
                  onClick={() => onChangeZoom(1)}
                >
                  {Math.round(activeZoom * 100)}%
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs btn-square"
                  aria-label={t('tabs.zoom_in')}
                  disabled={!activeTabId || activeZoom >= 3}
                  onClick={() => onChangeZoom(Number((activeZoom + 0.1).toFixed(1)))}
                >
                  <ZoomIn size={15} />
                </button>
              </li>
              <li className="menu-title mt-2 px-2 text-xs">{t('tabs.privacy')}</li>
              <li>
                <button type="button" disabled={!activeTabId} onClick={onClearCookiesAndSiteData}>
                  <Cookie size={16} />
                  {t('tabs.clear_cookies_and_site_data')}
                </button>
              </li>
              <li>
                <button type="button" disabled={!activeTabId} onClick={onClearCache}>
                  <RotateCcw size={16} />
                  {t('tabs.clear_cache')}
                </button>
              </li>
            </ul>
          </LiquidSurface>
        </div>

        <button
          id="browser-downloads-btn"
          className="btn btn-sm btn-ghost btn-square relative"
          onClick={onOpenDownloads}
          title={t('tabs.open_downloads')}
          aria-label={t('tabs.open_downloads')}
        >
          <Download size={18} />
          {pendingDownloadCount > 0 && (
            <span className="badge badge-primary badge-xs absolute -top-1 -right-1">
              {pendingDownloadCount}
            </span>
          )}
        </button>
      </div>
    </div>
  );
}
