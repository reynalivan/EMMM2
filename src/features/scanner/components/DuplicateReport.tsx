/**
 * Main container for Epic 9: Duplicate Scanner UI.
 * Displays duplicate groups with resolution controls.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03 (UI presentation and user actions)
 */

import { formatAppError } from '../../../shared/lib/appError';
import { useState } from 'react';
import { AlertCircle, Loader2 } from 'lucide-react';
import { useDedupReport, useResolveDuplicates } from '../hooks/useDedup';
import type { DuplicateSelection, ResolutionRequest } from '@/entities/workspace/model/scanner';
import { buildResolutionRequests } from '../utils/resolutionRequests';
import DuplicateTable from './DuplicateTable';
import ResolutionModal from './ResolutionModal';
import { toast } from '../../../app/store/useToastStore';
import { useTranslation } from 'react-i18next';

interface Props {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
  gameId?: string;
}

export default function DuplicateReport({ activeFilter = 'all', gameId = '' }: Props) {
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

  const handleApplyAll = () => {
    if (selections.size === 0) {
      toast.warning(t('scanner:report.no_actions_selected'));
      return;
    }

    setShowModal(true);
  };

  const convertSelectionsToRequests = (): ResolutionRequest[] =>
    report ? buildResolutionRequests(selections, report.groups) : [];

  const handleConfirm = () => {
    if (!report) return;

    const requests = convertSelectionsToRequests();
    resolve(
      { requests, gameId: report.gameId },
      {
        onSuccess: () => {
          setShowModal(false);
          setSelections(new Map()); // Clear selections after success
          toast.success(t('scanner:report.toast.actions_applied'));
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
      <div className="flex items-center justify-center h-64 gap-3">
        <Loader2 className="w-6 h-6 animate-spin text-primary" />
        <span className="text-base-content/60">{t('scanner:report.loading')}</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="alert alert-error shadow-lg">
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
      <div className="alert alert-info shadow-lg">
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
      <div className="alert alert-success shadow-lg">
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
          <h2 className="text-2xl font-bold flex items-center gap-2">
            {t('scanner:report.title')}
          </h2>
          <p className="text-sm text-base-content/60">
            {t('scanner:report.stats_summary', {
              totalGroups: report.totalGroups,
              totalMembers: report.totalMembers,
            })}
          </p>
        </div>

        <div className="flex gap-2">
          {/* Apply All Button */}
          <button
            className="btn btn-primary"
            onClick={handleApplyAll}
            disabled={selections.size === 0 || isPending}
          >
            {isPending ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                {t('scanner:report.applying')}
              </>
            ) : (
              t('scanner:report.apply_actions', { count: selections.size })
            )}
          </button>
        </div>
      </div>

      {/* Duplicate Table */}
      <DuplicateTable
        groups={filteredGroups}
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
}
