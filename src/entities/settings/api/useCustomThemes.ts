import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type CustomTheme, type ThemeMetadata } from '@/shared/api/tauri/bindings';
import { publishQueryInvalidations } from '@/shared/lib/queryRefresh';

export const customThemeKeys = {
  all: ['custom-themes'] as const,
  list: () => [...customThemeKeys.all, 'list'] as const,
  detail: (id: string) => [...customThemeKeys.all, 'detail', id] as const,
};

export function useCustomThemes() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery<ThemeMetadata[]>({
    queryKey: customThemeKeys.list(),
    queryFn: () => commands.listCustomThemes(),
    staleTime: Infinity,
  });

  return {
    customThemes: data ?? [],
    loading: isLoading,
    refreshCustomThemes: () =>
      publishQueryInvalidations(queryClient, [customThemeKeys.all], 'active'),
  };
}

export function useCustomTheme(id: string | null | undefined) {
  return useQuery<CustomTheme>({
    queryKey: customThemeKeys.detail(id ?? ''),
    queryFn: () => commands.loadCustomTheme(id as string),
    enabled: Boolean(id),
    staleTime: Infinity,
  });
}
