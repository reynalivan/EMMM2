import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CustomTheme } from '@/shared/api/tauri/bindings';
import { DynamicThemeInjector } from './DynamicThemeInjector';

const mocks = vi.hoisted(() => ({
  theme: 'ocean-night',
  rawTheme: 'ocean-night',
  useCustomTheme: vi.fn(),
}));

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({ settings: { theme: mocks.rawTheme } }),
  useResolvedTheme: () => mocks.theme,
  useCustomTheme: mocks.useCustomTheme,
}));

const customTheme: CustomTheme = {
  id: 'ocean-night',
  label: 'Ocean Night',
  config: {
    colors: {
      primary: '#112233',
      secondary: '#223344',
      accent: '#334455',
      neutral: '#445566',
      'base-100': '#556677',
    },
    glass: { bg: 'rgba(85, 102, 119, 0.5)', border: 'rgba(255, 255, 255, 0.2)' },
    liquid: {},
    background: {
      kind: 'gradient',
      value: 'linear-gradient(135deg, #112233, #556677)',
      dim_opacity: 0.6,
    },
  },
};

describe('DynamicThemeInjector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.getElementById('dynamic-theme-style')?.remove();
    mocks.theme = customTheme.id;
    mocks.rawTheme = customTheme.id;
    mocks.useCustomTheme.mockReturnValue({ data: customTheme, isError: false });
  });

  it('injects semantic colors, glass, liquid tint and background without DaisyUI 4 aliases', () => {
    render(<DynamicThemeInjector />);

    const css = document.getElementById('dynamic-theme-style')?.textContent;
    expect(css).toContain('[data-theme="ocean-night"]');
    expect(css).toContain('--color-primary: #112233;');
    expect(css).toContain('--color-base-100: #556677;');
    expect(css).toContain('--glass-bg: rgba(85, 102, 119, 0.5);');
    expect(css).toContain('--glass-border: rgba(255, 255, 255, 0.2);');
    expect(css).toContain('--liquid-tint-rgb: 85, 102, 119;');
    expect(css).toContain(
      '--theme-shell-background-image: linear-gradient(135deg, #112233, #556677);',
    );
    expect(css).toContain('--theme-shell-dim-opacity: 0.6;');
    expect(css).not.toMatch(/--(?:p|s|a|n|b1):/);
  });

  it('clears custom overrides when the resolved selection is built in', () => {
    const { rerender } = render(<DynamicThemeInjector />);
    mocks.theme = 'onyx';
    mocks.rawTheme = 'dark';
    rerender(<DynamicThemeInjector />);

    expect(mocks.useCustomTheme).toHaveBeenLastCalledWith(null);
    expect(document.getElementById('dynamic-theme-style')?.textContent).toBe('');
  });

  it('keeps a custom dark theme loadable', () => {
    mocks.theme = 'dark';
    mocks.rawTheme = 'dark';
    mocks.useCustomTheme.mockReturnValue({ data: { ...customTheme, id: 'dark' }, isError: false });
    render(<DynamicThemeInjector />);

    expect(mocks.useCustomTheme).toHaveBeenCalledWith('dark');
    expect(document.getElementById('dynamic-theme-style')?.textContent).toContain(
      '[data-theme="dark"]',
    );
  });
});
