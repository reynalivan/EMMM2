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
  isNewTab: false,
  isBookmarked: false,
  activeZoom: 1,
  isNavigating: false,
  isRefreshing: false,
  isMoreMenuOpen: false,
  onMoreMenuOpenChange: vi.fn(),
  onGoBack: vi.fn(),
  onGoForward: vi.fn(),
  onReload: vi.fn(),
  onNewTab: vi.fn(),
  onToggleBookmark: vi.fn(),
  onOpenLibrary: vi.fn(),
  onOpenExternally: vi.fn(),
  onChangeZoom: vi.fn(),
  onOpenFind: vi.fn(),
  adblockEnabled: true,
  onToggleAdblock: vi.fn(),
  onClearCookiesAndSiteData: vi.fn(),
  onClearCache: vi.fn(),
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

  it('uses the controlled menu state before opening a browser overlay', () => {
    const onMoreMenuOpenChange = vi.fn();
    const onOpenLibrary = vi.fn();
    render(
      <BrowserToolbar
        {...props}
        isMoreMenuOpen
        onMoreMenuOpenChange={onMoreMenuOpenChange}
        onOpenLibrary={onOpenLibrary}
      />,
    );

    expect(screen.getByRole('button', { name: 'Discover browser menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    expect(screen.getByTestId('browser-toolbar-menu-overlay')).toHaveClass('fixed');

    fireEvent.click(screen.getByRole('button', { name: 'Bookmarks' }));

    expect(onMoreMenuOpenChange).toHaveBeenCalledWith(false);
    expect(onOpenLibrary).toHaveBeenCalledWith('bookmarks');
  });

  it('opens a new local tab and routes history to its own library tab', () => {
    const onMoreMenuOpenChange = vi.fn();
    const onNewTab = vi.fn();
    const onOpenLibrary = vi.fn();
    render(
      <BrowserToolbar
        {...props}
        isMoreMenuOpen
        onMoreMenuOpenChange={onMoreMenuOpenChange}
        onNewTab={onNewTab}
        onOpenLibrary={onOpenLibrary}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'New Tab' }));
    expect(onNewTab).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole('button', { name: 'History' }));
    expect(onOpenLibrary).toHaveBeenCalledWith('history');
  });
});
