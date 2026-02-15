import { useEffect, useRef } from 'react';
import { CheckCircle2, XCircle } from 'lucide-react';
import { useScannerStore } from '../../stores/scannerStore';

interface Props {
  onCancel: () => void;
}

export default function ScanOverlay({ onCancel }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const { isScanning, progress, stats } = useScannerStore();

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isScanning) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  }, [isScanning]);

  // Calculate percentage
  const percentage = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onCancel={onCancel}>
      <div className="modal-box text-center">
        <h3 className="font-bold text-lg mb-4">Scanning Mods...</h3>

        {/* Circular Progress */}
        <div
          className="radial-progress text-primary mx-auto mb-4 transition-all duration-300"
          style={
            {
              '--value': percentage,
              '--size': '12rem',
              '--thickness': '1rem',
            } as React.CSSProperties
          }
          role="progressbar"
        >
          <div className="flex flex-col items-center justify-center">
            <span className="text-4xl font-bold">{percentage}%</span>
            <span className="text-xs opacity-60 mt-1">
              {progress.current} / {progress.total}
            </span>
          </div>
        </div>

        {/* Current Action Label */}
        <p className="py-2 text-sm font-mono bg-base-200 rounded-lg truncate px-4 mb-4">
          {progress.label}
        </p>

        {/* Stats Grid */}
        <div className="stats shadow w-full mb-6">
          <div className="stat place-items-center">
            <div className="stat-title text-success flex items-center gap-1">
              <CheckCircle2 className="w-4 h-4" /> Matched
            </div>
            <div className="stat-value text-success">{stats.matched}</div>
          </div>

          <div className="stat place-items-center">
            <div className="stat-title text-base-content/50 flex items-center gap-1">
              <XCircle className="w-4 h-4" /> Unmatched
            </div>
            <div className="stat-value">{stats.unmatched}</div>
          </div>
        </div>

        {/* Action */}
        <div className="modal-action justify-center">
          <button className="btn btn-outline btn-error" onClick={onCancel}>
            Cancel Scan
          </button>
        </div>
      </div>
    </dialog>
  );
}
