import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type CustomTheme, type ThemeMetadata } from '@/shared/api/tauri/bindings';
import { publishQueryInvalidations } from '@/shared/lib/queryRefresh';
import { getConfirmedCustomThemeIds, getCustomThemeId } from '@/shared/lib/themeOptions';

export const customThemeKeys = {
  all: ['custom-themes'] as const,
  list: () => [...customThemeKeys.all, 'list'] as const,
  detail: (id: string) => [...customThemeKeys.all, 'detail', id] as const,
};

export function useCustomThemes(enabled = true) {
  const queryClient = useQueryClient();
  const query = useQuery<ThemeMetadata[]>({
    queryKey: customThemeKeys.list(),
    queryFn: () => commands.listCustomThemes(),
    enabled,
    staleTime: Infinity,
  });

  return {
    customThemes: query.data ?? [],
    customThemeIds: getConfirmedCustomThemeIds(query),
    loading: query.isLoading,
    error: query.error,
    refreshing: query.isFetching,
    refreshCustomThemes: () =>
      publishQueryInvalidations(queryClient, [customThemeKeys.all], 'active'),
  };
}

export function useCustomTheme(id: string | null | undefined) {
  const { customThemeIds } = useCustomThemes(id === 'dark');
  const customThemeId = getCustomThemeId(id, customThemeIds);
  return useQuery<CustomTheme>({
    queryKey: customThemeKeys.detail(customThemeId ?? ''),
    queryFn: () => {
      if (!customThemeId) throw new Error('No custom theme selected');
      return commands.loadCustomTheme(customThemeId);
    },
    enabled: Boolean(customThemeId),
    staleTime: Infinity,
  });
}
