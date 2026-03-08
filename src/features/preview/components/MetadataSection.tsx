import { useState, useEffect, useRef } from 'react';
import { Pencil } from 'lucide-react';

interface MetadataSectionProps {
  activePath: string | null;
  authorDraft: string;
  versionDraft: string;
  descriptionDraft: string;
  metadataDirty: boolean;
  onAuthorChange: (value: string) => void;
  onVersionChange: (value: string) => void;
  onDescriptionChange: (value: string) => void;
  onDiscard: () => void;
}

export default function MetadataSection({
  activePath,
  authorDraft,
  versionDraft,
  descriptionDraft,
  metadataDirty,
  onAuthorChange,
  onVersionChange,
  onDescriptionChange,
  onDiscard,
}: MetadataSectionProps) {
  const [isEditing, setIsEditing] = useState(false);
  const prevDirty = useRef(metadataDirty);

  useEffect(() => {
    if (prevDirty.current && !metadataDirty) {
      setTimeout(() => setIsEditing(false), 0);
    }
    prevDirty.current = metadataDirty;
  }, [metadataDirty]);

  if (!isEditing) {
    return (
      <div className="mb-6 flex flex-col">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">Metadata</h3>
          <button
            className="btn btn-ghost btn-xs text-base-content/50 hover:text-base-content"
            onClick={() => setIsEditing(true)}
            title="Edit Metadata"
            disabled={!activePath}
          >
            <Pencil size={14} /> Edit
          </button>
        </div>

        <div
          className="flex-1 cursor-pointer rounded-lg hover:bg-white/5 p-2 -mx-2 transition-colors group"
          onDoubleClick={() => {
            if (activePath) setIsEditing(true);
          }}
          title={activePath ? 'Double click to edit' : undefined}
        >
          <div className="flex items-center gap-2 text-xs text-base-content/60 mb-3">
            <span>{authorDraft || 'Unknown Author'}</span>
            <span className="w-1 h-1 rounded-full bg-base-content/30" />
            <span>v{versionDraft || '1.0'}</span>
          </div>

          {descriptionDraft ? (
            <p className="text-sm text-base-content/80 whitespace-pre-wrap leading-relaxed">
              {descriptionDraft}
            </p>
          ) : (
            <p className="text-sm text-base-content/40 italic">No description available.</p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="mb-6 flex flex-col">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">Metadata</h3>
        <div className="flex items-center gap-2">
          {metadataDirty ? (
            <>
              <span className="text-[10px] text-base-content/40 italic">auto-saving…</span>
              <button
                className="btn btn-ghost btn-xs text-warning"
                onClick={onDiscard}
                title="Revert metadata changes and cancel auto-save"
              >
                Revert
              </button>
            </>
          ) : (
            <button
              className="btn btn-ghost btn-xs text-primary"
              onClick={() => setIsEditing(false)}
            >
              Done
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <div className="mb-2 grid grid-cols-2 gap-2">
          <div>
            <label className="label py-1" htmlFor="metadata-author-input">
              <span className="label-text text-xs">Author</span>
            </label>
            <input
              id="metadata-author-input"
              aria-label="Mod author"
              type="text"
              className="input input-bordered w-full bg-transparent text-sm"
              placeholder="Unknown"
              value={authorDraft}
              disabled={!activePath}
              onChange={(event) => onAuthorChange(event.target.value)}
            />
          </div>
          <div>
            <label className="label py-1" htmlFor="metadata-version-input">
              <span className="label-text text-xs">Version</span>
            </label>
            <input
              id="metadata-version-input"
              aria-label="Mod version"
              type="text"
              className="input input-bordered w-full bg-transparent text-sm"
              placeholder="1.0"
              value={versionDraft}
              disabled={!activePath}
              onChange={(event) => onVersionChange(event.target.value)}
            />
          </div>
        </div>

        <label className="label py-1" htmlFor="metadata-description-input">
          <span className="label-text text-xs">Description</span>
        </label>
        <textarea
          id="metadata-description-input"
          aria-label="Mod description"
          className="textarea textarea-bordered h-24 w-full resize-none bg-transparent text-sm"
          placeholder="No description available."
          value={descriptionDraft}
          disabled={!activePath}
          onChange={(event) => onDescriptionChange(event.target.value)}
        />
      </div>
    </div>
  );
}
