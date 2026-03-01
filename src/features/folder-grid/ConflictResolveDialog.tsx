/**
 * Enhanced modal dialog for resolving folder name conflicts.
 * Shown when both "X" and "DISABLED X" exist in the same directory.
 * Displays side-by-side comparison of both versions (files, sizes, thumbnails)
 * and provides 3 resolution strategies: Keep Enabled, Keep Disabled, Separate.
 */

import { useRef, useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  CheckCircle,
  XCircle,
  Copy,
  FileText,
  HardDrive,
  Loader2,
} from 'lucide-react';

import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';

type Strategy = 'keep_enabled' | 'keep_disabled' | 'separate';

interface FileEntry {
  name: string;
  size: number;
  is_ini: boolean;
}

interface FolderDetail {
  path: string;
  folder_name: string;
  is_enabled: boolean;
  total_size: number;
  file_count: number;
  files: FileEntry[];
  thumbnail_path: string | null;
}

interface ConflictDetails {
  enabled: FolderDetail;
  disabled: FolderDetail;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(i > 0 ? 1 : 0)} ${sizes[i]}`;
}

export default function ConflictResolveDialog() {
  const { conflictDialog, closeConflictDialog } = useAppStore();
  const queryClient = useQueryClient();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [loading, setLoading] = useState(false);
  const [details, setDetails] = useState<ConflictDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);

  const { open, conflict } = conflictDialog;

  // Fetch details when dialog opens
  const fetchDetails = useCallback(async () => {
    if (!conflict) return;
    setDetailsLoading(true);
    setDetails(null);
    try {
      const result = await invoke<ConflictDetails>('get_conflict_details', {
        enabledPath: conflict.attempted_target,
        disabledPath: conflict.existing_path,
      });
      setDetails(result);
    } catch (err) {
      console.error('Failed to fetch conflict details:', err);
    } finally {
      setDetailsLoading(false);
    }
  }, [conflict]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      fetchDetails();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open, fetchDetails]);

  if (!conflict) return null;

  const handleResolve = async (strategy: Strategy) => {
    setLoading(true);
    try {
      const keepPath =
        strategy === 'keep_disabled' ? conflict.existing_path : conflict.attempted_target;
      const duplicatePath =
        strategy === 'keep_disabled' ? conflict.attempted_target : conflict.existing_path;

      await invoke('resolve_conflict', {
        keepPath,
        duplicatePath,
        strategy,
      });

      await queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      await queryClient.invalidateQueries({ queryKey: ['objects'] });

      toast.success('Conflict resolved');
      closeConflictDialog();
    } catch (err) {
      toast.error(`Failed to resolve conflict: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const baseName = conflict.base_name;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      onClose={closeConflictDialog}
    >
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-2xl">
        {/* Header */}
        <div className="flex items-start gap-3 mb-4">
          <div className="p-2 rounded-lg bg-warning/10 text-warning">
            <AlertTriangle size={20} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base text-base-content">Name Conflict Detected</h3>
            <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
              Both an enabled and disabled version of <strong>{baseName}</strong> exist in the same
              directory. Compare and choose which to keep:
            </p>
          </div>
        </div>

        {/* Side-by-side comparison */}
        {detailsLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 size={20} className="animate-spin text-base-content/30" />
            <span className="text-sm text-base-content/40 ml-2">Loading folder details…</span>
          </div>
        ) : details ? (
          <div className="grid grid-cols-2 gap-3 mb-4">
            {/* Enabled side */}
            <FolderColumn
              detail={details.enabled}
              label="Enabled Version"
              accentClass="text-success"
              borderClass="border-success/30"
            />
            {/* Disabled side */}
            <FolderColumn
              detail={details.disabled}
              label="Disabled Version"
              accentClass="text-error"
              borderClass="border-error/30"
            />
          </div>
        ) : null}

        {/* Resolution options */}
        <div className="space-y-2">
          <button
            className="btn btn-sm btn-block btn-outline btn-success justify-start gap-2"
            disabled={loading}
            onClick={() => handleResolve('keep_enabled')}
          >
            <CheckCircle size={16} />
            <span className="flex-1 text-left">
              Keep Enabled
              {details && (
                <span className="text-[10px] opacity-60 ml-1">
                  ({formatBytes(details.enabled.total_size)}, {details.enabled.file_count} files)
                </span>
              )}
            </span>
          </button>

          <button
            className="btn btn-sm btn-block btn-outline btn-error justify-start gap-2"
            disabled={loading}
            onClick={() => handleResolve('keep_disabled')}
          >
            <XCircle size={16} />
            <span className="flex-1 text-left">
              Keep Disabled
              {details && (
                <span className="text-[10px] opacity-60 ml-1">
                  ({formatBytes(details.disabled.total_size)}, {details.disabled.file_count} files)
                </span>
              )}
            </span>
          </button>

          <button
            className="btn btn-sm btn-block btn-outline justify-start gap-2"
            disabled={loading}
            onClick={() => handleResolve('separate')}
          >
            <Copy size={16} />
            Treat as Two Separate Mods
          </button>
        </div>

        {/* Cancel */}
        <div className="modal-action mt-4">
          <button className="btn btn-sm btn-ghost" onClick={closeConflictDialog} disabled={loading}>
            Cancel
          </button>
        </div>
      </div>

      {/* Backdrop */}
      <form method="dialog" className="modal-backdrop">
        <button onClick={closeConflictDialog}>close</button>
      </form>
    </dialog>
  );
}

/** Column showing folder details for one side of the comparison */
function FolderColumn({
  detail,
  label,
  accentClass,
  borderClass,
}: {
  detail: FolderDetail;
  label: string;
  accentClass: string;
  borderClass: string;
}) {
  const thumbSrc = detail.thumbnail_path ? convertFileSrc(detail.thumbnail_path) : null;
  const [imgErr, setImgErr] = useState(false);

  return (
    <div className={`border rounded-lg p-3 ${borderClass} bg-base-200/30`}>
      {/* Label + thumbnail */}
      <div className="flex items-center gap-2 mb-2">
        {thumbSrc && !imgErr ? (
          <img
            src={thumbSrc}
            alt=""
            className="w-10 h-10 rounded-md object-cover shrink-0 border border-base-content/10"
            onError={() => setImgErr(true)}
          />
        ) : (
          <div className="w-10 h-10 rounded-md bg-base-300 flex items-center justify-center shrink-0">
            <HardDrive size={16} className="text-base-content/20" />
          </div>
        )}
        <div className="min-w-0 flex-1">
          <div className={`text-xs font-bold ${accentClass}`}>{label}</div>
          <div className="text-[10px] text-base-content/40 truncate" title={detail.folder_name}>
            {detail.folder_name}
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="flex gap-3 text-[10px] text-base-content/50 mb-2">
        <span>{detail.file_count} files</span>
        <span>{formatBytes(detail.total_size)}</span>
      </div>

      {/* File list (max 6) */}
      <div className="space-y-0.5 max-h-32 overflow-y-auto custom-scrollbar">
        {detail.files.slice(0, 6).map((f) => (
          <div key={f.name} className="flex items-center gap-1.5 text-[10px]">
            <FileText
              size={10}
              className={f.is_ini ? 'text-info shrink-0' : 'text-base-content/20 shrink-0'}
            />
            <span className="truncate flex-1 text-base-content/60">{f.name}</span>
            <span className="text-base-content/30 tabular-nums shrink-0">
              {formatBytes(f.size)}
            </span>
          </div>
        ))}
        {detail.files.length > 6 && (
          <div className="text-[10px] text-base-content/30 pl-4">
            +{detail.files.length - 6} more…
          </div>
        )}
      </div>
    </div>
  );
}
