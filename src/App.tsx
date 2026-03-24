import { useEffect, useState } from 'react';
import { useNavigate, Routes, Route, Navigate } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import { initLogger } from './lib/logger';
import { useAppStore } from './stores/useAppStore';
import { settingsKeys } from './hooks/useSettings';
import { useThemeRuntime } from './features/settings/theme/useThemeRuntime';
import type { PipelineTask } from './types/task';
import { RecoveryDialog } from './features/collections/components/RecoveryDialog';
import PinEntryModal from './features/safe-mode/PinEntryModal';
import MainLayout from './components/layout/MainLayout';
import WelcomeScreen from './features/onboarding/WelcomeScreen';
import { commands } from './lib/bindings';
import { useTranslation } from 'react-i18next';

function AppRouter() {
  const { t } = useTranslation('layout');
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [pendingTasks, setPendingTasks] = useState<PipelineTask[]>([]);
  const [isCheckingRecovery, setIsCheckingRecovery] = useState(true);
  const [needsPinUnlock, setNeedsPinUnlock] = useState(false);

  useEffect(() => {
    initLogger().catch(console.error);

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
        .then((status: unknown) => {
          if (status !== 'HasConfig') {
            navigate('/welcome', { replace: true });
          } else {
            // Epic 5/Safe Mode: Check for Safe Mode GUI lock on boot
            useAppStore
              .getState()
              .initStore()
              .then(async () => {
                const state = useAppStore.getState();
                if (!state.safeMode) {
                  try {
                    const hasPin = await commands.hasPin();
                    if (hasPin) {
                      setNeedsPinUnlock(true);
                      return; // Don't navigate to dashboard yet
                    }
                  } catch (e) {
                    console.error('Failed to check PIN status on boot:', e);
                  }
                }
                navigate('/dashboard', { replace: true });
              });
          }
        })
        .catch(() => {
          // Fallback for frontend-only dev mode
          console.warn('Backend not detected, defaulting to Dashboard');
          navigate('/dashboard', { replace: true });
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
          onResolved={() => {
            setPendingTasks([]);
            navigate('/dashboard', { replace: true });
            useAppStore.getState().initStore();
          }}
        />
      </div>
    );
  }

  if (needsPinUnlock) {
    return (
      <div className="h-screen w-screen bg-base-100 overflow-hidden relative">
        <PinEntryModal
          open={true}
          onClose={() => {}}
          onSuccess={() => {
            setNeedsPinUnlock(false);
            navigate('/dashboard', { replace: true });
          }}
          title={t('app_locked_title')}
          description={t('app_locked_description')}
          cancellable={false}
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
              // Force React Query to refetch settings so GameSelector sees new games
              await queryClient.invalidateQueries({ queryKey: settingsKeys.all });
              // Also bust the dashboard-stats cache (30s stale) so the dashboard
              // doesn't render the empty state using pre-onboarding cached data.
              await queryClient.invalidateQueries({ queryKey: ['dashboard-stats'] });
              await useAppStore.getState().initStore();
              navigate('/dashboard', { replace: true });
            }}
          />
        }
      />
      <Route path="/dashboard" element={<MainLayout />} />
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  );
}

import { useLanguageRuntime } from './features/settings/hooks/useLanguageRuntime';
import { ToastContainer } from './components/ui/Toast';
import ConflictResolveDialog from './features/folder-grid/ConflictResolveDialog';
import { DynamicThemeInjector } from './features/settings/theme/DynamicThemeInjector';
import { FileInUseDialog } from './components/dialogs/FileInUseDialog';


export default function App() {
  useThemeRuntime();
  useLanguageRuntime();

  return (
    <div className="flex flex-col h-screen bg-base-100 text-base-content overflow-hidden font-sans antialiased selection:bg-primary selection:text-primary-content">
      <AppRouter />
      <DynamicThemeInjector />
      <ToastContainer />
      <ConflictResolveDialog />
      <FileInUseDialog />
    </div>

  );
}
