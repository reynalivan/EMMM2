import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { BrowserToolbar, BrowserToolbarMenu } from './BrowserToolbar';

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

  it('keeps the toolbar menu trigger in browser chrome instead of rendering a floating overlay', () => {
    const onMoreMenuOpenChange = vi.fn();
    render(
      <BrowserToolbar {...props} isMoreMenuOpen onMoreMenuOpenChange={onMoreMenuOpenChange} />,
    );

    expect(screen.getByRole('button', { name: 'Discover browser menu' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    expect(screen.queryByTestId('browser-toolbar-menu-overlay')).not.toBeInTheDocument();
  });

  it('renders menu actions in the supplied chrome tray and closes before opening a panel', () => {
    const onClose = vi.fn();
    const onOpenLibrary = vi.fn();
    const { container } = render(
      <BrowserToolbarMenu
        activeTabUrl={props.activeTabUrl}
        activeZoom={1}
        adblockEnabled
        hasActiveWebview
        onChangeZoom={vi.fn()}
        onClearCache={vi.fn()}
        onClearCookiesAndSiteData={vi.fn()}
        onClose={onClose}
        onNewTab={vi.fn()}
        onOpenExternally={vi.fn()}
        onOpenFind={vi.fn()}
        onOpenLibrary={onOpenLibrary}
        onToggleAdblock={vi.fn()}
      />,
    );

    expect(container.querySelector('[data-testid="browser-toolbar-menu"]')).toBeInTheDocument();
    expect(screen.queryByTestId('browser-toolbar-menu-overlay')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Bookmarks' }));

    expect(onClose).toHaveBeenCalledOnce();
    expect(onOpenLibrary).toHaveBeenCalledWith('bookmarks');
  });
});
