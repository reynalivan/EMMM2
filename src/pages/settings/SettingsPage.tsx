import { formatAppError } from '../../shared/lib/appError';
import { Fragment, useEffect, useState, type ReactNode } from 'react';
import {
  Database,
  Gamepad2,
  Globe,
  Keyboard,
  ScrollText,
  Shield,
  SlidersHorizontal,
  Sparkles,
  PlugZap,
  Wrench,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { useAppStore } from '@/app/store';
import GamesTab from './components/tabs/GamesTab';
import PrivacyTab from './components/tabs/PrivacyTab';
import MaintenanceTab from './components/tabs/MaintenanceTab';
import GeneralTab from './components/tabs/GeneralTab';
import LogsTab from './components/tabs/LogsTab';
import AITab from './components/tabs/AITab';
import HotkeyTab from './components/tabs/HotkeyTab';
import BrowserTab from './components/tabs/BrowserTab';
import IntegrationsTab from './components/tabs/IntegrationsTab';
import CatalogTab from './components/tabs/CatalogTab';
import {
  WorkspacePageContent,
  WorkspacePageFrame,
} from '@/shared/ui/components/layout/WorkspacePageFrame';

const TABS = [
  { id: 'general', Component: GeneralTab, Icon: SlidersHorizontal },
  { id: 'games', Component: GamesTab, Icon: Gamepad2 },
  { id: 'catalog', Component: CatalogTab, Icon: Database },
  { id: 'browser', Component: BrowserTab, Icon: Globe },
  { id: 'privacy', Component: PrivacyTab, Icon: Shield },
  { id: 'hotkeys', Component: HotkeyTab, Icon: Keyboard },
  { id: 'ai', Component: AITab, Icon: Sparkles },
  { id: 'maintenance', Component: MaintenanceTab, Icon: Wrench },
  { id: 'integrations', Component: IntegrationsTab, Icon: PlugZap },
  { id: 'logs', Component: LogsTab, Icon: ScrollText, dividerBefore: true },
] as const;

type Tab = (typeof TABS)[number]['id'];

function resolveTab(tab: string): Tab {
  return TABS.some((candidate) => candidate.id === tab) ? (tab as Tab) : 'general';
}

export default function SettingsPage() {
  const { t } = useTranslation(['settings', 'common']);
  const requestedTab = useAppStore((state) => state.settingsTab);
  const savedTab = requestedTab as string;
  const persistActiveTab = useAppStore((state) => state.setSettingsTab);
  const { isLoading, error } = useSettings();
  const [activeTab, setActiveTab] = useState<Tab>(() => resolveTab(savedTab));

  useEffect(() => {
    if (savedTab !== 'updates') return;
    setActiveTab('general');
    persistActiveTab('general');
  }, [persistActiveTab, savedTab]);

  const handleTabChange = (tab: Tab) => {
    setActiveTab(tab);
    persistActiveTab?.(tab);
  };

  if (isLoading)
    return (
      <div className="p-10 pt-[calc(var(--workspace-topbar-height)+2.5rem)] text-center">
        {t('common:status.loading')}
      </div>
    );
  if (error)
    return (
      <div className="p-10 pt-[calc(var(--workspace-topbar-height)+2.5rem)] text-center text-error">
        {t('common:status.error')}: {formatAppError(error)}
      </div>
    );

  const ActiveTabComponent = TABS.find((tab) => tab.id === activeTab)?.Component ?? GeneralTab;

  return (
    <WorkspacePageFrame data-testid="settings-page">
      <div className="border-b border-base-300 px-4 pb-3 pt-[calc(var(--workspace-topbar-height)+0.75rem)] md:hidden">
        <label className="sr-only" htmlFor="settings-section-select">
          {t('page.title')}
        </label>
        <select
          id="settings-section-select"
          className="select select-sm select-bordered w-full bg-base-100"
          value={activeTab}
          onChange={(event) => handleTabChange(event.target.value as Tab)}
        >
          {TABS.map((tab) => (
            <option key={tab.id} value={tab.id}>
              {t(`tabs.${tab.id}`)}
            </option>
          ))}
        </select>
      </div>

      <div className="flex flex-1 min-h-0 overflow-hidden">
        <aside className="workspace-scroll-owner hidden w-52 shrink-0 flex-col overflow-y-auto border-r border-base-300 bg-base-200/15 px-2 pb-3 pt-[calc(var(--workspace-topbar-height)+0.75rem)] md:flex">
          <ul className="menu w-full gap-1 p-0">
            {TABS.map((tab) => (
              <Fragment key={tab.id}>
                {'dividerBefore' in tab && <div className="my-2 border-t border-base-300" />}
                <li>
                  <SettingsTabButton
                    active={activeTab === tab.id}
                    testId={`settings-tab-${tab.id}`}
                    label={t(`tabs.${tab.id}`)}
                    icon={<tab.Icon size={16} className="shrink-0" />}
                    onClick={() => handleTabChange(tab.id)}
                  />
                </li>
              </Fragment>
            ))}
          </ul>
        </aside>

        <main className="relative flex min-w-0 flex-1 flex-col">
          <WorkspacePageContent density="form" className="pb-7 sm:pb-9">
            <ActiveTabComponent />
          </WorkspacePageContent>
        </main>
      </div>
    </WorkspacePageFrame>
  );
}

function SettingsTabButton({
  active,
  testId,
  label,
  icon,
  onClick,
}: {
  active: boolean;
  testId: string;
  label: string;
  icon: ReactNode;
  onClick: () => void;
}) {
  const button = (
    <button
      data-testid={testId}
      aria-current={active ? 'page' : undefined}
      className={`relative flex h-9 w-full items-center gap-2.5 rounded-md px-3 text-sm transition-colors duration-150 ${
        active
          ? 'bg-base-content/7 font-medium text-base-content before:absolute before:inset-y-2 before:left-0 before:w-0.5 before:rounded-full before:bg-primary'
          : 'text-base-content/65 hover:bg-base-content/4 hover:text-base-content'
      }`}
      onClick={onClick}
    >
      {icon}
      {label}
    </button>
  );

  return button;
}
