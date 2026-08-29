import type { TFunction } from 'i18next';
import type { GameConfig } from '../../../types/game';
import { toast } from '../../../stores/useToastStore';
import { openObjectClassificationWizard } from '../../import-batches/classificationLauncher';

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
