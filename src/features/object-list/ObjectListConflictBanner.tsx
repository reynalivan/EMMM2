import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { openWorkspaceConflictDialog } from '../workspace-runtime/state/workspaceDialogs';
import type { GameConfig } from '../../types/game';
import type { WorkspaceObjectNode } from '../../types/workspace';

interface ObjectListConflictBannerProps {
  conflictObjects: WorkspaceObjectNode[];
  activeGame: GameConfig | null;
}

export default function ObjectListConflictBanner({
  conflictObjects,
  activeGame,
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
        onClick={() => {
          const obj = conflictObjects[0];
          if (!activeGame?.mod_path || !obj.folder_path) {
            return;
          }

          const baseName = obj.name;
          const modPath = activeGame.mod_path.replace(/\\/g, '/');
          openWorkspaceConflictDialog({
            type: 'RenameConflict',
            attempted_target: `${modPath}/${obj.folder_path}`,
            existing_path: `${modPath}/DISABLED ${baseName}`,
            base_name: baseName,
          });
        }}
      >
        {t('item.resolve')}
      </button>
    </div>
  );
}
