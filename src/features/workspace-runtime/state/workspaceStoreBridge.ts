import { useCallback, useMemo } from 'react';
import { useAppStore } from '../../../stores/useAppStore';
import type { WorkspaceRuntimeEvent } from './workspaceEvents';
import { reduceWorkspaceRuntimeState } from './workspaceReducer';
import { selectWorkspaceRuntimeState } from './workspaceSelectors';
import type { AppState } from '../../../stores/useAppStore';
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

function applyWorkspaceRuntimeState(nextState: WorkspaceRuntimeState): void {
  const store = useAppStore as typeof useAppStore & {
    setState?: (partial: Partial<AppState>) => void;
  };
  if (typeof store.setState !== 'function') {
    return;
  }

  store.setState({
    selectedObjectFolderPath: nextState.selectedObjectFolderPath,
    explorerSubPath: nextState.explorerSubPath,
    currentPath: nextState.currentPath,
    selectedModPath: nextState.selectedModPath,
    mobileActivePane: nextState.mobileActivePane,
    workspacePreviewDirty: nextState.previewDirty,
    workspacePreviewTransition: nextState.previewTransition,
    workspaceDialogState: nextState.dialogState,
  });
}

export function getWorkspaceRuntimeState(): WorkspaceRuntimeState {
  const storeState = readAppStoreState();
  if (!storeState) {
    return FALLBACK_RUNTIME_STATE;
  }

  return selectWorkspaceRuntimeState(storeState);
}

export function restoreWorkspaceRuntimeState(state: WorkspaceRuntimeState): void {
  applyWorkspaceRuntimeState(state);
}

export function dispatchWorkspaceRuntimeEvent(event: WorkspaceRuntimeEvent): WorkspaceRuntimeState {
  const currentState = getWorkspaceRuntimeState();
  const nextState = reduceWorkspaceRuntimeState(currentState, event);
  applyWorkspaceRuntimeState(nextState);
  return nextState;
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
