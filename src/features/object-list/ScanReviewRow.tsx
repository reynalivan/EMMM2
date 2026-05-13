import { useCallback, useRef, useState, type RefObject } from 'react';
import { Check, ExternalLink, FolderOpen, Pencil, SkipForward, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import { commands } from '../../lib/bindings';
import type { ScanPreviewItem } from '../../lib/services/scanService';
import type { GameConfig } from '../../types/game';
import FolderTooltip from './FolderTooltip';
import { type MasterDbEntry, getConfidenceColor, getConfidenceIcon } from './scanReviewHelpers';
import { ScanReviewMatchCell } from './ScanReviewMatchCell';

interface ScanReviewRowProps {
  item: ScanPreviewItem;
  override: MasterDbEntry | null;
  onOverride: (entry: MasterDbEntry | null) => void;
  onToggleSkip: () => void;
  isSkipped: boolean;
  isSelected: boolean;
  onToggleSelect: () => void;
  masterDbEntries: MasterDbEntry[];
  renamedName: string | null;
  onRename: (newName: string | null) => void;
  activeGame: GameConfig | null;
}

export default function ScanReviewRow({
  item,
  override,
  onOverride,
  onToggleSkip,
  isSkipped,
  isSelected,
  onToggleSelect,
  masterDbEntries,
  renamedName,
  onRename,
  activeGame,
}: ScanReviewRowProps) {
  const { t } = useTranslation(['objects', 'common']);
  const [isRenaming, setIsRenaming] = useState(false);
  const [editName, setEditName] = useState('');
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const displayFolderName = renamedName ?? item.displayName;
  const displayType = override?.object_type ?? item.objectType;

  const startRename = useCallback(() => {
    setEditName(displayFolderName);
    setIsRenaming(true);
    setTimeout(() => renameInputRef.current?.select(), 0);
  }, [displayFolderName]);

  const commitRename = useCallback(() => {
    const trimmed = editName.trim();
    onRename(trimmed && trimmed !== item.displayName ? trimmed : null);
    setIsRenaming(false);
  }, [editName, item.displayName, onRename]);

  const cancelRename = useCallback(() => {
    setIsRenaming(false);
  }, []);

  const contextMenuContent = (
    <>
      <ContextMenuItem
        icon={ExternalLink}
        onClick={() => {
          if (activeGame?.id) {
            commands
              .openInExplorer({ gameId: activeGame.id, path: item.folderPath })
              .catch(console.error);
          }
        }}
      >
        {t('context.reveal_source')}
      </ContextMenuItem>
      <ContextMenuItem
        icon={FolderOpen}
        disabled={(!item.matchedEntryKey && !override) || !activeGame}
        onClick={() => revealMatchedObject(activeGame, override, item, masterDbEntries)}
      >
        {t('context.reveal_dest')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pencil} onClick={startRename}>
        {t('context.rename_folder')}
      </ContextMenuItem>
    </>
  );

  return (
    <ContextMenu content={contextMenuContent}>
      <tr
        className={`group transition-all duration-150 ${
          isSkipped ? 'opacity-40 bg-base-300/10' : ''
        } ${item.alreadyMatched ? 'bg-base-200/20' : ''}`}
      >
        <td className="w-10 text-center">
          <input
            type="checkbox"
            className={`checkbox checkbox-sm checkbox-primary rounded transition-opacity duration-200 ${
              isSelected ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            checked={isSelected}
            onChange={onToggleSelect}
            disabled={isSkipped && !isSelected}
          />
        </td>
        <td className="max-w-xs truncate">
          <FolderTooltip
            folderPath={item.folderPath}
            thumbnailPath={item.thumbnailPath}
            gameId={activeGame?.id || ''}
          >
            <div className="flex flex-col">
              {isRenaming ? (
                <RenameControls
                  value={editName}
                  inputRef={renameInputRef}
                  onChange={setEditName}
                  onCommit={commitRename}
                  onCancel={cancelRename}
                />
              ) : (
                <span
                  className="font-medium text-sm text-base-content truncate cursor-default"
                  onDoubleClick={startRename}
                >
                  {displayFolderName}
                  {renamedName && <Pencil size={10} className="inline ml-1.5 text-info/60" />}
                  {item.alreadyMatched && (
                    <span className="badge badge-xs badge-ghost ml-2 opacity-60">
                      {t('objects:item.badge_existing')}
                    </span>
                  )}
                </span>
              )}
            </div>
          </FolderTooltip>
        </td>
        <ScanReviewMatchCell
          item={item}
          override={override}
          onOverride={onOverride}
          isSkipped={isSkipped}
          masterDbEntries={masterDbEntries}
          activeGame={activeGame}
        />
        <td className="w-24">
          {displayType ? (
            <span className="badge badge-sm bg-base-300/50 border-base-300/60 text-base-content/70">
              {displayType}
            </span>
          ) : (
            <span className="text-xs text-base-content/30 italic">
              {t('objects:item.status_unknown')}
            </span>
          )}
        </td>
        <td className="w-28 text-center">
          <ConfidenceBadge item={item} override={override} />
        </td>
        <td className="w-12 text-center">
          <button
            className={`btn btn-xs btn-square ${
              isSkipped
                ? 'btn-warning bg-warning/20'
                : 'btn-ghost text-base-content/30 hover:text-warning'
            }`}
            onClick={onToggleSkip}
            title={isSkipped ? t('context.include_mod') : t('context.skip_mod')}
          >
            <SkipForward size={14} />
          </button>
        </td>
      </tr>
    </ContextMenu>
  );
}

function RenameControls({
  value,
  inputRef,
  onChange,
  onCommit,
  onCancel,
}: {
  value: string;
  inputRef: RefObject<HTMLInputElement | null>;
  onChange: (value: string) => void;
  onCommit: () => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation(['objects']);

  return (
    <div className="flex items-center gap-1">
      <input
        ref={inputRef}
        className="input input-xs input-bordered w-full text-sm font-medium"
        value={value}
        onChange={(event) => onChange(stripDisabledPrefix(event.target.value))}
        onKeyDown={(event) => {
          if (event.key === 'Enter') onCommit();
          if (event.key === 'Escape') onCancel();
        }}
        onBlur={onCommit}
        autoFocus
      />
      <button
        type="button"
        className="btn btn-xs btn-ghost btn-square text-success hover:bg-success/20"
        onMouseDown={(event) => {
          event.preventDefault();
          onCommit();
        }}
        title={t('context.confirm_rename')}
      >
        <Check size={14} />
      </button>
      <button
        type="button"
        className="btn btn-xs btn-ghost btn-square text-error hover:bg-error/20"
        onMouseDown={(event) => {
          event.preventDefault();
          onCancel();
        }}
        title={t('context.cancel_rename')}
      >
        <X size={14} />
      </button>
    </div>
  );
}

function ConfidenceBadge({
  item,
  override,
}: {
  item: ScanPreviewItem;
  override: MasterDbEntry | null;
}) {
  if (override || item.confidenceScore <= 0) {
    return <span className="text-xs text-base-content/30">-</span>;
  }

  const confidence = item.confidence as 'High' | 'Medium' | 'Low' | 'None' | 'Manual';
  return (
    <div
      className={`badge badge-sm badge-outline gap-1 ${getConfidenceColor(confidence)}`}
      title={`${item.confidence} Confidence`}
    >
      {getConfidenceIcon(confidence)}
      <span className="font-medium">{item.confidenceScore}%</span>
    </div>
  );
}

function revealMatchedObject(
  activeGame: GameConfig | null,
  override: MasterDbEntry | null,
  item: ScanPreviewItem,
  masterDbEntries: MasterDbEntry[],
) {
  const objectName = override?.name ?? item.matchedAliasName;
  if (!activeGame || !objectName) {
    return;
  }

  const entry = masterDbEntries.find((candidate) => candidate.name === objectName);
  const objectId =
    entry?.metadata && typeof entry.metadata.id === 'string' ? entry.metadata.id : objectName;

  commands
    .revealObjectInExplorer({
      gameId: activeGame.id,
      objectId,
      objectName,
    })
    .catch(console.error);
}

function stripDisabledPrefix(value: string): string {
  return value.replace(/^(disabled|disable|dis)[_\-\s]+/i, '');
}
