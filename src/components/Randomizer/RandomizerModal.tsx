import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';

interface RandomizerModalProps {
  open: boolean;
  onClose: () => void;
  gameId: string;
}

interface RandomModResult {
  id: string;
  name: string;
  thumbnail_path?: string;
}

export default function RandomizerModal({ open, onClose, gameId }: RandomizerModalProps) {
  const [result, setResult] = useState<RandomModResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [safe, setSafe] = useState(true);

  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);

  const handleRoll = async () => {
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await invoke<RandomModResult | null>('pick_random_mod', {
        gameId,
        isSafe: safe,
      });

      if (res) {
        setResult(res);
      } else {
        setError('No eligible mods found (check filters or ensure you have disabled mods).');
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleEnable = async () => {
    if (!result) return;
    try {
      await invoke('toggle_mod', { id: result.id, enabled: true });
      onClose();
      // Ideally trigger a refresh here, but Sidebar sync button works too
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <dialog ref={dialogRef} className="modal" onClose={onClose}>
      <div className="modal-box relative border border-base-300">
        <button className="btn btn-sm btn-circle absolute right-2 top-2" onClick={onClose}>
          ✕
        </button>

        <h3 className="font-bold text-lg mb-4">Mod Randomizer</h3>

        <div className="flex flex-col gap-4 items-center">
          {/* Filter Toggle */}
          <label className="label cursor-pointer gap-2">
            <span className="label-text">Safe Mode Only</span>
            <input
              type="checkbox"
              className="toggle toggle-success toggle-sm"
              checked={safe}
              onChange={() => setSafe(!safe)}
            />
          </label>

          {/* Result Display */}
          <div className="w-full h-64 bg-base-300/50 rounded-lg flex flex-col items-center justify-center p-4 relative overflow-hidden">
            {loading ? (
              <span className="loading loading-spinner loading-lg text-primary"></span>
            ) : result ? (
              <>
                {result.thumbnail_path ? (
                  <img
                    src={convertFileSrc(result.thumbnail_path)}
                    alt={result.name}
                    className="absolute inset-0 w-full h-full object-cover opacity-50"
                  />
                ) : (
                  <div className="absolute inset-0 bg-base-200 opacity-50 flex items-center justify-center">
                    <span className="text-4xl">?</span>
                  </div>
                )}
                <div className="z-10 bg-base-100/80 backdrop-blur-md p-4 rounded-xl text-center shadow-lg border border-base-content/10">
                  <h4 className="font-bold text-xl">{result.name}</h4>
                  <p className="text-xs opacity-70 mt-1">Found a match!</p>
                </div>
              </>
            ) : (
              <p className="opacity-50 text-sm">Click Roll to handle your fate...</p>
            )}
          </div>

          {error && <p className="text-error text-sm text-center">{error}</p>}

          <div className="flex gap-2 w-full mt-2">
            <button className="btn btn-primary flex-1" onClick={handleRoll} disabled={loading}>
              {result ? 'Reroll' : 'Roll Luck'}
            </button>
            {result && (
              <button className="btn btn-success flex-1" onClick={handleEnable}>
                Enable & Close
              </button>
            )}
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
