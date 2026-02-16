/**
 * Accessible table for displaying duplicate groups with resolution controls.
 * Implements ARIA patterns for table, radiogroup, and keyboard navigation.
 * Covers: TC-9.5-02 (UI controls for resolution actions)
 */

import { FileWarning } from 'lucide-react';
import type { DupScanGroup, ResolutionAction } from '../../../types/dedup';

interface Props {
  groups: DupScanGroup[];
  selections: Map<string, ResolutionAction>;
  onSelectionChange: (groupId: string, action: ResolutionAction) => void;
  disabled?: boolean;
}

/**
 * Get DaisyUI badge class based on confidence score.
 * 90%+ = success, 70%+ = warning, <70% = error
 */
const getConfidenceBadge = (score: number): string => {
  if (score >= 90) return 'badge-success';
  if (score >= 70) return 'badge-warning';
  return 'badge-error';
};

/**
 * Format bytes to human-readable size.
 */
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

export default function DuplicateTable({
  groups,
  selections,
  onSelectionChange,
  disabled = false,
}: Props) {
  return (
    <div className="overflow-x-auto border border-base-300 rounded-lg bg-base-100">
      <table className="table table-sm table-pin-rows" role="table" aria-label="Duplicate groups">
        <thead>
          <tr role="row" className="bg-base-200 text-base-content/70">
            <th scope="col" className="w-24">
              Confidence
            </th>
            <th scope="col" className="w-1/3">
              Match Reason
            </th>
            <th scope="col" className="w-48">
              Members
            </th>
            <th scope="col">Action</th>
          </tr>
        </thead>
        <tbody>
          {groups.map((group) => {
            const selectedAction = selections.get(group.groupId);
            const isPair = group.members.length === 2;

            return (
              <tr key={group.groupId} className="hover:bg-base-200/50 transition-colors">
                {/* Confidence Score */}
                <td>
                  <span className={`badge ${getConfidenceBadge(group.confidenceScore)}`}>
                    {group.confidenceScore}%
                  </span>
                </td>

                {/* Match Reason */}
                <td>
                  <div className="flex items-start gap-2">
                    <FileWarning className="w-4 h-4 text-warning flex-shrink-0 mt-0.5" />
                    <div>
                      <p className="font-medium truncate max-w-xs" title={group.matchReason}>
                        {group.matchReason}
                      </p>
                      {group.signals.length > 0 && (
                        <p className="text-xs text-base-content/50 truncate max-w-xs">
                          {group.signals.map((s) => s.key).join(', ')}
                        </p>
                      )}
                    </div>
                  </div>
                </td>

                {/* Members */}
                <td>
                  <div className="flex flex-col gap-1">
                    {group.members.slice(0, 2).map((member, idx) => (
                      <div
                        key={member.folderPath}
                        className="text-xs bg-base-200 px-2 py-1 rounded truncate max-w-xs"
                        title={member.folderPath}
                      >
                        <span className="font-mono text-primary">{idx === 0 ? 'A' : 'B'}:</span>{' '}
                        {member.displayName}
                        <span className="text-base-content/50 ml-1">
                          ({formatBytes(member.totalSizeBytes)})
                        </span>
                      </div>
                    ))}
                    {group.members.length > 2 && (
                      <div className="text-xs text-base-content/50">
                        +{group.members.length - 2} more
                      </div>
                    )}
                  </div>
                </td>

                {/* Action Controls */}
                <td>
                  {!isPair ? (
                    <div className="alert alert-warning alert-sm py-2 px-3">
                      <span className="text-xs">Multi-member groups not supported</span>
                    </div>
                  ) : (
                    <div
                      className="flex gap-2 flex-wrap"
                      role="radiogroup"
                      aria-label={`Resolution actions for duplicate group ${group.groupId}`}
                    >
                      {/* Keep A (Delete B) */}
                      <label
                        className={`btn btn-sm btn-outline ${
                          selectedAction === 'KeepA' ? 'btn-primary' : 'btn-ghost'
                        }`}
                      >
                        <input
                          type="radio"
                          name={`action-${group.groupId}`}
                          value="KeepA"
                          checked={selectedAction === 'KeepA'}
                          onChange={() => onSelectionChange(group.groupId, 'KeepA')}
                          disabled={disabled}
                          className="sr-only"
                          aria-label="Keep A, delete B"
                        />
                        Keep A
                      </label>

                      {/* Keep B (Delete A) */}
                      <label
                        className={`btn btn-sm btn-outline ${
                          selectedAction === 'KeepB' ? 'btn-primary' : 'btn-ghost'
                        }`}
                      >
                        <input
                          type="radio"
                          name={`action-${group.groupId}`}
                          value="KeepB"
                          checked={selectedAction === 'KeepB'}
                          onChange={() => onSelectionChange(group.groupId, 'KeepB')}
                          disabled={disabled}
                          className="sr-only"
                          aria-label="Keep B, delete A"
                        />
                        Keep B
                      </label>

                      {/* Ignore (Whitelist) */}
                      <label
                        className={`btn btn-sm btn-outline ${
                          selectedAction === 'Ignore' ? 'btn-warning' : 'btn-ghost'
                        }`}
                      >
                        <input
                          type="radio"
                          name={`action-${group.groupId}`}
                          value="Ignore"
                          checked={selectedAction === 'Ignore'}
                          onChange={() => onSelectionChange(group.groupId, 'Ignore')}
                          disabled={disabled}
                          className="sr-only"
                          aria-label="Ignore this duplicate pair"
                        />
                        Ignore
                      </label>
                    </div>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>

      {groups.length === 0 && (
        <div className="text-center py-8 text-base-content/50 text-sm">
          No duplicate groups found. Your library is clean!
        </div>
      )}
    </div>
  );
}
