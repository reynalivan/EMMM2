import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Gamepad2, Search, FolderOpen, ChevronRight, Loader2, AlertCircle } from 'lucide-react';
import ManualSetupForm from './ManualSetupForm';
import AutoDetectResult from './AutoDetectResult';

interface GameConfig {
  id: string;
  name: string;
  game_type: string;
  path: string;
  mods_path: string;
  launcher_path: string;
  launch_args: string | null;
}

type Screen = 'welcome' | 'auto-detect' | 'manual' | 'result';

export default function WelcomeScreen({ onComplete }: { onComplete: () => void }) {
  const [screen, setScreen] = useState<Screen>('welcome');
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [detectedGames, setDetectedGames] = useState<GameConfig[]>([]);

  const handleAutoDetect = async () => {
    setError(null);
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Select your XXMI Launcher folder',
      });

      if (!selectedPath) return;

      setIsScanning(true);
      setScreen('auto-detect');

      const games = await invoke<GameConfig[]>('auto_detect_games', {
        rootPath: selectedPath,
      });

      setDetectedGames(games);
      setScreen('result');
    } catch (err) {
      setError(String(err));
      setScreen('welcome');
    } finally {
      setIsScanning(false);
    }
  };

  const handleManualComplete = (game: GameConfig) => {
    setDetectedGames([game]);
    setScreen('result');
  };

  // == Welcome Screen ==
  if (screen === 'welcome') {
    return (
      <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
        <div className="max-w-lg w-full text-center space-y-8">
          {/* Logo & Title */}
          <div className="space-y-4">
            <div className="mx-auto w-20 h-20 rounded-2xl bg-linear-to-br from-primary to-secondary flex items-center justify-center shadow-lg shadow-primary/25">
              <Gamepad2 className="w-10 h-10 text-primary-content" />
            </div>
            <h1 className="text-4xl font-bold bg-linear-to-r from-primary to-secondary bg-clip-text text-transparent">
              Welcome to EMMM2
            </h1>
            <p className="text-base-content/60 text-lg">Enhanced Mod Manager for 3DMigoto Games</p>
          </div>

          {/* Error Alert */}
          {error && (
            <div role="alert" className="alert alert-error alert-soft">
              <AlertCircle className="w-5 h-5" />
              <span>{error}</span>
            </div>
          )}

          {/* CTA Buttons */}
          <div className="space-y-4">
            <button
              id="btn-auto-detect"
              className="btn btn-primary btn-lg btn-block gap-3"
              onClick={handleAutoDetect}
            >
              <Search className="w-5 h-5" />
              XXMI Auto-Detect
              <ChevronRight className="w-5 h-5 ml-auto" />
            </button>

            <button
              id="btn-manual-setup"
              className="btn btn-outline btn-lg btn-block gap-3"
              onClick={() => {
                setError(null);
                setScreen('manual');
              }}
            >
              <FolderOpen className="w-5 h-5" />
              Add Game Manually
              <ChevronRight className="w-5 h-5 ml-auto" />
            </button>
          </div>

          <p className="text-base-content/40 text-sm">
            Auto-detect scans your XXMI Launcher folder for installed games.
          </p>
        </div>
      </div>
    );
  }

  // == Scanning State ==
  if (screen === 'auto-detect' && isScanning) {
    return (
      <div className="min-h-screen bg-base-100 flex items-center justify-center">
        <div className="text-center space-y-6">
          <Loader2 className="w-16 h-16 text-primary animate-spin mx-auto" />
          <div>
            <h2 className="text-2xl font-semibold">Scanning for games...</h2>
            <p className="text-base-content/60 mt-2">
              Looking for 3DMigoto instances in XXMI subfolders
            </p>
          </div>
          {/* Shimmer placeholder cards (EC-1.07) */}
          <div className="w-80 mx-auto space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 rounded-xl bg-base-200 animate-pulse" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  // == Manual Setup Screen ==
  if (screen === 'manual') {
    return (
      <ManualSetupForm onBack={() => setScreen('welcome')} onComplete={handleManualComplete} />
    );
  }

  // == Result Screen ==
  if (screen === 'result') {
    return (
      <AutoDetectResult
        games={detectedGames}
        onContinue={onComplete}
        onAddMore={() => setScreen('manual')}
      />
    );
  }

  return null;
}
