import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useNavigate, Routes, Route, Navigate } from 'react-router-dom';
import { Loader2 } from 'lucide-react';
import { initLogger } from './lib/logger';
import { useAppStore } from './stores/appStore';
import WelcomeScreen from './features/onboarding/WelcomeScreen';
import DashboardPlaceholder from './features/dashboard/DashboardPlaceholder';

function AppRouter() {
  const navigate = useNavigate();
  const { isLoading, setLoading, setFirstRun } = useAppStore();

  useEffect(() => {
    const checkStartup = async () => {
      try {
        await initLogger();

        const status = await invoke<string>('check_config_status');

        if (status === 'HasConfig') {
          setFirstRun(false);
          navigate('/dashboard', { replace: true });
        } else {
          setFirstRun(true);
          navigate('/welcome', { replace: true });
        }
      } catch (err) {
        console.error('Startup check failed:', err);
        navigate('/welcome', { replace: true });
      } finally {
        setLoading(false);
      }
    };

    checkStartup();
  }, [navigate, setLoading, setFirstRun]);

  if (isLoading) {
    return (
      <div
        className="min-h-screen bg-base-100 flex items-center justify-center"
        data-theme="darker"
      >
        <div className="text-center space-y-4">
          <Loader2 className="w-12 h-12 text-primary animate-spin mx-auto" />
          <p className="text-base-content/60">Loading EMMM2...</p>
        </div>
      </div>
    );
  }

  return (
    <Routes>
      <Route
        path="/welcome"
        element={<WelcomeScreen onComplete={() => navigate('/dashboard', { replace: true })} />}
      />
      <Route path="/dashboard" element={<DashboardPlaceholder />} />
      <Route path="*" element={<Navigate to="/welcome" replace />} />
    </Routes>
  );
}

export default function App() {
  return (
    <div data-theme="darker" className="min-h-screen">
      <AppRouter />
    </div>
  );
}
