import { formatAppError } from '../../../shared/lib/appError';
import type { MoveStatus } from '@/entities/mod/model/mod';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, Check, MoveRight, FolderTree } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import { toast } from '../../../app/store/useToastStore';
import { useActiveGame } from '@/pages/dashboard/hooks/useActiveGame';
import type { ObjectSummary } from '@/entities/game-object/model/object';

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
  const [selectedObjectId, setSelectedObjectId] = useState('');
  const [targetSubpath, setTargetSubpath] = useState<string | null>(null);
  const [disableAfterMove, setDisableAfterMove] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

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
  const moveTargets = Array.isArray(moveTargetsData) ? moveTargetsData : [];

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
    if (isOpen && !selectedObjectId && recommendedTarget) {
      setSelectedObjectId(recommendedTarget);
      setTargetSubpath(null);
    }
  }, [isOpen, recommendedTarget, selectedObjectId]);

  useEffect(() => {
    if (isOpen) setDisableAfterMove(false);
  }, [isOpen]);

  const handleSelectObject = (objectId: string) => {
    setSelectedObjectId(objectId);
    setTargetSubpath(null);
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
    <dialog open={isOpen} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-xl max-w-lg">
        <h3 className="font-bold text-lg mb-2">{t('folder_grid:move.title')}</h3>
        {recommendedTarget && (
          <div className="alert alert-info py-2 mb-3 text-xs">
            {t('folder_grid:move.match_suggestion', {
              name: objects.find((object) => object.id === recommendedTarget)?.name,
            })}
          </div>
        )}

        <div className="grid gap-4 sm:grid-cols-2">
          <div className="form-control w-full">
            <label className="block text-sm font-medium mb-1">{t('folder_grid:move.label')}</label>
            <div className="relative">
              <input
                type="text"
                className="input input-sm input-bordered w-full pr-10"
                placeholder={t('folder_grid:move.placeholder')}
                value={searchTerm}
                onChange={(event) => setSearchTerm(event.target.value)}
              />
              <div className="absolute right-3 top-2 opacity-40">
                <Search size={14} />
              </div>
            </div>

            <div className="mt-2 flex flex-col gap-1 max-h-56 overflow-y-auto scrollbar-thin border border-base-300 rounded-lg p-1">
              {availableObjects.length === 0 && (
                <div className="p-2 text-xs text-base-content/40">
                  {t('folder_grid:move.no_results')}
                </div>
              )}
              {availableObjects.map((object) => {
                const isCurrentObject = object.id === currentObjectId;

                return (
                  <button
                    key={object.id}
                    className={`flex items-center justify-between text-left px-3 py-2 rounded-md text-sm transition-colors ${
                      selectedObjectId === object.id
                        ? 'bg-primary text-primary-content font-semibold'
                        : 'hover:bg-base-200'
                    }`}
                    onClick={() => handleSelectObject(object.id)}
                  >
                    <div className="flex-1 truncate pr-2">
                      {object.name}
                      {isCurrentObject && (
                        <span className="ml-2 text-xs opacity-60">
                          {t('folder_grid:move.current_marker')}
                        </span>
                      )}
                    </div>
                    {selectedObjectId === object.id && <Check size={14} />}
                  </button>
                );
              })}
            </div>
          </div>

          <div className="form-control w-full">
            <label className="block text-sm font-medium mb-1">
              {t('folder_grid:move.location_label')}
            </label>
            <div className="flex flex-col gap-1 max-h-64 overflow-y-auto scrollbar-thin border border-base-300 rounded-lg p-1 min-h-32">
              {!selectedObjectId && (
                <div className="p-2 text-xs text-base-content/40">
                  {t('folder_grid:move.select_object_first')}
                </div>
              )}
              {selectedObjectId && targetsLoading && (
                <div className="p-2 text-xs text-base-content/40">
                  {t('folder_grid:move.loading_targets')}
                </div>
              )}
              {moveTargets.map((target) => (
                <button
                  key={target.target_subpath ?? '__root__'}
                  className={`flex items-center gap-2 text-left px-3 py-2 rounded-md text-sm transition-colors ${
                    targetSubpath === target.target_subpath
                      ? 'bg-primary text-primary-content font-semibold'
                      : 'hover:bg-base-200'
                  }`}
                  style={{ paddingLeft: `${12 + target.depth * 12}px` }}
                  onClick={() => setTargetSubpath(target.target_subpath)}
                >
                  <FolderTree size={14} className="shrink-0 opacity-70" />
                  <span className="truncate">{target.display_path}</span>
                </button>
              ))}
            </div>
          </div>
        </div>

        <label className="label cursor-pointer justify-start gap-3 mt-4 rounded-lg bg-base-200 px-3">
          <input
            type="checkbox"
            className="checkbox checkbox-sm"
            checked={disableAfterMove}
            onChange={(event) => setDisableAfterMove(event.target.checked)}
          />
          <span className="label-text">{t('folder_grid:move.disable_after_move')}</span>
        </label>

        <div className="modal-action">
          <button className="btn btn-ghost btn-sm px-6" onClick={onClose}>
            {t('common:actions.cancel')}
          </button>
          <button
            className="btn btn-primary btn-sm px-6 gap-2"
            disabled={!selectedObjectId || targetModPaths.length === 0 || isSubmitting}
            onClick={handleMove}
          >
            {isSubmitting ? (
              <span className="loading loading-spinner loading-xs"></span>
            ) : (
              <MoveRight size={14} />
            )}
            {t('common:actions.move')}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-sm">
        <button onClick={onClose}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
