import { Plus, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BrowserTab } from '../../../stores/useBrowserStore';
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
    <div className="flex items-end gap-1 px-2 pt-2 bg-base-300 border-b border-base-200 shrink-0 h-12 overflow-x-auto">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onSelectTab(tab.id)}
          className={`
              group flex items-center gap-2 max-w-48 px-3 py-1.5 rounded-t-lg border border-b-0 text-sm truncate transition-colors
              ${
                activeTabId === tab.id
                  ? 'bg-base-100 border-base-200 text-base-content font-medium opacity-100 relative'
                  : 'bg-base-200/50 border-transparent text-base-content/60 hover:bg-base-200 opacity-80'
              }
            `}
          style={{
            // Cover the bottom border line of the container when active
            marginBottom: activeTabId === tab.id ? '-1px' : '0',
            zIndex: activeTabId === tab.id ? 10 : 1,
          }}
        >
          <span className="truncate flex-1">{tabDisplayLabel(tab) ?? t('tabs.new_tab')}</span>
          <div
            className="w-5 h-5 rounded-md hover:bg-base-300 grid place-items-center opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={(e) => onCloseTab(tab.id, e)}
          >
            <X size={12} />
          </div>
        </button>
      ))}
      <button
        onClick={onNewTab}
        className="btn btn-sm btn-ghost btn-square rounded-full mb-1 ml-1"
        title={t('tabs.new_tab')}
      >
        <Plus size={16} />
      </button>
    </div>
  );
}
