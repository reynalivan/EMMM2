/**
 * useSafeModeToggle — Single source of truth for Safe/Unsafe mode switching.
 *
 * Handles: PIN verification gate, loading toasts, error handling.
 * Used by: GlobalActions, ContextControls, CollectionsPage, PrivacyTab.
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/useAppStore';
import { useSettings } from './useSettings';
import { toast, useToastStore } from '../stores/useToastStore';
import { queryClient } from '../lib/queryClient';
import { corridorPreviewKeys, corridorRuntimeKeys } from '../features/collections/queryKeys';
import type { CorridorRuntimeSnapshot } from '../types/collection';

interface UseSafeModeToggleReturn {
  /** Initiate safe mode toggle. Opens the confirmation modal first. */
  toggleSafeMode: () => Promise<void>;
  /** Handle the actual switch after confirmation (checks PIN if needed). */
  handleConfirmSwitch: () => Promise<void>;
  /** Called when PIN succeeds during global toggle flow. */
  handlePinSuccess: () => Promise<void>;
  /** Direct set after PIN verified (e.g. called from PinEntryModal onSuccess). */
  setSafeModeWithToast: (enabled: boolean) => Promise<void>;

  /** Whether the confirm modal should be open. */
  confirmModalOpen: boolean;
  /** The target corridor for the confirm modal. */
  confirmTargetSafeMode: boolean;
  /** Close the confirm modal. */
  closeConfirmModal: () => void;

  /** Whether the PIN modal should be open. */
  pinModalOpen: boolean;
  /** Close the PIN modal. */
  closePinModal: () => void;
  /** Current safe mode state. */
  safeMode: boolean;
}

export function useSafeModeToggle(): UseSafeModeToggleReturn {
  const { activeGameId, safeMode, setSafeMode } = useAppStore();
  const { settings, isLoading } = useSettings();
  const [pinModalOpen, setPinModalOpen] = useState(false);

  const [confirmModalOpen, setConfirmModalOpen] = useState(false);
  const [confirmTargetSafeMode, setConfirmTargetSafeMode] = useState(true);

  const prepareSwitchPreview = useCallback(
    async (targetSafeMode: boolean) => {
      if (!activeGameId) {
        return;
      }

      await queryClient.fetchQuery({
        queryKey: corridorRuntimeKeys.snapshot(activeGameId, safeMode),
        queryFn: () =>
          invoke<CorridorRuntimeSnapshot>('get_corridor_runtime_snapshot', {
            gameId: activeGameId,
            isSafe: safeMode,
          }),
        staleTime: 0,
      });

      queryClient.removeQueries({
        queryKey: [
          ...corridorPreviewKeys.all,
          activeGameId,
          safeMode,
          targetSafeMode,
        ],
        exact: false,
      });
    },
    [activeGameId, safeMode],
  );

  const setSafeModeWithToast = useCallback(
    async (enabled: boolean) => {
      const toastId = toast.info(enabled ? 'Switching to SAFE...' : 'Switching to UNSAFE...', 0);
      try {
        const result = await setSafeMode(enabled);
        useToastStore.getState().removeToast(toastId);

        const label = enabled ? 'SAFE Mode Enabled' : 'UNSAFE Mode Enabled';
        const parts: string[] = [];
        if (result.disabled_count > 0) parts.push(`Disabled ${result.disabled_count}`);
        if (result.restored_count > 0) parts.push(`Restored ${result.restored_count}`);
        const detail = parts.length > 0 ? ` — ${parts.join(', ')} mod(s)` : '';

        if (result.warnings.length > 0) {
          toast.warning(
            `${label}${detail} — ${result.warnings.length} could not be restored`,
            8000,
          );
        } else {
          toast.success(`${label}${detail}`);
        }
      } catch (e) {
        useToastStore.getState().removeToast(toastId);
        toast.error(String(e));
      }
    },
    [setSafeMode],
  );

  const toggleSafeMode = useCallback(async () => {
    // If moving Safe → Unsafe, check PIN first
    if (safeMode) {
      if (isLoading) {
        toast.warning('Still loading security settings. Please try again in a moment.', 3000);
        return;
      }
      if (settings?.safe_mode?.pin_hash) {
        setPinModalOpen(true);
        return;
      }
    }

    // No PIN or moving Unsafe → Safe: go straight to confirmation/preview
    await prepareSwitchPreview(!safeMode);
    setConfirmTargetSafeMode(!safeMode);
    setConfirmModalOpen(true);
  }, [isLoading, prepareSwitchPreview, safeMode, settings]);

  /** Called when PIN is successfully verified. Transitions to the confirmation modal. */
  const handlePinSuccess = useCallback(async () => {
    setPinModalOpen(false);
    await prepareSwitchPreview(false);
    setConfirmTargetSafeMode(false);
    setConfirmModalOpen(true);
  }, [prepareSwitchPreview]);

  const handleConfirmSwitch = useCallback(async () => {
    setConfirmModalOpen(false);
    // PIN was already checked by toggleSafeMode -> handlePinSuccess path
    await setSafeModeWithToast(confirmTargetSafeMode);
  }, [confirmTargetSafeMode, setSafeModeWithToast]);

  const closePinModal = useCallback(() => {
    setPinModalOpen(false);
  }, []);

  const closeConfirmModal = useCallback(() => {
    setConfirmModalOpen(false);
  }, []);

  return {
    toggleSafeMode,
    handleConfirmSwitch,
    handlePinSuccess,
    setSafeModeWithToast,
    confirmModalOpen,
    confirmTargetSafeMode,
    closeConfirmModal,
    pinModalOpen,
    closePinModal,
    safeMode,
  };
}
