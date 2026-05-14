import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useThemeRuntime } from './useThemeRuntime';

const mockUseSettings = vi.fn();

vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => mockUseSettings(),
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
    mockUseSettings.mockReturnValue({
      settings: {
        theme: 'cyberpunk',
      },
    });

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

    mockUseSettings.mockReturnValue({
      settings: {
        theme: 'system',
      },
    });

    render(<ThemeProbe />);

    expect(document.documentElement.getAttribute('data-theme')).toBe('onyx');
  });
});
