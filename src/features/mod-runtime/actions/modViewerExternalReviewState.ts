import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { QueryClient } from '@tanstack/react-query';
import i18next from 'i18next';
import { useActiveGame } from '@/entities/game';
import { modHealthKeys, type ModViewerExternalReview } from '@/entities/mod';
import { commands, type DiskReconcileResult } from '@/shared/api/tauri/bindings';
import { canonicalPathKey } from '@/shared/lib/pathKey';
import { toast } from '@/shared/ui/toast';
import {
  diffModViewerManifests,
  isModViewerSnapshotAffected,
  type ModViewerManifestFile,
} from './modViewerExternalReview';

const MAX_SNAPSHOTS = 8;

export interface ModViewerLaunchReceiptData {
  game_id: string;
  mod_folder: string;
  file_manifest: Array<{
    relative_path: string;
    size_bytes: number;
    blake3: string;
  }>;
}

interface ModViewerSnapshot {
  gameId: string;
  folderPath: string;
  manifest: ModViewerManifestFile[];
  launchedAt: number;
}

interface StoredModViewerExternalReview extends ModViewerExternalReview {
  fingerprint: string;
}

interface ModHealthManifestReport {
  file_manifest: Array<{
    relative_path: string;
    size_bytes: number;
    blake3: string;
  }>;
}

function toManifestFile(
  entry: ModViewerLaunchReceiptData['file_manifest'][number],
): ModViewerManifestFile {
  return {
    relative_path: entry.relative_path,
    size_bytes: entry.size_bytes,
    content_hash: entry.blake3,
  };
}

function snapshotsFor(queryClient: QueryClient): ModViewerSnapshot[] {
  return queryClient.getQueryData<ModViewerSnapshot[]>(modHealthKeys.viewerSnapshots()) ?? [];
}

function matchingSnapshotIndex(
  snapshots: readonly ModViewerSnapshot[],
  gameId: string,
  folderPath: string,
): number {
  const expected = canonicalPathKey(folderPath);
  return snapshots.findIndex(
    (snapshot) => snapshot.gameId === gameId && canonicalPathKey(snapshot.folderPath) === expected,
  );
}

function reviewFingerprint(review: ModViewerExternalReview): string {
  return JSON.stringify({ changes: review.changes, collectionImpact: review.collectionImpact });
}

export function recordModViewerLaunchSnapshot(
  queryClient: QueryClient,
  receipt: ModViewerLaunchReceiptData,
): void {
  const snapshots = snapshotsFor(queryClient);
  const existingIndex = matchingSnapshotIndex(snapshots, receipt.game_id, receipt.mod_folder);
  if (existingIndex >= 0) {
    snapshots.splice(existingIndex, 1);
  }

  snapshots.push({
    gameId: receipt.game_id,
    folderPath: receipt.mod_folder,
    manifest: receipt.file_manifest.map(toManifestFile),
    launchedAt: Date.now(),
  });
  const bounded = snapshots.slice(-MAX_SNAPSHOTS);
  queryClient.setQueryData(modHealthKeys.viewerSnapshots(), bounded);
  queryClient.removeQueries({
    queryKey: modHealthKeys.viewerReview(receipt.game_id, receipt.mod_folder),
    exact: true,
  });
}

export function dismissModViewerExternalReview(
  queryClient: QueryClient,
  gameId: string,
  folderPath: string,
): void {
  const snapshots = snapshotsFor(queryClient);
  const index = matchingSnapshotIndex(snapshots, gameId, folderPath);
  if (index >= 0) {
    snapshots.splice(index, 1);
    queryClient.setQueryData(modHealthKeys.viewerSnapshots(), snapshots);
  }
  queryClient.removeQueries({
    queryKey: modHealthKeys.viewerReview(gameId, folderPath),
    exact: true,
  });
}

export function useModViewerExternalReview(folderPath: string | null | undefined) {
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const gameId = activeGame?.id ?? '';
  const normalizedPath = folderPath?.trim() ?? '';
  const queryKey = modHealthKeys.viewerReview(gameId, normalizedPath);
  const review = useQuery({
    queryKey,
    queryFn: async () => queryClient.getQueryData<StoredModViewerExternalReview>(queryKey) ?? null,
    enabled: false,
    initialData: () => queryClient.getQueryData<StoredModViewerExternalReview>(queryKey) ?? null,
  });

  return {
    review: review.data,
    dismiss: () => {
      if (!gameId || !normalizedPath) {
        return;
      }
      dismissModViewerExternalReview(queryClient, gameId, normalizedPath);
    },
  };
}

/** Reconciler-owned bridge from a disk observation to any active viewer receipts. */
export async function reconcileModViewerExternalReviews(
  result: DiskReconcileResult,
  queryClient: QueryClient,
  modsPath: string | null | undefined,
  collectionImpact: string | null,
): Promise<void> {
  if (!modsPath || result.changed_roots.length === 0) {
    return;
  }

  const relevantSnapshots = snapshotsFor(queryClient).filter(
    (snapshot) =>
      snapshot.gameId === result.game_id &&
      isModViewerSnapshotAffected(snapshot.folderPath, result.changed_roots, modsPath),
  );

  await Promise.all(
    relevantSnapshots.map(async (snapshot) => {
      try {
        const report = (await commands.analyzeModHealth(
          snapshot.gameId,
          snapshot.folderPath,
        )) as ModHealthManifestReport;
        queryClient.setQueryData(
          modHealthKeys.report(snapshot.gameId, snapshot.folderPath),
          report,
        );
        const changes = diffModViewerManifests(
          snapshot.manifest,
          report.file_manifest.map(toManifestFile),
        );
        if (changes.length === 0) {
          return;
        }

        const review: ModViewerExternalReview = {
          changes: changes.map((change) => ({
            kind: change.kind,
            category: change.category,
            relativePath: change.relative_path,
          })),
          collectionImpact,
        };
        const next: StoredModViewerExternalReview = {
          ...review,
          fingerprint: reviewFingerprint(review),
        };
        const reviewKey = modHealthKeys.viewerReview(snapshot.gameId, snapshot.folderPath);
        const previous = queryClient.getQueryData<StoredModViewerExternalReview>(reviewKey);
        queryClient.setQueryData(reviewKey, next);
        if (previous?.fingerprint !== next.fingerprint) {
          toast.info(
            i18next.t('preview:mod_health.external_review.toast', { count: review.changes.length }),
          );
        }
      } catch (error) {
        console.warn('[ModHealth] External change review could not be refreshed:', error);
      }
    }),
  );
}
