import { ChevronLeft, ChevronRight, Download, Globe, RotateCcw, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface BrowserToolbarProps {
  urlInput: string;
  onUrlInputChange: (value: string) => void;
  onUrlSubmit: (e: React.FormEvent) => void;
  activeTabId: string | null;
  isNavigating: boolean;
  isRefreshing: boolean;
  finishedCount: number;
  onGoBack: () => void;
  onGoForward: () => void;
  onReload: () => void;
  onOpenDiscover: () => void;
  onClearData: () => void;
  onOpenDownloads: () => void;
}

export function BrowserToolbar({
  urlInput,
  onUrlInputChange,
  onUrlSubmit,
  activeTabId,
  isNavigating,
  isRefreshing,
  finishedCount,
  onGoBack,
  onGoForward,
  onReload,
  onOpenDiscover,
  onClearData,
  onOpenDownloads,
}: BrowserToolbarProps) {
  const { t } = useTranslation(['browser']);

  return (
    <div className="flex items-center gap-3 px-4 py-2 bg-base-100 border-b border-base-200 shrink-0 z-10 relative shadow-sm">
      {/* Navigation buttons */}
      <div className="flex items-center gap-1">
        <button
          id="browser-back-btn"
          className="btn btn-ghost btn-xs btn-square"
          title={t('tabs.back')}
          onClick={onGoBack}
          disabled={!activeTabId}
        >
          <ChevronLeft size={16} />
        </button>
        <button
          id="browser-forward-btn"
          className="btn btn-ghost btn-xs btn-square"
          title={t('tabs.forward')}
          onClick={onGoForward}
          disabled={!activeTabId}
        >
          <ChevronRight size={16} />
        </button>
        <button
          className="btn btn-ghost btn-xs btn-square"
          title={t('tabs.refresh')}
          onClick={onReload}
          disabled={!activeTabId || isRefreshing}
        >
          <RotateCcw size={14} className={isRefreshing ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Discover Hub quick link */}
      <button
        id="browser-gamebanana-btn"
        className="btn btn-ghost btn-sm gap-2"
        title={t('tabs.open_gamebanana')}
        onClick={onOpenDiscover}
      >
        <Globe size={16} className="text-info" />
        {t('tabs.discover')}
      </button>

      {/* URL bar */}
      <form onSubmit={onUrlSubmit} className="flex-1 flex gap-2">
        <input
          id="browser-url-input"
          type="text"
          className="input input-sm input-bordered flex-1 font-mono text-sm bg-base-200 focus:bg-base-100 transition-colors"
          placeholder={t('tabs.url_placeholder')}
          value={urlInput}
          onChange={(e) => onUrlInputChange(e.target.value)}
        />
        <button
          id="browser-navigate-btn"
          type="submit"
          className="btn btn-sm btn-primary"
          disabled={isNavigating}
        >
          {isNavigating ? <span className="loading loading-spinner loading-xs" /> : t('tabs.go')}
        </button>
      </form>

      {/* Action Buttons */}
      <div className="flex items-center gap-2">
        <button
          className="btn btn-sm btn-ghost text-error"
          onClick={onClearData}
          title={t('tabs.clear_data')}
          disabled={!activeTabId}
        >
          <Trash2 size={18} />
        </button>

        <button
          id="browser-downloads-btn"
          className="btn btn-sm btn-ghost relative"
          onClick={onOpenDownloads}
          title={t('tabs.open_downloads')}
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
  );
}
