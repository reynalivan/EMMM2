import type { QueryClient } from '@tanstack/react-query';
import type { Dispatch, SetStateAction } from 'react';
import type { TFunction } from 'i18next';
import { commands } from '../../lib/bindings';
import type { GameConfig } from '../../types/game';
import type { WorkspaceObjectNode } from '../../types/workspace';
import { toast } from '../../stores/useToastStore';
import { publishRuntimeDescriptor } from '../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../workspace-runtime/optimistic/descriptorBuilders';
import { createObjectUpdate } from './objectUpdateFactory';

interface BulkAutoRecognizeParams {
  ids: Set<string>;
  activeGame: GameConfig | null | undefined;
  objects: WorkspaceObjectNode[];
  queryClient: QueryClient;
  setIsSyncing: Dispatch<SetStateAction<boolean>>;
  t: TFunction<['objects', 'common']>;
}

function matchConfidenceValue(confidence: string | null | undefined): number {
  if (confidence === 'High') {
    return 1;
  }
  if (confidence === 'Medium') {
    return 0.7;
  }
  return 0.4;
}

export async function runBulkAutoRecognize({
  ids,
  activeGame,
  objects,
  queryClient,
  setIsSyncing,
  t,
}: BulkAutoRecognizeParams): Promise<void> {
  if (!activeGame) return;
  if (ids.size === 0) {
    toast.info(t('objects:auto_recognize.toast_none'));
    return;
  }

  try {
    setIsSyncing(true);
    let matched = 0;
    let skipped = 0;

    for (const object of objects.filter((candidate) => ids.has(candidate.id))) {
      const match = await commands.matchObjectWithDb({
        gameType: activeGame.game_type,
        objectName: object.name,
      });
      if (!match) {
        skipped += 1;
        continue;
      }

      await commands.applyObjectMatch({
        input: {
          game_id: activeGame.id,
          object_id: object.id,
          matched_entry_key: match.matched_entry_key ?? null,
          matched_alias_name: match.matched_alias_name ?? match.name,
          matched_confidence: matchConfidenceValue(match.match_confidence),
          matched_reason: match.match_detail,
          matched_source: 'auto_recognize',
        },
      });
      await commands.updateObject({
        id: object.id,
        updates: createObjectUpdate({
          object_type: match.object_type || null,
          metadata: match.metadata ?? null,
          tags: match.tags,
          thumbnail_path: match.thumbnail_path,
        }),
      });
      matched += 1;
    }

    await publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('objectRows'),
      'active',
    );
    if (matched === 0) {
      toast.info(t('objects:auto_recognize.toast_none'));
      return;
    }
    toast.success(t('objects:auto_recognize.toast_success', { matched, skipped }));
  } catch (error) {
    console.error('Auto-recognize failed:', error);
    toast.error(t('objects:auto_recognize.toast_error', { error: String(error) }));
  } finally {
    setIsSyncing(false);
  }
}
