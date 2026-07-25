import { useCallback, useMemo } from 'react';
import { reduceWorkspaceRuntimeState } from '../../../stores/appStore/workspaceRuntimeReducer';
import { selectWorkspaceRuntimeState } from '../../../stores/appStore/workspaceRuntimeSlice';
import { useAppStore } from '../../../stores/useAppStore';
import type { AppState } from '../../../stores/useAppStore';
import type { WorkspaceRuntimeEvent } from './workspaceEvents';
import type { WorkspaceRuntimeState, WorkspaceTransitionTarget } from './workspaceState';

const FALLBACK_RUNTIME_STATE: WorkspaceRuntimeState = {
  selectedObjectFolderPath: null,
  explorerSubPath: undefined,
  currentPath: [],
  selectedModPath: null,
  mobileActivePane: 'sidebar',
  previewDirty: false,
  previewTransition: { kind: 'idle', pendingTarget: null },
  dialogState: { kind: 'none' },
};

function readAppStoreState(): AppState | null {
  const store = useAppStore as typeof useAppStore & { getState?: () => AppState };
  if (typeof store.getState !== 'function') {
    return null;
  }

  return store.getState();
}

export function getWorkspaceRuntimeState(): WorkspaceRuntimeState {
  const storeState = readAppStoreState();
  if (!storeState) {
    return FALLBACK_RUNTIME_STATE;
  }

  return selectWorkspaceRuntimeState(storeState);
}

export function dispatchWorkspaceRuntimeEvent(event: WorkspaceRuntimeEvent): WorkspaceRuntimeState {
  const storeState = readAppStoreState();
  if (!storeState || typeof storeState.dispatchWorkspaceRuntime !== 'function') {
    // Partial store mocks in tests: compute the next state without writing to the store.
    return reduceWorkspaceRuntimeState(getWorkspaceRuntimeState(), event);
  }

  return storeState.dispatchWorkspaceRuntime(event);
}

export function useWorkspaceRuntimeSelector<T>(selector: (state: WorkspaceRuntimeState) => T): T {
  return useAppStore((state) => selector(selectWorkspaceRuntimeState(state)));
}

export function useWorkspaceRuntime() {
  const selectedObjectFolderPath = useWorkspaceRuntimeSelector(
    (state) => state.selectedObjectFolderPath,
  );
  const explorerSubPath = useWorkspaceRuntimeSelector((state) => state.explorerSubPath);
  const currentPath = useWorkspaceRuntimeSelector((state) => state.currentPath);
  const selectedModPath = useWorkspaceRuntimeSelector((state) => state.selectedModPath);
  const mobileActivePane = useWorkspaceRuntimeSelector((state) => state.mobileActivePane);
  const previewDirty = useWorkspaceRuntimeSelector((state) => state.previewDirty);
  const previewTransition = useWorkspaceRuntimeSelector((state) => state.previewTransition);
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);

  const runtimeState = useMemo(
    () => ({
      selectedObjectFolderPath,
      explorerSubPath,
      currentPath,
      selectedModPath,
      mobileActivePane,
      previewDirty,
      previewTransition,
      dialogState,
    }),
    [
      selectedObjectFolderPath,
      explorerSubPath,
      currentPath,
      selectedModPath,
      mobileActivePane,
      previewDirty,
      previewTransition,
      dialogState,
    ],
  );

  const dispatch = useCallback((event: WorkspaceRuntimeEvent) => {
    return dispatchWorkspaceRuntimeEvent(event);
  }, []);

  const focusObject = useCallback((folderPath: string) => {
    dispatchWorkspaceRuntimeEvent({ type: 'OBJECT_FOCUSED', folderPath });
  }, []);

  const navigateExplorer = useCallback((currentPath: string[], explorerSubPath?: string) => {
    dispatchWorkspaceRuntimeEvent({
      type: 'EXPLORER_NAVIGATED',
      currentPath,
      explorerSubPath,
    });
  }, []);

  const selectMod = useCallback(
    (path: string | null, mobilePane?: 'sidebar' | 'grid' | 'details') => {
      dispatchWorkspaceRuntimeEvent({ type: 'MOD_SELECTED', path, mobilePane });
    },
    [],
  );

  const clearSelection = useCallback(
    (target: Omit<Extract<WorkspaceTransitionTarget, { kind: 'clearSelection' }>, 'kind'>) => {
      dispatchWorkspaceRuntimeEvent({
        type: 'SELECTION_CLEARED',
        ...target,
      });
    },
    [],
  );

  return {
    state: runtimeState,
    dispatch,
    focusObject,
    navigateExplorer,
    selectMod,
    clearSelection,
  };
}
