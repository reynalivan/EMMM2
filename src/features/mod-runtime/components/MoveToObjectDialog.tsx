import { formatAppError } from '@/shared/lib/appError';
import type { MoveStatus } from '@/entities/mod';
import { useEffect, useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MoveRight } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { commands } from '@/shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import { useActiveGame } from '@/entities/game';
import type { ObjectSummary } from '@/entities/game-object';
import type { WorkspaceMoveTarget } from '@/shared/api/tauri/bindings.gen';
import MoveToObjectDialogPanels from './MoveToObjectDialogPanels';

interface MoveToObjectDialogProps {
  isOpen: boolean;
  onClose: () => void;
  objects: ObjectSummary[];
  currentObjectId?: string;
  targetModPaths: string[];
  onSubmit: (
    targetId: string,
    status: MoveStatus,
    targetSubpath: string | null,
  ) => Promise<void> | void;
}

export default function MoveToObjectDialog({
  isOpen,
  onClose,
  objects,
  currentObjectId,
  targetModPaths,
  onSubmit,
}: MoveToObjectDialogProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const { activeGame } = useActiveGame();
  const [searchTerm, setSearchTerm] = useState('');
  const [locationSearchTerm, setLocationSearchTerm] = useState('');
  const [selectedObjectId, setSelectedObjectId] = useState('');
  const [targetSubpath, setTargetSubpath] = useState<string | null>(null);
  const [disableAfterMove, setDisableAfterMove] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const dialogId = useId();
  const targetKey = targetModPaths.join('\u0000');

  const availableObjects = useMemo(() => {
    const normalizedSearch = searchTerm.trim().toLowerCase();
    if (!normalizedSearch) {
      return objects;
    }

    return objects.filter((object) => object.name.toLowerCase().includes(normalizedSearch));
  }, [objects, searchTerm]);

  const selectedObject = useMemo(
    () => objects.find((object) => object.id === selectedObjectId) ?? null,
    [objects, selectedObjectId],
  );

  const { data: moveTargetsData, isFetching: targetsLoading } = useQuery({
    queryKey: ['workspace-move-targets', activeGame?.id, selectedObjectId],
    queryFn: () => commands.listMoveTargetsForObject(activeGame?.id ?? '', selectedObjectId),
    enabled: isOpen && !!activeGame?.id && !!selectedObjectId,
  });
  const moveTargets = useMemo<WorkspaceMoveTarget[]>(
    () => (Array.isArray(moveTargetsData) ? moveTargetsData : []),
    [moveTargetsData],
  );

  const filteredMoveTargets = useMemo(() => {
    const normalizedSearch = locationSearchTerm.trim().toLowerCase();
    if (!normalizedSearch) {
      return moveTargets;
    }

    return moveTargets.filter((target) =>
      target.display_path.toLowerCase().includes(normalizedSearch),
    );
  }, [locationSearchTerm, moveTargets]);

  const selectedTarget = useMemo(
    () => moveTargets.find((target) => target.target_subpath === targetSubpath) ?? null,
    [moveTargets, targetSubpath],
  );

  const selectedLocation =
    selectedTarget?.display_path ?? selectedObject?.name ?? t('folder_grid:move.root_location');

  const { data: relocationPreview = [] } = useQuery({
    queryKey: ['relocation-preview', activeGame?.id, currentObjectId, targetModPaths],
    queryFn: () =>
      commands.previewRelocationBatch({
        gameId: activeGame?.id ?? '',
        sourcePaths: targetModPaths,
        currentObjectId: currentObjectId ?? null,
      }),
    enabled: isOpen && !!activeGame?.id && targetModPaths.length > 0,
  });
  const recommendedTarget = useMemo(() => {
    const scores = new Map<string, { score: number; count: number }>();
    for (const item of relocationPreview) {
      const top = item.suggestions[0];
      if (!top?.objectId) continue;
      const current = scores.get(top.objectId) ?? { score: 0, count: 0 };
      current.score += top.confidencePercentage;
      current.count += 1;
      scores.set(top.objectId, current);
    }
    return (
      [...scores.entries()]
        .filter(([, value]) => value.count === relocationPreview.length)
        .sort((left, right) => right[1].score - left[1].score)[0]?.[0] ?? null
    );
  }, [relocationPreview]);

  useEffect(() => {
    if (
      isOpen &&
      !selectedObjectId &&
      recommendedTarget &&
      objects.some((object) => object.id === recommendedTarget)
    ) {
      setSelectedObjectId(recommendedTarget);
      setTargetSubpath(null);
    }
  }, [isOpen, objects, recommendedTarget, selectedObjectId]);

  useEffect(() => {
    if (!isOpen) return;

    setSearchTerm('');
    setLocationSearchTerm('');
    setSelectedObjectId('');
    setTargetSubpath(null);
    setDisableAfterMove(false);
  }, [currentObjectId, isOpen, targetKey]);

  useEffect(() => {
    if (selectedObjectId && !selectedObject) {
      setSelectedObjectId('');
      setTargetSubpath(null);
    }
  }, [selectedObject, selectedObjectId]);

  const handleSelectObject = (objectId: string) => {
    setSelectedObjectId(objectId);
    setTargetSubpath(null);
    setLocationSearchTerm('');
  };

  const handleMove = async () => {
    if (!selectedObjectId || targetModPaths.length === 0) return;

    setIsSubmitting(true);
    try {
      const targetStatus: MoveStatus = disableAfterMove ? 'disabled' : 'keep';
      await onSubmit(selectedObjectId, targetStatus, targetSubpath);
      toast.success(
        t('folder_grid:move.toast.success', { name: selectedObject?.name || selectedObjectId }),
      );
      onClose();
    } catch (error) {
      toast.error(t('folder_grid:move.toast.failed', { error: formatAppError(error) }));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <dialog
      open={isOpen}
      className="modal modal-bottom sm:modal-middle"
      aria-labelledby={`${dialogId}-title`}
      aria-describedby={`${dialogId}-description`}
      onClose={onClose}
    >
      <div className="modal-box w-[min(94vw,72rem)] max-w-6xl overflow-hidden border border-base-content/10 bg-base-100 p-0 shadow-2xl">
        <div className="border-b border-base-content/10 px-6 py-5">
          <div className="flex items-start gap-3">
            <div className="mt-0.5 rounded-lg bg-primary/10 p-2 text-primary">
              <MoveRight size={20} aria-hidden="true" />
            </div>
            <div className="min-w-0">
              <h2 id={`${dialogId}-title`} className="text-xl font-bold tracking-tight">
                {t('folder_grid:move.title')}
              </h2>
              <p id={`${dialogId}-description`} className="mt-1 text-sm text-base-content/60">
                {t('folder_grid:move.description')}
              </p>
            </div>
          </div>

          <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-base-content/60">
            <span className="badge badge-primary badge-outline">
              {t('folder_grid:move.selection_summary', { count: targetModPaths.length })}
            </span>
            <span>{t('folder_grid:move.source_hint')}</span>
          </div>

          {recommendedTarget && selectedObjectId !== recommendedTarget && (
            <div className="alert alert-info mt-4 py-2 text-sm">
              {t('folder_grid:move.match_suggestion', {
                name: objects.find((object) => object.id === recommendedTarget)?.name,
              })}
            </div>
          )}
        </div>

        <MoveToObjectDialogPanels
          dialogId={dialogId}
          objects={objects}
          availableObjects={availableObjects}
          currentObjectId={currentObjectId}
          selectedObjectId={selectedObjectId}
          searchTerm={searchTerm}
          onSearchChange={setSearchTerm}
          onSelectObject={handleSelectObject}
          moveTargets={moveTargets}
          filteredMoveTargets={filteredMoveTargets}
          targetsLoading={targetsLoading}
          locationSearchTerm={locationSearchTerm}
          onLocationSearchChange={setLocationSearchTerm}
          targetSubpath={targetSubpath}
          onSelectTarget={setTargetSubpath}
        />

        <div className="border-t border-base-content/10 bg-base-100 px-6 py-4">
          <div className="flex flex-col gap-4">
            <label className="flex cursor-pointer items-start gap-3 rounded-xl bg-base-200/60 p-3">
              <input
                type="checkbox"
                className="checkbox checkbox-sm mt-0.5"
                aria-label={t('folder_grid:move.disable_after_move')}
                checked={disableAfterMove}
                onChange={(event) => setDisableAfterMove(event.target.checked)}
              />
              <span className="text-sm">
                <span className="block font-medium">
                  {t('folder_grid:move.disable_after_move')}
                </span>
                <span className="mt-1 block text-xs text-base-content/55">
                  {t('folder_grid:move.disable_after_move_hint')}
                </span>
              </span>
            </label>

            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="min-w-0 text-xs text-base-content/60">
                <span className="block font-medium text-base-content/80">
                  {t('folder_grid:move.destination_label')}
                </span>
                <span className="block truncate" title={selectedLocation}>
                  {selectedObjectId
                    ? t('folder_grid:move.destination_summary', {
                        count: targetModPaths.length,
                        location: selectedLocation,
                      })
                    : t('folder_grid:move.select_object_first')}
                </span>
              </div>

              <div className="flex shrink-0 justify-end gap-2">
                <button type="button" className="btn btn-ghost" onClick={onClose}>
                  {t('common:actions.cancel')}
                </button>
                <button
                  type="button"
                  className="btn btn-primary min-w-32 gap-2"
                  disabled={!selectedObjectId || targetModPaths.length === 0 || isSubmitting}
                  onClick={handleMove}
                >
                  {isSubmitting ? (
                    <span className="loading loading-spinner loading-xs" aria-hidden="true" />
                  ) : (
                    <MoveRight size={16} aria-hidden="true" />
                  )}
                  {t('common:actions.move')}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-sm">
        <button type="button" onClick={onClose}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>
  );
}
