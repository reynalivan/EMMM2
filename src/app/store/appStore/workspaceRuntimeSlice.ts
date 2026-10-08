import type { WorkspaceRuntimeEvent } from '@/features/workspace-runtime';
import type {
  WorkspaceDialogState,
  WorkspacePreviewTransitionState,
  WorkspaceRuntimeState,
} from '@/features/workspace-runtime';
import type { AppState } from '../useAppStore';
import type { AppSliceCreator } from './sliceTypes';
import {
  reduceWorkspaceRuntimeState,
  selectWorkspaceRuntimeState,
} from '@/features/workspace-runtime/@x/app';

export interface WorkspaceRuntimeSlice {
  // Workspace preview/dialog state, driven by dispatched runtime events.
  workspacePreviewDirty: boolean;
  workspacePreviewTransition: WorkspacePreviewTransitionState;
  workspaceDialogState: WorkspaceDialogState;

  dispatchWorkspaceRuntime: (event: WorkspaceRuntimeEvent) => WorkspaceRuntimeState;
}

function toAppStatePatch(runtimeState: WorkspaceRuntimeState): Partial<AppState> {
  return {
    selectedObjectFolderPath: runtimeState.selectedObjectFolderPath,
    explorerSubPath: runtimeState.explorerSubPath,
    currentPath: runtimeState.currentPath,
    selectedModPath: runtimeState.selectedModPath,
    gridSelection: runtimeState.gridSelection,
    mobileActivePane: runtimeState.mobileActivePane,
    workspacePreviewDirty: runtimeState.previewDirty,
    workspacePreviewTransition: runtimeState.previewTransition,
    workspaceDialogState: runtimeState.dialogState,
  };
}

export const createWorkspaceRuntimeSlice: AppSliceCreator<WorkspaceRuntimeSlice> = (set, get) => ({
  workspacePreviewDirty: false,
  workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
  workspaceDialogState: { kind: 'none' },

  dispatchWorkspaceRuntime: (event) => {
    const previousState = selectWorkspaceRuntimeState(get());
    const confirmedEffect =
      event.type === 'PREVIEW_TRANSITION_CONFIRMED' &&
      previousState.previewTransition.kind === 'pending' &&
      previousState.previewTransition.pendingTarget.kind === 'selectMod'
        ? previousState.previewTransition.pendingTarget.selectionEffect
        : undefined;
    const selectionEffect = event.type === 'MOD_SELECTED' ? event.selectionEffect : confirmedEffect;
    const effectIsCurrent = selectionEffect?.isCurrent?.() ?? true;
    if (event.type === 'MOD_SELECTED' && selectionEffect && !effectIsCurrent) {
      return previousState;
    }
    const eventToApply =
      event.type === 'PREVIEW_TRANSITION_CONFIRMED' && confirmedEffect && !effectIsCurrent
        ? { type: 'PREVIEW_TRANSITION_CANCELLED' as const }
        : event.type === 'MOD_SELECTED' && selectionEffect && !effectIsCurrent
          ? { ...event, selectionEffect: undefined }
          : event;
    const nextState = reduceWorkspaceRuntimeState(previousState, eventToApply);
    set(toAppStatePatch(nextState));
    const effectWasApplied =
      nextState.previewTransition.kind === 'idle' &&
      (event.type === 'PREVIEW_TRANSITION_CONFIRMED' ||
        (event.type === 'MOD_SELECTED' && nextState.selectedModPath === event.path));
    if (effectIsCurrent && effectWasApplied) {
      selectionEffect?.onApplied?.();
    }
    return nextState;
  },
});
