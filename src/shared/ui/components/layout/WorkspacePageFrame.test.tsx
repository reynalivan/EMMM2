import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import {
  WorkspaceContextBar,
  WorkspacePageContent,
  WorkspacePageFrame,
} from './WorkspacePageFrame';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children, className }: { children: ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
}));

describe('WorkspacePageFrame', () => {
  it('uses one scroll owner with a topbar-safe content inset', () => {
    render(
      <WorkspacePageFrame>
        <WorkspacePageContent>
          <h1>Workspace content</h1>
        </WorkspacePageContent>
      </WorkspacePageFrame>,
    );

    const heading = screen.getByRole('heading', { name: 'Workspace content' });
    const scrollOwner = heading.closest('.workspace-scroll-owner');

    expect(scrollOwner).toHaveClass('workspace-scroll-owner');
    expect(heading.parentElement).toHaveClass(
      'pt-[calc(var(--workspace-topbar-height)+var(--workspace-context-chrome-height)+1rem)]',
    );
  });

  it('keeps contextual controls below the topbar safe inset', () => {
    render(<WorkspaceContextBar description="Page context" />);

    expect(screen.getByText('Page context').parentElement).toHaveClass(
      'pt-[calc(var(--workspace-topbar-height)+0.75rem)]',
    );
  });
});
