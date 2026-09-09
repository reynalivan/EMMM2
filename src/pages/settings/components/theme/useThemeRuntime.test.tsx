import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useThemeRuntime } from '../../hooks/useThemeRuntime';

const mockUseResolvedTheme = vi.fn();

vi.mock('@/entities/settings', () => ({
  useResolvedTheme: () => mockUseResolvedTheme(),
}));

function ThemeProbe() {
  useThemeRuntime();
  return null;
}

describe('useThemeRuntime', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.documentElement.removeAttribute('data-theme');
  });

  it('applies data-theme from settings.theme on mount', () => {
    mockUseResolvedTheme.mockReturnValue('cyberpunk');

    render(<ThemeProbe />);

    expect(document.documentElement.getAttribute('data-theme')).toBe('cyberpunk');
  });

  it('maps settings.theme=system to onyx when prefers-color-scheme is dark', () => {
    vi.mocked(window.matchMedia).mockImplementation((query: string) => {
      return {
        matches: true,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      } as MediaQueryList;
    });

    mockUseResolvedTheme.mockReturnValue('onyx');

    render(<ThemeProbe />);

    expect(document.documentElement.getAttribute('data-theme')).toBe('onyx');
  });
});
