import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, Check, MoveRight, FolderTree } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import { toast } from '../../stores/useToastStore';
import { useActiveGame } from '../../hooks/useActiveGame';
import type { ObjectSummary } from '../../types/object';

export type MoveStatus = 'keep' | 'disabled' | 'only-enable';

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

const MOVE_STATUSES: MoveStatus[] = ['keep', 'disabled', 'only-enable'];

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
  const [targetStatus, setTargetStatus] = useState<MoveStatus>('keep');
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
    queryFn: () =>
      commands.listMoveTargetsForObject({
        gameId: activeGame?.id ?? '',
        objectId: selectedObjectId,
      }),
    enabled: isOpen && !!activeGame?.id && !!selectedObjectId,
  });
  const moveTargets = Array.isArray(moveTargetsData) ? moveTargetsData : [];

  const handleSelectObject = (objectId: string) => {
    setSelectedObjectId(objectId);
    setTargetSubpath(null);
  };

  const handleMove = async () => {
    if (!selectedObjectId || targetModPaths.length === 0) return;

    setIsSubmitting(true);
    try {
      await onSubmit(selectedObjectId, targetStatus, targetSubpath);
      toast.success(
        t('folder_grid:move.toast.success', { name: selectedObject?.name || selectedObjectId }),
      );
      onClose();
    } catch (error) {
      toast.error(t('folder_grid:move.toast.failed', { error: String(error) }));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <dialog open={isOpen} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-xl max-w-lg">
        <h3 className="font-bold text-lg mb-2">{t('folder_grid:move.title')}</h3>

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

        <div className="form-control w-full mt-4">
          <label className="block text-sm font-medium mb-1">
            {t('folder_grid:move.status_label')}
          </label>
          <div className="flex gap-1 bg-base-200 p-1 rounded-lg">
            {MOVE_STATUSES.map((status) => (
              <button
                key={status}
                className={`flex-1 py-1.5 rounded-md text-xs font-bold transition-all ${
                  targetStatus === status
                    ? 'bg-base-100 shadow-sm text-primary'
                    : 'text-base-content/40 hover:text-base-content/70'
                }`}
                onClick={() => setTargetStatus(status)}
              >
                {t(`folder_grid:move.status.${status}`)}
              </button>
            ))}
          </div>
        </div>

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
