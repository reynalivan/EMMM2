import type { AppState } from '../useAppStore';

export type AppStateUpdater =
  Partial<AppState> | ((state: AppState) => Partial<AppState> | AppState);

export type AppSliceCreator<TSlice> = (
  set: (partial: AppStateUpdater) => void,
  get: () => AppState,
) => TSlice;
