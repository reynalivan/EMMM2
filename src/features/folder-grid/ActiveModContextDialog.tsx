import { useState } from 'react';
import { ShieldAlert, Loader2 } from 'lucide-react';

interface ActiveModContextDialogProps {
  open: boolean;
  modName: string;
  targetSafeStatus: boolean;
  isProcessing: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ActiveModContextDialog({
  open,
  modName,
  targetSafeStatus,
  isProcessing,
  onConfirm,
  onCancel,
}: ActiveModContextDialogProps) {
  const [isChecked, setIsChecked] = useState(false);

  if (!open) return null;

  return (
    <dialog className={`modal ${open ? 'modal-open' : ''} bg-base-300/80 backdrop-blur-sm`}>
      <div className="modal-box border border-warning/20 shadow-2xl relative overflow-hidden">
        {isProcessing && (
          <div className="absolute inset-0 bg-base-100/50 backdrop-blur-sm z-50 flex flex-col items-center justify-center">
            <Loader2 size={32} className="animate-spin text-primary mb-4" />
            <p className="font-medium animate-pulse">Switching context...</p>
          </div>
        )}

        <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-4">
          <ShieldAlert size={20} />
          Active Mod Context Switch
        </h3>

        <div className="bg-base-200/50 rounded-lg p-4 mb-6 border border-base-content/5">
          <p className="text-sm mb-2">
            You are trying to move <strong className="text-primary">{modName}</strong> to the{' '}
            <strong className={targetSafeStatus ? 'text-success' : 'text-error'}>
              {targetSafeStatus ? 'Safe' : 'Unsafe'}
            </strong>{' '}
            context.
          </p>
          <p className="text-sm text-base-content/70">
            However, this mod is currently <strong>Active (Enabled)</strong>. Active mods cannot
            switch privacy contexts. You must disable it first.
          </p>
        </div>

        <label className="label cursor-pointer justify-start gap-4 bg-base-200 p-4 rounded-lg border border-base-content/10 hover:border-primary/30 transition-colors">
          <input
            type="checkbox"
            className="checkbox checkbox-primary"
            checked={isChecked}
            onChange={(e) => setIsChecked(e.target.checked)}
            disabled={isProcessing}
          />
          <span className="label-text font-medium text-sm">
            Disable this mod now so I can switch its privacy mode
          </span>
        </label>

        <div className="modal-action mt-6">
          <button className="btn btn-ghost" onClick={onCancel} disabled={isProcessing}>
            Cancel
          </button>
          <button
            className="btn btn-warning"
            disabled={!isChecked || isProcessing}
            onClick={onConfirm}
          >
            Disable & Switch Context
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel} disabled={isProcessing}>
          close
        </button>
      </form>
    </dialog>
  );
}
