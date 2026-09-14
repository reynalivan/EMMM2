/**
 * Main container for Epic 9: Duplicate Scanner UI.
 * Displays duplicate groups with resolution controls.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03 (UI presentation and user actions)
 */

import { formatAppError } from '../../../shared/lib/appError';
import { forwardRef, useCallback, useEffect, useImperativeHandle, useState } from 'react';
import { AlertCircle, Loader2 } from 'lucide-react';
import { useDedupReport, useResolveDuplicates } from '../hooks/useDedup';
import type { DuplicateSelection, ResolutionRequest } from '@/entities/workspace';
import { buildResolutionRequests } from '../utils/resolutionRequests';
import DuplicateTable from './DuplicateTable';
import ResolutionModal from './ResolutionModal';
import { toast } from '@/shared/ui/toast';
import { useTranslation } from 'react-i18next';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';

interface Props {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
  gameId?: string;
  showApplyAction?: boolean;
  onActionStateChange?: (state: DuplicateReportActionState) => void;
}

export interface DuplicateReportActionState {
  selectionCount: number;
  isApplying: boolean;
}

export interface DuplicateReportHandle {
  requestApply: () => void;
}

const DuplicateReport = forwardRef<DuplicateReportHandle, Props>(function DuplicateReport(
  { activeFilter = 'all', gameId = '', showApplyAction = true, onActionStateChange },
  ref,
) {
  const { t } = useTranslation(['scanner']);
  const { data: report, isLoading, error } = useDedupReport(gameId);
  const { mutate: resolve, isPending } = useResolveDuplicates();

  const [selections, setSelections] = useState<Map<string, DuplicateSelection>>(new Map());
  const [showModal, setShowModal] = useState(false);

  const filteredGroups =
    report?.groups.filter((g) => {
      if (activeFilter === 'all') return true;
      if (activeFilter === 'high') return g.confidenceScore >= 100; // Requirement says High is 100% BLAKE3 confirm
      if (activeFilter === 'medium') return g.confidenceScore >= 70 && g.confidenceScore < 100;
      if (activeFilter === 'low') return g.confidenceScore < 70;
      return true;
    }) || [];

  const handleActionChange = (groupId: string, action: DuplicateSelection) => {
    const newSelections = new Map(selections);
    newSelections.set(groupId, action);
    setSelections(newSelections);
  };

  const handleApplyAll = useCallback(() => {
    if (selections.size === 0) {
      toast.warning(t('scanner:report.no_actions_selected'));
      return;
    }

    setShowModal(true);
  }, [selections.size, t]);

  useImperativeHandle(ref, () => ({ requestApply: handleApplyAll }), [handleApplyAll]);

  useEffect(() => {
    onActionStateChange?.({ selectionCount: selections.size, isApplying: isPending });
  }, [isPending, onActionStateChange, selections.size]);

  const convertSelectionsToRequests = (): ResolutionRequest[] =>
    report ? buildResolutionRequests(selections, report.groups) : [];

  const handleConfirm = () => {
    if (!report) return;

    const requests = convertSelectionsToRequests();
    resolve(
      { requests, gameId: report.gameId },
      {
        onSuccess: (summary) => {
          setShowModal(false);
          if (summary.failed === 0) {
            setSelections(new Map());
            return;
          }

          const failedGroupIds = new Set(summary.errors.map((entry) => entry.groupId));
          if (failedGroupIds.size > 0) {
            setSelections(
              (current) => new Map([...current].filter(([groupId]) => failedGroupIds.has(groupId))),
            );
          }
        },
        onError: (err) => {
          toast.error(t('scanner:report.toast.action_failed', { error: formatAppError(err) }));
        },
      },
    );
  };

  const handleCancel = () => {
    setShowModal(false);
  };

  // Loading state
  if (isLoading) {
    return (
      <div className="space-y-4" aria-busy="true" role="status">
        <WorkspacePanelSkeleton variant="list" />
        <p className="text-center text-sm text-base-content/60">{t('scanner:report.loading')}</p>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="alert alert-error">
        <AlertCircle className="w-5 h-5" />
        <div>
          <h3 className="font-bold">{t('scanner:report.error_title')}</h3>
          <div className="text-xs">{formatAppError(error)}</div>
        </div>
      </div>
    );
  }

  // No report yet
  if (!report) {
    return (
      <div className="alert alert-info">
        <AlertCircle className="w-5 h-5" />
        <div>
          <h3 className="font-bold">{t('scanner:report.no_results')}</h3>
          <div className="text-sm">{t('scanner:report.no_results_desc')}</div>
        </div>
      </div>
    );
  }

  // Empty report (no duplicates found)
  if (report.groups.length === 0) {
    return (
      <div className="alert alert-success">
        <AlertCircle className="w-5 h-5" />
        <div>
          <h3 className="font-bold">{t('scanner:report.clean_title')}</h3>
          <div className="text-sm">{t('scanner:report.clean_desc')}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between mt-2">
        <div>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            {t('scanner:report.title')}
          </h2>
          <p className="text-sm text-base-content/60">
            {t('scanner:report.stats_summary', {
              totalGroups: report.totalGroups,
              totalMembers: report.totalMembers,
            })}
          </p>
        </div>

        {showApplyAction && (
          <DuplicateReportApplyButton
            selectionCount={selections.size}
            isApplying={isPending}
            onApply={handleApplyAll}
          />
        )}
      </div>

      {/* Duplicate Table */}
      <DuplicateTable
        groups={filteredGroups}
        gameId={gameId}
        selections={selections}
        onSelectionChange={handleActionChange}
        disabled={isPending}
      />

      {/* Resolution Modal */}
      <ResolutionModal
        isOpen={showModal}
        selections={selections}
        groups={report.groups}
        onConfirm={handleConfirm}
        onCancel={handleCancel}
        isPending={isPending}
      />
    </div>
  );
});

DuplicateReport.displayName = 'DuplicateReport';

interface DuplicateReportApplyButtonProps {
  selectionCount: number;
  isApplying: boolean;
  onApply: () => void;
}

export function DuplicateReportApplyButton({
  selectionCount,
  isApplying,
  onApply,
}: DuplicateReportApplyButtonProps) {
  const { t } = useTranslation(['scanner']);

  return (
    <button
      className="btn btn-primary btn-sm gap-2 whitespace-nowrap"
      onClick={onApply}
      disabled={selectionCount === 0 || isApplying}
    >
      {isApplying ? (
        <>
          <Loader2 className="h-4 w-4 animate-spin" />
          {t('scanner:report.applying')}
        </>
      ) : (
        t('scanner:report.apply_actions', { count: selectionCount })
      )}
    </button>
  );
}

export default DuplicateReport;
