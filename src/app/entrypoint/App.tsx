import { lazy, Suspense, type ReactNode, useEffect, useState } from 'react';
import { useLocation, useNavigate, Routes, Route, Navigate } from 'react-router-dom';
import { FaroRoutes } from '@grafana/faro-react';
import { useQueryClient } from '@tanstack/react-query';
import { initLogger } from '@/shared/lib/logger';
import { useAppStore } from '@/app/store';
import { useSettings } from '@/entities/settings';
import i18n from '@/shared/i18n/config';
import { DynamicThemeInjector } from '@/pages/settings/components/theme/DynamicThemeInjector';
import { useThemeRuntime } from '@/pages/settings/hooks/useThemeRuntime';
import type { PipelineTask } from '@/entities/task';
import { RecoveryDialog } from '@/pages/collections/components/RecoveryDialog';
import { WelcomeScreen } from '@/pages/onboarding';
import { commands } from '@/shared/api/tauri/bindings';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { setFrontendTelemetryEnabled } from '@/shared/lib/telemetry';
import { dismissSplash } from '@/shared/lib/dismissSplash';
import { isDemoMode } from '@/shared/lib/appMode';
import { DiagnosticsErrorDialog } from '@/shared/ui/components/ui/DiagnosticsErrorDialog';
import { CrashRecoveryDialog } from '@/shared/ui/components/ui/CrashRecoveryDialog';
import { AppShell } from '@/widgets/app-shell';
import { TopBar } from '@/widgets/top-bar';
import CollectionContextControls from '@/pages/collections/components/CollectionContextControls';
import { ExternalChangeHandler } from '@/features/file-watcher';
import { ImportBatchWizardHost } from '@/features/import-batches';
import { ObjectClassificationWizardHost } from '@/features/match-wizard';
import { LaunchBar } from '@/widgets/launch-bar';
import { DownloadConfirmationHost } from '@/pages/browser/components/DownloadConfirmationHost';
import FolderConflictManager from '@/widgets/mod-explorer/modals/FolderConflictManager';
import RenameConfirmationManager from '@/widgets/mod-explorer/modals/RenameConfirmationManager';
import WorkspaceSourceUnavailableDialog from '@/widgets/mod-explorer/components/WorkspaceSourceUnavailableDialog';

const Dashboard = lazy(() => import('@/pages/dashboard/Dashboard'));
const CollectionsPage = lazy(() => import('@/pages/collections/CollectionsPage'));
const SettingsPage = lazy(() => import('@/pages/settings/SettingsPage'));
const ModInboxPage = lazy(() => import('@/pages/mod-inbox/ModInboxPage'));
const StorageOptimizerPage = lazy(() => import('@/features/scanner/StorageOptimizerPage'));
const BrowserPage = lazy(() =>
  import('@/pages/browser/components/BrowserPage').then(({ BrowserPage: Component }) => ({
    default: Component,
  })),
);
const DownloadsPage = lazy(() => import('@/pages/browser/components/DownloadsPage'));
const ObjectList = lazy(() => import('@/widgets/object-sidebar/ObjectList'));
const FolderGrid = lazy(() => import('@/widgets/mod-explorer/FolderGrid'));
const PreviewPanel = lazy(() => import('@/widgets/mod-preview/PreviewPanel'));
const ExplorerEmptyState = lazy(
  () => import('@/widgets/mod-explorer/components/ExplorerEmptyState'),
);

function WorkspaceContentFallback() {
  return <div className="h-full" aria-busy="true" />;
}

function deferWorkspaceContent(content: ReactNode) {
  return <Suspense fallback={<WorkspaceContentFallback />}>{content}</Suspense>;
}

function AppRouter() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [pendingTasks, setPendingTasks] = useState<PipelineTask[]>([]);
  const [isCheckingRecovery, setIsCheckingRecovery] = useState(true);

  useEffect(() => {
    if (isDemoMode) {
      if (location.pathname !== '/welcome') {
        navigate('/dashboard', { replace: true });
      }
      dismissSplash();
      setIsCheckingRecovery(false);
      return;
    }

    initLogger().catch(console.error);

    // Passive startup must not rename anything on disk.
    // Only recovery resume or apply_collection may perform physical renames.
    // Disk Reconcile at boot is read/projection-only.
    // Run recovery check first
    commands
      .appStartupCheck()
      .then((tasks: PipelineTask[]) => {
        if (tasks && (tasks as PipelineTask[]).length > 0) {
          setPendingTasks(tasks as PipelineTask[]);
        } else {
          checkConfigStatus();
        }
      })
      .catch((e: unknown) => {
        console.error('Failed recovery check:', e);
        checkConfigStatus(); // fallback
      })
      .finally(() => {
        setIsCheckingRecovery(false);
      });

    function checkConfigStatus() {
      // Check config status
      commands
        .checkConfigStatus()
        .then((configStatus) => {
          if (configStatus !== 'HasConfig') {
            navigate('/welcome', { replace: true });
            dismissSplash();
          } else {
            useAppStore
              .getState()
              .initStore()
              .then(() => {
                navigate('/dashboard', { replace: true });
              })
              .catch((e) => {
                console.error('Failed to init store:', e);
                navigate('/dashboard', { replace: true });
              })
              .finally(() => {
                dismissSplash();
              });
          }
        })
        .catch(() => {
          // Fallback for frontend-only dev mode
          console.warn('Backend not detected, defaulting to Welcome');
          navigate('/welcome', { replace: true });
          dismissSplash();
        });
    }
  }, [location.pathname, navigate]);

  if (isCheckingRecovery) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-base-100">
        <span className="loading loading-spinner text-primary loading-lg"></span>
      </div>
    );
  }

  if (pendingTasks.length > 0) {
    return (
      <div className="h-screen w-screen bg-base-100 overflow-hidden relative">
        <RecoveryDialog
          tasks={pendingTasks}
          onResolved={(remainingTasks) => {
            setPendingTasks(remainingTasks);
            if (remainingTasks.length > 0) {
              return;
            }

            navigate('/dashboard', { replace: true });
            void useAppStore.getState().initStore();
          }}
        />
      </div>
    );
  }

  return (
    <FaroRoutes routesComponent={Routes}>
      <Route
        path="/welcome"
        element={
          <WelcomeScreen
            onComplete={async (games) => {
              if (games && games.length > 0) {
                await useAppStore.getState().setActiveGameId(games[0].id);
              }
              await publishQueryScopes(queryClient, ['settings', 'dashboard']);
              await useAppStore.getState().initStore();
              navigate('/dashboard', { replace: true });
            }}
          />
        }
      />
      <Route path="/dashboard" element={<DashboardWorkspace />} />
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </FaroRoutes>
  );
}

function DashboardWorkspace() {
  const workspaceView = useAppStore((state) => state.workspaceView);
  const selectedObjectFolderPath = useAppStore((state) => state.selectedObjectFolderPath);

  return (
    <AppShell
      workspaceView={workspaceView}
      selectedObjectFolderPath={selectedObjectFolderPath}
      topBar={<TopBar launchBar={<LaunchBar />} contextControls={<CollectionContextControls />} />}
      runtimeHosts={
        isDemoMode ? undefined : (
          <>
            <ExternalChangeHandler />
            <ImportBatchWizardHost />
            <ObjectClassificationWizardHost />
          </>
        )
      }
      dashboard={deferWorkspaceContent(<Dashboard />)}
      collections={deferWorkspaceContent(<CollectionsPage />)}
      settings={deferWorkspaceContent(<SettingsPage />)}
      browser={deferWorkspaceContent(<BrowserPage />)}
      downloads={deferWorkspaceContent(<DownloadsPage />)}
      storageOptimizer={deferWorkspaceContent(<StorageOptimizerPage />)}
      modInbox={deferWorkspaceContent(<ModInboxPage />)}
      objectList={deferWorkspaceContent(<ObjectList />)}
      folderGrid={deferWorkspaceContent(<FolderGrid />)}
      previewPanel={deferWorkspaceContent(<PreviewPanel />)}
      explorerEmptyState={deferWorkspaceContent(<ExplorerEmptyState />)}
    />
  );
}

import { ToastContainer } from '@/shared/ui/toast';
import { FileInUseDialog } from '@/features/file-watcher';
import { WorkspaceParentEnableDialogHost } from '@/features/workspace-runtime';

export default function App() {
  useThemeRuntime();
  const { settings } = useSettings();

  useEffect(() => {
    if (settings?.language && i18n.language !== settings.language) {
      i18n.changeLanguage(settings.language).catch(console.error);
    }
  }, [settings?.language]);

  useEffect(() => {
    setFrontendTelemetryEnabled(settings?.diagnostics?.telemetry_enabled ?? false);
  }, [settings?.diagnostics?.telemetry_enabled]);

  return (
    <div className="flex flex-col h-screen bg-base-100 text-base-content overflow-hidden font-sans antialiased selection:bg-primary selection:text-primary-content">
      <AppRouter />
      <DynamicThemeInjector />
      <ToastContainer />
      {!isDemoMode && (
        <>
          <DownloadConfirmationHost />
          <FolderConflictManager />
          <RenameConfirmationManager />
          <FileInUseDialog />
          <WorkspaceSourceUnavailableDialog />
          <WorkspaceParentEnableDialogHost />
          <DiagnosticsErrorDialog />
          <CrashRecoveryDialog />
        </>
      )}
    </div>
  );
}
