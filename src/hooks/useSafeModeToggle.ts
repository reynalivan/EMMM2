/**
 * useSafeModeToggle — Single source of truth for Safe/Unsafe mode switching.
 *
 * Handles: PIN verification gate, loading toasts, error handling.
 * Used by: GlobalActions, ContextControls, CollectionsPage, PrivacyTab.
 */

import { useState, useCallback } from 'react';
import { useAppStore } from '../stores/useAppStore';
import { useSettings } from './useSettings';
import { toast, useToastStore } from '../stores/useToastStore';

interface UseSafeModeToggleReturn {
  /** Initiate safe mode toggle. Opens the confirmation modal first. */
  toggleSafeMode: () => Promise<void>;
  /** Handle the actual switch after confirmation (checks PIN if needed). */
  handleConfirmSwitch: () => Promise<void>;
  /** Called when PIN succeeds during global toggle flow. */
  handlePinSuccess: () => void;
  /** Direct set after PIN verified (e.g. called from PinEntryModal onSuccess). */
  setSafeModeWithToast: (enabled: boolean) => Promise<void>;

  /** Whether the confirm modal should be open. */
  confirmModalOpen: boolean;
  /** The target safe mode state for the confirm modal (true = Safe/Private). */
  confirmTargetEnabled: boolean;
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
  const { safeMode, setSafeMode } = useAppStore();
  const { settings, isLoading } = useSettings();
  const [pinModalOpen, setPinModalOpen] = useState(false);

  const [confirmModalOpen, setConfirmModalOpen] = useState(false);
  const [confirmTargetEnabled, setConfirmTargetEnabled] = useState(true);

  const setSafeModeWithToast = useCallback(
    async (enabled: boolean) => {
      const toastId = toast.info(
        enabled ? 'Enabling Privacy Mode...' : 'Disabling Privacy Mode...',
        0,
      );
      try {
        const result = await setSafeMode(enabled);
        useToastStore.getState().removeToast(toastId);

        const label = enabled ? 'Privacy Mode Enabled' : 'Privacy Mode Disabled';
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
    setConfirmTargetEnabled(!safeMode);
    setConfirmModalOpen(true);
  }, [safeMode, settings, isLoading]);

  /** Called when PIN is successfully verified. Transitions to the confirmation modal. */
  const handlePinSuccess = useCallback(() => {
    setPinModalOpen(false);
    setConfirmTargetEnabled(false); // We were Safe, going Unsafe
    setConfirmModalOpen(true);
  }, []);

  const handleConfirmSwitch = useCallback(async () => {
    setConfirmModalOpen(false);
    // PIN was already checked by toggleSafeMode -> handlePinSuccess path
    await setSafeModeWithToast(confirmTargetEnabled);
  }, [confirmTargetEnabled, setSafeModeWithToast]);

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
    confirmTargetEnabled,
    closeConfirmModal,
    pinModalOpen,
    closePinModal,
    safeMode,
  };
}
