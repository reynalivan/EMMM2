import { create } from 'zustand';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { acceptHotkeyNotice } from '@/shared/lib/hotkeyNotice';

interface HotkeyNoticeState {
  notice: HotkeyNotice | null;
  lastSequence: number;
  receive: (notice: HotkeyNotice, gameId: string, modsPath: string) => void;
  dismiss: (id?: string) => void;
}

export const useHotkeyNoticeStore = create<HotkeyNoticeState>((set) => ({
  notice: null,
  lastSequence: 0,
  receive: (candidate, gameId, modsPath) =>
    set((state) => {
      if (
        candidate.sequence < state.lastSequence ||
        (candidate.sequence === state.lastSequence && state.notice?.id !== candidate.id) ||
        !acceptHotkeyNotice(state.notice, candidate, {
          gameId,
          modsPath,
          now: Date.now(),
        })
      )
        return state;
      return { notice: candidate, lastSequence: candidate.sequence };
    }),
  dismiss: (id) => set((state) => (id && state.notice?.id !== id ? state : { notice: null })),
}));
