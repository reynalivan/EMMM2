import { handled, type DemoCommandResult } from './commandResult';
export type { DemoCommandResult } from './commandResult';
import { resolveDemoCollectionCommand } from './collectionCommands';
import {
  demoModInbox,
  demoModInboxBatch,
  readCreateModInboxBatchInput,
  readBatchId,
  createDemoModInboxBatch,
  completeDemoModInboxBatch,
} from './modInboxCommands';
import { demoDupScanReport } from './dupScanData';
import { readDemoSettings, saveDemoSettings } from './settingsCommands';
import type { BrowserDownloadDto, ModHealthReport } from '@/shared/api/tauri/bindings.gen';

import { demoDashboardGateway } from './dashboard';
import { getDemoSettings, setDemoSettings } from './game';
import {
  buildDemoWorkspacePreview,
  buildDemoWorkspaceStructure,
  demoGameSchema,
} from './workspace';

const demoModHealth: ModHealthReport = {
  support_level: 'supported',
  issues: [],
  manifest: {
    referenced: [],
    inactive_only: [],
    orphan: [],
    external_reference: [],
    counts: {
      referenced: 0,
      inactive_only: 0,
      orphan: 0,
      external_reference: 0,
    },
  },
  file_manifest: [],
  controls: [],
};

let demoDownloads: BrowserDownloadDto[] = [
  {
    id: 'demo-download-active',
    game_id: 'demo-game',
    session_id: null,
    filename: 'Nekomata Streetwear.zip',
    file_path: null,
    source_url: 'https://example.invalid/nekomata-streetwear.zip',
    status: 'in_progress',
    bytes_total: 48_234_496,
    bytes_received: 31_678_464,
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 0,
    started_at: '2026-09-13T06:00:00Z',
    finished_at: null,
  },
  {
    id: 'demo-download-finished',
    game_id: 'demo-game',
    session_id: null,
    filename: 'Lumina Square Recolor.zip',
    file_path: 'C:\\Demo\\Downloads\\Lumina Square Recolor.zip',
    source_url: 'https://example.invalid/lumina-square-recolor.zip',
    status: 'finished',
    bytes_total: 12_582_912,
    bytes_received: 12_582_912,
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 1,
    started_at: '2026-09-13T05:30:00Z',
    finished_at: '2026-09-13T05:31:00Z',
  },
];

let demoBrowserHomepage = 'https://gamebanana.com';
let demoBrowserRetentionDays = 30;

/**
 * Development-only command data. This is an adapter behind the existing
 * bindings contract, never a replacement component or a browser Tauri shim.
 * Each case must be safe to repeat and must keep all state in memory.
 */
export function resolveDemoCommand(name: string, _args: unknown[]): DemoCommandResult {
  const collectionResult = resolveDemoCollectionCommand(name, _args);
  if (collectionResult) return collectionResult;
  switch (name) {
    case 'getSettings':
      return handled(getDemoSettings());
    case 'saveSettings': {
      const settings = readDemoSettings(_args[0]);
      return settings
        ? handled(saveDemoSettings(settings))
        : handled(Promise.reject(new Error('Demo settings payload is invalid.')));
    }
    case 'setAiApiKey':
      return handled(
        setDemoSettings({
          ...getDemoSettings(),
          ai: { ...getDemoSettings().ai, has_api_key: true },
        }),
      );
    case 'deleteAiApiKey':
      return handled(
        setDemoSettings({
          ...getDemoSettings(),
          ai: { ...getDemoSettings().ai, has_api_key: false },
        }),
      );
    case 'testAiConnection':
      return handled(undefined);
    case 'getDashboardStats':
      return handled(demoDashboardGateway.getDashboardStats());
    case 'getActiveKeybindings':
      return handled(demoDashboardGateway.getActiveKeybindings(String(_args[0] ?? '')));
    case 'getGameSchema':
      return handled(demoGameSchema);
    case 'getWorkspaceStructure':
      return handled(buildDemoWorkspaceStructure(_args[0]));
    case 'getWorkspacePreview':
      return handled(buildDemoWorkspacePreview(_args[0]));
    case 'getObjectsCmd':
      return handled({ objects: buildDemoWorkspaceStructure({}).objects, lost_objects: [] });
    case 'getGames':
      return handled(
        getDemoSettings().games.map((game) => ({
          ...game,
          ready_to_move_path: null,
        })),
      );
    case 'listModIniFiles':
    case 'listModPreviewImages':
      return handled([]);
    case 'analyzeModHealth':
      return handled(demoModHealth);
    case 'getModInbox':
      return handled(demoModInbox);
    case 'createModInboxBatch': {
      const input = readCreateModInboxBatchInput(_args[0]);
      return input
        ? handled(createDemoModInboxBatch(input))
        : handled(Promise.reject(new Error('Demo Mod Inbox payload is invalid.')));
    }
    case 'getImportBatch': {
      const batchId = readBatchId(_args[0]);
      return demoModInboxBatch && batchId === demoModInboxBatch.id
        ? handled(demoModInboxBatch)
        : handled(Promise.reject(new Error('Demo import batch was not found.')));
    }
    case 'markImportBatchReviewStarted':
      return handled(undefined);
    case 'retryObjectIdentitySuggestions':
      return handled(undefined);
    case 'previewImportLibraryReadiness': {
      const batchId = typeof _args[0] === 'string' ? _args[0] : '';
      return handled({ batchId, items: [], highCount: 0, mediumCount: 0, reviewStarted: true });
    }
    case 'listImportBatches':
      return handled(demoModInboxBatch ? [demoModInboxBatch] : []);
    case 'commitImportBatch': {
      const batchId = readBatchId(_args[0]);
      return batchId
        ? handled(completeDemoModInboxBatch(batchId))
        : handled(Promise.reject(new Error('Demo import batch payload is invalid.')));
    }
    case 'getIgnoredPairs':
      return handled([]);
    case 'dupScanGetReport':
      return handled(demoDupScanReport);
    case 'browserGetAdblockEnabled':
      return handled(true);
    case 'browserListBookmarks':
    case 'browserListHistory':
    case 'browserGetSessionTabs':
      return handled([]);
    case 'browserGetPrivacySummary':
      return handled({
        bookmarks: 0,
        history_entries: 0,
        saved_permissions: 0,
      });
    case 'browserListDownloads':
      return handled(demoDownloads);
    case 'browserGetHomepage':
      return handled(demoBrowserHomepage);
    case 'browserGetRetentionDays':
      return handled(demoBrowserRetentionDays);
    case 'browserSaveSessionTabs':
    case 'browserSetAdblockEnabled':
    case 'browserClearHistory':
    case 'launchGame':
      return handled(undefined);
    case 'setActiveGame': {
      const gameId = typeof _args[0] === 'string' ? _args[0] : null;
      const settings = getDemoSettings();
      if (gameId !== null && !settings.games.some((game) => game.id === gameId)) {
        return handled(Promise.reject(new Error(`Unknown demo game: ${gameId}`)));
      }
      return handled(setDemoSettings({ ...settings, active_game_id: gameId }));
    }
    case 'browserSetHomepage':
      demoBrowserHomepage = String(_args[0] ?? demoBrowserHomepage);
      return handled(undefined);
    case 'browserSetRetentionDays':
      demoBrowserRetentionDays = Number(_args[0]) || demoBrowserRetentionDays;
      return handled(undefined);
    case 'browserClearOldDownloads': {
      const removable = demoDownloads.filter((download) => download.status === 'finished').length;
      demoDownloads = demoDownloads.filter((download) => download.status !== 'finished');
      return handled(removable);
    }
    case 'browserCancelDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.map((download) =>
        download.id === id
          ? { ...download, status: 'canceled', finished_at: '2026-09-13T06:15:00Z' }
          : download,
      );
      return handled(undefined);
    }
    case 'browserRetryDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.map((download) =>
        download.id === id
          ? { ...download, status: 'in_progress', error_msg: null, finished_at: null }
          : download,
      );
      return handled(undefined);
    }
    case 'browserDeleteDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.filter((download) => download.id !== id);
      return handled(undefined);
    }
    default:
      // Fail closed: an unmodelled command must never fall through to Tauri
      // while browser QA is running against fixture data.
      return handled(Promise.reject(new Error(`Demo data has no handler for ${name}.`)));
  }
}
