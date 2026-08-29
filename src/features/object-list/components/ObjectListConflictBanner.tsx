import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { openFolderConflictManagerDialog } from '../../workspace-runtime/state/workspaceDialogs';
import type { WorkspaceObjectNode } from '../../../types/workspace';

interface ObjectListConflictBannerProps {
  conflictObjects: WorkspaceObjectNode[];
}

export default function ObjectListConflictBanner({
  conflictObjects,
}: ObjectListConflictBannerProps) {
  const { t } = useTranslation(['objects']);

  if (conflictObjects.length === 0) {
    return null;
  }

  return (
    <div className="mx-2 mt-1 mb-0.5 flex items-center gap-1.5 bg-warning/10 border border-warning/20 rounded-md px-2 py-1">
      <AlertTriangle size={12} className="text-warning shrink-0" />
      <span className="text-[10px] text-warning flex-1 truncate">
        {t('item.naming_conflict', { count: conflictObjects.length })}
      </span>
      <button
        className="text-[10px] text-warning font-semibold hover:underline shrink-0"
        onClick={openFolderConflictManagerDialog}
      >
        {t('item.resolve')}
      </button>
    </div>
  );
}
