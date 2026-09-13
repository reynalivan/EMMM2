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
      <div className="h-full overflow-hidden bg-base-100/85 pt-[var(--workspace-topbar-height)]">
        {browser}
      </div>
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
      <div className="app-theme-background" aria-hidden="true" />
      <div className="app-theme-background-dim" aria-hidden="true" />
      {runtimeHosts}
      {topBar}
      <div data-testid="workspace-content" className="relative min-h-0 flex-1 overflow-hidden">
        <ErrorBoundary>{content}</ErrorBoundary>
      </div>
    </div>
  );
}
