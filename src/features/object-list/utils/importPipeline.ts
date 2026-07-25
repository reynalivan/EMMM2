/**
 * Shared steps of the ObjectList import flow.
 *
 * Dropping files and extracting archives are two entry points into the same
 * pipeline (stage loose files → Deep Match preview → review modal), so the
 * steps live here instead of being copy-pasted into each hook.
 */

import { commands, type IngestResult } from '../../../lib/bindings';
import type { GameConfig } from '../../../types/game';
import { getGameTypeKey } from '../../../types/game';
import type { ScanPreviewItem } from '../../../types/scanner';
import { scanService } from '../../../lib/services/scanService';
import { toast } from '../../../stores/useToastStore';
import { parseMasterDb } from '../../mod-runtime/operations/sharedOperations';
import type { MasterDbEntry } from '../modals/scanReviewHelpers';
import { buildMismatchWarning, type MatchCheckOutcome } from './archiveSummary';

/** What a drop is waiting on while the archive modal is open. */
export type PendingDropContext = {
  type: 'item' | 'auto-organize' | 'new-object';
  pathsToIngest: string[];
  targetFolder?: string;
  targetObjectId?: string;
  baseFolderPaths?: string[];
  baseLooseFiles?: string[];
};

export interface ScanReviewState {
  open: boolean;
  items: ScanPreviewItem[];
  masterDbEntries: MasterDbEntry[];
  isCommitting: boolean;
}

export const CLOSED_SCAN_REVIEW: ScanReviewState = {
  open: false,
  items: [],
  masterDbEntries: [],
  isCommitting: false,
};

/** Staging folder auto-organize extracts and ingests into before review. */
export function tempStagingPath(modPath: string): string {
  return `${modPath}\\.emmm_temp`;
}

export async function ensureDirectoryExists(path: string): Promise<void> {
  if (await commands.checkPathExists({ path })) {
    return;
  }

  await commands.ensureDir({ path });
}

/**
 * Moves loose .ini/image drops into the staging folder so Deep Match sees them
 * as folders. Returns the staged folder paths (empty when nothing was loose).
 */
export async function ingestLooseFiles(
  activeGame: GameConfig,
  looseFiles: string[],
  targetPath: string,
): Promise<string[]> {
  if (looseFiles.length === 0) {
    return [];
  }

  await ensureDirectoryExists(targetPath);
  const ingestResult: IngestResult = await commands.ingestDroppedFolders({
    paths: looseFiles,
    modsPath: targetPath,
    gameId: activeGame.id,
    gameName: activeGame.name,
    gameType: getGameTypeKey(activeGame.game_type),
  });

  return ingestResult.moved;
}

/** Deep Match preview + master DB lookup, shaped for the scan review modal. */
export async function buildScanReviewState(
  activeGame: GameConfig,
  folderPaths: string[],
): Promise<ScanReviewState> {
  const previewItemsRaw = await scanService.runDeepmatchPreview(
    activeGame.id,
    activeGame.game_type,
    activeGame.mod_path,
    undefined,
    folderPaths,
  );
  const items = previewItemsRaw.map((item) => ({
    ...item,
    moveFromTemp: folderPaths.includes(item.folderPath),
  }));
  const dbJson = await scanService.getMasterDb(activeGame.game_type);

  return {
    open: true,
    items,
    masterDbEntries: parseMasterDb(dbJson),
    isCommitting: false,
  };
}

interface FolderMismatchWarningInput {
  activeGame: GameConfig;
  folders: string[];
  objectName: string;
  /** Noun used in the warning copy, e.g. "archive(s)" or "dropped folder(s)". */
  noun: string;
  logLabel: string;
  onFix: (paths: string[]) => void;
}

/**
 * Best-effort check that imported folders actually belong to the target object.
 * Never throws: a failed check must not fail an otherwise successful import.
 */
export async function warnOnFolderMismatch({
  activeGame,
  folders,
  objectName,
  noun,
  logLabel,
  onFix,
}: FolderMismatchWarningInput): Promise<void> {
  try {
    const checks: MatchCheckOutcome[] = [];
    for (const folder of folders) {
      const check = await scanService.matchCheckFolder(folder, objectName, activeGame.game_type);
      checks.push({ folder, ...check });
    }

    const warning = buildMismatchWarning(checks, objectName, noun);
    if (!warning) {
      return;
    }

    toast.withAction(
      'warning',
      warning.message,
      {
        label: 'Fix',
        onClick: () => onFix(warning.mismatchedPaths),
      },
      9999999,
    );
  } catch (err) {
    console.warn(logLabel, err);
  }
}
