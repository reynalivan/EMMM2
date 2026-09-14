import { Copy, Globe, LoaderCircle, Plus, RotateCw, Undo2, X } from 'lucide-react';
import { createPortal } from 'react-dom';
import { useCallback, useEffect, useState, type MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type { BrowserTab } from '@/entities/browser';
import { LiquidSurface } from '@/shared/ui/liquid';
import { tabDisplayLabel } from '../utils/browserUrl';

interface BrowserTabBarProps {
  tabs: BrowserTab[];
  activeTabId: string | null;
  canRestoreLastClosedTab: boolean;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onNewTab: () => void;
  onReloadTab: (id: string) => void;
  onDuplicateTab: (id: string) => void;
  onRestoreLastClosedTab: () => void;
  onContextMenuOpenChange: (isOpen: boolean) => void;
}

type TabContextMenu = {
  tab: BrowserTab;
  left: number;
  top: number;
};

const CONTEXT_MENU_WIDTH = 224;
const CONTEXT_MENU_HEIGHT = 184;
const VIEWPORT_GUTTER = 8;

function TabFavicon({
  favicon,
  isLoading,
  loadingLabel,
}: Pick<BrowserTab, 'favicon' | 'isLoading'> & { loadingLabel: string }) {
  const [failed, setFailed] = useState(false);

  useEffect(() => setFailed(false), [favicon]);

  if (isLoading) {
    return <LoaderCircle size={14} className="animate-spin" aria-label={loadingLabel} />;
  }
  if (favicon && !failed) {
    return (
      <img
        src={favicon}
        alt=""
        className="h-4 w-4 rounded-sm object-contain"
        onError={() => setFailed(true)}
      />
    );
  }
  return <Globe size={14} aria-hidden="true" />;
}

export function BrowserTabBar({
  tabs,
  activeTabId,
  canRestoreLastClosedTab,
  onSelectTab,
  onCloseTab,
  onNewTab,
  onReloadTab,
  onDuplicateTab,
  onRestoreLastClosedTab,
  onContextMenuOpenChange,
}: BrowserTabBarProps) {
  const { t } = useTranslation(['browser']);
  const [contextMenu, setContextMenu] = useState<TabContextMenu | null>(null);

  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
    onContextMenuOpenChange(false);
  }, [onContextMenuOpenChange]);

  useEffect(() => {
    if (!contextMenu) return;

    const closeOnPointerDown = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      if (!target?.closest('[data-browser-tab-context-menu]')) {
        closeContextMenu();
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') closeContextMenu();
    };

    document.addEventListener('pointerdown', closeOnPointerDown);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnPointerDown);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [closeContextMenu, contextMenu]);

  const openContextMenu = (event: MouseEvent<HTMLDivElement>, tab: BrowserTab) => {
    event.preventDefault();
    onSelectTab(tab.id);
    setContextMenu({
      tab,
      left: Math.max(
        VIEWPORT_GUTTER,
        Math.min(event.clientX, window.innerWidth - CONTEXT_MENU_WIDTH - VIEWPORT_GUTTER),
      ),
      top: Math.max(
        VIEWPORT_GUTTER,
        Math.min(event.clientY, window.innerHeight - CONTEXT_MENU_HEIGHT - VIEWPORT_GUTTER),
      ),
    });
    onContextMenuOpenChange(true);
  };

  const runTabAction = (action: (tabId: string) => void) => {
    const selectedTab = contextMenu?.tab;
    closeContextMenu();
    if (selectedTab) action(selectedTab.id);
  };

  return (
    <>
      <div
        className="browser-tab-strip flex h-11 shrink-0 items-end gap-1 overflow-x-auto overflow-y-hidden border-b border-base-content/10 bg-base-200/80 px-2.5 pt-1.5"
        role="tablist"
        aria-label={t('tabs.discover')}
      >
        {tabs.map((tab) => {
          const displayLabel = tabDisplayLabel(tab) ?? t('tabs.new_tab');
          const active = activeTabId === tab.id;
          const className = `
            group relative flex h-9 max-w-56 items-center gap-2 rounded-t-md border border-b-0 px-3 text-sm transition-[background-color,border-color,color] duration-150 focus-within:outline focus-within:outline-2 focus-within:outline-primary
            ${
              active
                ? '-mb-px border-base-content/15 bg-base-100 font-medium text-base-content'
                : 'border-transparent bg-transparent text-base-content/60 hover:bg-base-100/55 hover:text-base-content'
            }
          `;

          return (
            <div
              key={tab.id}
              className={className}
              onContextMenu={(event) => openContextMenu(event, tab)}
            >
              {active && (
                <span
                  className="pointer-events-none absolute inset-x-3 top-0 h-px rounded-full bg-primary"
                  aria-hidden="true"
                />
              )}
              <button
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => onSelectTab(tab.id)}
                className="flex min-w-0 flex-1 items-center gap-2 text-left focus-visible:outline-none"
              >
                <span className="grid h-4 w-4 shrink-0 place-items-center text-base-content/60">
                  <TabFavicon
                    favicon={tab.favicon}
                    isLoading={tab.isLoading}
                    loadingLabel={t('tabs.loading')}
                  />
                </span>
                <span className="flex-1 truncate" title={displayLabel}>
                  {displayLabel}
                </span>
              </button>
              <button
                type="button"
                aria-label={`${t('tabs.close')} ${displayLabel}`}
                className={`grid h-6 w-6 shrink-0 place-items-center rounded-md transition-[background-color,opacity] duration-150 hover:bg-base-content/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary ${
                  active
                    ? 'opacity-100'
                    : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100'
                }`}
                onClick={() => onCloseTab(tab.id)}
                onContextMenu={(event) => event.stopPropagation()}
              >
                <X size={12} aria-hidden="true" />
              </button>
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

      {contextMenu &&
        createPortal(
          <div
            data-browser-tab-context-menu
            data-testid="browser-tab-context-menu"
            className="fixed z-[var(--workspace-layer-popover)] w-56"
            style={{ left: contextMenu.left, top: contextMenu.top }}
          >
            <LiquidSurface liquidRole="overlay" className="rounded-box shadow-lg">
              <ul className="menu p-2" role="menu" aria-label={t('tabs.tab_menu')}>
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={!canRestoreLastClosedTab}
                    onClick={() => {
                      closeContextMenu();
                      onRestoreLastClosedTab();
                    }}
                  >
                    <Undo2 size={16} />
                    {t('tabs.reopen_last_closed_tab')}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={contextMenu.tab.isNewTab}
                    onClick={() => runTabAction(onReloadTab)}
                  >
                    <RotateCw size={16} />
                    {t('tabs.reload_tab')}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    onClick={() => runTabAction(onDuplicateTab)}
                  >
                    <Copy size={16} />
                    {t('tabs.duplicate_tab')}
                  </button>
                </li>
                <li>
                  <button type="button" role="menuitem" onClick={() => runTabAction(onCloseTab)}>
                    <X size={16} />
                    {t('tabs.close_tab')}
                  </button>
                </li>
              </ul>
            </LiquidSurface>
          </div>,
          document.body,
        )}
    </>
  );
}
