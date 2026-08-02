import { formatAppError } from '../../lib/appError';
import { Fragment, useState } from 'react';
import { ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../hooks/useSettings';
import { useAppStore } from '../../stores/useAppStore'; // Import Store
import GamesTab from './tabs/GamesTab';
import PrivacyTab from './tabs/PrivacyTab';
import MaintenanceTab from './tabs/MaintenanceTab';
import GeneralTab from './tabs/GeneralTab';
import LogsTab from './tabs/LogsTab';
import AITab from './tabs/AITab';
import UpdateTab from './tabs/UpdateTab';
import HotkeyTab from './tabs/HotkeyTab';
import BrowserTab from './tabs/BrowserTab';

// `dividerBefore` keeps the visual break above Logs without a second array.
const TABS = [
  { id: 'general', Component: GeneralTab },
  { id: 'games', Component: GamesTab },
  { id: 'browser', Component: BrowserTab },
  { id: 'privacy', Component: PrivacyTab },
  { id: 'hotkeys', Component: HotkeyTab },
  { id: 'ai', Component: AITab },
  { id: 'maintenance', Component: MaintenanceTab },
  { id: 'updates', Component: UpdateTab },
  { id: 'logs', Component: LogsTab, dividerBefore: true },
] as const;

type Tab = (typeof TABS)[number]['id'];

export default function SettingsPage() {
  const { t } = useTranslation(['settings', 'common']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const { isLoading, error } = useSettings();
  const [activeTab, setActiveTab] = useState<Tab>('general');

  const handleBack = () => {
    // Close Settings View and return to Dashboard
    setWorkspaceView('dashboard');
  };

  if (isLoading) return <div className="p-10 text-center">{t('common:status.loading')}</div>;
  if (error)
    return (
      <div className="p-10 text-center text-error">
        {t('common:status.error')}: {formatAppError(error)}
      </div>
    );

  const ActiveTabComponent = TABS.find((tab) => tab.id === activeTab)?.Component ?? GeneralTab;

  return (
    <div className="h-full flex flex-col bg-base-100 overflow-hidden">
      <div className="navbar bg-base-200 min-h-12 px-4 border-b border-base-300 gap-4">
        <button className="btn btn-ghost btn-circle btn-sm" onClick={handleBack}>
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-bold flex-1">{t('page.title')}</h2>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar Navigation for Settings */}
        <aside className="w-64 bg-base-200/50 flex flex-col border-r border-base-300 overflow-y-auto">
          <ul className="menu menu-lg w-full p-2 gap-1.5">
            {TABS.map((tab) => (
              <Fragment key={tab.id}>
                {'dividerBefore' in tab && <div className="divider my-1"></div>}
                <li>
                  <button
                    className={activeTab === tab.id ? 'active' : ''}
                    onClick={() => setActiveTab(tab.id)}
                  >
                    {t(`tabs.${tab.id}`)}
                  </button>
                </li>
              </Fragment>
            ))}
          </ul>
        </aside>

        {/* Content Area */}
        <main className="flex-1 overflow-y-auto bg-base-100 relative">
          <div className="max-w-4xl mx-auto p-6">
            <ActiveTabComponent />
          </div>
        </main>
      </div>
    </div>
  );
}
