import type { TFunction } from 'i18next';
import type { GameConfig } from '@/entities/game';
import { toast } from '@/shared/ui/toast';
import { openObjectClassificationWizard } from '@/features/import-batches';

interface BulkClassifyAndMatchParams {
  ids: Set<string>;
  activeGame: GameConfig | null | undefined;
  t: TFunction<['objects', 'common']>;
}

export async function runBulkClassifyAndMatch({
  ids,
  activeGame,
  t,
}: BulkClassifyAndMatchParams): Promise<void> {
  if (!activeGame) return;
  if (ids.size === 0) {
    toast.info(t('objects:classify_match.toast_none'));
    return;
  }
  openObjectClassificationWizard({
    gameId: activeGame.id,
    objectIds: [...ids],
  });
}
