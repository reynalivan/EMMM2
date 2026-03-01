interface MetadataSectionProps {
  activePath: string | null;
  titleDraft: string;
  authorDraft: string;
  versionDraft: string;
  descriptionDraft: string;
  metadataDirty: boolean;
  onTitleChange: (value: string) => void;
  onAuthorChange: (value: string) => void;
  onVersionChange: (value: string) => void;
  onDescriptionChange: (value: string) => void;
  onDiscard: () => void;
}

export default function MetadataSection({
  activePath,
  titleDraft,
  authorDraft,
  versionDraft,
  descriptionDraft,
  metadataDirty,
  onTitleChange,
  onAuthorChange,
  onVersionChange,
  onDescriptionChange,
  onDiscard,
}: MetadataSectionProps) {
  return (
    <div className="mb-6 flex flex-col">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">Metadata</h3>
        <div className="flex items-center gap-2">
          {metadataDirty && (
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
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <label className="label py-1" htmlFor="metadata-title-input">
          <span className="label-text text-xs">Title</span>
        </label>
        <input
          id="metadata-title-input"
          aria-label="Mod title"
          type="text"
          className="input input-bordered mb-2 w-full bg-transparent text-sm"
          placeholder="Mod title"
          value={titleDraft}
          disabled={!activePath}
          onChange={(event) => onTitleChange(event.target.value)}
        />

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
