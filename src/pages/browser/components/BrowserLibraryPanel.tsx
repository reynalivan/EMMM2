import { Bookmark, History, ShieldCheck, Trash2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  BrowserBookmark,
  BrowserHistoryEntry,
  BrowserPrivacySummary,
} from '@/shared/api/tauri/bindings';

interface BrowserLibraryPanelProps {
  bookmarks: BrowserBookmark[];
  history: BrowserHistoryEntry[];
  privacy: BrowserPrivacySummary | null;
  onClose: () => void;
  onNavigate: (url: string) => void;
  onDeleteBookmark: (id: string) => void;
  onClearHistory: () => void;
}

export function BrowserLibraryPanel({
  bookmarks,
  history,
  privacy,
  onClose,
  onNavigate,
  onDeleteBookmark,
  onClearHistory,
}: BrowserLibraryPanelProps) {
  const { t } = useTranslation(['browser']);

  return (
    <div className="fixed inset-0 z-[var(--workspace-layer-overlay)] flex items-start justify-end bg-overlay-mask p-4 backdrop-blur-sm">
      <section
        role="dialog"
        aria-modal="true"
        aria-label={t('library.title')}
        className="flex h-full w-full max-w-lg flex-col rounded-box border border-base-200 bg-base-100 shadow-2xl"
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

        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <div className="mb-5">
            <h3 className="mb-2 flex items-center gap-2 text-sm font-semibold">
              <Bookmark size={16} />
              {t('library.bookmarks')}
            </h3>
            {bookmarks.length === 0 ? (
              <p className="text-sm text-base-content/60">{t('library.empty_bookmarks')}</p>
            ) : (
              <ul className="space-y-1">
                {bookmarks.map((bookmark) => (
                  <li
                    key={bookmark.id}
                    className="flex items-center gap-2 rounded-btn hover:bg-base-200"
                  >
                    <button
                      className="min-w-0 flex-1 px-2 py-2 text-left"
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
                      onClick={() => onDeleteBookmark(bookmark.id)}
                      aria-label={t('library.remove_bookmark')}
                    >
                      <Trash2 size={14} />
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div>
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
              <ul className="space-y-1">
                {history.map((entry) => (
                  <li key={entry.url}>
                    <button
                      className="w-full rounded-btn px-2 py-2 text-left hover:bg-base-200"
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
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <footer className="flex items-center gap-2 border-t border-base-200 px-4 py-3 text-xs text-base-content/60">
          <ShieldCheck size={15} />
          {t('library.permissions_default_deny')}
        </footer>
      </section>
    </div>
  );
}
