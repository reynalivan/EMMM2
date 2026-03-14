import TopBar from './top-bar/index';
import ResizableWorkspace from './ResizableWorkspace';
import Dashboard from '../../features/dashboard/Dashboard';
import ObjectList from '../../features/object-list/ObjectList';
import FolderGrid from '../../features/folder-grid/FolderGrid';
import PreviewPanel from '../../features/preview/PreviewPanel';
import SettingsPage from '../../features/settings/SettingsPage';
import CollectionsPage from '../../features/collections/CollectionsPage';
import ExplorerEmptyState from '../../features/folder-grid/ExplorerEmptyState';
import { BrowserPage } from '../../features/browser/components/BrowserPage';
import DownloadsPage from '../../features/downloads/DownloadsPage';
import { useAppStore } from '../../stores/useAppStore';
import { ExternalChangeHandler } from '../../features/file-watcher/ExternalChangeHandler';
import { ErrorBoundary } from '../ui/ErrorBoundary';

export default function MainLayout() {
  const { workspaceView, selectedObjectFolderPath } = useAppStore();

  return (
    <div
      data-testid="dashboard-layout"
      className="flex flex-col h-screen overflow-hidden bg-base-100 font-sans text-base-content selection:bg-primary/20 relative"
    >
      <ExternalChangeHandler />

      {/* Top Navigation Bar */}
      <TopBar />

      {/* Main Workspace Area */}
      <div className="flex-1 min-h-0 relative">
        <ErrorBoundary>
          {workspaceView === 'dashboard' ? (
            <Dashboard />
          ) : workspaceView === 'collections' ? (
            <CollectionsPage />
          ) : workspaceView === 'settings' ? (
            <SettingsPage />
          ) : workspaceView === 'browser' ? (
            <BrowserPage />
          ) : workspaceView === 'downloads' ? (
            <DownloadsPage />
          ) : (
            <ResizableWorkspace
              leftPanel={<ObjectList />}
              mainPanel={
                selectedObjectFolderPath ? (
                  <ErrorBoundary>
                    <FolderGrid />
                  </ErrorBoundary>
                ) : (
                  <ExplorerEmptyState />
                )
              }
              rightPanel={<PreviewPanel />}
            />
          )}
        </ErrorBoundary>
      </div>
    </div>
  );
}
