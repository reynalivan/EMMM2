/**
 * Types & IPC bindings for EMMM.
 * Directly sourced from `src/types/*` to avoid giant monolithic files.
 * Completely Type-Safe.
 */

import { invoke, Channel } from '@tauri-apps/api/core';

// Internal types that are too small for separate files
export type TaskStatus = 'PENDING' | 'COMPLETED' | 'FAILED';
export type PipelineTask = {
  id: string;
  game_id: string;
  task_type: string;
  status: TaskStatus;
  target_id: string | null;
  created_at: string;
  updated_at: string;
};

export type IngestResult = {
  moved: string[];
  failed: string[];
  skipped: string[];
};

export interface IniVariable {
  name: string;
  value: string;
  line_idx: number;
}

export interface KeyBinding {
  section_name: string;
  key: string | null;
  back: string | null;
  key_line_idx: number | null;
  back_line_idx: number | null;
}

export interface IniDocument {
  file_path: string;
  raw_lines: string[];
  variables: IniVariable[];
  key_bindings: KeyBinding[];
  had_bom: boolean;
  newline_style: 'Lf' | 'CrLf';
  mode: 'Structured' | 'RawFallback';
}

export interface IniLineUpdate {
  line_idx: number;
  content: string;
}

/** Shape returned by Rust `match_object_with_db` command. */
export interface MatchedDbEntry {
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
  /** Pipeline level: "L1Name" | "L2Token" | "L5Fuzzy" */
  match_level: string;
  /** Confidence: "High" | "Medium" | "Low" */
  match_confidence: string;
  /** Human-readable match detail */
  match_detail: string;
}

export type WatcherState = { status: string; path: string | null; game_id: string | null };

// Domain Types
import type { FolderEntry } from '../types/scanner';
import type { GameConfig } from '../types/game';
import type {
  DbEntry,
  GameSchema,
  ObjectSummary,
  CategoryCount,
  FolderGridResponse,
  ModInfo,
  RenameResult,
  FolderContentInfo,
  ObjectFilter,
  CreateObjectInput,
  UpdateObjectInput,
  ModInfoUpdate,
} from '../types/object';
import type {
  CollectionSummary,
  CorridorSnapshot,
  CorridorSwitchPreview,
  CollectionPreview,
  ApplyPreview,
  ApplyResult,
  PinStatus,
  SwitchResult,
} from '../types/collection';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ScanResultItem,
  ConflictInfo,
  ScanPreviewItem,
  ConflictDetails,
  SyncResult,
  TrashMetadata,
  DuplicateInfo,
  BulkResult,
  DupScanReport,
  ResolutionSummary,
  MetadataSyncResult,
  MatchCheckResult,
  ResolutionRequest,
  ScanEvent,
  ExtractionEvent,
  DupScanEvent,
  ConfirmedScanItem,
  WhitelistEntry,
  IgnoredConflict,
} from '../types/scanner';
import type { DashboardPayload } from '../types/dashboard';
import type { AppSettings, ActiveKeyBinding } from '../types/settings';

export interface ThemeMetadata {
  id: string;
  label: string;
}

export interface ThemeConfig {
  colors: Record<string, string>;
  glass: Record<string, string>;
}

export interface CustomTheme {
  id: string;
  label: string;
  config: ThemeConfig;
}
import type { BrowserDownloadItem, ImportJobItem } from '../features/browser/types';

export interface RandomModProposal {
  object_id: string;
  object_name: string;
  mod_id: string;
  name: string;
  display_name: string;
  thumbnail_path?: string | null;
  folder_path: string;
}

// ----- COMMANDS REGISTRY -----
export const commands = {
  // App & System
  appStartupCheck: () => invoke<PipelineTask[]>('app_startup_check'),
  checkConfigStatus: () => invoke<boolean>('check_config_status'),
  checkMetadataUpdate: () => invoke<MetadataSyncResult>('check_metadata_update'),
  getLogs: (params: { count?: number; limit?: number; offset?: number }) =>
    invoke<string[]>('get_logs', params),
  getLogLines: (params: { count?: number; limit?: number; offset?: number }) =>
    invoke<string[]>('get_logs', params),
  openLogFolder: () => invoke<void>('open_log_folder'),
  resetDatabase: () => invoke<void>('reset_database'),
  fetchMissingAsset: (params: { assetName: string }) => invoke<void>('fetch_missing_asset', params),
  checkPathExists: (params: { path: string }) => invoke<boolean>('check_path_exists_cmd', params),
  checkPathExistsCmd: (params: { path: string }) =>
    invoke<boolean>('check_path_exists_cmd', params),
  getSettings: () => invoke<AppSettings>('get_settings'),
  saveSettings: (params: { settings: AppSettings }) => invoke<void>('save_settings', params),
  runMaintenance: (params: { gameId?: string; id?: string }) =>
    invoke<string>('run_maintenance', params),
  clearOldThumbnails: () => invoke<string>('clear_old_thumbnails'),
  updateHotkeyConfig: (params: { config?: Record<string, string> }) =>
    invoke<void>('update_hotkey_config', params),
  getDashboardStats: (params: { gameId?: string; id?: string; safeMode?: boolean }) =>
    invoke<DashboardPayload>('get_dashboard_stats', params),
  getActiveKeybindings: (params: { gameId?: string; id?: string }) =>
    invoke<ActiveKeyBinding[]>('get_active_keybindings', params),

  // Game Management
  getGames: () => invoke<GameConfig[]>('get_games'),
  autoDetectGames: (params?: { rootPath?: string }) =>
    invoke<GameConfig[]>('auto_detect_games', params),
  addGameManual: (params: { name?: string; path: string; gameType: number | string }) =>
    invoke<GameConfig>('add_game_manual', params),
  saveOnboardingGames: (params: { games: GameConfig[] }) =>
    invoke<void>('save_onboarding_games', params),
  removeGame: (params: { id?: string; gameId?: string }) => invoke<void>('remove_game', params),
  launchGame: (params: { id?: string; gameId?: string }) => invoke<void>('launch_game', params),
  setActiveGame: (params: { gameId?: string | null; id?: string | null }) =>
    invoke<void>('set_active_game', params),
  setAutoCloseLauncher: (params: { enabled: boolean }) =>
    invoke<void>('set_auto_close_launcher', params),

  // Master DB & Objects
  getGameSchema: (params: { gameType: number }) => invoke<GameSchema>('get_game_schema', params),
  getMasterDb: (params: { gameType: number }) => invoke<string>('get_master_db', params),
  searchMasterDb: (params: { gameType: number; query: string; objectType?: string | null }) =>
    invoke<{ score: number; item: DbEntry }[]>('search_master_db', params),
  getObject: (params: { id?: string; gameId?: string }) =>
    invoke<ObjectSummary>('get_object', params),
  // Note: getGame doesn't exist explicitly in lib.rs registry as _cmd, likely using get_games or a plugin.
  // Reverting to get_game if it was intended to exist via specta.
  getGame: (params: { id?: string; gameId?: string }) => invoke<GameConfig>('get_game', params),
  getObjects: (params?: { filter?: ObjectFilter } | { gameId?: string; safeMode?: boolean }) =>
    invoke<{ objects: ObjectSummary[]; lost_objects: string[] }>('get_objects_cmd', params),
  getCategoryCounts: (params?: { gameId?: string; safeMode?: boolean }) =>
    invoke<CategoryCount[]>('get_category_counts_cmd', params),
  syncObjects: (params?: { gameId?: string; id?: string }) =>
    invoke<SyncResult>('sync_objects_cmd', params),
  gcLostObjects: (params?: { gameId?: string; id?: string }) =>
    invoke<string[]>('gc_lost_objects_cmd', params),
  createObject: (params: { input: CreateObjectInput }) =>
    invoke<string>('create_object_cmd', params),
  updateObject: (params: { id?: string; updates: UpdateObjectInput }) =>
    invoke<void>('update_object_cmd', params),
  deleteObject: (params: { id?: string }) => invoke<void>('delete_object_cmd', params),
  pinObject: (params: { id?: string; isPinned?: boolean; pin?: boolean }) =>
    invoke<void>('pin_object', params),
  matchObjectWithDb: (params: { gameType: number; objectName: string }) =>
    invoke<MatchedDbEntry | null>('match_object_with_db', params),

  // Mod Management (Core)
  listModFolders: (params: {
    gameId?: string;
    id?: string;
    modsPath: string;
    subPath?: string;
    objectId?: string | null;
  }) => invoke<FolderGridResponse>('list_mod_folders', params),
  toggleMod: (params: { path?: string; folderPath?: string; enable: boolean; gameId?: string }) =>
    invoke<string>('toggle_mod', params),
  renameModFolder: (params: {
    folderPath?: string;
    path?: string;
    newName: string;
    gameId?: string;
  }) => invoke<RenameResult>('rename_mod_folder', params),
  deleteMod: (params: { path?: string; folderPath?: string; gameId?: string }) =>
    invoke<void>('delete_mod', params),
  preDeleteCheck: (params: { path?: string }) =>
    invoke<FolderContentInfo>('pre_delete_check', params),
  openInExplorer: (params: { gameId: string; path: string }) =>
    invoke<void>('open_in_explorer', params),
  revealObjectInExplorer: (params: { gameId: string; objectId: string; objectName: string }) =>
    invoke<string>('reveal_object_in_explorer', params),

  // Mod Metadata & Tags
  setModCategory: (params: {
    path?: string;
    folderPath?: string;
    category: string;
    gameId?: string;
  }) => invoke<void>('set_mod_category', params),
  toggleModSafe: (params: {
    gameId?: string;
    id?: string;
    path?: string;
    folderPath?: string;
    safe?: boolean;
  }) => invoke<void>('toggle_mod_safe', params),
  toggleFavorite: (params: {
    path?: string;
    folderPath?: string;
    favorite?: boolean;
    gameId?: string;
  }) => invoke<void>('toggle_favorite', params),
  getActiveModConflicts: (params: { gameId?: string }) =>
    invoke<ConflictInfo[]>('get_active_mod_conflicts', params),
  suggestRandomMods: (params: { gameId: string; isSafe: boolean }) =>
    invoke<RandomModProposal[]>('suggest_random_mods', params),
  moveModToObject: (params: {
    gameId?: string;
    folderPath?: string;
    objectId?: string;
    targetObjectId?: string;
    status?: string;
  }) => invoke<void>('move_mod_to_object', params),

  // Previews & Ini
  readModInfo: (params: { folderPath?: string; path?: string }) =>
    invoke<ModInfo>('read_mod_info', params),
  updateModInfo: (params: { folderPath?: string; path?: string; update: ModInfoUpdate }) =>
    invoke<void>('update_mod_info', params),
  listModIniFiles: (params: { folderPath?: string; path?: string }) =>
    invoke<string[]>('list_mod_ini_files', params),
  readModIni: (params: { folderPath?: string; path?: string; fileName?: string }) =>
    invoke<IniDocument>('read_mod_ini', params),
  writeModIni: (params: {
    folderPath?: string;
    path?: string;
    fileName?: string;
    lineUpdates?: IniLineUpdate[];
    content?: string;
  }) => invoke<void>('write_mod_ini', params),
  listModPreviewImages: (params: { folderPath?: string; path?: string }) =>
    invoke<string[]>('list_mod_preview_images', params),
  saveModPreviewImage: (params: {
    folderPath?: string;
    modPath?: string;
    objectName?: string;
    imageData?: number[];
    imagePath?: string;
  }) => invoke<void>('save_mod_preview_image', params),
  removeModPreviewImage: (params: { folderPath?: string; imagePath?: string }) =>
    invoke<void>('remove_mod_preview_image', params),
  clearModPreviewImages: (params: { folderPath?: string }) =>
    invoke<void>('clear_mod_preview_images', params),

  // Thumbnails
  getThumbnail: (params: { folderPath?: string; path?: string }) =>
    invoke<string | null>('get_thumbnail', params),
  getModThumbnail: (params: { gameId: string; folderPath: string }) =>
    invoke<string | null>('get_mod_thumbnail', params),
  updateModThumbnail: (params: { folderPath?: string; path?: string; sourcePath: string }) =>
    invoke<string>('update_mod_thumbnail', params),
  deleteModThumbnail: (params: { folderPath?: string; path?: string }) =>
    invoke<void>('delete_mod_thumbnail', params),
  pasteThumbnail: (params: { folderPath?: string; path?: string; imageData: number[] }) =>
    invoke<string>('paste_thumbnail', params),

  // Bulk Operations
  bulkToggleMods: (params: { gameId: string; paths: string[]; enable: boolean }) =>
    invoke<BulkResult>('bulk_toggle_mods', params),
  bulkDeleteMods: (params: { paths: string[]; gameId?: string }) =>
    invoke<BulkResult>('bulk_delete_mods', params),
  bulkUpdateInfo: (params: { gameId: string; paths: string[]; update: ModInfoUpdate }) =>
    invoke<BulkResult>('bulk_update_info', params),
  bulkToggleFavorite: (params: { gameId: string; folderPaths: string[]; favorite: boolean }) =>
    invoke<BulkResult>('bulk_toggle_favorite', params),
  bulkPinMods: (params: { gameId: string; folderPaths: string[]; pin: boolean }) =>
    invoke<BulkResult>('bulk_pin_mods', params),

  // Scanner (Archives)
  detectArchives: (params: { modsPath: string }) =>
    invoke<ArchiveInfo[]>('detect_archives_cmd', params),
  extractArchive: (params: {
    archivePath: string;
    modsDir: string;
    password?: string | null;
    overwrite: boolean;
    customName?: string | null;
    disableAfter: boolean;
    unpackNested: boolean;
    onProgress: Channel<ExtractionEvent>;
  }) => invoke<ExtractionResult>('extract_archive_cmd', params),
  analyzeArchive: (params: { archivePath: string }) =>
    invoke<ArchiveAnalysis>('analyze_archive_cmd', params),
  analyzeArchiveCmd: (params: { archivePath: string }) =>
    invoke<ArchiveAnalysis>('analyze_archive_cmd', params),
  matchCheckFolder: (params: { folderPath: string; targetObjectName: string; dbJson: string }) =>
    invoke<MatchCheckResult>('match_check_folder_cmd', params),
  listFolderEntriesCmd: (params: { folderPath: string; gameId: string }) =>
    invoke<FolderEntry[]>('list_folder_entries_cmd', params),
  abortExtraction: () => invoke<void>('abort_extraction_cmd'),
  abortExtractionCmd: () => invoke<void>('abort_extraction_cmd'),

  // Scanner (General)
  startScan: (params: { modsPath: string; dbJson: string; onProgress: Channel<ScanEvent> }) =>
    invoke<ScanResultItem[]>('start_scan', params),
  getScanResult: (params: { modsPath: string; dbJson: string }) =>
    invoke<ScanResultItem[]>('get_scan_result', params),
  cancelScan: () => invoke<void>('cancel_scan_cmd'),
  cancelScanCmd: () => invoke<void>('cancel_scan_cmd'),
  syncDatabase: (params: {
    gameId?: string;
    gameName?: string;
    gameType?: string;
    modsPath?: string;
    dbJson?: string;
    preserveExistingMappings?: boolean;
    onProgress?: Channel<ScanEvent>;
  }) => invoke<SyncResult>('sync_database_cmd', params),
  scanPreview: (params: {
    gameId: string;
    modsPath: string;
    dbJson: string;
    onProgress: Channel<ScanEvent>;
    specificPaths?: string[];
  }) => invoke<ScanPreviewItem[]>('scan_preview_cmd', params),
  commitScan: (params: {
    gameId?: string;
    gameName?: string;
    gameType?: string;
    modsPath?: string;
    items?: ConfirmedScanItem[];
  }) => invoke<SyncResult>('commit_scan_cmd', params),
  scoreCandidatesBatch: (params: {
    folderPath: string;
    candidateNames: string[];
    dbJson: string;
  }) => invoke<Record<string, number>>('score_candidates_batch_cmd', params),
  importModsFromPaths: (params: {
    paths: string[];
    targetDir: string;
    strategy: string;
    dbJson?: string;
  }) => invoke<BulkResult>('import_mods_from_paths', params),
  ingestDroppedFolders: (params: {
    paths?: string[];
    modsPath?: string;
    gameId?: string;
    gameName?: string;
    gameType?: string;
    dbJson?: string;
  }) => invoke<IngestResult>('ingest_dropped_folders', params),
  autoOrganizeMods: (params: { paths: string[]; targetRoot: string; dbJson: string }) =>
    invoke<BulkResult>('auto_organize_mods', params),

  // Conflicts & Duplicates
  checkShaderConflicts: (params: { folderPath: string }) =>
    invoke<ConflictInfo[]>('check_shader_conflicts', params),
  detectConflicts: (params: { iniPaths: string[] }) =>
    invoke<ConflictInfo[]>('detect_conflicts_cmd', params),
  detectConflictsInFolder: (params: { modsPath: string }) =>
    invoke<ConflictInfo[]>('detect_conflicts_in_folder_cmd', params),
  checkDuplicateEnabled: (params: { folderPath: string; gameId: string }) =>
    invoke<DuplicateInfo[]>('check_duplicate_enabled', params),
  enableOnlyThis: (params: { targetPath: string; gameId: string }) =>
    invoke<BulkResult>('enable_only_this', params),
  getConflictDetails: (params: { path?: string; enabledPath?: string; disabledPath?: string }) =>
    invoke<ConflictDetails>('get_conflict_details', params),
  resolveConflict: (params: {
    path?: string;
    resolution?: string;
    keepPath?: string;
    duplicatePath?: string;
    strategy?: string;
  }) => invoke<void>('resolve_conflict', params),
  ignoreObjectConflict: (params: { gameId: string; objectId: string; modIds: string[] }) =>
    invoke<void>('ignore_object_conflict', params),
  revokeObjectConflict: (params: { gameId: string; objectId: string }) =>
    invoke<void>('revoke_object_conflict', params),
  listIgnoredObjectConflicts: (params: { gameId: string }) =>
    invoke<IgnoredConflict[]>('list_ignored_object_conflicts', params),

  // Duplicates Scanner
  dupScanStart: (params: { gameId: string; modsRoot: string; onEvent: Channel<DupScanEvent> }) =>
    invoke<void>('dup_scan_start', params),
  dupScanCancel: () => invoke<void>('dup_scan_cancel'),
  dupScanCancelCmd: () => invoke<void>('dup_scan_cancel'),
  dupScanGetReport: (params: { pin?: string }) =>
    invoke<DupScanReport | null>('dup_scan_get_report', params),
  dupScanGetReportCmd: (params: { pin?: string }) =>
    invoke<DupScanReport | null>('dup_scan_get_report', params),
  dupResolveBatch: (params: { gameId: string; requests: ResolutionRequest[] }) =>
    invoke<ResolutionSummary>('dup_resolve_batch', params),
  getIgnoredPairs: (params: { gameId: string }) =>
    invoke<WhitelistEntry[]>('get_ignored_pairs', params),
  removeIgnoredPair: (params: { entryId: string }) => invoke<number>('remove_ignored_pair', params),

  // Watcher
  startWatcher: (params?: { gameId?: string; path?: string }) =>
    invoke<void>('start_watcher_cmd', params),
  startWatcherCmd: (params?: { gameId?: string; path?: string }) =>
    invoke<void>('start_watcher_cmd', params),
  stopWatcher: () => invoke<void>('stop_watcher_cmd'),
  stopWatcherCmd: () => invoke<void>('stop_watcher_cmd'),
  setWatcherSuppression: (params: { suppressed: boolean; gameId?: string }) =>
    invoke<void>('set_watcher_suppression_cmd', params),
  setWatcherSuppressionCmd: (params: { suppressed: boolean; gameId?: string }) =>
    invoke<void>('set_watcher_suppression_cmd', params),
  getWatcherState: () => invoke<WatcherState>('get_file_watcher_state'),

  // Collections
  getCorridorState: (params: { gameId: string; isSafe: boolean }) =>
    invoke<CorridorSnapshot>('get_corridor_state', params),
  listCollections: (params: { gameId: string }) =>
    invoke<CollectionSummary[]>('list_collections', params),
  getCollectionPreview: (params: { id?: string; collectionId?: string; gameId?: string }) =>
    invoke<CollectionPreview>('get_collection_preview', params),
  previewApplyCollection: (params: {
    id?: string;
    collectionId?: string;
    gameId?: string;
    isSafe?: boolean;
  }) => invoke<ApplyPreview>('preview_apply_collection', params),
  createCollection: (params: { gameId?: string; name: string; description?: string }) =>
    invoke<CollectionSummary>('create_collection', params),
  updateCollection: (params: { id?: string; gameId?: string; name?: string }) =>
    invoke<CollectionSummary>('update_collection', params),
  deleteCollection: (params: { id?: string; collectionId?: string; gameId?: string }) =>
    invoke<void>('delete_collection', params),
  applyCollection: (params: {
    id?: string;
    collectionId?: string;
    gameId?: string;
    ignoreMissing?: boolean;
  }) => invoke<ApplyResult>('apply_collection', params),
  undoCollection: (params: { gameId?: string }) => invoke<ApplyResult>('undo_collection', params),
  switchCorridor: (params: { gameId?: string; targetSafe?: boolean }) =>
    invoke<SwitchResult>('switch_corridor', params),
  previewCorridorSwitch: (params: { gameId?: string; targetSafe?: boolean }) =>
    invoke<CorridorSwitchPreview>('preview_corridor_switch', params),
  clearPendingTasks: (params?: { gameId?: string }) => invoke<void>('clear_pending_tasks', params),

  // Security (PIN)
  hasPin: () => invoke<boolean>('has_pin'),
  setPin: (params: { pin: string; recoveryCode?: string }) => invoke<void>('set_pin', params),
  verifyPin: (params: { pin?: string }) => invoke<boolean>('verify_pin', params),
  clearPin: (params: { pin?: string }) => invoke<void>('clear_pin', params),
  getPinStatus: () => invoke<PinStatus>('get_pin_status'),
  resetPinWithRecoveryCode: (params: { code?: string; recoveryCode?: string }) =>
    invoke<boolean>('reset_pin_with_recovery_code', params),

  // Browser
  browserOpenTab: (params: { url: string }) => invoke<string>('browser_open_tab', params),
  browserNavigate: (params: { id?: string; url: string; label?: string }) =>
    invoke<void>('browser_navigate', params),
  browserReloadTab: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_reload_tab', params),
  browserCloseTab: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_close_tab', params),
  browserClearData: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_clear_data', params),
  browserGetHomepage: () => invoke<string>('browser_get_homepage'),
  browserSetHomepage: (params: { url: string }) => invoke<void>('browser_set_homepage', params),
  browserImportSelected: (params: { id?: string; ids?: string[]; gameId?: string }) =>
    invoke<void>('browser_import_selected', params),
  browserListDownloads: () => invoke<BrowserDownloadItem[]>('browser_list_downloads'),
  browserDeleteDownload: (params: { id?: string; ids?: string[]; deleteFile?: boolean }) =>
    invoke<void>('browser_delete_download', params),
  browserCancelDownload: (params: { id?: string; ids?: string[]; deleteFile?: boolean }) =>
    invoke<void>('browser_cancel_download', params),
  browserClearImported: () => invoke<void>('browser_clear_imported'),
  browserClearOldDownloads: () => invoke<void>('browser_clear_old_downloads'),
  browserListImportQueue: () => invoke<ImportJobItem[]>('browser_list_import_queue'),
  browserConfirmImport: (params: {
    id?: string;
    jobId?: string;
    gameId?: string;
    category?: string;
    objectId?: string;
  }) => invoke<void>('browser_confirm_import', params),
  browserCancelImport: (params: { id?: string; jobId?: string }) =>
    invoke<void>('browser_cancel_import', params),

  // Trash
  listTrash: () => invoke<TrashMetadata[]>('list_trash'),
  emptyTrash: () => invoke<number>('empty_trash'),
  restoreMod: (params: { trashId: string; gameId?: string }) => invoke<void>('restore_mod', params),

  // Themes
  listCustomThemes: () => invoke<ThemeMetadata[]>('list_custom_themes'),
  loadCustomTheme: (params: { id: string }) => invoke<CustomTheme>('load_custom_theme', params),
  saveCustomTheme: (params: { theme: CustomTheme }) => invoke<void>('save_custom_theme', params),
  deleteCustomTheme: (params: { id: string }) => invoke<void>('delete_custom_theme', params),
};
