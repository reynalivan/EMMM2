import { useState, useMemo } from 'react';
import type { ObjectSummary } from '../../types/object';

interface MoveToObjectDialogProps {
  open: boolean;
  onClose: () => void;
  objects: ObjectSummary[];
  currentObjectId?: string;
  currentStatus: boolean; // true = enabled, false = disabled
  onSubmit: (targetObjectId: string, status: 'disabled' | 'only-enable' | 'keep') => void;
}

const STATUS_OPTIONS = [
  { value: 'disabled', label: 'Set Disabled (Default)' },
  { value: 'only-enable', label: 'Only Enable This' },
  { value: 'keep', label: 'Keep Status (*)' },
];

export default function MoveToObjectDialog({
  open,
  onClose,
  objects,
  currentObjectId,
  currentStatus,
  onSubmit,
}: MoveToObjectDialogProps) {
  const [search, setSearch] = useState('');
  const [selectedObject, setSelectedObject] = useState<string | undefined>(undefined);
  const [status, setStatus] = useState<'disabled' | 'only-enable' | 'keep'>('disabled');

  const filteredObjects = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? objects.filter((o) => o.name.toLowerCase().includes(q)) : objects;
  }, [objects, search]);

  // Status label with current state
  const statusOptions = useMemo(() => {
    return STATUS_OPTIONS.map((opt) => {
      let label = opt.label;
      if (opt.value === 'keep') {
        label = `Keep Status (${currentStatus ? 'Enabled' : 'Disabled'})`;
      }
      return { ...opt, label };
    });
  }, [currentStatus]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedObject) return;
    onSubmit(selectedObject, status);
  };

  return (
    <dialog open={open} className="modal modal-bottom sm:modal-middle">
      <form
        method="dialog"
        className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-md"
        onSubmit={handleSubmit}
      >
        <h3 className="font-bold text-lg mb-2">Move to Object</h3>
        <div className="mb-4">
          <label className="block text-sm font-medium mb-1">Object</label>
          <input
            type="text"
            className="input input-bordered input-sm w-full mb-2"
            placeholder="Search objects..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            autoFocus
          />
          <div className="max-h-40 overflow-y-auto rounded border border-base-content/10 bg-base-200">
            {filteredObjects.length === 0 && (
              <div className="p-2 text-xs text-base-content/40">No objects found</div>
            )}
            {filteredObjects.map((obj) => (
              <button
                type="button"
                key={obj.id}
                className={`block w-full text-left px-3 py-2 text-sm rounded transition-all ${
                  selectedObject === obj.id
                    ? 'bg-primary/20 text-primary font-semibold'
                    : 'hover:bg-base-300'
                }`}
                onClick={() => setSelectedObject(obj.id)}
                disabled={obj.id === currentObjectId}
              >
                {obj.name}
                {obj.id === currentObjectId && (
                  <span className="ml-2 text-xs text-base-content/40">(Current)</span>
                )}
              </button>
            ))}
          </div>
        </div>
        <div className="mb-4">
          <label className="block text-sm font-medium mb-1">Set Status</label>
          <div className="flex flex-col gap-1">
            {statusOptions.map((opt) => (
              <label key={opt.value} className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="status"
                  value={opt.value}
                  checked={status === opt.value}
                  onChange={() => setStatus(opt.value as 'disabled' | 'only-enable' | 'keep')}
                  className="radio radio-xs"
                />
                <span className="text-sm">{opt.label}</span>
              </label>
            ))}
          </div>
        </div>
        <div className="modal-action mt-4 flex gap-2">
          <button type="button" className="btn btn-sm btn-ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!selectedObject || selectedObject === currentObjectId}
          >
            Move
          </button>
        </div>
      </form>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
