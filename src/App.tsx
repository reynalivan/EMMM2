import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useNavigate, Routes, Route, Navigate } from 'react-router-dom';
import { initLogger } from './lib/logger';
import { useAppStore } from './stores/useAppStore';

// Components
import MainLayout from './components/layout/MainLayout';
import WelcomeScreen from './features/onboarding/WelcomeScreen';

function AppRouter() {
  const navigate = useNavigate();

  useEffect(() => {
    initLogger().catch(console.error);

    // Check config status
    invoke('check_config_status')
      .then((status) => {
        if (status !== 'HasConfig') {
          navigate('/welcome', { replace: true });
        } else {
          navigate('/dashboard', { replace: true });
        }
      })
      .catch(() => {
        // Fallback for frontend-only dev mode
        console.warn('Backend not detected, defaulting to Dashboard');
        navigate('/dashboard', { replace: true });
      });

    // Initialize the store (load config.json)
    useAppStore.getState().initStore();
  }, [navigate]);

  return (
    <Routes>
      <Route
        path="/welcome"
        element={<WelcomeScreen onComplete={() => navigate('/dashboard')} />}
      />
      <Route path="/dashboard" element={<MainLayout />} />
      <Route path="*" element={<Navigate to="/dashboard" replace />} />
    </Routes>
  );
}

import { ToastContainer } from './components/ui/Toast';

export default function App() {
  return (
    <div className="flex flex-col h-screen bg-base-100 text-base-content overflow-hidden font-sans antialiased selection:bg-primary selection:text-primary-content">
      <AppRouter />
      <ToastContainer />
    </div>
  );
}
