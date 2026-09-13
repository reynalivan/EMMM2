import { useQuery } from '@tanstack/react-query';
import { useActiveGame } from '@/entities/game';
import { modHealthKeys } from '@/entities/mod';
import { commands } from '@/shared/api/tauri/bindings';

function normalizeFolderPath(folderPath: string | null | undefined): string | null {
  const normalized = folderPath?.trim();
  return normalized || null;
}

/** Health runs only for the mod currently shown by Preview. */
export function useModHealth(folderPath: string | null | undefined) {
  const { activeGame } = useActiveGame();
  const normalizedPath = normalizeFolderPath(folderPath);
  const gameId = activeGame?.id ?? null;

  return useQuery({
    queryKey: modHealthKeys.report(gameId ?? '', normalizedPath ?? ''),
    queryFn: () => commands.analyzeModHealth(gameId!, normalizedPath!),
    enabled: Boolean(gameId && normalizedPath),
    staleTime: Number.POSITIVE_INFINITY,
  });
}
