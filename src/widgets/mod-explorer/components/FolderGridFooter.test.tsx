import { render, screen } from '../../../tests/testing/test-utils';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import FolderGridFooter from './FolderGridFooter';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children, liquidRole }: { children: ReactNode; liquidRole: string }) => (
    <div data-liquid-role={liquidRole}>{children}</div>
  ),
}));

describe('FolderGridFooter', () => {
  it('keeps the visible item count out of the action toolbar', () => {
    render(<FolderGridFooter visibleCount={6} />);

    expect(screen.getByText('6 items')).toBeInTheDocument();
    expect(screen.getByTestId('folder-grid-footer')).toHaveClass(
      'pointer-events-none',
      'justify-end',
    );
    expect(document.querySelector('[data-liquid-role="overlay"]')).toBeInTheDocument();
  });
});
