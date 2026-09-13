import { Globe, LoaderCircle, Plus, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BrowserTab } from '@/entities/browser';
import { tabDisplayLabel } from '../utils/browserUrl';

interface BrowserTabBarProps {
  tabs: BrowserTab[];
  activeTabId: string | null;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string, e: React.MouseEvent) => void;
  onNewTab: () => void;
}

export function BrowserTabBar({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onNewTab,
}: BrowserTabBarProps) {
  const { t } = useTranslation(['browser']);

  return (
    <div
      className="flex h-11 shrink-0 items-end gap-1 overflow-x-auto border-b border-base-300 bg-base-200 px-2.5 pt-1.5"
      role="tablist"
      aria-label={t('tabs.discover')}
    >
      {tabs.map((tab) => {
        const displayLabel = tabDisplayLabel(tab) ?? t('tabs.new_tab');
        const active = activeTabId === tab.id;
        const className = `
          group flex h-9 max-w-56 items-center gap-2 rounded-t-md border border-b-0 px-3 text-sm transition-[background-color,border-color,color] duration-150 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary
          ${
            active
              ? '-mb-px border-base-300 font-medium text-base-content'
              : 'border-transparent bg-transparent text-base-content/60 hover:bg-base-100/70 hover:text-base-content'
          }
        `;
        const content = (
          <>
            <button
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => onSelectTab(tab.id)}
              className="flex min-w-0 flex-1 items-center gap-2 text-left focus-visible:outline-none"
            >
              <span className="relative grid h-4 w-4 shrink-0 place-items-center text-base-content/60">
                {tab.isLoading ? (
                  <LoaderCircle size={14} className="animate-spin" aria-label={t('tabs.loading')} />
                ) : (
                  <Globe size={14} aria-hidden="true" />
                )}
                {tab.favicon && !tab.isLoading && (
                  <img
                    src={tab.favicon}
                    alt=""
                    className="absolute inset-0 h-4 w-4 rounded-sm object-contain"
                    onError={(event) => {
                      event.currentTarget.style.display = 'none';
                    }}
                  />
                )}
              </span>
              <span className="flex-1 truncate" title={displayLabel}>
                {displayLabel}
              </span>
            </button>
            <button
              type="button"
              aria-label={`${t('tabs.close')} ${displayLabel}`}
              className={`grid h-6 w-6 shrink-0 place-items-center rounded-md transition-[background-color,opacity] duration-150 hover:bg-base-300 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary ${
                active
                  ? 'opacity-100'
                  : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100'
              }`}
              onClick={(event) => onCloseTab(tab.id, event)}
            >
              <X size={12} aria-hidden="true" />
            </button>
          </>
        );

        return (
          <div key={tab.id} className={className}>
            {content}
          </div>
        );
      })}
      <button
        type="button"
        onClick={onNewTab}
        className="btn btn-sm btn-ghost btn-square mb-0.5 ml-1 rounded-md"
        title={t('tabs.new_tab')}
        aria-label={t('tabs.new_tab')}
      >
        <Plus size={16} />
      </button>
    </div>
  );
}
