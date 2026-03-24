import { createPortal } from 'react-dom';
import { useState } from 'react';
import { Save, X, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../../stores/useAppStore';
import { useCreateCollection } from '../hooks/useCollections';

interface SaveCollectionModalProps {
  onClose: () => void;
  onSaved?: (collectionId: string) => void;
}

function buildDefaultCollectionName(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, '0');
  const dd = String(now.getDate()).padStart(2, '0');
  const hh = String(now.getHours()).padStart(2, '0');
  const min = String(now.getMinutes()).padStart(2, '0');
  return `Preset ${yyyy}${mm}${dd}${hh}${min}`;
}

export function SaveCollectionModal({ onClose, onSaved }: SaveCollectionModalProps) {
  const { t } = useTranslation('collections');
  const { activeGameId } = useAppStore();
  const [name, setName] = useState(buildDefaultCollectionName());
  const createMutation = useCreateCollection();

  const handleSave = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!activeGameId || !name.trim()) return;

    createMutation.mutate(
      { gameId: activeGameId, name: name.trim() },
      {
        onSuccess: (result) => {
          onSaved?.(result.id);
          onClose();
        },
      },
    );
  };

  return createPortal(
    <div className="fixed inset-0 z-1000 flex items-center justify-center bg-overlay-mask backdrop-blur-sm p-4">
      <div className="card bg-base-200 border border-base-content/10 shadow-2xl w-full max-w-sm my-auto animate-in fade-in zoom-in-95 duration-200">
        <div className="card-body p-6">
          <div className="flex items-center justify-between mb-2">
            <h2 className="card-title text-xl flex gap-2 items-center">
              <Save size={20} className="text-secondary" />
              {t('save.title')}
            </h2>
            <button
              className="btn btn-sm btn-circle btn-ghost"
              onClick={onClose}
              disabled={createMutation.isPending}
            >
              <X size={16} />
            </button>
          </div>

          <p className="text-sm text-base-content/60 mb-6">{t('save.desc')}</p>

          <form onSubmit={handleSave} className="space-y-5">
            <div className="form-control">
              <label className="label py-1">
                <span className="label-text font-medium text-base-content/80">
                  {t('save.label')}
                </span>
              </label>
              <input
                className="input input-bordered focus:border-secondary bg-base-300 w-full"
                placeholder={t('save.placeholder')}
                value={name}
                onChange={(event) => setName(event.target.value)}
                autoFocus
                required
              />
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={!name.trim() || createMutation.isPending}
                className="btn btn-secondary w-full"
              >
                {createMutation.isPending ? (
                  <>
                    <Loader2 size={18} className="animate-spin" /> {t('save.actions.saving')}
                  </>
                ) : (
                  t('save.actions.save')
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>,
    document.body,
  );
}
