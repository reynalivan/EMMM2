import { AlertTriangle, X } from 'lucide-react';
import type { ConflictInfo } from '../../types/scanner';

interface Props {
  conflicts: ConflictInfo[];
  onDismiss: () => void;
}

export default function ConflictToast({ conflicts, onDismiss }: Props) {
  if (conflicts.length === 0) return null;

  return (
    <div className="toast toast-end toast-bottom z-50">
      <div className="alert alert-warning shadow-lg flex-row gap-4">
        <AlertTriangle className="w-6 h-6" />
        <div className="flex flex-col">
          <span className="font-bold">Shader Conflict Detected!</span>
          <span className="text-xs">{conflicts.length} conflict(s) detected.</span>
        </div>
        <button className="btn btn-sm btn-ghost btn-circle" onClick={onDismiss}>
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}
