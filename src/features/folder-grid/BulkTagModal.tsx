import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from '../../stores/useToastStore';
import { useBulkUpdateInfo } from '../../hooks/useFolderMutations';
import { useActiveGame } from '../../hooks/useActiveGame';
import { X, Tag, Plus } from 'lucide-react';

interface BulkTagModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedPaths: string[];
}

export function BulkTagModal({ isOpen, onClose, selectedPaths }: BulkTagModalProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const [tagInput, setTagInput] = useState('');
  const [tagsToAdd, setTagsToAdd] = useState<string[]>([]);
  const { activeGame } = useActiveGame();
  const { mutate, isPending } = useBulkUpdateInfo();
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input on open
  useEffect(() => {
    // Component is conditionally rendered, so this runs on mount (open)
    setTimeout(() => inputRef.current?.focus(), 50);
  }, []);

  if (!isOpen) return null;

  const handleAddTag = () => {
    const trimmed = tagInput.trim();
    if (trimmed && !tagsToAdd.includes(trimmed)) {
      setTagsToAdd([...tagsToAdd, trimmed]);
      setTagInput('');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddTag();
    }
  };

  const removeTag = (tag: string) => {
    setTagsToAdd(tagsToAdd.filter((t) => t !== tag));
  };

  const startBulkUpdate = () => {
    if (tagsToAdd.length === 0) return;

    mutate(
      {
        gameId: activeGame?.id || '',
        paths: selectedPaths,
        update: { tags_add: tagsToAdd },
      },
      {
        onSuccess: () => {
          toast.success(t('folder_grid:tags.toast.success', { count: selectedPaths.length }));
          onClose();
        },
        onError: (err) => {
          toast.error(t('folder_grid:tags.toast.failed', { error: String(err) }));
        },
      },
    );
  };

  return (
    <dialog className="modal modal-open">
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-sm w-full p-6 animate-in zoom-in-95 duration-200">
        <div className="flex justify-between items-center mb-4">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <Tag size={18} className="text-primary" />
            {t('folder_grid:tags.title')} ({selectedPaths.length})
          </h3>
          <button
            onClick={onClose}
            className="btn btn-sm btn-ghost btn-circle"
            aria-label={t('common:actions.close')}
          >
            <X size={18} />
          </button>
        </div>

        <div className="flex flex-col gap-4">
          <div className="flex gap-2">
            <input
              ref={inputRef}
              type="text"
              placeholder={t('folder_grid:tags.placeholder')}
              className="input input-bordered w-full input-sm"
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={handleKeyDown}
            />
            <button
              className="btn btn-sm btn-square"
              onClick={handleAddTag}
              aria-label={t('common:actions.add')}
            >
              <Plus size={16} />
            </button>
          </div>

          <div className="flex flex-wrap gap-2 min-h-10 p-2 bg-base-200/50 rounded-lg">
            {tagsToAdd.length === 0 && (
              <span className="text-base-content/40 text-sm italic py-1">
                {t('folder_grid:tags.empty')}
              </span>
            )}
            {tagsToAdd.map((tag) => (
              <div key={tag} className="badge badge-neutral gap-1 pr-1">
                {tag}
                <button
                  onClick={() => removeTag(tag)}
                  className="hover:bg-base-content/20 rounded-full p-0.5"
                  aria-label={t('common:actions.remove')}
                >
                  <X size={10} />
                </button>
              </div>
            ))}
          </div>

          <div className="modal-action mt-2">
            <button className="btn btn-sm btn-ghost" onClick={onClose}>
              {t('common:actions.cancel')}
            </button>
            <button
              className="btn btn-sm btn-primary"
              onClick={startBulkUpdate}
              disabled={isPending || tagsToAdd.length === 0}
            >
              {isPending ? t('common:status.updating') : t('common:actions.add_tags')}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop backdrop-blur-sm bg-overlay-mask">
        <button onClick={onClose}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
