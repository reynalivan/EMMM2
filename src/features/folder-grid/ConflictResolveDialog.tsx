/**
 * Enhanced modal dialog for resolving folder name conflicts.
 * Shown when both "X" and "DISABLED X" exist in the same directory.
 * Displays side-by-side comparison of both versions (files, sizes, thumbnails)
 * and provides 3 resolution strategies: Keep Enabled, Keep Disabled, Separate.
 */

import { useRef, useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../lib/bindings';
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

import { toast } from '../../stores/useToastStore';
import { applyRuntimeMutationResult } from '../workspace-runtime/actions/sharedRuntimeResultMapper';
import { closeWorkspaceDialog } from '../workspace-runtime/state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '../workspace-runtime/state/workspaceStoreBridge';

import type { ConflictDetails, FolderDetail } from '../../types/scanner';
import { formatBytes } from '../../utils/formatters';

type Strategy = 'keep_enabled' | 'keep_disabled' | 'separate';

export default function ConflictResolveDialog() {
  const { t } = useTranslation(['folder_grid', 'common']);
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const queryClient = useQueryClient();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [loading, setLoading] = useState(false);
  const [details, setDetails] = useState<ConflictDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);

  const open = dialogState.kind === 'conflict';
  const conflict = dialogState.kind === 'conflict' ? dialogState.conflict : null;

  // Fetch details when dialog opens
  const fetchDetails = useCallback(async () => {
    if (!conflict) return;
    setDetailsLoading(true);
    setDetails(null);
    try {
      const result = await commands.getConflictDetails({
        enabledPath: conflict.attempted_target,
        disabledPath: conflict.existing_path,
      });
      setDetails(result);
    } catch (err) {
      toast.error(t('folder_grid:conflicts.toast.resolve_failed', { error: String(err) }));
    } finally {
      setDetailsLoading(false);
    }
  }, [conflict, t]);

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

      await commands.resolveConflict({
        keepPath,
        duplicatePath,
        strategy,
      });

      await applyRuntimeMutationResult(queryClient, 'workspaceStructure');

      toast.success(t('folder_grid:conflicts.toast.resolved'));
      closeWorkspaceDialog('conflict');
    } catch (err) {
      toast.error(t('folder_grid:conflicts.toast.resolve_failed', { error: String(err) }));
    } finally {
      setLoading(false);
    }
  };

  const baseName = conflict.base_name;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      onClose={() => closeWorkspaceDialog('conflict')}
    >
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-2xl">
        {/* Header */}
        <div className="flex items-start gap-3 mb-4">
          <div className="p-2 rounded-lg bg-warning/10 text-warning">
            <AlertTriangle size={20} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base text-base-content">
              {t('folder_grid:resolution.title')}
            </h3>
            <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
              {t('folder_grid:resolution.description', { name: baseName })}
            </p>
          </div>
        </div>

        {/* Side-by-side comparison */}
        {detailsLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 size={20} className="animate-spin text-base-content/30" />
            <span className="text-sm text-base-content/40 ml-2">
              {t('folder_grid:resolution.loading')}
            </span>
          </div>
        ) : details ? (
          <div className="grid grid-cols-2 gap-3 mb-4">
            {/* Enabled side */}
            <FolderColumn
              detail={details.enabled}
              label={t('folder_grid:resolution.enabled_label')}
              accentClass="text-success"
              borderClass="border-success/30"
            />
            {/* Disabled side */}
            <FolderColumn
              detail={details.disabled}
              label={t('folder_grid:resolution.disabled_label')}
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
              {t('common:actions.keep_enabled')}
              {details && (
                <span className="text-[10px] opacity-60 ml-1">
                  ({formatBytes(details.enabled.total_size)},{' '}
                  {t('folder_grid:resolution.file_count', { count: details.enabled.file_count })})
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
              {t('common:actions.keep_disabled')}
              {details && (
                <span className="text-[10px] opacity-60 ml-1">
                  ({formatBytes(details.disabled.total_size)},{' '}
                  {t('folder_grid:resolution.file_count', { count: details.disabled.file_count })})
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
            {t('common:actions.separate')}
          </button>
        </div>

        {/* Cancel */}
        <div className="modal-action mt-4">
          <button
            className="btn btn-sm btn-ghost"
            onClick={() => closeWorkspaceDialog('conflict')}
            disabled={loading}
          >
            {t('common:actions.cancel')}
          </button>
        </div>
      </div>

      {/* Backdrop */}
      <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-sm">
        <button onClick={() => closeWorkspaceDialog('conflict')}>
          {t('common:actions.close')}
        </button>
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
  const { t } = useTranslation('folder_grid');
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
        <span>{t('resolution.file_count', { count: detail.file_count })}</span>
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
            {t('resolution.more_files', { count: detail.files.length - 6 })}
          </div>
        )}
      </div>
    </div>
  );
}
