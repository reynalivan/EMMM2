import { formatAppError } from '../../shared/lib/appError';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useState, useRef, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  commands,
  type RandomizerSafetyFilter,
  type RandomizerScope,
  type RandomModProposal,
  type RandomizedLoadoutPreview,
  type StableCategory,
} from '../../shared/api/tauri/bindings';
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
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import { ModThumbnail } from '@/entities/mod';

interface RandomizerModalProps {
  open: boolean;
  onClose: () => void;
  gameId: string;
}

const RANDOMIZER_CATEGORIES: ReadonlyArray<{
  value: StableCategory;
  translationKey: 'character' | 'weapon' | 'ui' | 'other';
}> = [
  { value: 'Character', translationKey: 'character' },
  { value: 'Weapon', translationKey: 'weapon' },
  { value: 'UI', translationKey: 'ui' },
  { value: 'Other', translationKey: 'other' },
];

function defaultScope(): RandomizerScope {
  return { categories: ['Character'], include_unclassified: false };
}

function categoryLabel(objectType: string | null, t: (key: string) => string): string {
  switch (objectType) {
    case 'Character':
      return t('randomizer.scope_character');
    case 'Weapon':
      return t('randomizer.scope_weapon');
    case 'UI':
      return t('randomizer.scope_ui');
    case 'Other':
      return t('randomizer.scope_other');
    default:
      return t('randomizer.scope_unclassified');
  }
}

function buildBackupCollectionName(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, '0');
  const dd = String(now.getDate()).padStart(2, '0');
  const hh = String(now.getHours()).padStart(2, '0');
  const min = String(now.getMinutes()).padStart(2, '0');
  return `Backup before shuffle ${yyyy}-${mm}-${dd} ${hh}:${min}`;
}

export default function RandomizerModal({ open, onClose, gameId }: RandomizerModalProps) {
  const { t } = useTranslation('collections');
  const queryClient = useQueryClient();
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const [proposals, setProposals] = useState<RandomModProposal[]>([]);
  const [selectedModIds, setSelectedModIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backupEnabled, setBackupEnabled] = useState(true);
  const [backupName, setBackupName] = useState(buildBackupCollectionName);
  const [hasUnsavedRuntime, setHasUnsavedRuntime] = useState<boolean | null>(null);
  const [scope, setScope] = useState<RandomizerScope>(defaultScope);
  const [rolledScope, setRolledScope] = useState<RandomizerScope | null>(null);
  const [rolledSafetyFilter, setRolledSafetyFilter] = useState<RandomizerSafetyFilter | null>(null);
  const [lockedObjectIds, setLockedObjectIds] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState<RandomizedLoadoutPreview | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [warningsAcknowledged, setWarningsAcknowledged] = useState(false);

  const dialogRef = useRef<HTMLDialogElement>(null);
  const sessionRef = useRef(0);
  const rollRevisionRef = useRef(0);
  const previewRevisionRef = useRef(0);
  const recentModIdsRef = useRef<Record<string, string[]>>({});

  const handleRoll = useCallback(async () => {
    const session = sessionRef.current;
    const revision = ++rollRevisionRef.current;
    const requestedScope = {
      categories: [...scope.categories],
      include_unclassified: scope.include_unclassified,
    };
    const requestedSafetyFilter = safetyFilter;
    setLoading(true);
    setError(null);
    setPreview(null);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;

    try {
      const res = await commands.suggestRandomMods({
        game_id: gameId,
        safety_filter: requestedSafetyFilter,
        scope: requestedScope,
        recent_mod_ids_by_object: recentModIdsRef.current,
        excluded_object_ids: [...lockedObjectIds],
      });
      if (sessionRef.current !== session || rollRevisionRef.current !== revision) {
        return;
      }

      if (res && res.length > 0) {
        setProposals(res);
        setRolledScope(requestedScope);
        setRolledSafetyFilter(requestedSafetyFilter);
        // Default to all checked
        setSelectedModIds(new Set(res.map((r) => r.mod_id)));
        const nextRecent = { ...recentModIdsRef.current };
        for (const proposal of res) {
          nextRecent[proposal.object_id] = [
            proposal.mod_id,
            ...(nextRecent[proposal.object_id] ?? []).filter((id) => id !== proposal.mod_id),
          ].slice(0, 3);
        }
        recentModIdsRef.current = nextRecent;
      } else {
        setProposals([]);
        setRolledScope(null);
        setRolledSafetyFilter(null);
        setError(t('randomizer.no_eligible'));
      }
    } catch (e) {
      if (sessionRef.current === session && rollRevisionRef.current === revision) {
        setError(formatAppError(e));
      }
    } finally {
      if (sessionRef.current === session && rollRevisionRef.current === revision) {
        setLoading(false);
      }
    }
  }, [gameId, lockedObjectIds, safetyFilter, scope, t]);

  useEffect(() => {
    sessionRef.current += 1;
    rollRevisionRef.current += 1;
    setProposals([]);
    setSelectedModIds(new Set());
    setLoading(false);
    setApplying(false);
    setError(null);
    setBackupEnabled(true);
    setBackupName(buildBackupCollectionName());
    setHasUnsavedRuntime(null);
    setScope(defaultScope());
    setRolledScope(null);
    setRolledSafetyFilter(null);
    setLockedObjectIds(new Set());
    setPreview(null);
    setReviewing(false);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;
    recentModIdsRef.current = {};
  }, [gameId, open]);

  useEffect(() => {
    if (!open) return;
    rollRevisionRef.current += 1;
    setLoading(false);
    setProposals([]);
    setSelectedModIds(new Set());
    setRolledScope(null);
    setRolledSafetyFilter(null);
    setLockedObjectIds(new Set());
    setPreview(null);
    setReviewing(false);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;
    setError(null);
  }, [open, safetyFilter, scope]);

  useEffect(() => {
    if (!open) return;
    const session = sessionRef.current;
    void commands
      .getCollectionRuntimeState(gameId)
      .then((runtime) => {
        if (sessionRef.current === session) {
          setHasUnsavedRuntime(runtime.is_dirty);
        }
      })
      .catch(() => {
        if (sessionRef.current === session) setHasUnsavedRuntime(null);
      });
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
  }, [open]);

  const toggleSelection = (modId: string) => {
    const next = new Set(selectedModIds);
    if (next.has(modId)) {
      next.delete(modId);
    } else {
      next.add(modId);
    }
    setSelectedModIds(next);
    setPreview(null);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;
  };

  const toggleAll = () => {
    if (selectedModIds.size === proposals.length) {
      setSelectedModIds(new Set()); // Deselect all
    } else {
      setSelectedModIds(new Set(proposals.map((r) => r.mod_id))); // Select all
    }
    setPreview(null);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;
  };

  const toggleKeepCurrent = (proposal: RandomModProposal) => {
    setLockedObjectIds((current) => {
      const next = new Set(current);
      if (next.has(proposal.object_id)) next.delete(proposal.object_id);
      else next.add(proposal.object_id);
      return next;
    });
    setSelectedModIds((current) => {
      const next = new Set(current);
      if (lockedObjectIds.has(proposal.object_id)) next.add(proposal.mod_id);
      else next.delete(proposal.mod_id);
      return next;
    });
    setPreview(null);
    setWarningsAcknowledged(false);
    previewRevisionRef.current += 1;
  };

  const toggleScopeCategory = (category: StableCategory) => {
    setScope((current) => ({
      ...current,
      categories: current.categories.includes(category)
        ? current.categories.filter((value) => value !== category)
        : [...current.categories, category],
    }));
  };

  const handleReview = async () => {
    if (selectedModIds.size === 0 || !rolledScope || !rolledSafetyFilter) return;
    const session = sessionRef.current;
    const revision = ++previewRevisionRef.current;
    setReviewing(true);
    setError(null);
    try {
      const result = await commands.previewRandomizedLoadout({
        game_id: gameId,
        mod_ids: proposals
          .filter((proposal) => selectedModIds.has(proposal.mod_id))
          .map((proposal) => proposal.mod_id),
        safety_filter: rolledSafetyFilter,
        scope: rolledScope,
      });
      if (sessionRef.current === session && previewRevisionRef.current === revision) {
        setPreview(result);
        setWarningsAcknowledged(false);
      }
    } catch (e) {
      if (sessionRef.current === session && previewRevisionRef.current === revision)
        setError(formatAppError(e));
    } finally {
      if (sessionRef.current === session && previewRevisionRef.current === revision)
        setReviewing(false);
    }
  };

  const handleApply = async () => {
    if (selectedModIds.size === 0 || !rolledScope || !rolledSafetyFilter || !preview) return;
    if (
      (preview.unsafe_mod_names.length > 0 || preview.runtime_conflicts.length > 0) &&
      !warningsAcknowledged
    )
      return;

    const session = sessionRef.current;
    setApplying(true);
    setError(null);
    try {
      const result = await commands.applyRandomizedLoadout({
        game_id: gameId,
        mod_ids: proposals
          .filter((proposal) => selectedModIds.has(proposal.mod_id))
          .map((proposal) => proposal.mod_id),
        safety_filter: rolledSafetyFilter,
        scope: rolledScope,
        preview_fingerprint: preview.fingerprint,
        backup:
          backupEnabled && hasUnsavedRuntime === true
            ? { collection_name: backupName.trim() }
            : null,
      });
      if (sessionRef.current !== session) return;
      applyRuntimeEffects(
        queryClient,
        buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
      );
      await publishRuntimeDescriptor(
        queryClient,
        buildRandomizerRefreshDescriptor(result.impact),
        'active',
      );
      if (result.backup) {
        toast.success(
          result.backup.reused
            ? t('randomizer.backup_reused', { name: result.backup.collection_name })
            : t('randomizer.backup_created', { name: result.backup.collection_name }),
        );
      }
      if (result.history_warning) toast.error(result.history_warning);
      onClose();
    } catch (e) {
      if (sessionRef.current === session) setError(formatAppError(e));
    } finally {
      if (sessionRef.current === session) setApplying(false);
    }
  };

  const hasSelections = selectedModIds.size > 0;
  const hasResults = proposals.length > 0;
  const allSelected = proposals.length > 0 && selectedModIds.size === proposals.length;
  const hasScope = scope.categories.length > 0 || scope.include_unclassified;
  const hasWarnings =
    !!preview && (preview.unsafe_mod_names.length > 0 || preview.runtime_conflicts.length > 0);

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

        <fieldset className="mb-4 rounded-lg border border-base-300 p-3">
          <legend className="px-1 text-sm font-semibold">{t('randomizer.scope_title')}</legend>
          <div className="flex flex-wrap gap-x-4 gap-y-2">
            {RANDOMIZER_CATEGORIES.map((category) => (
              <label key={category.value} className="label cursor-pointer justify-start gap-2 p-0">
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={scope.categories.includes(category.value)}
                  disabled={applying}
                  onChange={() => toggleScopeCategory(category.value)}
                />
                <span className="label-text">
                  {t(`randomizer.scope_${category.translationKey}`)}
                </span>
              </label>
            ))}
            <label className="label cursor-pointer justify-start gap-2 p-0">
              <input
                type="checkbox"
                className="checkbox checkbox-sm checkbox-primary"
                checked={scope.include_unclassified}
                disabled={applying}
                onChange={(event) =>
                  setScope((current) => ({
                    ...current,
                    include_unclassified: event.target.checked,
                  }))
                }
              />
              <span className="label-text">{t('randomizer.scope_unclassified')}</span>
            </label>
          </div>
          {!hasScope && <p className="mt-2 text-xs text-error">{t('randomizer.scope_required')}</p>}
        </fieldset>

        {/* Proposals List */}
        <div className="flex-1 min-h-75 overflow-y-auto rounded-xl border border-base-300 bg-base-200/50 p-2">
          {loading ? (
            <div className="flex flex-col items-center justify-center h-full gap-4 opacity-70">
              <span className="loading loading-spinner loading-lg text-primary"></span>
              <p>{t('randomizer.consulting')}</p>
            </div>
          ) : hasResults ? (
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
                const isLocked = lockedObjectIds.has(proposal.object_id);
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

                    <ModThumbnail
                      gameId={gameId}
                      thumbnailSrc={
                        proposal.thumbnail_path ? convertFileSrc(proposal.thumbnail_path) : null
                      }
                      sizeClassName="size-10"
                    />

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1 min-w-0">
                        <p className="text-xs uppercase font-bold text-primary tracking-wider truncate">
                          {proposal.object_name}
                        </p>
                        <span className="badge badge-outline badge-xs shrink-0">
                          {categoryLabel(proposal.object_type, t)}
                        </span>
                        <span className="badge badge-ghost badge-xs shrink-0">
                          {proposal.mode === 'exclusive'
                            ? t('randomizer.mode_exclusive')
                            : t('randomizer.mode_additive')}
                        </span>
                      </div>
                      <h4 className="font-semibold text-sm truncate" title={proposal.name}>
                        {proposal.name}
                      </h4>
                      {proposal.active_mod_names.length > 0 && (
                        <p className="text-xs opacity-60 truncate">
                          {t('randomizer.current_mods', {
                            names: proposal.active_mod_names.join(', '),
                          })}
                        </p>
                      )}
                    </div>
                    {proposal.active_mod_names.length > 0 && (
                      <button
                        type="button"
                        className={`btn btn-xs ${isLocked ? 'btn-outline' : 'btn-ghost'}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          toggleKeepCurrent(proposal);
                        }}
                        disabled={applying || reviewing}
                      >
                        {isLocked ? t('randomizer.keeping_current') : t('randomizer.keep_current')}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="flex h-full min-h-70 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
              <RefreshCw size={44} className="text-base-content/15" aria-hidden="true" />
              <p className="max-w-sm text-sm text-base-content/60">{t('randomizer.empty_desc')}</p>
            </div>
          )}
        </div>

        {hasUnsavedRuntime === true && (
          <div className="mt-4 space-y-2 rounded-lg border border-base-300 p-3">
            <label
              className="label cursor-pointer justify-start gap-3 p-0"
              htmlFor="randomizer-backup-enabled"
            >
              <input
                id="randomizer-backup-enabled"
                type="checkbox"
                className="checkbox checkbox-primary checkbox-sm"
                checked={backupEnabled}
                disabled={applying}
                onChange={(event) => {
                  setBackupEnabled(event.target.checked);
                  setPreview(null);
                  previewRevisionRef.current += 1;
                }}
              />
              <span className="label-text">{t('randomizer.backup_enabled')}</span>
            </label>
            {backupEnabled && (
              <input
                className="input input-sm input-bordered w-full"
                value={backupName}
                onChange={(event) => {
                  setBackupName(event.target.value);
                  setPreview(null);
                  previewRevisionRef.current += 1;
                }}
                placeholder={t('randomizer.backup_name_placeholder')}
                disabled={applying}
                required
              />
            )}
          </div>
        )}

        {preview && (
          <section className="mt-4 space-y-2 rounded-lg border border-primary/30 bg-primary/5 p-3">
            <h4 className="font-semibold text-sm">{t('randomizer.review_title')}</h4>
            <p className="text-xs opacity-70">
              {t('randomizer.review_totals', {
                enable: preview.enable_count,
                disable: preview.disable_count,
                keep: lockedObjectIds.size,
              })}
            </p>
            <ul className="space-y-1 text-xs">
              {preview.items.map((item) => (
                <li key={item.object_id}>
                  <span className="font-medium">{item.object_name}</span>{' '}
                  {item.mode === 'exclusive'
                    ? t('randomizer.review_replace', {
                        name: item.selected_mod_name,
                        count: item.disable_count,
                      })
                    : t('randomizer.review_add', { name: item.selected_mod_name })}
                </li>
              ))}
            </ul>
            {hasWarnings && (
              <div className="alert alert-warning text-xs py-2">
                <div>
                  {preview.unsafe_mod_names.length > 0 && (
                    <p>
                      {t('randomizer.warning_unsafe', {
                        names: preview.unsafe_mod_names.join(', '),
                      })}
                    </p>
                  )}
                  {preview.runtime_conflicts.length > 0 && (
                    <p>
                      {t('randomizer.warning_conflicts', {
                        count: preview.runtime_conflicts.length,
                      })}
                    </p>
                  )}
                  <label className="label cursor-pointer justify-start gap-2 p-0 mt-2">
                    <input
                      type="checkbox"
                      className="checkbox checkbox-xs"
                      checked={warningsAcknowledged}
                      onChange={(event) => setWarningsAcknowledged(event.target.checked)}
                      disabled={applying}
                    />
                    <span className="label-text text-xs">
                      {t('randomizer.warning_acknowledge')}
                    </span>
                  </label>
                </div>
              </div>
            )}
          </section>
        )}

        <div className="mt-4 flex w-full gap-2 border-t border-base-300 pt-4">
          <button
            className={`btn gap-2 ${hasResults ? 'btn-neutral flex-1' : 'btn-primary mx-auto min-w-40'}`}
            onClick={handleRoll}
            disabled={!hasScope || loading || applying}
          >
            <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
            {hasResults ? t('randomizer.reroll') : t('randomizer.roll')}
          </button>

          {hasResults &&
            (preview ? (
              <button
                className={`btn btn-primary flex-1 gap-2 ${applying ? 'loading' : ''}`}
                onClick={handleApply}
                disabled={
                  applying ||
                  (hasWarnings && !warningsAcknowledged) ||
                  hasUnsavedRuntime === null ||
                  (backupEnabled && hasUnsavedRuntime === true && !backupName.trim())
                }
              >
                {!applying && <Check size={18} />}
                {applying
                  ? t('randomizer.applying')
                  : t('randomizer.confirm_apply', { count: selectedModIds.size })}
              </button>
            ) : (
              <button
                className={`btn btn-primary flex-1 gap-2 ${reviewing ? 'loading' : ''}`}
                onClick={handleReview}
                disabled={
                  !hasSelections ||
                  !rolledScope ||
                  !rolledSafetyFilter ||
                  loading ||
                  reviewing ||
                  hasUnsavedRuntime === null ||
                  (backupEnabled && hasUnsavedRuntime === true && !backupName.trim())
                }
              >
                {!reviewing && <Check size={18} />}
                {reviewing
                  ? t('randomizer.reviewing')
                  : t('randomizer.review_changes', { count: selectedModIds.size })}
              </button>
            ))}
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
