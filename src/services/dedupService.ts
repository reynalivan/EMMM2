/**
 * Service layer for Epic 9: Duplicate Scanner.
 * Follows the same pattern as scanService.ts:
 * - Channel-based streaming for progress events
 * - invoke() for request-response commands
 * - No business logic, pure wrapper around Tauri commands
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  DupScanEvent,
  DupScanReport,
  ResolutionRequest,
  ResolutionSummary,
} from '../types/dedup';

/**
 * Service object exposing duplicate scanner commands.
 * All commands map to backend dup_scan_* and dup_resolve_* commands.
 */
export const dedupService = {
  /**
   * Start a duplicate scan with streaming progress events.
   *
   * Spawns a background task that:
   * 1. Enumerates mod folders
   * 2. Hashes each folder (BLAKE3)
   * 3. Detects duplicate groups
   * 4. Emits progress events via channel
   *
   * @param gameId - Game identifier (e.g., 'genshin')
   * @param modsRoot - Absolute path to mods folder
   * @param onEvent - Callback for each event (Started, Progress, Match, Finished, Cancelled)
   * @throws On validation error (path not found, not a directory)
   */
  async startDedupScan(
    gameId: string,
    modsRoot: string,
    onEvent: (event: DupScanEvent) => void,
  ): Promise<void> {
    const channel = new Channel<DupScanEvent>();

    channel.onmessage = (message) => {
      onEvent(message);
    };

    return invoke('dup_scan_start', {
      gameId,
      modsRoot,
      onEvent: channel,
    });
  },

  /**
   * Cancel the currently running duplicate scan.
   * Safe to call if no scan is running.
   *
   * @throws On command execution error
   */
  async cancelDedupScan(): Promise<void> {
    return invoke('dup_scan_cancel');
  },

  /**
   * Fetch the last completed scan report.
   * Returns null if no scan has completed yet.
   *
   * @returns Last report or null if none exists
   */
  async getReport(): Promise<DupScanReport | null> {
    return invoke<DupScanReport | null>('dup_scan_get_report');
  },

  /**
   * Resolve duplicate groups with batch actions.
   * Executes resolution in background, sends progress events via Tauri event emitter.
   *
   * Resolution actions:
   * - KeepA: Delete folder B
   * - KeepB: Delete folder A
   * - Ignore: Whitelist pair (prevent re-detection)
   *
   * @param requests - Array of resolution actions
   * @param gameId - Game identifier for context
   * @returns Summary of resolution outcomes
   */
  async resolveBatch(requests: ResolutionRequest[], gameId: string): Promise<ResolutionSummary> {
    return invoke<ResolutionSummary>('dup_resolve_batch', {
      requests,
      gameId,
    });
  },
};
