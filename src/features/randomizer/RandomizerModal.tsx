import { formatAppError } from '../../shared/lib/appError';
import { useState, useRef, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { commands, type RandomModProposal } from '../../shared/api/tauri/bindings';
import { RefreshCw, Check, CheckSquare, Square } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import { applyRuntimeEffects } from '@/features/workspace-runtime/@x/randomizer';
import {
  buildRuntimeMutationDescriptor,
  buildRefreshDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '@/features/workspace-runtime/@x/randomizer';
import type { WorkspaceImpact } from '@/entities/workspace';

interface RandomizerModalProps {
  open: boolean;
  onClose: () => void;
  gameId: string;
}

export default function RandomizerModal({ open, onClose, gameId }: RandomizerModalProps) {
  const { t } = useTranslation('collections');
  const queryClient = useQueryClient();
  const [proposals, setProposals] = useState<RandomModProposal[]>([]);
  const [selectedModIds, setSelectedModIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dialogRef = useRef<HTMLDialogElement>(null);
  const hasAutoRolledRef = useRef(false);
  const sessionRef = useRef(0);

  const handleRoll = useCallback(async () => {
    const session = sessionRef.current;
    setLoading(true);
    setError(null);

    try {
      const res = await commands.suggestRandomMods(gameId);
      if (sessionRef.current !== session) {
        return;
      }

      if (res && res.length > 0) {
        setProposals(res);
        // Default to all checked
        setSelectedModIds(new Set(res.map((r) => r.mod_id)));
      } else {
        setProposals([]);
        setError(t('randomizer.no_eligible'));
      }
    } catch (e) {
      if (sessionRef.current === session) {
        setError(formatAppError(e));
      }
    } finally {
      if (sessionRef.current === session) {
        setLoading(false);
      }
    }
  }, [gameId, t]);

  useEffect(() => {
    sessionRef.current += 1;
    hasAutoRolledRef.current = false;
    setProposals([]);
    setSelectedModIds(new Set());
    setLoading(false);
    setApplying(false);
    setError(null);
  }, [gameId, open]);

  useEffect(
    () => () => {
      sessionRef.current += 1;
    },
    [],
  );

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    if (open && !dialog.open) {
      dialog.showModal();
    }

    if (!open && dialog.open) {
      dialog.close();
      return;
    }

    if (!open) return;

    if (proposals.length > 0 || loading || applying || hasAutoRolledRef.current) {
      return;
    }

    hasAutoRolledRef.current = true;
    void handleRoll();
  }, [applying, handleRoll, loading, open, proposals.length]);

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

    const session = sessionRef.current;
    setApplying(true);
    setError(null);
    const toApply = proposals.filter((p) => selectedModIds.has(p.mod_id));
    const failures: string[] = [];

    for (const proposal of toApply) {
      try {
        const result = await commands.executeWorkspaceSwitch({
          game_id: gameId,
          target: {
            kind: 'mod_path',
            value: proposal.folder_path,
          },
          desired_enabled: true,
          resolution: 'enable_only_this',
          origin_surface: 'collections',
        });
        if (sessionRef.current !== session) {
          return;
        }
        if (result.status === 'applied') {
          // No thumbnail drop: the randomizer only toggles, and toggles keep
          // the folder identity the thumbnail cache is keyed by.
          applyRuntimeEffects(
            queryClient,
            buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
          );
          await publishRuntimeDescriptor(
            queryClient,
            buildRandomizerRefreshDescriptor(result.impact),
            'active',
          );
        } else {
          failures.push(t('randomizer.apply_no_change', { name: proposal.name }));
        }
      } catch (e) {
        if (sessionRef.current !== session) {
          return;
        }
        failures.push(formatAppError(e));
      }
    }

    if (sessionRef.current !== session) {
      return;
    }

    if (failures.length === 0) {
      onClose();
    } else {
      setError(failures.join('\n'));
    }
    setApplying(false);
  };

  const hasSelections = selectedModIds.size > 0;
  const allSelected = proposals.length > 0 && selectedModIds.size === proposals.length;

  return (
    <dialog ref={dialogRef} className="modal bg-overlay-mask backdrop-blur-sm" onClose={onClose}>
      <div className="modal-box w-11/12 max-w-2xl relative border border-base-300 overflow-hidden flex flex-col max-h-[90vh]">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2 z-10"
          onClick={onClose}
          disabled={applying}
        >
          ✕
        </button>

        <div className="flex justify-between items-center mb-4 pr-8">
          <h3 className="font-bold text-xl">{t('randomizer.title')}</h3>
        </div>

        <p className="text-sm opacity-70 mb-4">{t('randomizer.desc')}</p>

        {error && <div className="alert alert-error text-sm py-2 mb-4">{error}</div>}

        {/* Proposals List */}
        <div className="flex-1 overflow-y-auto bg-base-200/50 rounded-xl border border-base-300 p-2 min-h-75">
          {loading ? (
            <div className="flex flex-col items-center justify-center h-full gap-4 opacity-70">
              <span className="loading loading-spinner loading-lg text-primary"></span>
              <p>{t('randomizer.consulting')}</p>
            </div>
          ) : proposals.length > 0 ? (
            <div className="space-y-2">
              <div className="flex justify-between items-center px-2 py-1 bg-base-300/30 rounded-lg mb-2">
                <button className="btn btn-xs btn-ghost gap-2" onClick={toggleAll}>
                  {allSelected ? <CheckSquare size={14} /> : <Square size={14} />}
                  {allSelected ? t('randomizer.deselect_all') : t('randomizer.select_all')}
                </button>
                <span className="text-xs opacity-60">
                  {t('randomizer.selection_status', {
                    selected: selectedModIds.size,
                    total: proposals.length,
                  })}
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
              <p className="text-sm">{t('randomizer.empty_desc')}</p>
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
            {proposals.length > 0 ? t('randomizer.reroll') : t('randomizer.roll')}
          </button>

          <button
            className={`btn btn-primary flex-1 gap-2 ${applying ? 'loading' : ''}`}
            onClick={handleApply}
            disabled={!hasSelections || loading || applying}
          >
            {!applying && <Check size={18} />}
            {applying
              ? t('randomizer.applying')
              : t('randomizer.apply', { count: selectedModIds.size })}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose} disabled={applying}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>
  );
}

function buildRandomizerRefreshDescriptor(impact: WorkspaceImpact) {
  if (impact.refresh_scopes.length > 0) {
    return buildRefreshDescriptor(impact.refresh_scopes);
  }

  return buildRuntimeMutationDescriptor(['folderSwitch', 'collectionsCatalog']);
}
