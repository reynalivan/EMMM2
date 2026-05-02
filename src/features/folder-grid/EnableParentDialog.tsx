import { Lock, CheckCircle2, PowerOff, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceExplorerNode } from '../../types/workspace';

interface EnableParentDialogProps {
  open: boolean;
  onClose: () => void;
  /** Display name of the nearest disabled ancestor folder */
  ancestorName: string;
  /** Child folders whose own name has NO DISABLED prefix — will become active */
  willActivate: WorkspaceExplorerNode[];
  /** Child folders whose own name HAS DISABLED prefix — stay disabled regardless */
  stayDisabled: WorkspaceExplorerNode[];
  /** Callback to trigger the actual parent enable toggle */
  onConfirm: () => void;
}

function nodeIcon(nodeType: string) {
  switch (nodeType) {
    case 'ModPackRoot':
    case 'FlatModRoot':
    case 'VariantContainer':
      return <FolderOpen size={12} className="shrink-0 text-primary/70" />;
    default:
      return <FolderOpen size={12} className="shrink-0 text-base-content/40" />;
  }
}

export default function EnableParentDialog({
  open,
  onClose,
  ancestorName,
  willActivate,
  stayDisabled,
  onConfirm,
}: EnableParentDialogProps) {
  const { t } = useTranslation(['grid']);

  if (!open) return null;

  const totalWillActivate = willActivate.length;
  const totalStayDisabled = stayDisabled.length;

  return (
    <dialog className="modal modal-open" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal-box max-w-md w-full">
        {/* Header */}
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 rounded-lg bg-warning/15 text-warning">
            <Lock size={18} />
          </div>
          <div>
            <h3 className="font-bold text-base">{t('enable_parent_dialog.title')}</h3>
            <p className="text-xs text-base-content/50 mt-0.5">
              {t('enable_parent_dialog.ancestor_label', { name: ancestorName })}
            </p>
          </div>
        </div>

        {/* Description */}
        <p className="text-sm text-base-content/70 mb-4">
          {t('enable_parent_dialog.desc', { name: ancestorName })}
        </p>

        {/* Impact list */}
        {(totalWillActivate > 0 || totalStayDisabled > 0) && (
          <div className="rounded-lg border border-base-content/10 overflow-hidden mb-5">
            {/* Will activate */}
            {totalWillActivate > 0 && (
              <div>
                <div className="flex items-center gap-2 px-3 py-1.5 bg-success/8 border-b border-base-content/8">
                  <CheckCircle2 size={11} className="text-success" />
                  <span className="text-[10px] font-semibold text-success uppercase tracking-wider">
                    {t('enable_parent_dialog.will_activate', { count: totalWillActivate })}
                  </span>
                </div>
                <ul className="divide-y divide-base-content/5 max-h-32 overflow-y-auto scrollbar-thin scrollbar-thumb-base-content/10">
                  {willActivate.map((f) => (
                    <li key={f.path} className="flex items-center gap-2 px-3 py-1.5">
                      {nodeIcon(f.node_type)}
                      <span className="text-xs truncate text-base-content/80">{f.name}</span>
                      <span className="ml-auto text-[9px] text-base-content/30 font-mono shrink-0">
                        {f.node_type}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Stay disabled */}
            {totalStayDisabled > 0 && (
              <div>
                <div className="flex items-center gap-2 px-3 py-1.5 bg-base-200 border-b border-base-content/8">
                  <PowerOff size={11} className="text-base-content/40" />
                  <span className="text-[10px] font-semibold text-base-content/40 uppercase tracking-wider">
                    {t('enable_parent_dialog.stay_disabled', { count: totalStayDisabled })}
                  </span>
                </div>
                <ul className="divide-y divide-base-content/5 max-h-24 overflow-y-auto scrollbar-thin scrollbar-thumb-base-content/10">
                  {stayDisabled.map((f) => (
                    <li key={f.path} className="flex items-center gap-2 px-3 py-1.5 opacity-50">
                      {nodeIcon(f.node_type)}
                      <span className="text-xs truncate line-through text-base-content/50">
                        {f.name}
                      </span>
                      <span className="ml-auto text-[9px] text-base-content/30 font-mono shrink-0 no-underline">
                        {t('enable_parent_dialog.own_prefix')}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}

        {/* Actions */}
        <div className="modal-action mt-0">
          <button className="btn btn-sm btn-ghost" onClick={onClose}>
            {t('enable_parent_dialog.cancel')}
          </button>
          <button className="btn btn-sm btn-warning" onClick={onConfirm}>
            <Lock size={13} />
            {t('enable_parent_dialog.confirm', { name: ancestorName })}
          </button>
        </div>
      </div>
    </dialog>
  );
}
