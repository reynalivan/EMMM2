import TopBar from './top-bar';
import ResizableWorkspace from './ResizableWorkspace';
import Dashboard from '@/pages/dashboard/Dashboard';
import ObjectList from '@/widgets/object-sidebar/ObjectList';
import FolderGrid from '@/widgets/mod-explorer/FolderGrid';
import PreviewPanel from '@/widgets/mod-preview/PreviewPanel';
import SettingsPage from '@/pages/settings/SettingsPage';
import CollectionsPage from '@/pages/collections/CollectionsPage';
import StorageOptimizerPage from '@/features/scanner/StorageOptimizerPage';
import ExplorerEmptyState from '@/widgets/mod-explorer/components/ExplorerEmptyState';
import { BrowserPage } from '@/pages/browser/components/BrowserPage';
import DownloadsPage from '@/pages/browser/components/DownloadsPage';
import { useAppStore } from '@/app/store/useAppStore';
import { ExternalChangeHandler } from '@/features/file-watcher/ExternalChangeHandler';
import { ErrorBoundary } from '../ui/ErrorBoundary';
import { ImportBatchWizardHost } from '@/features/import-batches/ImportBatchWizardHost';
import { ObjectClassificationWizardHost } from '@/features/match-wizard/components/ObjectClassificationWizardHost';
import ModInboxPage from '@/pages/mod-inbox/ModInboxPage';

export default function MainLayout() {
  const workspaceView = useAppStore((state) => state.workspaceView);
  const selectedObjectFolderPath = useAppStore((state) => state.selectedObjectFolderPath);

  return (
    <div
      data-testid="dashboard-layout"
      data-workspace-view={workspaceView}
      className="flex flex-col h-screen overflow-hidden bg-base-100 font-sans text-base-content selection:bg-primary/20 relative"
    >
      <ExternalChangeHandler />
      <ImportBatchWizardHost />
      <ObjectClassificationWizardHost />

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
            <div className="h-full bg-base-100 overflow-hidden relative">
              <BrowserPage />
            </div>
          ) : workspaceView === 'downloads' ? (
            <DownloadsPage />
          ) : workspaceView === 'storage-optimizer' ? (
            <StorageOptimizerPage />
          ) : workspaceView === 'mod-inbox' ? (
            <ModInboxPage />
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
