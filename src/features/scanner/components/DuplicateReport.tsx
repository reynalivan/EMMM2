/**
 * Main container for Epic 9: Duplicate Scanner UI.
 * Displays duplicate groups with resolution controls.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03 (UI presentation and user actions)
 */

import { useState } from 'react';
import { AlertCircle, Loader2 } from 'lucide-react';
import { useDedupReport, useResolveDuplicates } from '../../../hooks/useDedup';
import type { ResolutionAction, ResolutionRequest } from '../../../types/dedup';
import DuplicateTable from './DuplicateTable';
import ResolutionModal from './ResolutionModal';
import { toast } from '../../../stores/useToastStore';

export default function DuplicateReport() {
  const { data: report, isLoading, error } = useDedupReport();
  const { mutate: resolve, isPending } = useResolveDuplicates();

  // Local state: Map<groupId, ResolutionAction>
  const [selections, setSelections] = useState<Map<string, ResolutionAction>>(new Map());
  const [showModal, setShowModal] = useState(false);

  const handleActionChange = (groupId: string, action: ResolutionAction) => {
    const newSelections = new Map(selections);
    newSelections.set(groupId, action);
    setSelections(newSelections);
  };

  const handleApplyAll = () => {
    if (selections.size === 0) {
      toast.warning(
        'No actions selected. Please choose an action for at least one duplicate group.',
      );
      return;
    }

    // Validate all selected groups have 2 members (pairs only)
    const invalidGroups = Array.from(selections.keys()).filter((groupId) => {
      const group = report?.groups.find((g) => g.groupId === groupId);
      return !group || group.members.length !== 2;
    });

    if (invalidGroups.length > 0) {
      toast.error(`Cannot resolve groups with non-pair members: ${invalidGroups.join(', ')}`);
      return;
    }

    setShowModal(true);
  };

  const convertSelectionsToRequests = (): ResolutionRequest[] => {
    if (!report) return [];

    const requests: ResolutionRequest[] = [];
    selections.forEach((action, groupId) => {
      const group = report.groups.find((g) => g.groupId === groupId);
      if (!group || group.members.length !== 2) return;

      requests.push({
        groupId,
        action,
        folderA: group.members[0].folderPath,
        folderB: group.members[1].folderPath,
      });
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
        <span className="text-base-content/60">Loading duplicate report...</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="alert alert-error shadow-lg">
        <AlertCircle className="w-5 h-5" />
        <div>
          <h3 className="font-bold">Failed to load duplicate report</h3>
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
          <h3 className="font-bold">No scan results available</h3>
          <div className="text-sm">Run a duplicate scan to see results here.</div>
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
          <h3 className="font-bold">No duplicates found</h3>
          <div className="text-sm">Your mod library is clean!</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Duplicate Scan Results</h2>
          <p className="text-sm text-base-content/60">
            Found {report.totalGroups} duplicate group(s) with {report.totalMembers} total members
          </p>
        </div>

        {/* Apply All Button */}
        <button
          className="btn btn-primary"
          onClick={handleApplyAll}
          disabled={selections.size === 0 || isPending}
        >
          {isPending ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Applying...
            </>
          ) : (
            `Apply ${selections.size} Action${selections.size !== 1 ? 's' : ''}`
          )}
        </button>
      </div>

      {/* Duplicate Table */}
      <DuplicateTable
        groups={report.groups}
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
