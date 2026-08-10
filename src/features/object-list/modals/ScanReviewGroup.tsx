import { useTranslation } from 'react-i18next';
import type { ScanPreviewItem } from '../../../types/scanner';
import type { GameConfig } from '../../../types/game';
import ScanReviewRow from './ScanReviewRow';
import {
  getConfidenceColor,
  getConfidenceIcon,
  type ConfidenceTier,
  type MasterDbEntry,
} from './scanReviewHelpers';

interface ScanReviewGroupProps {
  tier: ConfidenceTier;
  items: ScanPreviewItem[];
  overrides: Record<string, MasterDbEntry | null>;
  skips: Record<string, boolean>;
  selected: Set<string>;
  renames: Record<string, string>;
  masterDbEntries: MasterDbEntry[];
  activeGame: GameConfig | null;
  onOverride: (folderPath: string, entry: MasterDbEntry | null) => void;
  onToggleSkip: (folderPath: string) => void;
  onToggleSelect: (folderPath: string) => void;
  onRename: (folderPath: string, newName: string | null) => void;
  onItemReplaced: (oldPath: string, replacement: ScanPreviewItem) => void;
  onSetGroupSkipped: (items: ScanPreviewItem[], skipped: boolean) => void;
}

export default function ScanReviewGroup(props: ScanReviewGroupProps) {
  const { t } = useTranslation(['objects']);
  const skippedCount = props.items.filter((item) => props.skips[item.folderPath]).length;
  const allSkipped = skippedCount === props.items.length;

  return (
    <>
      <tr className="sticky top-8 z-20 bg-base-300/90 backdrop-blur-md">
        <td className="text-center">
          <input
            type="checkbox"
            className="checkbox checkbox-xs rounded"
            checked={allSkipped}
            ref={(input) => {
              if (input) input.indeterminate = skippedCount > 0 && !allSkipped;
            }}
            onChange={() => props.onSetGroupSkipped(props.items, !allSkipped)}
          />
        </td>
        <td colSpan={5}>
          <div className="flex items-center gap-2">
            <span
              className={`badge badge-sm badge-outline gap-1 ${getConfidenceColor(props.tier)}`}
            >
              {getConfidenceIcon(props.tier)}
              {props.tier} ({props.items.length})
            </span>
            {props.tier === 'None' && (
              <span className="text-warning text-xs">
                {t('objects:scan_review.none_group_warning')}
              </span>
            )}
          </div>
        </td>
      </tr>
      {props.items.map((item) => (
        <ScanReviewRow
          key={item.folderPath}
          item={item}
          override={props.overrides[item.folderPath] ?? null}
          onOverride={(entry) => props.onOverride(item.folderPath, entry)}
          onToggleSkip={() => props.onToggleSkip(item.folderPath)}
          isSkipped={!!props.skips[item.folderPath]}
          isSelected={props.selected.has(item.folderPath)}
          onToggleSelect={() => props.onToggleSelect(item.folderPath)}
          masterDbEntries={props.masterDbEntries}
          renamedName={props.renames[item.folderPath] ?? null}
          onRename={(newName) => props.onRename(item.folderPath, newName)}
          onItemReplaced={props.onItemReplaced}
          activeGame={props.activeGame}
        />
      ))}
    </>
  );
}
