import { AlertTriangle, X } from 'lucide-react';
import type { ConflictInfo } from '../../types/mod';

interface ConflictModalProps {
  open: boolean;
  onClose: () => void;
  conflicts: ConflictInfo[];
}

export default function ConflictModal({ open, onClose, conflicts }: ConflictModalProps) {
  if (!open) return null;

  return (
    <dialog className="modal modal-open">
      <div className="modal-box w-11/12 max-w-3xl border border-warning/20 bg-base-100 shadow-2xl">
        <form method="dialog">
          <button
            className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </form>

        <h3 className="font-bold text-lg text-warning flex items-center gap-2 pb-4 border-b border-base-content/10">
          <AlertTriangle className="fill-warning/20" />
          Shader Conflicts Detected
        </h3>

        <div className="py-4 space-y-4 max-h-[60vh] overflow-y-auto">
          {conflicts.length === 0 ? (
            <p className="text-success text-center italic">No conflicts found.</p>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="alert alert-warning text-xs shadow-sm">
                <span>
                  These mods modify the same shader/buffer signatures (hashes). Enabling them
                  together may cause graphical glitches or crashes.
                </span>
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
                          📁 {name}
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
            Acknowledge
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-black/50" onClick={onClose} />
    </dialog>
  );
}
