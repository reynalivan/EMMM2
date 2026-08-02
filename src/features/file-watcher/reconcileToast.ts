import i18next from 'i18next';
import type { DiskReconcileResult } from '../../lib/bindings';
import { formatCollectionReferenceImpact } from '../../hooks/collectionReferenceImpact';
import { toast } from '../../stores/useToastStore';

const TOAST_SAMPLE_LIMIT = 2;

function totalChangeCount(counts: {
  added: number;
  removed: number;
  renamed: number;
  modified: number;
}): number {
  return counts.added + counts.removed + counts.renamed + counts.modified;
}

function formatToastNames(names: string[]): string {
  if (names.length === 0) {
    return '';
  }

  if (names.length <= TOAST_SAMPLE_LIMIT) {
    return names.join(', ');
  }

  return `${names.slice(0, TOAST_SAMPLE_LIMIT).join(', ')}, +${names.length - TOAST_SAMPLE_LIMIT} others`;
}

export function maybeShowExternalChangeToast(result: DiskReconcileResult): void {
  if (
    result.reason === 'StartupBoot' ||
    result.reason === 'InternalMutation' ||
    result.reason === 'OnboardingCompleted' ||
    result.reason === 'GameSwitched' ||
    !result.change_summary.has_user_visible_changes
  ) {
    return;
  }

  const objectCount = totalChangeCount(result.change_summary.object_changes);
  const modCount = totalChangeCount(result.change_summary.mod_changes);
  const messages: string[] = [];

  if (objectCount > 0) {
    const names = formatToastNames(result.change_summary.object_sample_names);
    messages.push(
      names
        ? i18next.t('common:watcher.toast.object_changes', { count: objectCount, names })
        : i18next.t('common:watcher.toast.object_changes_detected', { count: objectCount }),
    );
  }

  if (modCount > 0) {
    const names = formatToastNames(result.change_summary.mod_sample_names);
    messages.push(
      names
        ? i18next.t('common:watcher.toast.mod_changes', { count: modCount, names })
        : i18next.t('common:watcher.toast.mod_changes_detected', { count: modCount }),
    );
  }

  const collectionImpactMessage = formatCollectionReferenceImpact(
    result.collection_reference_impact,
  );
  if (collectionImpactMessage) {
    messages.push(collectionImpactMessage);
  }

  if (messages.length > 0) {
    toast.info(messages.join(' | '), 5000);
  }
}
