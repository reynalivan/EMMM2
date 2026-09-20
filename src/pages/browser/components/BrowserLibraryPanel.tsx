import { Bookmark, History, Pencil, ShieldCheck, Trash2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';
import { LiquidSurface } from '@/shared/ui/liquid';
import type { BrowserSidePanelLayout } from '../browserSurfacePresentation';
import type {
  BrowserBookmark,
  BrowserHistoryEntry,
  BrowserPrivacySummary,
} from '@/shared/api/tauri/bindings';

const getBookmarkKey = (bookmark: BrowserBookmark) => bookmark.id;
const getHistoryEntryKey = (entry: BrowserHistoryEntry) => entry.url;

export type BrowserLibraryTab = 'bookmarks' | 'history';

interface BrowserLibraryPanelProps {
  layout: BrowserSidePanelLayout;
  bookmarks: BrowserBookmark[];
  history: BrowserHistoryEntry[];
  privacy: BrowserPrivacySummary | null;
  isLoading?: boolean;
  activeTab: BrowserLibraryTab;
  onTabChange: (tab: BrowserLibraryTab) => void;
  onClose: () => void;
  onNavigate: (url: string) => void;
  onEditBookmark: (bookmark: BrowserBookmark) => void;
  onDeleteBookmark: (id: string) => void;
  onClearHistory: () => void;
}

export function BrowserLibraryPanel({
  layout,
  bookmarks,
  history,
  privacy,
  isLoading = false,
  activeTab,
  onTabChange,
  onClose,
  onNavigate,
  onEditBookmark,
  onDeleteBookmark,
  onClearHistory,
}: BrowserLibraryPanelProps) {
  const { t } = useTranslation(['browser']);

  return (
    <aside
      role="complementary"
      aria-label={t('library.title')}
      className={`flex h-full min-h-0 flex-col border-l border-base-200 bg-base-100 shadow-2xl ${
        layout === 'docked' ? 'w-[512px] shrink-0' : 'w-full'
      }`}
    >
      <header className="flex items-center justify-between border-b border-base-200 px-4 py-3">
        <div>
          <h2 className="font-semibold">{t('library.title')}</h2>
          <p className="text-xs text-base-content/60">{t('library.description')}</p>
        </div>
        <button
          className="btn btn-ghost btn-sm btn-square"
          onClick={onClose}
          aria-label={t('library.close')}
        >
          <X size={18} />
        </button>
      </header>

      {privacy && (
        <div className="grid grid-cols-3 gap-2 border-b border-base-200 p-4 text-center text-xs">
          <div>
            <strong className="block text-base">{privacy.bookmarks}</strong>
            {t('library.bookmarks')}
          </div>
          <div>
            <strong className="block text-base">{privacy.history_entries}</strong>
            {t('library.history')}
          </div>
          <div>
            <strong className="block text-base">{privacy.saved_permissions}</strong>
            {t('library.permissions')}
          </div>
        </div>
      )}

      <div className="flex shrink-0 items-center border-b border-base-200 px-4 py-3">
        <div role="tablist" aria-label={t('library.title')}>
          <LiquidSurface
            liquidRole="control"
            className="rounded-[var(--radius-box)]"
            contentClassName="flex gap-1 p-1"
          >
            <LibraryTab
              tab="bookmarks"
              activeTab={activeTab}
              label={t('library.bookmarks')}
              onClick={onTabChange}
            />
            <LibraryTab
              tab="history"
              activeTab={activeTab}
              label={t('library.history')}
              onClick={onTabChange}
            />
          </LiquidSurface>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {isLoading ? (
          <WorkspacePanelSkeleton variant="list" />
        ) : (
          <>
            {activeTab === 'bookmarks' && (
              <div
                id="browser-library-bookmarks"
                role="tabpanel"
                aria-labelledby="browser-library-tab-bookmarks"
                className="mb-5"
              >
                <h3 className="mb-2 flex items-center gap-2 text-sm font-semibold">
                  <Bookmark size={16} />
                  {t('library.bookmarks')}
                </h3>
                {bookmarks.length === 0 ? (
                  <p className="text-sm text-base-content/60">{t('library.empty_bookmarks')}</p>
                ) : (
                  <VirtualList
                    items={bookmarks}
                    getItemKey={getBookmarkKey}
                    estimateSize={() => 58}
                    ariaLabel={t('library.bookmarks')}
                    className="max-h-52 pr-1"
                    renderItem={(bookmark) => (
                      <div className="flex items-center gap-2 rounded-btn">
                        <button
                          className="workspace-interactive min-w-0 flex-1 rounded-btn px-2 py-2 text-left hover:bg-base-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-1"
                          onClick={() => onNavigate(bookmark.url)}
                          title={bookmark.url}
                        >
                          <span className="block truncate text-sm font-medium">
                            {bookmark.title || bookmark.url}
                          </span>
                          <span className="block truncate text-xs text-base-content/60">
                            {bookmark.url}
                          </span>
                        </button>
                        <button
                          className="btn btn-ghost btn-xs btn-square"
                          onClick={() => onEditBookmark(bookmark)}
                          aria-label={t('library.edit_bookmark')}
                        >
                          <Pencil size={14} />
                        </button>
                        <button
                          className="btn btn-ghost btn-xs btn-square"
                          onClick={() => onDeleteBookmark(bookmark.id)}
                          aria-label={t('library.remove_bookmark')}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    )}
                  />
                )}
              </div>
            )}

            {activeTab === 'history' && (
              <div
                id="browser-library-history"
                role="tabpanel"
                aria-labelledby="browser-library-tab-history"
              >
                <div className="mb-2 flex items-center justify-between gap-2">
                  <h3 className="flex items-center gap-2 text-sm font-semibold">
                    <History size={16} />
                    {t('library.history')}
                  </h3>
                  <button
                    className="btn btn-ghost btn-xs text-error"
                    onClick={onClearHistory}
                    disabled={history.length === 0}
                  >
                    <Trash2 size={14} />
                    {t('library.clear_history')}
                  </button>
                </div>
                {history.length === 0 ? (
                  <p className="text-sm text-base-content/60">{t('library.empty_history')}</p>
                ) : (
                  <VirtualList
                    items={history}
                    getItemKey={getHistoryEntryKey}
                    estimateSize={() => 58}
                    ariaLabel={t('library.history')}
                    className="max-h-72 pr-1"
                    renderItem={(entry) => (
                      <button
                        className="workspace-interactive w-full rounded-btn px-2 py-2 text-left hover:bg-base-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-1"
                        onClick={() => onNavigate(entry.url)}
                        title={entry.url}
                      >
                        <span className="block truncate text-sm font-medium">
                          {entry.title || entry.hostname}
                        </span>
                        <span className="block truncate text-xs text-base-content/60">
                          {entry.hostname}
                        </span>
                      </button>
                    )}
                  />
                )}
              </div>
            )}
          </>
        )}
      </div>

      <footer className="flex items-center gap-2 border-t border-base-200 px-4 py-3 text-xs text-base-content/60">
        <ShieldCheck size={15} />
        {t('library.permissions_default_deny')}
      </footer>
    </aside>
  );
}

function LibraryTab({
  tab,
  activeTab,
  label,
  onClick,
}: {
  tab: BrowserLibraryTab;
  activeTab: BrowserLibraryTab;
  label: string;
  onClick: (tab: BrowserLibraryTab) => void;
}) {
  const isActive = activeTab === tab;

  return (
    <button
      id={`browser-library-tab-${tab}`}
      type="button"
      role="tab"
      aria-selected={isActive}
      aria-controls={`browser-library-${tab}`}
      className={`flex min-h-8 items-center justify-center gap-2 rounded-[calc(var(--radius-box)-0.25rem)] px-3 text-sm font-medium transition-[background-color,color] duration-150 ${
        isActive
          ? 'bg-base-content/[0.08] text-base-content'
          : 'text-base-content/55 hover:bg-base-content/[0.05] hover:text-base-content'
      }`}
      onClick={() => onClick(tab)}
    >
      {tab === 'bookmarks' ? <Bookmark size={15} /> : <History size={15} />}
      {label}
    </button>
  );
}
