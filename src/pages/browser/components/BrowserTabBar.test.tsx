import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { BrowserTabBar, BrowserTabContextMenu } from './BrowserTabBar';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

const props = {
  tabs: [{ id: 'browser-tab-1', url: 'https://example.test', title: 'Example' }],
  activeTabId: 'browser-tab-1',
  canRestoreLastClosedTab: true,
  onSelectTab: vi.fn(),
  onCloseTab: vi.fn(),
  onNewTab: vi.fn(),
  onReloadTab: vi.fn(),
  onDuplicateTab: vi.fn(),
  onRestoreLastClosedTab: vi.fn(),
  onOpenContextMenu: vi.fn(),
  onContextMenuOpenChange: vi.fn(),
};

describe('BrowserTabBar', () => {
  it('renders a clear active tab state without a vertical tab-strip scrollbar', () => {
    const { container } = render(<BrowserTabBar {...props} />);

    expect(screen.getByRole('tab', { name: 'Example' })).toHaveAttribute('aria-selected', 'true');
    expect(container.querySelector('.browser-tab-strip')).toHaveClass('overflow-y-hidden');
  });

  it('requests an inline context-menu tray for the selected tab', () => {
    render(<BrowserTabBar {...props} />);

    fireEvent.contextMenu(screen.getByRole('tab', { name: 'Example' }), {
      clientX: 24,
      clientY: 24,
    });

    expect(props.onSelectTab).toHaveBeenCalledWith('browser-tab-1');
    expect(props.onOpenContextMenu).toHaveBeenCalledWith(props.tabs[0], 24);
    expect(screen.queryByTestId('browser-tab-context-menu')).not.toBeInTheDocument();
  });

  it('renders context actions in the supplied chrome tray and closes before acting', () => {
    const onClose = vi.fn();
    const onDuplicateTab = vi.fn();
    const { container } = render(
      <BrowserTabContextMenu
        canRestoreLastClosedTab={false}
        onClose={onClose}
        onCloseTab={vi.fn()}
        onDuplicateTab={onDuplicateTab}
        onReloadTab={vi.fn()}
        onRestoreLastClosedTab={vi.fn()}
        tab={props.tabs[0]}
      />,
    );

    expect(container.querySelector('[data-testid="browser-tab-context-menu"]')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('menuitem', { name: 'Duplicate tab' }));

    expect(onClose).toHaveBeenCalledOnce();
    expect(onDuplicateTab).toHaveBeenCalledWith('browser-tab-1');
  });
});
