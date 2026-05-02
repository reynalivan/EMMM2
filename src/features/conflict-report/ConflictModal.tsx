import { AlertTriangle, Folder, X } from 'lucide-react';
import { useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '../../types/mod';

interface ConflictModalProps {
  open: boolean;
  onClose: () => void;
  conflicts: ConflictInfo[];
}

export default function ConflictModal({ open, onClose, conflicts }: ConflictModalProps) {
  const { t } = useTranslation(['scanner', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog ref={dialogRef} className="modal bg-overlay-mask backdrop-blur-sm" onClose={onClose}>
      <div className="modal-box w-11/12 max-w-3xl border border-warning/20 bg-base-100 shadow-2xl">
        <button
          className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
          onClick={onClose}
        >
          <X size={18} />
        </button>

        <h3 className="font-bold text-lg text-warning flex items-center gap-2 pb-4 border-b border-base-content/10">
          <AlertTriangle className="fill-warning/20" />
          {t('scanner:conflict_modal.title')}
        </h3>

        <div className="py-4 space-y-4 max-h-[60vh] overflow-y-auto">
          {conflicts.length === 0 ? (
            <p className="text-success text-center italic">{t('scanner:conflict_modal.empty')}</p>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="alert alert-warning text-xs shadow-sm">
                <span>{t('scanner:conflict_modal.description')}</span>
              </div>

              {conflicts.map((conflict, idx) => (
                <div
                  key={idx}
                  className="bg-base-200/50 p-3 rounded-lg border border-base-content/5"
                >
                  <div className="flex justify-between items-start mb-2">
                    <span className="badge badge-sm badge-neutral font-mono opacity-70">
                      {conflict.hash.substring(0, 16)}...
                    </span>
                    <span className="text-xs font-mono text-base-content/50">
                      [{conflict.section_name}]
                    </span>
                  </div>

                  <div className="flex flex-col gap-1 pl-2 border-l-2 border-warning/30">
                    {conflict.mod_paths.map((path, pIdx) => {
                      // Extract folder name from path for cleaner display
                      const name = path.split(/[\\/]/).pop() || path;
                      return (
                        <div
                          key={pIdx}
                          className="text-sm truncate hover:text-primary transition-colors cursor-default"
                          title={path}
                        >
                          <span className="inline-flex items-center gap-1">
                            <Folder size={14} className="shrink-0" />
                            <span className="truncate">{name}</span>
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="modal-action border-t border-base-content/10 pt-4">
          <button className="btn btn-primary" onClick={onClose}>
            {t('scanner:conflict_modal.acknowledge')}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
