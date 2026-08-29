import type { AppSliceCreator } from './sliceTypes';

export interface LayoutSlice {
  // Desktop Layout State
  isPreviewOpen: boolean;

  // Layout State (Persisted in LocalStorage via Zustand)
  leftPanelWidth: number;
  rightPanelWidth: number;

  // Ignore Management State
  isIgnoreManagementOpen: boolean;

  setPanelWidths: (left: number, right: number) => void;
  togglePreview: () => void;
  setIgnoreManagementOpen: (open: boolean) => void;
}

export const createLayoutSlice: AppSliceCreator<LayoutSlice> = (set) => ({
  isPreviewOpen: true,
  leftPanelWidth: 260,
  rightPanelWidth: 320,
  isIgnoreManagementOpen: false,

  setPanelWidths: (left, right) => set({ leftPanelWidth: left, rightPanelWidth: right }),
  togglePreview: () => set((state) => ({ isPreviewOpen: !state.isPreviewOpen })),
  setIgnoreManagementOpen: (open) => set({ isIgnoreManagementOpen: open }),
});
