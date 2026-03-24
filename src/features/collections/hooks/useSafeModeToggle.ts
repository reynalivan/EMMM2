/**
 * useSafeModeToggle — V2 version of useSafeModeToggle.
 *
 * Composes useCorridorSwitch + useV2HasPin for the PIN gate → confirm modal → switch flow.
 * Replaces: useSafeModeToggle (209 lines → ~90 lines).
 *
 * Key simplifications:
 * - No manual invoke/query cache manipulation — useCorridorSwitch handles it
 * - No normalizeSwitchWarnings — v2 backend returns structured result
 * - No prepareSwitchPreview — v2 backend computes preview server-side
 *
 * IMPORTANT: flowState is stored in Zustand (not local useState) to prevent race
 * conditions when multiple instances of this hook mount simultaneously
 * (ContextControls + GlobalActions + CollectionsPage all use this hook).
 */

import { useCallback, useRef } from 'react';
import { useAppStore } from '../../../stores/useAppStore';
import { useCorridorSwitch } from './useCorridorSwitch';
import { useV2HasPin } from './usePin';

export function useSafeModeToggle() {
  const { activeGameId, safeMode, safeModeFlow, setSafeModeFlow } = useAppStore();
  const switchMutation = useCorridorSwitch();
  const hasPinQuery = useV2HasPin();
  const inFlightRef = useRef(false);

  /** Initiate toggle: checks PIN gate first, then opens confirm modal. */
  const toggleSafeMode = useCallback(async () => {
    if (inFlightRef.current) return;

    const targetSafe = !safeMode;

    // If Safe → Unsafe and PIN exists, require PIN first
    if (safeMode && hasPinQuery.data) {
      setSafeModeFlow({ kind: 'pin' });
      return;
    }

    // No PIN needed — go straight to confirm
    setSafeModeFlow({ kind: 'confirm', targetSafe });
  }, [safeMode, hasPinQuery.data, setSafeModeFlow]);

  /** Called when PIN is verified. Transitions to confirm modal. */
  const handlePinSuccess = useCallback(() => {
    setSafeModeFlow({ kind: 'confirm', targetSafe: !safeMode });
  }, [safeMode, setSafeModeFlow]);

  /** Called when user confirms the switch in the modal. Executes the actual switch. */
  const handleConfirmSwitch = useCallback(() => {
    if (safeModeFlow.kind !== 'confirm') return;
    if (!activeGameId) return;
    if (inFlightRef.current) return;

    const targetSafe = safeModeFlow.targetSafe;
    inFlightRef.current = true;
    setSafeModeFlow({ kind: 'idle' });

    switchMutation.mutate(
      { gameId: activeGameId, targetSafe },
      {
        onSettled: () => {
          inFlightRef.current = false;
        },
      },
    );
  }, [activeGameId, safeModeFlow, switchMutation, setSafeModeFlow]);

  const closePinModal = useCallback(() => setSafeModeFlow({ kind: 'idle' }), [setSafeModeFlow]);
  const closeConfirmModal = useCallback(() => setSafeModeFlow({ kind: 'idle' }), [setSafeModeFlow]);

  return {
    toggleSafeMode,
    handleConfirmSwitch,
    handlePinSuccess,
    confirmModalOpen: safeModeFlow.kind === 'confirm',
    confirmTargetSafeMode: safeModeFlow.kind === 'confirm' ? safeModeFlow.targetSafe : safeMode,
    closeConfirmModal,
    pinModalOpen: safeModeFlow.kind === 'pin',
    closePinModal,
    safeMode,
    isSwitching: switchMutation.isPending,
  };
}
