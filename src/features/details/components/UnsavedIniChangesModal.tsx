interface UnsavedIniChangesModalProps {
  open: boolean;
  isSaving: boolean;
  onSave: () => void;
  onDiscard: () => void;
  onCancel: () => void;
}

export default function UnsavedIniChangesModal({
  open,
  isSaving,
  onSave,
  onDiscard,
  onCancel,
}: UnsavedIniChangesModalProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-xl border border-base-content/10 bg-base-100 p-5 shadow-2xl">
        <h3 className="text-base font-semibold text-base-content">Unsaved Changes</h3>
        <p className="mt-2 text-sm text-base-content/70">
          You have unsaved edits. Save before navigating to keep changes.
        </p>

        <div className="mt-5 flex justify-end gap-2">
          <button className="btn btn-sm btn-ghost" onClick={onCancel} disabled={isSaving}>
            Cancel
          </button>
          <button className="btn btn-sm btn-warning" onClick={onDiscard} disabled={isSaving}>
            Discard
          </button>
          <button className="btn btn-sm btn-primary" onClick={onSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
