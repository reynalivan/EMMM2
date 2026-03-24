/**
 * Resolution Confirmation Modal for Epic 9.
 * Shows a summary of intended actions (Keep/Ignore) before applying.
 */

import { Trash2, ShieldCheck, AlertTriangle, X } from 'lucide-react';
import type { DupScanGroup, ResolutionAction } from '../../../types/scanner';
import { useTranslation } from 'react-i18next';

interface Props {
  isOpen: boolean;
  selections: Map<string, ResolutionAction>;
  groups: DupScanGroup[];
  onConfirm: () => void;
  onCancel: () => void;
  isPending?: boolean;
}

export default function ResolutionModal({
  isOpen,
  selections,
  groups,
  onConfirm,
  onCancel,
  isPending,
}: Props) {
  const { t } = useTranslation(['scanner']);
  if (!isOpen) return null;

  const selectedGroups = Array.from(selections.entries())
    .map(([groupId, action]) => ({
      group: groups.find((g) => g.groupId === groupId),
      action,
    }))
    .filter((item) => item.group && item.action);

  const totalToDelete = selectedGroups.reduce((acc, item) => {
    if (item.action?.type === 'Keep') {
      return acc + (item.group?.members.length || 0) - 1;
    }
    return acc;
  }, 0);

  const totalToIgnore = selectedGroups.filter((s) => s.action?.type === 'Ignore').length;

  return (
    <div
      className="modal modal-open"
      role="dialog"
      aria-modal="true"
      aria-labelledby="resolution-modal-title"
      aria-describedby="resolution-modal-description"
    >
      <div className="modal-box max-w-2xl bg-base-100 border border-base-300 shadow-2xl p-0 overflow-hidden">
        {/* Header */}
        <div className="bg-base-200/50 px-6 py-4 flex items-center justify-between border-b border-base-300">
          <div className="flex items-center gap-2">
            <AlertTriangle className="text-warning w-5 h-5" />
            <h3 id="resolution-modal-title" className="text-lg font-bold">
              {t('scanner:resolution.title')}
            </h3>
          </div>
          <button
            className="btn btn-sm btn-ghost btn-circle"
            onClick={onCancel}
            disabled={isPending}
          >
            <X size={18} />
          </button>
        </div>

        <div className="p-6">
          <div className="alert alert-warning mb-6 shadow-sm">
            <InfoIcon className="w-5 h-5" />
            <p id="resolution-modal-description" className="text-sm">
              {t('scanner:resolution.summary', {
                count: selectedGroups.length,
                deleted: totalToDelete,
                ignored: totalToIgnore,
              })}
            </p>
          </div>

          <div className="space-y-4 max-h-96 overflow-y-auto pr-2 custom-scrollbar" role="list">
            {selectedGroups.map(({ group, action }) => (
              <div
                key={group?.groupId}
                className="p-4 rounded-xl border border-base-300 bg-base-200/30"
              >
                <div className="flex justify-between items-start mb-2">
                  <span className="text-xs font-mono opacity-50 uppercase tracking-tighter">
                    {t('scanner:table.header.group')}: {group?.groupId.slice(0, 8)}...
                  </span>
                  {action?.type === 'Keep' ? (
                    <span className="badge badge-primary badge-sm gap-1">
                      <ShieldCheck size={12} /> {t('scanner:resolution.keep_specific')}
                    </span>
                  ) : (
                    <span className="badge badge-warning badge-sm gap-1">
                      <ShieldCheck size={12} /> {t('scanner:resolution.whitelist')}
                    </span>
                  )}
                </div>

                <div className="space-y-1">
                  {action?.type === 'Keep' ? (
                    <>
                      <div className="text-sm flex items-center gap-2">
                        <span className="text-success font-bold">
                          {t('scanner:resolution.keep_label')}
                        </span>
                        <span className="truncate">
                          {
                            group?.members.find((m) => m.folderPath === action.targetPath)
                              ?.displayName
                          }
                        </span>
                      </div>
                      <div className="text-xs text-error flex items-center gap-2 pl-2 border-l-2 border-error/30 mt-1 italic">
                        <Trash2 size={12} />
                        {t('scanner:resolution.delete_others', {
                          count: group!.members.length - 1,
                        })}
                      </div>
                    </>
                  ) : (
                    <div className="text-sm flex items-center gap-2">
                      <span className="text-warning font-bold">
                        {t('scanner:resolution.ignore_label')}
                      </span>
                      <span>
                        {t('scanner:resolution.ignore_desc', { count: group?.members.length })}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>

          <div className="mt-8 flex justify-end gap-3">
            <button className="btn btn-ghost" onClick={onCancel} disabled={isPending}>
              {t('common:actions.cancel')}
            </button>
            <button
              className="btn btn-primary px-8 shadow-lg shadow-primary/20"
              onClick={onConfirm}
              disabled={isPending}
            >
              {isPending ? (
                <>
                  <span className="loading loading-spinner loading-xs"></span>
                  {t('scanner:resolution.processing')}
                </>
              ) : (
                t('scanner:resolution.confirm_button')
              )}
            </button>
          </div>
        </div>
      </div>
      <div className="modal-backdrop bg-overlay-mask backdrop-blur-sm" onClick={onCancel}></div>
    </div>
  );
}

function InfoIcon({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      className={`stroke-current shrink-0 h-6 w-6 ${className}`}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      ></path>
    </svg>
  );
}
