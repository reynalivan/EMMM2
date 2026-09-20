import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BrowserLibraryPanel } from './BrowserLibraryPanel';

describe('BrowserLibraryPanel', () => {
  it('renders as a docked non-modal region', () => {
    render(
      <BrowserLibraryPanel
        layout="docked"
        bookmarks={[]}
        history={[]}
        privacy={null}
        activeTab="bookmarks"
        onTabChange={vi.fn()}
        onClose={vi.fn()}
        onNavigate={vi.fn()}
        onEditBookmark={vi.fn()}
        onDeleteBookmark={vi.fn()}
        onClearHistory={vi.fn()}
      />,
    );

    const panel = screen.getByRole('complementary', { name: 'Bookmarks & history' });
    expect(panel).not.toHaveClass('fixed');
    expect(panel).toHaveClass('w-[512px]');
  });
});
