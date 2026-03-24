import { useState } from 'react';
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

type Tab =
  | 'general'
  | 'games'
  | 'browser'
  | 'privacy'
  | 'hotkeys'
  | 'ai'
  | 'maintenance'
  | 'updates'
  | 'logs';

export default function SettingsPage() {
  const { t } = useTranslation(['settings', 'common']);
  const { setWorkspaceView } = useAppStore();
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
        {t('common:status.error')}: {String(error)}
      </div>
    );

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
            <li>
              <button
                className={activeTab === 'general' ? 'active' : ''}
                onClick={() => setActiveTab('general')}
              >
                {t('tabs.general')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'games' ? 'active' : ''}
                onClick={() => setActiveTab('games')}
              >
                {t('tabs.games')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'browser' ? 'active' : ''}
                onClick={() => setActiveTab('browser')}
              >
                {t('tabs.browser')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'privacy' ? 'active' : ''}
                onClick={() => setActiveTab('privacy')}
              >
                {t('tabs.privacy')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'hotkeys' ? 'active' : ''}
                onClick={() => setActiveTab('hotkeys')}
              >
                {t('tabs.hotkeys')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'ai' ? 'active' : ''}
                onClick={() => setActiveTab('ai')}
              >
                {t('tabs.ai')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'maintenance' ? 'active' : ''}
                onClick={() => setActiveTab('maintenance')}
              >
                {t('tabs.maintenance')}
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'updates' ? 'active' : ''}
                onClick={() => setActiveTab('updates')}
              >
                {t('tabs.updates')}
              </button>
            </li>
            <div className="divider my-1"></div>
            <li>
              <button
                className={activeTab === 'logs' ? 'active' : ''}
                onClick={() => setActiveTab('logs')}
              >
                {t('tabs.logs')}
              </button>
            </li>
          </ul>
        </aside>

        {/* Content Area */}
        <main className="flex-1 overflow-y-auto bg-base-100 relative">
          <div className="max-w-4xl mx-auto p-6">
            {activeTab === 'general' && <GeneralTab />}
            {activeTab === 'games' && <GamesTab />}
            {activeTab === 'browser' && <BrowserTab />}
            {activeTab === 'privacy' && <PrivacyTab />}
            {activeTab === 'hotkeys' && <HotkeyTab />}
            {activeTab === 'ai' && <AITab />}
            {activeTab === 'maintenance' && <MaintenanceTab />}
            {activeTab === 'updates' && <UpdateTab />}
            {activeTab === 'logs' && <LogsTab />}
          </div>
        </main>
      </div>
    </div>
  );
}
