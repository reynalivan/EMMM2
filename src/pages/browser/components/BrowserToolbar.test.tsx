import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { BrowserToolbar } from './BrowserToolbar';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

const props = {
  urlInput: 'https://gamebanana.com/mods/123?sort=recent#files',
  onUrlInputChange: vi.fn(),
  onUrlSubmit: vi.fn(),
  activeTabId: 'browser-tab-1',
  activeTabUrl: 'https://gamebanana.com/mods/123?sort=recent#files',
  isBookmarked: false,
  activeZoom: 1,
  isNavigating: false,
  isRefreshing: false,
  activeDownloadCount: 0,
  queuedDownloadCount: 0,
  onGoBack: vi.fn(),
  onGoForward: vi.fn(),
  onReload: vi.fn(),
  onOpenDiscover: vi.fn(),
  onToggleBookmark: vi.fn(),
  onOpenLibrary: vi.fn(),
  onOpenExternally: vi.fn(),
  onChangeZoom: vi.fn(),
  onOpenFind: vi.fn(),
  adblockEnabled: true,
  onToggleAdblock: vi.fn(),
  onClearCookiesAndSiteData: vi.fn(),
  onClearCache: vi.fn(),
  onOpenDownloads: vi.fn(),
};

describe('BrowserToolbar', () => {
  it('emphasizes the site identity, then exposes the full address for editing', async () => {
    render(<BrowserToolbar {...props} />);

    expect(screen.getByText('gamebanana.com')).toBeInTheDocument();
    expect(screen.getByText('/mods/123?sort=recent#files')).toHaveClass('text-base-content/45');
    expect(screen.getByRole('button', { name: 'Add bookmark' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: props.urlInput }));

    await waitFor(() => {
      expect(screen.getByDisplayValue(props.urlInput)).toHaveFocus();
    });
  });
});
