import { Bookmark, Library, Pencil, Search } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import type { BrowserBookmark } from '@/shared/api/tauri/bindings';

interface BrowserNewTabPageProps {
  bookmarks: BrowserBookmark[];
  onNavigate: (url: string) => void;
  onSearchGoogle: (query: string) => void;
  onEditBookmark: (bookmark: BrowserBookmark) => void;
  onOpenBookmarks: () => void;
}

const getBookmarkKey = (bookmark: BrowserBookmark) => bookmark.id;

export function BrowserNewTabPage({
  bookmarks,
  onNavigate,
  onSearchGoogle,
  onEditBookmark,
  onOpenBookmarks,
}: BrowserNewTabPageProps) {
  const { t } = useTranslation(['browser']);
  const [searchQuery, setSearchQuery] = useState('');

  const handleSearch = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const query = searchQuery.trim();
    if (!query) return;
    onSearchGoogle(query);
  };

  return (
    <section
      aria-label={t('new_tab.title')}
      className="absolute inset-0 flex min-h-0 flex-col overflow-hidden bg-base-100 px-5 py-8 sm:px-8"
    >
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-6">
        <div>
          <h2 className="text-lg font-semibold">{t('new_tab.title')}</h2>
        </div>

        <form className="flex gap-2" onSubmit={handleSearch}>
          <label className="sr-only" htmlFor="browser-new-tab-search">
            {t('new_tab.search_label')}
          </label>
          <input
            id="browser-new-tab-search"
            className="input input-bordered min-w-0 flex-1"
            placeholder={t('new_tab.search_placeholder')}
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
          />
          <button type="submit" className="btn btn-primary gap-2" disabled={!searchQuery.trim()}>
            <Search size={16} />
            {t('new_tab.search')}
          </button>
        </form>
      </div>

      <div className="mx-auto mt-8 flex min-h-0 w-full max-w-2xl flex-1 flex-col">
        <div className="mb-2 flex items-center justify-between gap-3">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            <Bookmark size={16} />
            {t('new_tab.bookmarks')}
          </h3>
          <button type="button" className="btn btn-ghost btn-sm gap-2" onClick={onOpenBookmarks}>
            <Library size={16} />
            {t('new_tab.open_library')}
          </button>
        </div>

        {bookmarks.length === 0 ? (
          <p className="text-sm text-base-content/60">{t('new_tab.empty_bookmarks')}</p>
        ) : (
          <VirtualList
            items={bookmarks}
            getItemKey={getBookmarkKey}
            estimateSize={() => 62}
            ariaLabel={t('new_tab.bookmarks')}
            className="min-h-0 flex-1 pr-1"
            renderItem={(bookmark) => (
              <div className="flex items-center gap-2 rounded-btn">
                <button
                  type="button"
                  className="workspace-interactive min-w-0 flex-1 rounded-btn px-3 py-2.5 text-left hover:bg-base-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-1"
                  title={bookmark.url}
                  onClick={() => onNavigate(bookmark.url)}
                >
                  <span className="block truncate text-sm font-medium">
                    {bookmark.title || bookmark.url}
                  </span>
                  <span className="block truncate text-xs text-base-content/60">
                    {bookmark.url}
                  </span>
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-sm btn-square"
                  aria-label={t('library.edit_bookmark')}
                  title={t('library.edit_bookmark')}
                  onClick={() => onEditBookmark(bookmark)}
                >
                  <Pencil size={15} />
                </button>
              </div>
            )}
          />
        )}
      </div>
    </section>
  );
}
