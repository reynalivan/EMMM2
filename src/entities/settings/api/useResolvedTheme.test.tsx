import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CustomTheme, ThemeMetadata } from '@/shared/api/tauri/bindings';
import { useLiquidThemeConfig } from '@/shared/ui/liquid/liquidTheme';
import { useResolvedTheme } from './useResolvedTheme';

vi.unmock('@tanstack/react-query');

const mocks = vi.hoisted(() => ({ theme: 'dark' }));

vi.mock('./useSettings', () => ({
  useSettings: () => ({ settings: { theme: mocks.theme } }),
}));

const darkTheme: CustomTheme = {
  id: 'dark',
  label: 'Custom Dark',
  config: {
    colors: {},
    glass: {},
    liquid: { control: { blur: 17, material: 'thick', appearance: 'light' } },
    background: { kind: 'solid', value: '#101827', dim_opacity: 0.7 },
  },
};

function renderThemes(cachedThemes?: ThemeMetadata[]) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(['settings'], { theme: mocks.theme });
  if (cachedThemes) client.setQueryData(['custom-themes', 'list'], cachedThemes);
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const hook = renderHook(
    () => ({ resolved: useResolvedTheme(), liquid: useLiquidThemeConfig('control').config }),
    { wrapper },
  );
  return { ...hook, client };
}

describe('resolved theme consumers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.theme = 'dark';
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'list_custom_themes') return [];
      if (command === 'load_custom_theme') return darkTheme;
      throw new Error(`Unexpected command: ${command}`);
    });
  });

  it('maps legacy dark to onyx consistently without loading a nonexistent custom theme', async () => {
    const { result } = renderThemes();

    await waitFor(() => expect(result.current.resolved).toBe('onyx'));
    expect(result.current.liquid.appearance).toBe('dark');
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('list_custom_themes');
  });

  it('preserves custom dark and uses its liquid role configuration', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'list_custom_themes') return [{ id: 'dark', label: darkTheme.label }];
      if (command === 'load_custom_theme') return darkTheme;
      throw new Error(`Unexpected command: ${command}`);
    });
    const { result } = renderThemes();

    await waitFor(() => expect(result.current.liquid.blur).toBe(17));
    expect(result.current.resolved).toBe('dark');
    expect(result.current.liquid.appearance).toBe('light');
    expect(result.current.liquid.material).toBe('thick');
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it('keeps the raw dark setting while the custom list is pending', async () => {
    let finishList: (themes: ThemeMetadata[]) => void = () => {
      throw new Error('Theme list has not started');
    };
    vi.mocked(invoke).mockImplementation((command) => {
      if (command !== 'list_custom_themes') {
        throw new Error(`Unexpected command while resolving legacy dark: ${command}`);
      }
      return new Promise<ThemeMetadata[]>((resolve) => (finishList = resolve));
    });
    const { result } = renderThemes();
    expect(result.current.resolved).toBe('dark');
    expect(invoke).toHaveBeenCalledTimes(1);
    await act(async () => finishList([]));

    await waitFor(() => expect(result.current.resolved).toBe('onyx'));
  });

  it('keeps raw dark when custom theme discovery fails', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'list_custom_themes') throw new Error('Theme directory is unavailable');
      if (command === 'record_native_error_metric') return;
      throw new Error(`Unexpected command: ${command}`);
    });
    const { result, client } = renderThemes();

    await waitFor(() =>
      expect(client.getQueryState(['custom-themes', 'list'])?.status).toBe('error'),
    );
    expect(invoke).toHaveBeenCalledWith('list_custom_themes');
    expect(invoke).not.toHaveBeenCalledWith('load_custom_theme', expect.anything());
    expect(result.current.resolved).toBe('dark');
    expect(result.current.liquid.appearance).toBe('auto');
  });

  it('does not treat a cached empty list as confirmed absence after a failed refresh, then recovers', async () => {
    let themeReadable = false;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'list_custom_themes') {
        if (!themeReadable) throw new Error('Custom dark theme is temporarily unreadable');
        return [{ id: 'dark', label: darkTheme.label }];
      }
      if (command === 'load_custom_theme') return darkTheme;
      if (command === 'record_native_error_metric') return;
      throw new Error(`Unexpected command: ${command}`);
    });
    const { result, client } = renderThemes([]);
    expect(result.current.resolved).toBe('onyx');
    await act(async () => {
      await client.refetchQueries({ queryKey: ['custom-themes', 'list'] });
    });

    await waitFor(() =>
      expect(client.getQueryState(['custom-themes', 'list'])?.status).toBe('error'),
    );
    expect(result.current.resolved).toBe('dark');
    expect(result.current.liquid.appearance).toBe('auto');
    expect(invoke).not.toHaveBeenCalledWith('load_custom_theme', expect.anything());

    themeReadable = true;
    await act(async () => {
      await client.refetchQueries({ queryKey: ['custom-themes', 'list'] });
    });
    await waitFor(() => expect(result.current.liquid.blur).toBe(17));
    expect(result.current.resolved).toBe('dark');
    expect(result.current.liquid.appearance).toBe('light');
  });

  it('keeps raw dark while a cached empty list is being refreshed', async () => {
    let finishList: (themes: ThemeMetadata[]) => void = () => {
      throw new Error('Theme list refresh has not started');
    };
    vi.mocked(invoke).mockImplementation((command) => {
      if (command !== 'list_custom_themes') throw new Error(`Unexpected command: ${command}`);
      return new Promise<ThemeMetadata[]>((resolve) => (finishList = resolve));
    });
    const { result, client } = renderThemes([]);
    expect(result.current.resolved).toBe('onyx');
    let refresh: Promise<void>;
    act(() => {
      refresh = client.refetchQueries({ queryKey: ['custom-themes', 'list'] });
    });
    await waitFor(() => expect(result.current.resolved).toBe('dark'));
    expect(result.current.liquid.appearance).toBe('auto');
    await act(async () => {
      finishList([]);
      await refresh;
    });
    await waitFor(() => expect(result.current.resolved).toBe('onyx'));
  });

  it.each(['onyx', 'light', 'system'])('does not read the custom list for builtin %s', (theme) => {
    mocks.theme = theme;
    const { result } = renderThemes();

    expect(result.current.resolved).toBe(theme === 'system' ? 'light' : theme);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('loads an ordinary custom theme without a list read', async () => {
    mocks.theme = 'ocean-night';
    const { result } = renderThemes();

    await waitFor(() => expect(result.current.liquid.blur).toBe(17));
    expect(result.current.resolved).toBe('ocean-night');
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('load_custom_theme', { id: 'ocean-night' });
  });
});
