/**
 * usePin — v2 PIN security hooks.
 *
 * Replaces: PIN logic scattered across useSettings, SafeModeConfig, useSafeModeToggle.
 * PIN is now isolated in its own DB table + service + hooks.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from '../../../stores/useToastStore';
import { pinKeys } from '../queryKeys';
import type { PinStatus } from '../../../types/collection';
import { commands } from '../../../lib/bindings';
import { publishQueryScopes } from '../../runtime-sync/queryRefresh';

/** Check if a PIN is set (lightweight boolean query). */
export function useV2HasPin() {
  return useQuery<boolean>({
    queryKey: pinKeys.hasPin(),
    queryFn: () => commands.hasPin(),
    staleTime: 60_000,
  });
}

/** Get full PIN status (has_pin, is_locked, failed_attempts). */
export function usePinStatus() {
  return useQuery<PinStatus>({
    queryKey: pinKeys.status(),
    queryFn: () => commands.getPinStatus(),
    staleTime: 10_000,
  });
}

/** Verify a PIN code. */
export function useV2VerifyPin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pin: string) => commands.verifyPin({ pin }),

    onSuccess: async (isValid) => {
      await publishQueryScopes(queryClient, ['pins']);

      if (!isValid) {
        toast.error('Incorrect PIN');
      }
    },

    onError: async (err) => {
      await publishQueryScopes(queryClient, ['pins']);
      toast.error(String(err));
    },
  });
}

/** Set or update the PIN. */
export function useV2SetPin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ pin, recoveryCode }: { pin: string; recoveryCode?: string }) =>
      commands.setPin({ pin, recoveryCode: recoveryCode ?? undefined }),

    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['pins']);
      toast.success('PIN updated');
    },

    onError: (err) => {
      toast.error(String(err));
    },
  });
}

/** Clear the PIN. */
export function useV2ClearPin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => commands.clearPin({}),

    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['pins']);
      toast.success('PIN removed');
    },

    onError: (err) => {
      toast.error(String(err));
    },
  });
}
