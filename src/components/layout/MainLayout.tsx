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
import { useAppStore } from '../../stores/useAppStore';
import { ExternalChangeHandler } from '../../features/file-watcher/ExternalChangeHandler';
import { ErrorBoundary } from '../ui/ErrorBoundary';

export default function MainLayout() {
  const { workspaceView, selectedObject } = useAppStore();

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
          ) : (
            <ResizableWorkspace
              leftPanel={<ObjectList />}
              mainPanel={
                selectedObject ? (
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
