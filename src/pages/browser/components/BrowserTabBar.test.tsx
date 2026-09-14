import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { BrowserTabBar } from './BrowserTabBar';

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
  onContextMenuOpenChange: vi.fn(),
};

describe('BrowserTabBar', () => {
  it('renders a clear active tab state without a vertical tab-strip scrollbar', () => {
    const { container } = render(<BrowserTabBar {...props} />);

    expect(screen.getByRole('tab', { name: 'Example' })).toHaveAttribute('aria-selected', 'true');
    expect(container.querySelector('.browser-tab-strip')).toHaveClass('overflow-y-hidden');
  });

  it('offers tab actions from the selected tab context menu', () => {
    render(<BrowserTabBar {...props} />);

    fireEvent.contextMenu(screen.getByRole('tab', { name: 'Example' }), {
      clientX: 24,
      clientY: 24,
    });

    expect(props.onSelectTab).toHaveBeenCalledWith('browser-tab-1');
    expect(screen.getByTestId('browser-tab-context-menu')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('menuitem', { name: 'Duplicate tab' }));
    expect(props.onDuplicateTab).toHaveBeenCalledWith('browser-tab-1');

    fireEvent.contextMenu(screen.getByRole('tab', { name: 'Example' }), {
      clientX: 24,
      clientY: 24,
    });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Reload tab' }));
    expect(props.onReloadTab).toHaveBeenCalledWith('browser-tab-1');
  });

  it('restores the last closed tab only when one is available', () => {
    render(<BrowserTabBar {...props} />);

    fireEvent.contextMenu(screen.getByRole('tab', { name: 'Example' }), {
      clientX: 24,
      clientY: 24,
    });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Reopen last closed tab' }));

    expect(props.onRestoreLastClosedTab).toHaveBeenCalledOnce();
  });
});
