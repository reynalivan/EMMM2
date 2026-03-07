import { useState, useMemo } from 'react';
import { X, TagIcon } from 'lucide-react';

interface BulkTagModalProps {
  open: boolean;
  mode: 'add' | 'remove';
  /** Union of all tags from the selected objects */
  existingTags: string[];
  onSubmit: (tags: string[]) => void;
  onClose: () => void;
}

export default function BulkTagModal({
  open,
  mode,
  existingTags,
  onSubmit,
  onClose,
}: BulkTagModalProps) {
  const [inputValue, setInputValue] = useState('');
  const [checkedTags, setCheckedTags] = useState<Set<string>>(new Set());

  // Reset state when modal opens
  const uniqueTags = useMemo(() => [...new Set(existingTags)].sort(), [existingTags]);

  if (!open) return null;

  const handleAddSubmit = () => {
    const tags = inputValue
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);
    if (tags.length > 0) onSubmit(tags);
    setInputValue('');
    onClose();
  };

  const handleRemoveSubmit = () => {
    const tags = [...checkedTags];
    if (tags.length > 0) onSubmit(tags);
    setCheckedTags(new Set());
    onClose();
  };

  const toggleTag = (tag: string) => {
    setCheckedTags((prev) => {
      const next = new Set(prev);
      if (next.has(tag)) next.delete(tag);
      else next.add(tag);
      return next;
    });
  };

  return (
    <dialog className="modal modal-open" onClick={onClose}>
      <div className="modal-box max-w-sm" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <TagIcon size={18} />
            {mode === 'add' ? 'Add Tags' : 'Remove Tags'}
          </h3>
          <button className="btn btn-sm btn-ghost btn-circle" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        {mode === 'add' ? (
          <>
            <p className="text-sm text-base-content/60 mb-3">
              Enter tags separated by commas. They will be appended to all selected objects.
            </p>
            <input
              type="text"
              className="input input-bordered w-full"
              placeholder="e.g. Pyro, 5-star, Swimsuit"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddSubmit();
              }}
              autoFocus
            />
            <div className="modal-action">
              <button className="btn btn-ghost btn-sm" onClick={onClose}>
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleAddSubmit}
                disabled={!inputValue.trim()}
              >
                Add Tags
              </button>
            </div>
          </>
        ) : (
          <>
            <p className="text-sm text-base-content/60 mb-3">
              Select tags to remove from all selected objects.
            </p>
            {uniqueTags.length === 0 ? (
              <p className="text-sm text-base-content/40 italic">
                No tags found on selected objects.
              </p>
            ) : (
              <div className="flex flex-wrap gap-2 max-h-48 overflow-y-auto">
                {uniqueTags.map((tag) => (
                  <label
                    key={tag}
                    className={`badge badge-lg cursor-pointer gap-1.5 transition-colors ${
                      checkedTags.has(tag) ? 'badge-error' : 'badge-outline hover:badge-error/50'
                    }`}
                  >
                    <input
                      type="checkbox"
                      className="checkbox checkbox-xs checkbox-error"
                      checked={checkedTags.has(tag)}
                      onChange={() => toggleTag(tag)}
                    />
                    {tag}
                  </label>
                ))}
              </div>
            )}
            <div className="modal-action">
              <button className="btn btn-ghost btn-sm" onClick={onClose}>
                Cancel
              </button>
              <button
                className="btn btn-error btn-sm"
                onClick={handleRemoveSubmit}
                disabled={checkedTags.size === 0}
              >
                Remove {checkedTags.size > 0 ? `(${checkedTags.size})` : ''}
              </button>
            </div>
          </>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
