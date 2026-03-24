import { AlertCircle, CheckCircle, Info, Trash2 } from 'lucide-react';

import type { DupScanGroup, ResolutionAction } from '../../../types/scanner';
import { formatSize } from '../../../utils/formatters';
import { useTranslation } from 'react-i18next';

interface Props {
  groups: DupScanGroup[];
  selections: Map<string, ResolutionAction>;
  onSelectionChange: (groupId: string, action: ResolutionAction) => void;
  disabled?: boolean;
}

export default function DuplicateTable({ groups, selections, onSelectionChange, disabled }: Props) {
  const { t } = useTranslation(['scanner']);
  if (groups.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 px-4 border-2 border-dashed border-base-300 rounded-xl bg-base-200/30">
        <Info className="w-12 h-12 text-base-content/20 mb-4" />
        <h3 className="text-lg font-semibold text-base-content/60">{t('scanner:table.empty')}</h3>
        <p className="text-sm text-base-content/40">{t('scanner:table.empty_desc')}</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto rounded-xl border border-base-300 bg-base-100 shadow-sm">
      <table className="table table-zebra w-full border-separate border-spacing-0">
        <thead className="bg-base-200/50">
          <tr>
            <th className="w-16 text-center">{t('scanner:table.header.group')}</th>
            <th className="w-1/3">{t('scanner:table.header.members')}</th>
            <th className="w-1/4">{t('scanner:table.header.reason')}</th>
            <th className="w-1/4">{t('scanner:table.header.action')}</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-base-300">
          {groups.map((group, index) => {
            const selectedAction = selections.get(group.groupId);

            return (
              <tr key={group.groupId} className="hover:bg-base-200/30 transition-colors">
                {/* ID/Index */}
                <td className="text-center">
                  <span className="badge badge-outline font-mono text-xs opacity-50">
                    {index + 1}
                  </span>
                </td>

                {/* Members List */}
                <td>
                  <div className="flex flex-col gap-3">
                    {group.members.map((member, mIdx) => (
                      <div
                        key={member.folderPath}
                        className={`p-3 rounded-lg border transition-all ${
                          selectedAction?.type === 'Keep' &&
                          selectedAction.targetPath === member.folderPath
                            ? 'bg-primary/5 border-primary/30 ring-1 ring-primary/20'
                            : selectedAction?.type === 'Keep'
                              ? 'bg-error/5 border-error/20 opacity-60 grayscale-[0.5]'
                              : 'bg-base-200/50 border-base-300'
                        }`}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center gap-2">
                            <span className="badge badge-sm font-bold bg-base-content text-base-100">
                              {String.fromCharCode(65 + mIdx)}
                            </span>
                            <div className="flex flex-col">
                              <span
                                className="font-medium text-sm truncate max-w-50"
                                title={member.displayName}
                              >
                                {member.displayName}
                              </span>
                              <span
                                className="text-[10px] text-base-content/50 font-mono truncate max-w-50"
                                title={member.folderPath}
                              >
                                {member.folderPath}
                              </span>
                            </div>
                          </div>
                          <div className="flex items-center gap-2 shrink-0">
                            <span className="text-[10px] opacity-70 font-mono">
                              {formatSize(member.totalSizeBytes)}
                            </span>
                            {selectedAction?.type === 'Keep' &&
                            selectedAction.targetPath === member.folderPath ? (
                              <CheckCircle
                                className="w-4 h-4 text-primary"
                                data-testid="check-circle-icon"
                              />
                            ) : selectedAction?.type === 'Keep' ? (
                              <Trash2 className="w-4 h-4 text-error" data-testid="trash-icon" />
                            ) : null}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </td>

                {/* Reason & Stats */}
                <td>
                  <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2">
                      <div
                        className={`radial-progress text-primary bg-base-300 border-4 border-base-300`}
                        style={{
                          // @ts-expect-error - radial progress custom props
                          '--value': group.confidenceScore,
                          '--size': '2.5rem',
                          '--thickness': '3px',
                        }}
                        role="progressbar"
                        aria-valuenow={group.confidenceScore}
                        aria-valuemin={0}
                        aria-valuemax={100}
                      >
                        <span className="text-[10px] font-bold">{group.confidenceScore}%</span>
                      </div>
                      <div className="flex flex-col">
                        <span className="text-xs font-bold uppercase tracking-wider opacity-60">
                          {t('scanner:table.confidence')}
                        </span>
                        <span
                          className={`text-[10px] font-bold ${
                            group.confidenceScore >= 90
                              ? 'text-success'
                              : group.confidenceScore >= 70
                                ? 'text-primary'
                                : 'text-warning'
                          }`}
                        >
                          {group.confidenceScore >= 90
                            ? t('scanner:optimizer.tabs.high')
                            : group.confidenceScore >= 70
                              ? t('scanner:optimizer.tabs.medium')
                              : t('scanner:optimizer.tabs.low')}
                        </span>
                      </div>
                    </div>

                    <div className="flex flex-wrap gap-1 mt-1">
                      {group.matchReason.split(',').map((reason, rIdx) => (
                        <span
                          key={rIdx}
                          className="badge badge-sm badge-ghost text-[10px] whitespace-nowrap"
                        >
                          {reason.trim()}
                        </span>
                      ))}
                    </div>

                    {group.isUnsafe && (
                      <div className="flex items-center gap-1 text-warning">
                        <AlertCircle size={12} />
                        <span className="text-[10px] font-bold uppercase">
                          {t('scanner:table.unsafe')}
                        </span>
                      </div>
                    )}
                  </div>
                </td>

                {/* Action Controls */}
                <td>
                  <div className="flex flex-col gap-2">
                    <select
                      className={`select select-sm select-bordered w-full max-w-xs transition-colors ${
                        selectedAction?.type === 'Keep'
                          ? 'select-primary border-primary/50'
                          : selectedAction?.type === 'Ignore'
                            ? 'select-warning border-warning/50'
                            : ''
                      }`}
                      value={
                        selectedAction?.type === 'Keep'
                          ? selectedAction.targetPath
                          : selectedAction?.type === 'Ignore'
                            ? 'ignore'
                            : 'pending'
                      }
                      onChange={(e) => {
                        const val = e.target.value;
                        if (val === 'pending') {
                          // No-op
                        } else if (val === 'ignore') {
                          onSelectionChange(group.groupId, { type: 'Ignore' });
                        } else {
                          onSelectionChange(group.groupId, { type: 'Keep', targetPath: val });
                        }
                      }}
                      disabled={disabled}
                      aria-label={t('scanner:table.header.action')}
                    >
                      <option value="pending" disabled>
                        {t('scanner:table.select_action')}
                      </option>
                      <optgroup label={t('scanner:table.keep_one')}>
                        {group.members.map((m, idx) => (
                          <option key={m.folderPath} value={m.folderPath}>
                            {t('scanner:table.keep_label', {
                              id: String.fromCharCode(65 + idx),
                              name: m.displayName,
                            })}
                          </option>
                        ))}
                      </optgroup>
                      <optgroup label={t('scanner:table.general')}>
                        <option value="ignore">{t('scanner:table.ignore')}</option>
                      </optgroup>
                    </select>

                    {selectedAction?.type === 'Keep' && (
                      <span className="text-[10px] text-error flex items-center gap-1 px-1">
                        <Trash2 size={10} />
                        {t('scanner:table.will_delete', { count: group.members.length - 1 })}
                      </span>
                    )}
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
