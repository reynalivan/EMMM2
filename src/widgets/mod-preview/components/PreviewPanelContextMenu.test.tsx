import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import PreviewPanelContextMenu from './PreviewPanelContextMenu';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('@/features/mod-runtime', () => ({
  useModContextMenuActions: () => ({
    openExplorer: vi.fn(),
    pasteThumbnailFromClipboard: vi.fn(),
    importThumbnail: vi.fn(),
  }),
  useModContextMenuItems: () => [
    {
      id: 'rename',
      label: 'Rename',
      icon: () => null,
      onClick: vi.fn(),
    },
  ],
}));

const folder = { name: 'Streetwear' } as unknown as WorkspaceExplorerNode;

describe('PreviewPanelContextMenu', () => {
  it('portals the action menu above workspace panes and closes it with Escape', () => {
    render(
      <PreviewPanelContextMenu
        folder={folder}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={vi.fn()}
        onEnableOnlyThis={vi.fn()}
        onToggleSafe={vi.fn()}
      />,
    );

    const trigger = screen.getByRole('button', { name: 'actions.more_actions' });
    fireEvent.click(trigger);

    const menu = screen.getByRole('menu');
    expect(menu.closest('.fixed')?.parentElement).toBe(document.body);
    expect(menu.closest('.fixed')).toHaveClass('z-[calc(var(--workspace-layer-overlay)+1)]');
    expect(trigger).toHaveAttribute('aria-expanded', 'true');

    fireEvent.keyDown(menu, { key: 'Escape' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });
});
