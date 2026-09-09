import type { ReactNode } from 'react';
import { ErrorBoundary } from '@/shared/ui/components/ui/ErrorBoundary';
import ResizableWorkspace from './ResizableWorkspace';

export type AppShellView =
  | 'dashboard'
  | 'collections'
  | 'settings'
  | 'browser'
  | 'downloads'
  | 'storage-optimizer'
  | 'mod-inbox'
  | 'mods';

export interface AppShellProps {
  workspaceView?: AppShellView;
  selectedObjectFolderPath?: string | null;
  topBar?: ReactNode;
  runtimeHosts?: ReactNode;
  dashboard?: ReactNode;
  collections?: ReactNode;
  settings?: ReactNode;
  browser?: ReactNode;
  downloads?: ReactNode;
  storageOptimizer?: ReactNode;
  modInbox?: ReactNode;
  objectList?: ReactNode;
  folderGrid?: ReactNode;
  previewPanel?: ReactNode;
  explorerEmptyState?: ReactNode;
}

export default function AppShell({
  workspaceView = 'dashboard',
  selectedObjectFolderPath = null,
  topBar,
  runtimeHosts,
  dashboard,
  collections,
  settings,
  browser,
  downloads,
  storageOptimizer,
  modInbox,
  objectList,
  folderGrid,
  previewPanel,
  explorerEmptyState,
}: AppShellProps) {
  const content =
    workspaceView === 'dashboard' ? (
      dashboard
    ) : workspaceView === 'collections' ? (
      collections
    ) : workspaceView === 'settings' ? (
      settings
    ) : workspaceView === 'browser' ? (
      <div className="h-full bg-base-100 overflow-hidden relative">{browser}</div>
    ) : workspaceView === 'downloads' ? (
      downloads
    ) : workspaceView === 'storage-optimizer' ? (
      storageOptimizer
    ) : workspaceView === 'mod-inbox' ? (
      modInbox
    ) : (
      <ResizableWorkspace
        leftPanel={objectList}
        mainPanel={
          selectedObjectFolderPath ? (
            <ErrorBoundary>{folderGrid}</ErrorBoundary>
          ) : (
            explorerEmptyState
          )
        }
        rightPanel={previewPanel}
      />
    );

  return (
    <div
      data-testid="dashboard-layout"
      data-workspace-view={workspaceView}
      className="flex flex-col h-screen overflow-hidden bg-base-100 font-sans text-base-content selection:bg-primary/20 relative"
    >
      {runtimeHosts}
      {topBar}
      <div className="flex-1 min-h-0 relative">
        <ErrorBoundary>{content}</ErrorBoundary>
      </div>
    </div>
  );
}
