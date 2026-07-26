import { useAppStore } from '../../../stores/useAppStore';
import { recordInternalWorkspacePathRewrites } from '../selectionReconciliation';
import { dispatchWorkspaceRuntimeEvent } from '../state/workspaceStoreBridge';
import type { RuntimeEffectDescriptor } from '../../../lib/runtimeEffects';

type RuntimePathRewrite = RuntimeEffectDescriptor['rewrites'][number];
type WorkspacePathRewriteSource = 'internal' | 'disk_reconcile';

/**
 * Store-side path rewrites after a rename/toggle: keep grid selection and the
 * workspace runtime store pointing at the new paths. Cached view models are NOT
 * patched — the accompanying refresh descriptor refetches them.
 */
export function applyWorkspacePathRewrites(
  rewrites: RuntimePathRewrite[],
  source: WorkspacePathRewriteSource,
): void {
  if (rewrites.length === 0) {
    return;
  }

  if (source === 'internal') {
    recordInternalWorkspacePathRewrites(rewrites, Date.now());
  }

  const appStore = useAppStore.getState();
  appStore.replaceGridSelections(rewrites);
  dispatchWorkspaceRuntimeEvent({
    type: 'PATHS_REWRITTEN',
    rewrites,
  });
}
