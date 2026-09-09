import { useEffect, useState } from 'react';
import { useNavigate, Routes, Route, Navigate } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import { initLogger } from '@/shared/lib/logger';
import { useAppStore } from '@/app/store';
import { useSettings } from '@/entities/settings';
import i18n from '@/shared/i18n/config';
import { useThemeRuntime, DynamicThemeInjector } from '@/pages/settings';
import type { PipelineTask } from '@/entities/task';
import { RecoveryDialog } from '@/pages/collections';
import { WelcomeScreen } from '@/pages/onboarding';
import { commands } from '@/shared/api/tauri/bindings';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { AppShell } from '@/widgets/app-shell';
import { TopBar } from '@/widgets/top-bar';
import { Dashboard } from '@/pages/dashboard';
import { CollectionContextControls, CollectionsPage } from '@/pages/collections';
import { SettingsPage } from '@/pages/settings';
import { ModInboxPage } from '@/pages/mod-inbox';
import { StorageOptimizerPage } from '@/features/scanner';
import { ExternalChangeHandler } from '@/features/file-watcher';
import { ImportBatchWizardHost } from '@/features/import-batches';
import { ObjectClassificationWizardHost } from '@/features/match-wizard';
import { FolderGrid, ExplorerEmptyState } from '@/widgets/mod-explorer';
import { PreviewPanel } from '@/widgets/mod-preview';
import { ObjectList } from '@/widgets/object-sidebar';
import { LaunchBar } from '@/widgets/launch-bar';
import { BrowserPage, DownloadConfirmationHost, DownloadsPage } from '@/pages/browser';

/** Duration of the splash fade-out; must match the `#splash` transition in `index.html`. */
const SPLASH_FADE_MS = 220;

/**
 * Fades out and removes the boot splash that `index.html` paints before React
 * mounts. Removal is on a timer rather than `transitionend` so a skipped
 * transition (reduced motion, backgrounded window) can never strand the
 * overlay on top of the app.
 */
function dismissSplash() {
  const splash = document.getElementById('splash');
  if (!splash) return;
  splash.classList.add('is-done');
  setTimeout(() => splash.remove(), SPLASH_FADE_MS);
}

function AppRouter() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [pendingTasks, setPendingTasks] = useState<PipelineTask[]>([]);
  const [isCheckingRecovery, setIsCheckingRecovery] = useState(true);

  useEffect(() => {
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

      // Epic 12: Silent background metadata sync on startup
      commands
        .checkMetadataUpdate()
        .catch((e: unknown) => console.warn('Metadata sync skipped:', e));
    }
  }, [navigate]);

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
    <Routes>
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
    </Routes>
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
        <>
          <ExternalChangeHandler />
          <ImportBatchWizardHost />
          <ObjectClassificationWizardHost />
        </>
      }
      dashboard={<Dashboard />}
      collections={<CollectionsPage />}
      settings={<SettingsPage />}
      browser={<BrowserPage />}
      downloads={<DownloadsPage />}
      storageOptimizer={<StorageOptimizerPage />}
      modInbox={<ModInboxPage />}
      objectList={<ObjectList />}
      folderGrid={<FolderGrid />}
      previewPanel={<PreviewPanel />}
      explorerEmptyState={<ExplorerEmptyState />}
    />
  );
}

import { ToastContainer } from '@/shared/ui/toast';
import { FileInUseDialog } from '@/features/file-watcher';
import {
  FolderConflictManager,
  RenameConfirmationManager,
  WorkspaceSourceUnavailableDialog,
} from '@/widgets/mod-explorer';

export default function App() {
  useThemeRuntime();
  const { settings } = useSettings();

  useEffect(() => {
    if (settings?.language && i18n.language !== settings.language) {
      i18n.changeLanguage(settings.language).catch(console.error);
    }
  }, [settings?.language]);

  return (
    <div className="flex flex-col h-screen bg-base-100 text-base-content overflow-hidden font-sans antialiased selection:bg-primary selection:text-primary-content">
      <AppRouter />
      <DownloadConfirmationHost />
      <DynamicThemeInjector />
      <ToastContainer />
      <FolderConflictManager />
      <RenameConfirmationManager />
      <FileInUseDialog />
      <WorkspaceSourceUnavailableDialog />
    </div>
  );
}
