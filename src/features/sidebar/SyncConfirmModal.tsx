/**
 * SyncConfirmModal — Shows matched MasterDB entry with diff preview
 * and lets user edit match fields before applying.
 *
 * Features:
 * - Per-field diff: "Current → New" comparison with change highlighting
 * - Editable match fields: name, object_type, metadata can be modified pre-apply
 * - Match quality badge (confidence + level)
 */

import { X, Check, Edit, AlertTriangle, Image as ImageIcon, RotateCcw } from 'lucide-react';
import { useState, useEffect } from 'react';

/** Shape returned by Rust `match_object_with_db` command. */
export interface MatchedDbEntry {
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
  /** Pipeline level: "L1Name" | "L2Token" | "L5Fuzzy" */
  match_level: string;
  /** Confidence: "High" | "Medium" | "Low" */
  match_confidence: string;
  /** Human-readable match detail */
  match_detail: string;
}

/** Current object/folder data for diff comparison */
interface CurrentData {
  name: string;
  object_type: string;
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
}

interface SyncConfirmModalProps {
  open: boolean;
  objectName: string;
  currentData: CurrentData | null;
  match: MatchedDbEntry | null;
  isLoading: boolean;
  onApply: (match: MatchedDbEntry) => void;
  onEditManually: () => void;
  onClose: () => void;
}

/** Diff row: shows "old → new" with color coding + inline editing */
function DiffField({
  label,
  current,
  incoming,
  onEdit,
}: {
  label: string;
  current: string;
  incoming: string;
  onEdit?: (val: string) => void;
}) {
  const changed = current !== incoming;
  return (
    <div className="flex items-start gap-2 text-xs">
      <span className="text-base-content/40 w-20 shrink-0 text-right pt-0.5">{label}</span>
      <div className="flex-1 min-w-0">
        {changed ? (
          <div className="flex flex-col gap-0.5">
            <span className="text-base-content/30 line-through truncate">{current || '—'}</span>
            {onEdit ? (
              <input
                type="text"
                className="input input-xs input-bordered w-full text-primary font-medium"
                value={incoming}
                onChange={(e) => onEdit(e.target.value)}
              />
            ) : (
              <span className="text-primary font-medium truncate">{incoming || '—'}</span>
            )}
          </div>
        ) : (
          <span className="text-base-content/50 truncate block pt-0.5">{current || '—'}</span>
        )}
      </div>
    </div>
  );
}

export default function SyncConfirmModal({
  open,
  objectName,
  currentData,
  match,
  isLoading,
  onApply,
  onEditManually,
  onClose,
}: SyncConfirmModalProps) {
  // Editable copy of the match — user can tweak fields before applying
  const [editedMatch, setEditedMatch] = useState<MatchedDbEntry | null>(null);

  // Reset editable state when match changes
  useEffect(() => {
    if (match) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setEditedMatch({ ...match, metadata: match.metadata ? { ...match.metadata } : null });
    } else {
      setEditedMatch(null);
    }
  }, [match]);

  if (!open) return null;

  const displayThumbnail = editedMatch?.thumbnail_path
    ? `asset://${editedMatch.thumbnail_path}`
    : null;

  const setField = (key: keyof MatchedDbEntry, value: string) => {
    if (!editedMatch) return;
    setEditedMatch({ ...editedMatch, [key]: value });
  };

  const setMeta = (key: string, value: string) => {
    if (!editedMatch) return;
    setEditedMatch({ ...editedMatch, metadata: { ...(editedMatch.metadata ?? {}), [key]: value } });
  };

  const resetToOriginal = () => {
    if (match) {
      setEditedMatch({ ...match, metadata: match.metadata ? { ...match.metadata } : null });
    }
  };

  // Collect all metadata keys from both current and incoming
  const allMetaKeys = new Set<string>();
  if (currentData?.metadata) Object.keys(currentData.metadata).forEach((k) => allMetaKeys.add(k));
  if (editedMatch?.metadata) Object.keys(editedMatch.metadata).forEach((k) => allMetaKeys.add(k));

  const hasEdits = editedMatch && match && JSON.stringify(editedMatch) !== JSON.stringify(match);

  return (
    <div className="modal modal-open">
      <div className="modal-box relative w-11/12 max-w-lg">
        <button
          className="btn btn-sm btn-circle absolute right-2 top-2"
          onClick={onClose}
          aria-label="Close"
        >
          <X size={16} />
        </button>

        <h3 className="font-bold text-lg mb-4">Sync with Database</h3>

        {isLoading && (
          <div className="flex justify-center items-center p-8">
            <span className="loading loading-spinner loading-md text-primary" />
            <span className="ml-3 text-base-content/60">Matching "{objectName}"…</span>
          </div>
        )}

        {!isLoading && !match && (
          <div className="flex flex-col items-center gap-3 p-6">
            <AlertTriangle size={32} className="text-warning/60" />
            <p className="text-sm text-base-content/60 text-center">
              No match found for <strong>"{objectName}"</strong> in the database.
            </p>
            <p className="text-xs text-base-content/40 text-center">
              You can edit metadata manually instead.
            </p>
            <div className="flex gap-2 mt-2">
              <button className="btn btn-sm btn-outline" onClick={onEditManually}>
                <Edit size={14} />
                Edit Manually
              </button>
              <button className="btn btn-sm" onClick={onClose}>
                Cancel
              </button>
            </div>
          </div>
        )}

        {!isLoading && editedMatch && (
          <>
            {/* Match quality + thumbnail header */}
            <div className="flex gap-4 p-3 rounded-xl bg-base-200/50 border border-base-300/30">
              <div className="w-16 h-16 rounded-lg bg-base-300 overflow-hidden flex items-center justify-center border border-base-content/10 shrink-0">
                {displayThumbnail ? (
                  <img
                    src={displayThumbnail}
                    alt={editedMatch.name}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <ImageIcon size={24} className="opacity-20" />
                )}
              </div>
              <div className="flex flex-col gap-1 min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-base-content truncate">{editedMatch.name}</span>
                  {hasEdits && (
                    <button
                      className="btn btn-ghost btn-xs"
                      onClick={resetToOriginal}
                      title="Reset to original match"
                    >
                      <RotateCcw size={12} />
                    </button>
                  )}
                </div>
                <div className="flex gap-1 flex-wrap">
                  <span className="badge badge-xs badge-primary badge-outline">
                    {editedMatch.object_type}
                  </span>
                  {editedMatch.match_level && (
                    <span
                      className={`badge badge-xs ${editedMatch.match_confidence === 'High' ? 'badge-success' : editedMatch.match_confidence === 'Medium' ? 'badge-warning' : 'badge-error'}`}
                      title={editedMatch.match_detail}
                    >
                      {editedMatch.match_level} — {editedMatch.match_confidence}
                    </span>
                  )}
                  {hasEdits && <span className="badge badge-xs badge-info">Edited</span>}
                </div>
              </div>
            </div>

            {/* Diff table — per-field comparison (editable) */}
            <div className="mt-3 flex flex-col gap-1.5 p-2 rounded-lg bg-base-200/30 border border-base-300/20">
              <div className="text-[10px] uppercase tracking-wider text-base-content/30 font-medium mb-1">
                Field Comparison (click new value to edit)
              </div>
              <DiffField
                label="Name"
                current={currentData?.name ?? objectName}
                incoming={editedMatch.name}
                onEdit={(v) => setField('name', v)}
              />
              <DiffField
                label="Category"
                current={currentData?.object_type ?? ''}
                incoming={editedMatch.object_type}
                onEdit={(v) => setField('object_type', v)}
              />
              {[...allMetaKeys].map((key) => (
                <DiffField
                  key={key}
                  label={key}
                  current={String(currentData?.metadata?.[key] ?? '')}
                  incoming={String(editedMatch.metadata?.[key] ?? '')}
                  onEdit={(v) => setMeta(key, v)}
                />
              ))}
              <DiffField
                label="Thumbnail"
                current={currentData?.thumbnail_path ? '✓ Set' : '—'}
                incoming={editedMatch.thumbnail_path ? '✓ Set' : '—'}
              />
            </div>

            {/* Actions */}
            <div className="modal-action border-t border-base-200 pt-4 mt-4">
              <button className="btn btn-sm" onClick={onClose}>
                Cancel
              </button>
              <button className="btn btn-sm btn-outline" onClick={onEditManually}>
                <Edit size={14} />
                Edit Manually
              </button>
              <button className="btn btn-sm btn-primary" onClick={() => onApply(editedMatch)}>
                <Check size={14} />
                {hasEdits ? 'Apply Edited' : 'Apply Match'}
              </button>
            </div>
          </>
        )}
      </div>
      <div className="modal-backdrop" onClick={onClose} />
    </div>
  );
}
