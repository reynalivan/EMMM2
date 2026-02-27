import { useState } from 'react';
import { Plus, Edit2, Trash2, Play, RefreshCcw } from 'lucide-react';
import { useSettings, GameConfig } from '../../../hooks/useSettings';
import GameFormModal from '../modals/GameFormModal';
import { useAppStore } from '../../../stores/useAppStore';
import { useToastStore } from '../../../stores/useToastStore';
import { scanService } from '../../../services/scanService';

export default function GamesTab() {
  const { settings, saveSettings } = useSettings();
  const { setActiveGameId, activeGameId } = useAppStore();
  const { addToast } = useToastStore();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGame, setEditingGame] = useState<GameConfig | null>(null);
  const [scanningId, setScanningId] = useState<string | null>(null);

  const handleAdd = () => {
    setEditingGame(null);
    setIsModalOpen(true);
  };

  const handleEdit = (game: GameConfig) => {
    setEditingGame(game);
    setIsModalOpen(true);
  };

  const handleDelete = (id: string) => {
    if (!settings) return;
    if (window.confirm('Are you sure you want to remove this game configuration?')) {
      const newGames = settings.games.filter((g) => g.id !== id);
      saveSettings({ ...settings, games: newGames });

      // If deleted active game, deselect it
      if (activeGameId === id) {
        setActiveGameId(null);
      }
    }
  };

  const handleSave = (game: GameConfig) => {
    if (!settings) return;

    const newGames = [...settings.games];
    const index = newGames.findIndex((g) => g.id === game.id);

    if (index >= 0) {
      // Edit
      newGames[index] = game;
    } else {
      // Add
      newGames.push(game);
    }

    saveSettings({ ...settings, games: newGames });
  };

  const handleRescan = async (game: GameConfig) => {
    if (scanningId) return;
    setScanningId(game.id);
    const toastId = addToast('info', `Scanning library for ${game.name}...`, 0); // Persist toast

    try {
      const result = await scanService.syncDatabase(
        game.id,
        game.name,
        game.game_type,
        game.mod_path,
      );

      // Update the toast after scan completes
      useToastStore.getState().removeToast(toastId);
      addToast(
        'success',
        `Scan complete! Found ${result.new_mods} new mods and ${result.updated_mods} updates.`,
      );
    } catch (e) {
      console.error(e);
      useToastStore.getState().removeToast(toastId);
      addToast('error', `Scan failed: ${String(e)}`);
    } finally {
      setScanningId(null);
    }
  };

  if (!settings) return <div>Loading...</div>;

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center bg-base-200/50 p-4 rounded-xl border border-base-300">
        <div>
          <h2 className="text-xl font-bold">Games Library</h2>
          <p className="text-sm opacity-70">
            Manage your install locations and game configurations.
          </p>
        </div>
        <button className="btn btn-primary gap-2" onClick={handleAdd}>
          <Plus size={18} /> Add Game
        </button>
      </div>

      <div className="grid grid-cols-1 gap-4">
        {settings.games.length === 0 ? (
          <div className="text-center py-12 opacity-50 border-2 border-dashed border-base-300 rounded-xl">
            <p>No games configured. Click "Add Game" to get started.</p>
          </div>
        ) : (
          settings.games.map((game) => (
            <div
              key={game.id}
              className={`card bg-base-200 shadow-md border-l-4 transition-all hover:shadow-lg ${activeGameId === game.id ? 'border-primary' : 'border-base-300 opacity-90 hover:opacity-100'}`}
            >
              <div className="card-body p-5 flex flex-row items-center justify-between gap-4">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-base-300 rounded-lg flex items-center justify-center font-bold text-xl text-primary/50">
                    {game.name.charAt(0)}
                  </div>
                  <div>
                    <h3 className="card-title text-base flex items-center gap-2">
                      {game.name}
                      {activeGameId === game.id && (
                        <span className="badge badge-primary badge-xs">ACTIVE</span>
                      )}
                    </h3>
                    <div className="text-xs space-y-1 mt-1 opacity-70">
                      <p className="flex items-center gap-1">
                        <span className="font-semibold">Mods:</span> {game.mod_path}
                      </p>
                      <p className="flex items-center gap-1">
                        <span className="font-semibold">Exe:</span> {game.game_exe}
                      </p>
                    </div>
                  </div>
                </div>

                <div className="join">
                  <button
                    className="btn btn-ghost btn-sm join-item text-primary"
                    onClick={() => setActiveGameId(game.id)}
                    disabled={activeGameId === game.id}
                    title="Set as Active"
                  >
                    <Play size={16} />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item text-secondary hover:bg-secondary/10"
                    onClick={() => void handleRescan(game)}
                    disabled={scanningId !== null}
                    title="Rescan Library"
                  >
                    <RefreshCcw
                      size={16}
                      className={scanningId === game.id ? 'animate-spin' : ''}
                    />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item"
                    onClick={() => handleEdit(game)}
                    disabled={scanningId !== null}
                    title="Edit Game"
                  >
                    <Edit2 size={16} />
                  </button>
                  <button
                    className="btn btn-ghost btn-sm join-item text-error hover:bg-error/10"
                    onClick={() => handleDelete(game.id)}
                    disabled={scanningId !== null}
                    title="Remove Game"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <GameFormModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSave={handleSave}
        initialData={editingGame}
        existingModPaths={settings.games
          .filter((game) => game.id !== editingGame?.id)
          .map((game) => game.mod_path)}
      />
    </div>
  );
}
