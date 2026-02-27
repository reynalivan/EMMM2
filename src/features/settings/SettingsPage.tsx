import { useState } from 'react';
import { ArrowLeft } from 'lucide-react';
import { useSettings } from '../../hooks/useSettings';
import { useAppStore } from '../../stores/useAppStore'; // Import Store
import GamesTab from './tabs/GamesTab';
import PrivacyTab from './tabs/PrivacyTab';
import MaintenanceTab from './tabs/MaintenanceTab';
import GeneralTab from './tabs/GeneralTab';
import LogsTab from './tabs/LogsTab';
import AITab from './tabs/AITab';
import UpdateTab from './tabs/UpdateTab';

type Tab = 'general' | 'games' | 'privacy' | 'ai' | 'maintenance' | 'updates' | 'logs';

export default function SettingsPage() {
  const [activeTab, setActiveTab] = useState<Tab>('general');
  const { setWorkspaceView } = useAppStore(); // Use Store Action
  const { isLoading, error } = useSettings();

  const handleBack = () => {
    // Close Settings View and return to Dashboard
    setWorkspaceView('dashboard');
  };

  if (isLoading) return <div className="p-10 text-center">Loading settings...</div>;
  if (error)
    return (
      <div className="p-10 text-center text-error">Error loading settings: {String(error)}</div>
    );

  return (
    <div className="h-full flex flex-col bg-base-100 overflow-hidden">
      <div className="navbar bg-base-200 min-h-12 px-4 border-b border-base-300 gap-4">
        <button className="btn btn-ghost btn-circle btn-sm" onClick={handleBack}>
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-bold flex-1">Settings</h2>
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
                General
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'games' ? 'active' : ''}
                onClick={() => setActiveTab('games')}
              >
                Games
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'privacy' ? 'active' : ''}
                onClick={() => setActiveTab('privacy')}
              >
                Privacy & Safe Mode
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'ai' ? 'active' : ''}
                onClick={() => setActiveTab('ai')}
              >
                AI Configuration
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'maintenance' ? 'active' : ''}
                onClick={() => setActiveTab('maintenance')}
              >
                Maintenance
              </button>
            </li>
            <li>
              <button
                className={activeTab === 'updates' ? 'active' : ''}
                onClick={() => setActiveTab('updates')}
              >
                Updates
              </button>
            </li>
            <div className="divider my-1"></div>
            <li>
              <button
                className={activeTab === 'logs' ? 'active' : ''}
                onClick={() => setActiveTab('logs')}
              >
                Logs
              </button>
            </li>
          </ul>
        </aside>

        {/* Content Area */}
        <main className="flex-1 overflow-y-auto bg-base-100 relative">
          <div className="max-w-4xl mx-auto p-6">
            {activeTab === 'general' && <GeneralTab />}
            {activeTab === 'games' && <GamesTab />}
            {activeTab === 'privacy' && <PrivacyTab />}
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
