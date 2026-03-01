import { useState, useRef, useEffect } from 'react';
import { useBulkUpdateInfo } from '../../hooks/useFolders';
import { X, Tag, Plus } from 'lucide-react';
import { createPortal } from 'react-dom';

interface BulkTagModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedPaths: string[];
}

export function BulkTagModal({ isOpen, onClose, selectedPaths }: BulkTagModalProps) {
  const [tagInput, setTagInput] = useState('');
  const [tagsToAdd, setTagsToAdd] = useState<string[]>([]);
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
        paths: selectedPaths,
        update: { tags_add: tagsToAdd },
      },
      {
        onSuccess: () => {
          onClose();
        },
      },
    );
  };

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-sm w-full p-6 animate-in zoom-in-95 duration-200">
        <div className="flex justify-between items-center mb-4">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <Tag size={18} className="text-primary" />
            Add Tags to {selectedPaths.length} Mods
          </h3>
          <button onClick={onClose} className="btn btn-sm btn-ghost btn-circle">
            <X size={18} />
          </button>
        </div>

        <div className="flex flex-col gap-4">
          <div className="flex gap-2">
            <input
              ref={inputRef}
              type="text"
              placeholder="Type tag & press Enter..."
              className="input input-bordered w-full input-sm"
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={handleKeyDown}
            />
            <button className="btn btn-sm btn-square" onClick={handleAddTag}>
              <Plus size={16} />
            </button>
          </div>

          <div className="flex flex-wrap gap-2 min-h-[40px] p-2 bg-base-200/50 rounded-lg">
            {tagsToAdd.length === 0 && (
              <span className="text-base-content/40 text-sm italic py-1">No tags added yet</span>
            )}
            {tagsToAdd.map((tag) => (
              <div key={tag} className="badge badge-neutral gap-1 pr-1">
                {tag}
                <button
                  onClick={() => removeTag(tag)}
                  className="hover:bg-base-content/20 rounded-full p-0.5"
                >
                  <X size={10} />
                </button>
              </div>
            ))}
          </div>

          <div className="modal-action mt-2">
            <button className="btn btn-sm btn-ghost" onClick={onClose}>
              Cancel
            </button>
            <button
              className="btn btn-sm btn-primary"
              onClick={startBulkUpdate}
              disabled={isPending || tagsToAdd.length === 0}
            >
              {isPending ? 'Updating...' : `Add Tags`}
            </button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
