import { useCallback } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { selectWorkspaceRuntimeState } from '../../../stores/appStore/workspaceRuntimeSlice';
import { useAppStore } from '../../../stores/useAppStore';
import type { WorkspaceRuntimeEvent } from './workspaceEvents';
import type { WorkspaceRuntimeState, WorkspaceTransitionTarget } from './workspaceState';

export function getWorkspaceRuntimeState(): WorkspaceRuntimeState {
  return selectWorkspaceRuntimeState(useAppStore.getState());
}

export function dispatchWorkspaceRuntimeEvent(event: WorkspaceRuntimeEvent): WorkspaceRuntimeState {
  return useAppStore.getState().dispatchWorkspaceRuntime(event);
}

export function useWorkspaceRuntimeSelector<T>(selector: (state: WorkspaceRuntimeState) => T): T {
  return useAppStore((state) => selector(selectWorkspaceRuntimeState(state)));
}

export function useWorkspaceRuntime() {
  const runtimeState = useAppStore(useShallow(selectWorkspaceRuntimeState));

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
