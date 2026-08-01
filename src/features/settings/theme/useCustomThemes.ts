import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type CustomTheme, type ThemeMetadata } from '../../../lib/bindings';

export const themeKeys = {
  all: ['custom-themes'] as const,
  list: () => [...themeKeys.all, 'list'] as const,
  detail: (id: string) => [...themeKeys.all, 'detail', id] as const,
};

/**
 * The user's custom themes. Cached by react-query so the theme injector and
 * the settings list share one fetch; mutations invalidate rather than refetch
 * by hand.
 */
export function useCustomThemes() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery<ThemeMetadata[]>({
    queryKey: themeKeys.list(),
    queryFn: () => commands.listCustomThemes(),
    staleTime: Infinity,
  });

  const refreshCustomThemes = () => queryClient.invalidateQueries({ queryKey: themeKeys.all });

  return { customThemes: data ?? [], loading: isLoading, refreshCustomThemes };
}

/**
 * A single custom theme's definition. Pass null for built-in themes so the
 * query stays disabled — switching back and forth then costs no extra IPC.
 */
export function useCustomTheme(id: string | null | undefined) {
  return useQuery<CustomTheme>({
    queryKey: themeKeys.detail(id ?? ''),
    queryFn: () => commands.loadCustomTheme(id as string),
    enabled: !!id,
    staleTime: Infinity,
  });
}
