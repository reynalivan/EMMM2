import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, Check, CheckSquare, Square } from 'lucide-react';

interface RandomizerModalProps {
  open: boolean;
  onClose: () => void;
  gameId: string;
}

interface RandomModProposal {
  object_id: string;
  object_name: string;
  mod_id: string;
  name: string;
  thumbnail_path?: string;
  folder_path: string;
}

export default function RandomizerModal({ open, onClose, gameId }: RandomizerModalProps) {
  const [proposals, setProposals] = useState<RandomModProposal[]>([]);
  const [selectedModIds, setSelectedModIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [safe, setSafe] = useState(true);

  const dialogRef = useRef<HTMLDialogElement>(null);

  const handleRoll = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const res = await invoke<RandomModProposal[]>('suggest_random_mods', {
        gameId,
        isSafe: safe,
      });

      if (res && res.length > 0) {
        setProposals(res);
        // Default to all checked
        setSelectedModIds(new Set(res.map((r) => r.mod_id)));
      } else {
        setProposals([]);
        setError(
          'No eligible character mods found (check filters or ensure you have disabled mods).',
        );
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [gameId, safe]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      // Auto-fetch on open if empty
      if (proposals.length === 0) {
        handleRoll();
      }
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open, proposals.length, handleRoll]);

  const toggleSelection = (modId: string) => {
    const next = new Set(selectedModIds);
    if (next.has(modId)) {
      next.delete(modId);
    } else {
      next.add(modId);
    }
    setSelectedModIds(next);
  };

  const toggleAll = () => {
    if (selectedModIds.size === proposals.length) {
      setSelectedModIds(new Set()); // Deselect all
    } else {
      setSelectedModIds(new Set(proposals.map((r) => r.mod_id))); // Select all
    }
  };

  const handleApply = async () => {
    if (selectedModIds.size === 0) return;

    setApplying(true);
    setError(null);
    try {
      // Collect the proposals to apply
      const toApply = proposals.filter((p) => selectedModIds.has(p.mod_id));

      // We apply logic by calling enable_only_this sequentially.
      // EMMM2's enable_only_this operates on object_id to disable siblings.
      for (const proposal of toApply) {
        await invoke('enable_only_this', {
          gameId: gameId,
          folderPath: proposal.folder_path,
        });
      }

      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setApplying(false);
    }
  };

  const hasSelections = selectedModIds.size > 0;
  const allSelected = proposals.length > 0 && selectedModIds.size === proposals.length;

  return (
    <dialog ref={dialogRef} className="modal" onClose={onClose}>
      <div className="modal-box w-11/12 max-w-2xl relative border border-base-300 overflow-hidden flex flex-col max-h-[90vh]">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2 z-10"
          onClick={onClose}
          disabled={applying}
        >
          ✕
        </button>

        <div className="flex justify-between items-center mb-4 pr-8">
          <h3 className="font-bold text-xl">Smart Randomizer</h3>
          <label className="label cursor-pointer gap-2 py-0">
            <span className="label-text text-sm">Safe Mode</span>
            <input
              type="checkbox"
              className="toggle toggle-success toggle-sm"
              checked={safe}
              onChange={() => setSafe(!safe)}
              disabled={loading || applying}
            />
          </label>
        </div>

        <p className="text-sm opacity-70 mb-4">
          Recommends 1 random disabled mod for every active Character. Review and apply.
        </p>

        {error && <div className="alert alert-error text-sm py-2 mb-4">{error}</div>}

        {/* Proposals List */}
        <div className="flex-1 overflow-y-auto bg-base-200/50 rounded-xl border border-base-300 p-2 min-h-[300px]">
          {loading ? (
            <div className="flex flex-col items-center justify-center h-full gap-4 opacity-70">
              <span className="loading loading-spinner loading-lg text-primary"></span>
              <p>Consulting the RNG Gods...</p>
            </div>
          ) : proposals.length > 0 ? (
            <div className="space-y-2">
              <div className="flex justify-between items-center px-2 py-1 bg-base-300/30 rounded-lg mb-2">
                <button className="btn btn-xs btn-ghost gap-2" onClick={toggleAll}>
                  {allSelected ? <CheckSquare size={14} /> : <Square size={14} />}
                  {allSelected ? 'Deselect All' : 'Select All'}
                </button>
                <span className="text-xs opacity-60">
                  {selectedModIds.size} of {proposals.length} selected
                </span>
              </div>

              {proposals.map((proposal) => {
                const isSelected = selectedModIds.has(proposal.mod_id);
                return (
                  <div
                    key={proposal.mod_id}
                    className={`flex items-center gap-4 p-3 rounded-xl border transition-all cursor-pointer ${
                      isSelected
                        ? 'bg-primary/10 border-primary/30'
                        : 'bg-base-100 border-base-300 opacity-60 hover:opacity-100'
                    }`}
                    onClick={() => toggleSelection(proposal.mod_id)}
                  >
                    <div className="text-primary mt-1">
                      {isSelected ? <CheckSquare size={20} /> : <Square size={20} />}
                    </div>

                    <div className="flex-1 min-w-0">
                      <p className="text-xs uppercase font-bold text-primary tracking-wider truncate mb-1">
                        {proposal.object_name}
                      </p>
                      <h4 className="font-semibold text-sm truncate" title={proposal.name}>
                        {proposal.name}
                      </h4>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4 opacity-50">
              <RefreshCw size={48} className="opacity-20" />
              <p className="text-sm">Click 'Roll Luck' to generate recommendations.</p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex gap-2 w-full mt-4 pt-4 border-t border-base-300">
          <button
            className="btn btn-neutral flex-1 gap-2"
            onClick={handleRoll}
            disabled={loading || applying}
          >
            <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
            {proposals.length > 0 ? 'Reroll All' : 'Roll Luck'}
          </button>

          <button
            className={`btn btn-primary flex-1 gap-2 ${applying ? 'loading' : ''}`}
            onClick={handleApply}
            disabled={!hasSelections || loading || applying}
          >
            {!applying && <Check size={18} />}
            {applying ? 'Applying...' : `Apply (${selectedModIds.size})`}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose} disabled={applying}>
          close
        </button>
      </form>
    </dialog>
  );
}
