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
  const { settings } = useSettings();
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
    // Open the confirmation modal first, setting the target state
    setConfirmTargetEnabled(!safeMode);
    setConfirmModalOpen(true);
  }, [safeMode]);

  const handleConfirmSwitch = useCallback(async () => {
    setConfirmModalOpen(false);

    if (safeMode) {
      // Safe → Unsafe: check PIN
      if (settings?.safe_mode?.pin_hash) {
        setPinModalOpen(true);
        return;
      }
      await setSafeModeWithToast(false);
    } else {
      // Unsafe → Safe: always allowed immediately
      await setSafeModeWithToast(true);
    }
  }, [safeMode, settings, setSafeModeWithToast]);

  const closePinModal = useCallback(() => {
    setPinModalOpen(false);
  }, []);

  const closeConfirmModal = useCallback(() => {
    setConfirmModalOpen(false);
  }, []);

  return {
    toggleSafeMode,
    handleConfirmSwitch,
    setSafeModeWithToast,
    confirmModalOpen,
    confirmTargetEnabled,
    closeConfirmModal,
    pinModalOpen,
    closePinModal,
    safeMode,
  };
}
