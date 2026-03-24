import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, Check, MoveRight } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import { useAppStore } from '../../stores/useAppStore';
import { folderKeys } from '../../hooks/useFolders';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary } from '../../types/object';

export type ModStatus = 'ENABLED' | 'DISABLED';

interface MoveToObjectDialogProps {
  isOpen: boolean;
  onClose: () => void;
  objects?: ObjectSummary[]; // Optional fallback
  currentObjectId?: string;
  targetModPaths: string[];
  currentStatus?: boolean; // For compatibility
  onSubmit?: (targetId: string, status: 'disabled' | 'keep' | 'only-enable') => void; // For compatibility
}

export default function MoveToObjectDialog({
  isOpen,
  onClose,
  currentObjectId,
  targetModPaths,
}: MoveToObjectDialogProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const queryClient = useQueryClient();
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedObjectId, setSelectedObjectId] = useState('');
  const [targetStatus, setTargetStatus] = useState<ModStatus>('DISABLED');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const activeGameId = useAppStore((state) => state.activeGameId);

  const { data: objectsData } = useQuery({
    queryKey: ['move-to-objects-search', searchTerm, activeGameId],
    queryFn: async () => {
      if (!activeGameId) return [];
      const resp = await commands.getObjects({ gameId: activeGameId, safeMode: false });
      return resp.objects.filter((o) => o.name.toLowerCase().includes(searchTerm.toLowerCase()));
    },
    enabled: isOpen && !!activeGameId,
  });
  const objects = objectsData || [];

  const handleMove = async () => {
    if (!selectedObjectId || targetModPaths.length === 0 || !activeGameId) return;

    setIsSubmitting(true);
    try {
      for (const path of targetModPaths) {
        await commands.moveModToObject({
          gameId: activeGameId,
          folderPath: path,
          targetObjectId: selectedObjectId,
          status: targetStatus,
        });
      }

      const movedTo = objects.find((o) => o.id === selectedObjectId);
      toast.success(
        t('folder_grid:move.toast.success', { name: movedTo?.name || selectedObjectId }),
      );

      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      onClose();
    } catch (error) {
      console.error(error);
      toast.error(t('folder_grid:move.toast.failed', { error: String(error) }));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <dialog open={isOpen} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-xl max-w-sm">
        <h3 className="font-bold text-lg mb-2">{t('folder_grid:move.title')}</h3>

        <div className="form-control w-full mb-4">
          <label className="block text-sm font-medium mb-1">{t('folder_grid:move.label')}</label>
          <div className="relative">
            <input
              type="text"
              className="input input-sm input-bordered w-full pr-10"
              placeholder={t('folder_grid:move.placeholder')}
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
            <div className="absolute right-3 top-2 opacity-40">
              <Search size={14} />
            </div>
          </div>

          <div className="mt-2 flex flex-col gap-1 max-h-48 overflow-y-auto scrollbar-thin border border-base-300 rounded-lg p-1">
            {objects.length === 0 && (
              <div className="p-2 text-xs text-base-content/40">
                {t('folder_grid:move.no_results')}
              </div>
            )}
            {objects.map((obj) => (
              <button
                key={obj.id}
                className={`flex items-center justify-between text-left px-3 py-2 rounded-md text-sm transition-colors ${
                  selectedObjectId === obj.id
                    ? 'bg-primary text-primary-content font-semibold'
                    : 'hover:bg-base-200'
                }`}
                onClick={() => setSelectedObjectId(obj.id)}
              >
                <div className="flex-1 truncate pr-2">
                  {obj.name}
                  {obj.id === currentObjectId && (
                    <span className="ml-2 text-xs text-base-content/40">
                      {t('folder_grid:move.current_marker')}
                    </span>
                  )}
                </div>
                {selectedObjectId === obj.id && <Check size={14} />}
              </button>
            ))}
          </div>
        </div>

        <div className="form-control w-full mb-6">
          <label className="block text-sm font-medium mb-1">
            {t('folder_grid:move.status_label')}
          </label>
          <div className="flex gap-1 bg-base-200 p-1 rounded-lg">
            {(['ENABLED', 'DISABLED'] as ModStatus[]).map((status) => (
              <button
                key={status}
                className={`flex-1 py-1.5 rounded-md text-xs font-bold transition-all ${
                  targetStatus === status
                    ? 'bg-base-100 shadow-sm text-primary'
                    : 'text-base-content/40 hover:text-base-content/70'
                }`}
                onClick={() => setTargetStatus(status)}
              >
                {status}
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
            disabled={!selectedObjectId || isSubmitting}
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
