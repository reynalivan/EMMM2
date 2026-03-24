/**
 * Main container for Epic 9: Duplicate Scanner UI.
 * Displays duplicate groups with resolution controls.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03 (UI presentation and user actions)
 */

import { useState } from 'react';
import { AlertCircle, Loader2, ShieldOff, Lock } from 'lucide-react';
import { useDedupReport, useResolveDuplicates } from '../hooks/useDedup';
import type { ResolutionAction, ResolutionRequest } from '../../../types/scanner';
import DuplicateTable from './DuplicateTable';
import ResolutionModal from './ResolutionModal';
import { toast } from '../../../stores/useToastStore';
import { useSettings } from '../../../hooks/useSettings';
import PinEntryModal from '../../safe-mode/PinEntryModal';
import { useTranslation } from 'react-i18next';

interface Props {
  activeFilter?: 'all' | 'high' | 'medium' | 'low';
}

export default function DuplicateReport({ activeFilter = 'all' }: Props) {
  const { t } = useTranslation(['scanner']);
  const [pin, setPin] = useState<string | undefined>();
  const [isPinModalOpen, setIsPinModalOpen] = useState(false);
  const { data: report, isLoading, error } = useDedupReport(pin);
  const { mutate: resolve, isPending } = useResolveDuplicates();
  const { settings } = useSettings();

  const [selections, setSelections] = useState<Map<string, ResolutionAction>>(new Map());
  const [showModal, setShowModal] = useState(false);

  const filteredGroups =
    report?.groups.filter((g) => {
      if (activeFilter === 'all') return true;
      if (activeFilter === 'high') return g.confidenceScore >= 100; // Requirement says High is 100% BLAKE3 confirm
      if (activeFilter === 'medium') return g.confidenceScore >= 70 && g.confidenceScore < 100;
      if (activeFilter === 'low') return g.confidenceScore < 70;
      return true;
    }) || [];

  const handleActionChange = (groupId: string, action: ResolutionAction) => {
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

  const convertSelectionsToRequests = (): ResolutionRequest[] => {
    if (!report) return [];

    const requests: ResolutionRequest[] = [];
    selections.forEach((selection, groupId) => {
      const group = report.groups.find((g) => g.groupId === groupId);
      if (!group || !selection) return;

      if (selection.type === 'Keep') {
        requests.push({
          groupId,
          action: 'Keep',
          targetPath: selection.targetPath,
          allMembers: group.members.map((m) => m.folderPath),
        });
      } else if (selection.type === 'Ignore') {
        requests.push({
          groupId,
          action: 'Ignore',
          allMembers: group.members.map((m) => m.folderPath),
        });
      }
    });

    return requests;
  };

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
          toast.error(t('scanner:report.toast.action_failed', { error: String(err) }));
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
          <div className="text-xs">{String(error)}</div>
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
            {pin && (
              <ShieldOff
                className="text-warning h-5 w-5"
                aria-label={t('scanner:report.unsafe_revealed_label')}
              />
            )}
          </h2>
          <p className="text-sm text-base-content/60">
            {t('scanner:report.stats_summary', {
              totalGroups: report.totalGroups,
              totalMembers: report.totalMembers,
            })}
          </p>
        </div>

        <div className="flex gap-2">
          {/* Reveal Unsafe Toggle */}
          {settings?.safe_mode.enabled && !pin && (
            <button
              className="btn btn-warning btn-outline gap-2"
              onClick={() => setIsPinModalOpen(true)}
            >
              <Lock size={16} />
              {t('scanner:report.reveal_unsafe')}
            </button>
          )}

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

      {/* PIN Entry Modal */}
      <PinEntryModal
        open={isPinModalOpen}
        onClose={() => setIsPinModalOpen(false)}
        onSuccess={(v) => {
          setPin(v);
          toast.success(t('scanner:report.unsafe_revealed'));
        }}
        title={t('scanner:report.pin_title')}
        description={t('scanner:report.pin_desc')}
      />
    </div>
  );
}
